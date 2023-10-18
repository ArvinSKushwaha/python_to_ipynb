use argh::FromArgs;
use ipynb_parse::{
    Author, CellType, KernelSpecification, LanguageInfo, Metadata, Notebook, NotebookCell,
    NotebookMetadata,
};
use std::{
    fs::File,
    io::{BufRead, BufReader, BufWriter},
    path::PathBuf,
    str::FromStr,
};
use thiserror::Error;

/// Convert a python script to an ipynb
#[derive(FromArgs)]
pub struct Python2Ipynb {
    /// output path
    #[argh(option, short = 'o')]
    output_path: Option<PathBuf>,

    /// input path
    #[argh(positional)]
    input_path: PathBuf,

    /// language: `python` or `julia`
    #[argh(option, default = "Language::Python")]
    language: Language,

    /// list of authors
    #[argh(positional, greedy)]
    authors: Vec<String>,
}

#[derive(Default, PartialEq, Eq)]
pub enum Language {
    #[default]
    Python,
    Julia,
}

#[derive(Debug, Error)]
#[error("Attempted to parse {attempted}, but expected 'julia' or 'python'")]
pub struct LanguageParseError {
    attempted: String,
}

impl FromStr for Language {
    type Err = LanguageParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "python" => Ok(Language::Python),
            "julia" => Ok(Language::Julia),
            _ => Err(LanguageParseError {
                attempted: s.to_string(),
            }),
        }
    }
}

impl ToString for Language {
    fn to_string(&self) -> String {
        self.name().to_string()
    }
}

impl Language {
    fn info(&self) -> LanguageInfo {
        LanguageInfo {
            file_extension: self.file_extension(),
            mimetype: self.mimetype(),
            name: self.name(),
        }
    }

    fn file_extension(&self) -> &str {
        match self {
            Language::Python => ".py",
            Language::Julia => ".jl",
        }
    }

    fn mimetype(&self) -> &str {
        match self {
            Language::Python => "application/python",
            Language::Julia => "application/julia",
        }
    }

    fn name(&self) -> &str {
        match self {
            Language::Python => "python",
            Language::Julia => "julia",
        }
    }
}

fn main() -> anyhow::Result<()> {
    let args: Python2Ipynb = argh::from_env();

    let input_path = args.input_path;
    let output_path = args.output_path.unwrap_or(
        format!(
            "./{}.ipynb",
            input_path.file_stem().unwrap().to_string_lossy()
        )
        .into(),
    );

    let input_file = File::open(input_path)?;
    let output_file = File::create(output_path)?;

    let input_buf = BufReader::new(input_file);
    let output_buf = BufWriter::new(output_file);

    let mut notebook = Notebook {
        metadata: NotebookMetadata {
            kernel_spec: KernelSpecification {
                argv: None,
                display_name: "python3",
                language: &args.language.to_string(),
                interrupt_mode: None,
                env: None,
            },
            language_info: Some(args.language.info()),
            authors: args
                .authors
                .into_iter()
                .map(|name| Author { name })
                .collect(),
        },
        nbformat: 4,
        nbformat_minor: 4,
        cells: vec![],
    };

    let mut code_lines = input_buf.lines();
    let mut current_cell_type = None;
    let mut sources = vec![];

    while let Some(line) = code_lines.next() {
        let line = line?;

        if line.starts_with("#%%") {
            let cell_type = current_cell_type.take();
            if let Some(cell_type) = cell_type {
                notebook.cells.push(NotebookCell {
                    cell_type,
                    source: sources.join("\n").trim_matches('\n').to_string(),
                    metadata: Metadata {
                        ..Default::default()
                    },
                });
            }

            sources.clear();

            let cell_type = line.split_at(3).1.trim();
            current_cell_type = CellType::from_str(cell_type).ok();
        } else {
            let line = if current_cell_type != Some(CellType::Code) && line.starts_with("# ") {
                line.split_at(2).1.to_string()
            } else {
                line
            };

            if current_cell_type.is_some() {
                sources.push(line);
            }
        }
    }

    serde_json::to_writer(output_buf, &notebook)?;

    Ok(())
}
