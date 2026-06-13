use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use crate::verification::{format_report, verify_bundle_root, verify_concept_document};

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
        /// Concept Document or explicit Bundle Root to verify
        target: PathBuf,
    },
}

pub fn run() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Some(Command::Check { target }) => check(target),
        None => ExitCode::SUCCESS,
    }
}

fn check(target: PathBuf) -> ExitCode {
    let report = if target.is_dir() {
        match verify_bundle_root(&target) {
            Ok(report) => report,
            Err(error) => {
                eprintln!("{}: error: {error}", target.display());
                return ExitCode::from(2);
            }
        }
    } else {
        let contents = match std::fs::read_to_string(&target) {
            Ok(contents) => contents,
            Err(error) => {
                eprintln!("{}: error: {error}", target.display());
                return ExitCode::from(2);
            }
        };
        verify_concept_document(&target, &contents)
    };

    print!("{}", format_report(&report));

    if report.is_success() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
