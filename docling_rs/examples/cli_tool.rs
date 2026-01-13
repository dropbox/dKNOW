//! Complete CLI Tool Example
//!
//! This example demonstrates how to build a complete command-line application
//! with subcommands, argument parsing, and error handling.
//!
//! Run with:
//! ```bash
//! cargo run --example cli_tool -- convert document.pdf
//! cargo run --example cli_tool -- batch *.pdf --output ./output
//! cargo run --example cli_tool -- info document.pdf
//! ```

use docling_backend::DocumentConverter;
use docling_core::Result;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
enum Command {
    Convert {
        input: PathBuf,
        output: Option<PathBuf>,
        ocr: bool,
    },
    Batch {
        inputs: Vec<PathBuf>,
        output_dir: PathBuf,
        ocr: bool,
    },
    Info {
        input: PathBuf,
    },
    Help,
}

fn main() {
    // Parse command-line arguments
    let command = match parse_args() {
        Ok(cmd) => cmd,
        Err(e) => {
            eprintln!("Error: {e}");
            print_usage();
            std::process::exit(1);
        }
    };

    // Execute command
    let result = match command {
        Command::Convert { input, output, ocr } => cmd_convert(&input, output.as_deref(), ocr),
        Command::Batch {
            inputs,
            output_dir,
            ocr,
        } => cmd_batch(&inputs, &output_dir, ocr),
        Command::Info { input } => cmd_info(&input),
        Command::Help => {
            print_usage();
            Ok(())
        }
    };

    // Handle result
    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

/// Parse command-line arguments into a Command
fn parse_args() -> std::result::Result<Command, String> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        return Ok(Command::Help);
    }

    let subcommand = &args[1];

    match subcommand.as_str() {
        "convert" => {
            if args.len() < 3 {
                return Err("convert requires an input file".to_string());
            }

            let input = PathBuf::from(&args[2]);
            let mut output = None;
            let mut ocr = false;

            // Parse optional flags
            let mut i = 3;
            while i < args.len() {
                match args[i].as_str() {
                    "-o" | "--output" => {
                        if i + 1 < args.len() {
                            output = Some(PathBuf::from(&args[i + 1]));
                            i += 2;
                        } else {
                            return Err("--output requires a path".to_string());
                        }
                    }
                    "--ocr" => {
                        ocr = true;
                        i += 1;
                    }
                    _ => {
                        return Err(format!("Unknown flag: {}", args[i]));
                    }
                }
            }

            Ok(Command::Convert { input, output, ocr })
        }

        "batch" => {
            if args.len() < 3 {
                return Err("batch requires input files".to_string());
            }

            let mut inputs = Vec::new();
            let mut output_dir = None;
            let mut ocr = false;

            let mut i = 2;
            while i < args.len() {
                match args[i].as_str() {
                    "-o" | "--output" => {
                        if i + 1 < args.len() {
                            output_dir = Some(PathBuf::from(&args[i + 1]));
                            i += 2;
                        } else {
                            return Err("--output requires a path".to_string());
                        }
                    }
                    "--ocr" => {
                        ocr = true;
                        i += 1;
                    }
                    arg if arg.starts_with('-') => {
                        return Err(format!("Unknown flag: {arg}"));
                    }
                    _ => {
                        inputs.push(PathBuf::from(&args[i]));
                        i += 1;
                    }
                }
            }

            let output_dir = output_dir.ok_or("batch requires --output directory")?;

            if inputs.is_empty() {
                return Err("batch requires at least one input file".to_string());
            }

            Ok(Command::Batch {
                inputs,
                output_dir,
                ocr,
            })
        }

        "info" => {
            if args.len() < 3 {
                return Err("info requires an input file".to_string());
            }

            let input = PathBuf::from(&args[2]);
            Ok(Command::Info { input })
        }

        "help" | "-h" | "--help" => Ok(Command::Help),

        _ => Err(format!("Unknown subcommand: {subcommand}")),
    }
}

/// Convert a single document
fn cmd_convert(input: &Path, output: Option<&Path>, ocr: bool) -> Result<()> {
    println!("Converting: {}", input.display());

    let converter = if ocr {
        println!("OCR enabled");
        DocumentConverter::with_ocr(true)?
    } else {
        DocumentConverter::new()?
    };

    let result = converter.convert(input)?;

    let markdown = &result.document.markdown;

    if let Some(output_path) = output {
        fs::write(output_path, markdown)?;
        println!("✓ Saved to: {}", output_path.display());
    } else {
        println!("{markdown}");
    }

    println!();
    println!("Conversion completed in {:?}", result.latency);
    println!("Characters: {:?}", result.document.metadata.num_characters);

    Ok(())
}

/// Convert multiple documents in batch
fn cmd_batch(inputs: &[PathBuf], output_dir: &Path, ocr: bool) -> Result<()> {
    println!("Batch converting {} documents", inputs.len());
    println!("Output directory: {}", output_dir.display());
    println!();

    fs::create_dir_all(output_dir)?;

    let converter = if ocr {
        println!("OCR enabled");
        DocumentConverter::with_ocr(true)?
    } else {
        DocumentConverter::new()?
    };

    let mut success = 0;
    let mut failed = 0;

    for (i, input) in inputs.iter().enumerate() {
        print!("[{}/{}] {}... ", i + 1, inputs.len(), input.display());

        match converter.convert(input) {
            Ok(result) => {
                let output_filename =
                    input.file_stem().unwrap().to_string_lossy().to_string() + ".md";
                let output_path = output_dir.join(&output_filename);

                if let Err(e) = fs::write(&output_path, &result.document.markdown) {
                    println!("✗ Write failed: {e}");
                    failed += 1;
                } else {
                    println!("✓ {} chars", result.document.metadata.num_characters);
                    success += 1;
                }
            }
            Err(e) => {
                println!("✗ {e}");
                failed += 1;
            }
        }
    }

    println!();
    println!("Batch conversion complete");
    println!("Success: {success}");
    println!("Failed: {failed}");

    Ok(())
}

/// Display document information
fn cmd_info(input: &Path) -> Result<()> {
    println!("Document Information: {}", input.display());
    println!();

    let converter = DocumentConverter::new()?;
    let result = converter.convert(input)?;

    println!(
        "Pages: {:?}",
        result.document.metadata.num_pages.unwrap_or(0)
    );
    println!("Characters: {:?}", result.document.metadata.num_characters);
    println!("Conversion time: {:?}", result.latency);
    println!();

    let markdown = &result.document.markdown;
    println!("Content Preview (first 500 characters):");
    println!("{}", "-".repeat(80));
    if markdown.len() > 500 {
        println!("{}...", &markdown[..500]);
    } else {
        println!("{markdown}");
    }
    println!("{}", "-".repeat(80));

    Ok(())
}

/// Print usage information
fn print_usage() {
    println!("docling-cli - Document Conversion Tool");
    println!();
    println!("USAGE:");
    println!("    cli_tool <COMMAND> [OPTIONS]");
    println!();
    println!("COMMANDS:");
    println!("    convert <INPUT>              Convert a single document");
    println!("    batch <INPUTS...>            Convert multiple documents");
    println!("    info <INPUT>                 Display document information");
    println!("    help                         Show this help message");
    println!();
    println!("CONVERT OPTIONS:");
    println!("    -o, --output <FILE>          Output file (default: stdout)");
    println!("    --ocr                        Enable OCR for scanned documents");
    println!();
    println!("BATCH OPTIONS:");
    println!("    -o, --output <DIR>           Output directory (required)");
    println!("    --ocr                        Enable OCR for scanned documents");
    println!();
    println!("EXAMPLES:");
    println!("    cli_tool convert document.pdf");
    println!("    cli_tool convert scanned.pdf --ocr -o output.md");
    println!("    cli_tool batch *.pdf --output ./converted");
    println!("    cli_tool info report.docx");
}
