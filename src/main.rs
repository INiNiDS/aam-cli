// SPDX-FileCopyrightText: 2026 Nikita Goncharov
// SPDX-License-Identifier: GPL-3.0-or-later
//
// Ported from APACHE 2.0
#![deny(clippy::pedantic, clippy::nursery, clippy::cargo)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::option_if_let_else)]
#![allow(clippy::cargo_common_metadata)]
#![allow(clippy::multiple_crate_versions)]

pub mod lsp;
pub mod tui;
pub mod utils;

use aam_rs::aam::AAM;
use aam_rs::pipeline::FormattingOptions;
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;
use utils::strip_ansi_codes;

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

fn run_check(file: &std::path::Path) {
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
        }
        Err(errors) => {
            eprintln!("✗ Errors in file {}:", file.display());
            for err in &errors {
                eprintln!("  {}", strip_ansi_codes(&err.to_string()));
            }
            std::process::exit(1);
        }
    }
}

fn run_format(file: &std::path::Path, dry_run: bool) -> Result<()> {
    let content = fs::read_to_string(file)
        .with_context(|| format!("Failed to read file: {}", file.display()))?;

    let aam = AAM::load(file).map_err(|errors| {
        anyhow::anyhow!(
            "File contains parsing errors:\n{}",
            errors
                .iter()
                .map(|e| strip_ansi_codes(&e.to_string()))
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

fn run_get(file: &std::path::Path, key: &str) -> Result<()> {
    let content = fs::read_to_string(file)
        .with_context(|| format!("Failed to read file: {}", file.display()))?;

    let aam = AAM::load(&content).map_err(|errors| {
        anyhow::anyhow!(
            "File contains parsing errors:\n{}",
            errors
                .iter()
                .map(|e| strip_ansi_codes(&e.to_string()))
                .collect::<Vec<_>>()
                .join("\n")
        )
    })?;

    if let Some(value) = aam.get(key) {
        println!("{value}");
        Ok(())
    } else {
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
                eprintln!("    - {k}");
            }
        }
        std::process::exit(1);
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Check { file }) => {
            run_check(&file);
            Ok(())
        }
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
