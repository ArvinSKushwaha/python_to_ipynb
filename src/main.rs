use argh::FromArgs;
use ipynb_parse::{
    Author, CellType, KernelSpecification, Metadata, Notebook, NotebookCell, NotebookMetadata,
};
use std::{
    fs::File,
    io::{BufRead, BufReader, BufWriter},
    path::PathBuf,
    str::FromStr,
};

/// Convert a python script to an ipynb
#[derive(FromArgs)]
pub struct Python2Ipynb {
    /// output path
    #[argh(option)]
    output_path: Option<PathBuf>,

    /// input path
    #[argh(positional)]
    input_path: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let args: Python2Ipynb = argh::from_env();
    // The line #%% is special

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
                argv: vec!["python", "-m", "IPython.kernel", "-f", "{connection_file}"],
                display_name: "python3",
                language: "python",
                interrupt_mode: None,
                env: None,
            },
            authors: vec![Author {
                name: "Arvin Kushwaha",
            }],
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
                    source: sources.join("\n"),
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
                line.split_at(2).1.to_owned()
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
