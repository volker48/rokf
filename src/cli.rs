use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use crate::verification::{format_report, verify_concept_document};

#[derive(Debug, Parser)]
#[command(
    name = "rokf",
    version,
    about = "Create, inspect, and maintain Open Knowledge Format knowledge bundles"
)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Verify OKF content and report Findings
    Check {
        /// Concept Document to verify
        concept_document: PathBuf,
    },
}

pub fn run() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Some(Command::Check { concept_document }) => check(concept_document),
        None => ExitCode::SUCCESS,
    }
}

fn check(concept_document: PathBuf) -> ExitCode {
    let contents = match std::fs::read_to_string(&concept_document) {
        Ok(contents) => contents,
        Err(error) => {
            eprintln!("{}: error: {error}", concept_document.display());
            return ExitCode::from(2);
        }
    };

    let report = verify_concept_document(&concept_document, &contents);
    print!("{}", format_report(&report));

    if report.is_success() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
