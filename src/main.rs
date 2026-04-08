pub mod lsp;
pub mod tui;

use aam_rs::aam::AAM;
use aam_rs::pipeline::FormattingOptions;
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser)]
#[command(name = "aam")]
#[command(author = "INiNiDS")]
#[command(version = VERSION)]
#[command(about = "CLI for working with AAM files", long_about = None)]
struct Cli {
    /// Optional file(s) to open in TUI mode (default)
    #[arg(value_name = "FILE")]
    files: Vec<PathBuf>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Check an AAM file for errors
    Check {
        /// Path to the AAM file
        file: PathBuf,
    },
    /// Format an AAM file (writes to file by default)
    Format {
        /// Path to the AAM file
        file: PathBuf,
        /// Only show formatted output without writing (dry run)
        #[arg(long)]
        dry_run: bool,
    },
    /// Get a value by key from an AAM file
    Get {
        /// Path to the AAM file
        file: PathBuf,
        /// Key to look up
        key: String,
    },
    /// Run the LSP server
    Lsp,
}

fn run_check(file: &PathBuf) -> Result<()> {
    match AAM::load(file) {
        Ok(aam) => {
            println!("✓ File {} is valid", file.display());
            println!("  Found {} key(s)", aam.keys().len());
            if let Some(schemas) = aam.schemas() {
                println!("  Found {} schema(s)", schemas.len());
            }
            if let Some(types) = aam.types() {
                println!("  Found {} type(s)", types.len());
            }
            Ok(())
        }
        Err(errors) => {
            eprintln!("✗ Errors in file {}:", file.display());
            for err in &errors {
                eprintln!("  {err}");
            }
            std::process::exit(1);
        }
    }
}

fn run_format(file: &PathBuf, dry_run: bool) -> Result<()> {
    let content = fs::read_to_string(file)
        .with_context(|| format!("Failed to read file: {}", file.display()))?;

    let aam = AAM::load(file).map_err(|errors| {
        anyhow::anyhow!(
            "File contains parsing errors:\n{}",
            errors
                .iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join("\n")
        )
    })?;

    let formatted = aam.format(&content, &FormattingOptions::default())?;

    if dry_run {
        print!("{formatted}");
    } else {
        fs::write(file, &formatted)
            .with_context(|| format!("Failed to write to file: {}", file.display()))?;
        println!("✓ File {} formatted", file.display());
    }

    Ok(())
}

fn run_get(file: &PathBuf, key: &str) -> Result<()> {
    let content = fs::read_to_string(file)
        .with_context(|| format!("Failed to read file: {}", file.display()))?;

    let aam = AAM::load(&content).map_err(|errors| {
        anyhow::anyhow!(
            "File contains parsing errors:\n{}",
            errors
                .iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join("\n")
        )
    })?;

    match aam.get(key) {
        Some(value) => {
            println!("{}", value);
            Ok(())
        }
        None => {
            eprintln!("✗ Key '{}' not found in file {}", key, file.display());
            // Try to find similar keys
            let similar: Vec<&str> = aam
                .keys()
                .iter()
                .filter(|k| k.contains(key) || key.contains(*k))
                .copied()
                .collect();
            if !similar.is_empty() {
                eprintln!("  Did you mean:");
                for k in similar.iter().take(5) {
                    eprintln!("    - {}", k);
                }
            }
            std::process::exit(1);
        }
    }
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Check { file }) => run_check(&file),
        Some(Commands::Format { file, dry_run }) => run_format(&file, dry_run),
        Some(Commands::Get { file, key }) => run_get(&file, &key),
        Some(Commands::Lsp) => Ok(lsp::run_lsp()?),
        None => {
            // No subcommand - open TUI with optional files
            if cli.files.is_empty() {
                tui::run_tui(None)
            } else {
                tui::run_tui(Some(&cli.files))
            }
        }
    }
}
