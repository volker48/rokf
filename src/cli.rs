use std::io::Read;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};

use crate::verification::{
    Severity, format_report_with_threshold, verify_bundle_root, verify_concept_document,
};

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
        /// Minimum Finding Severity that causes Verification to fail
        #[arg(long, value_enum, default_value_t = FailureThreshold::Warning)]
        failure_threshold: FailureThreshold,

        /// Concept Document or explicit Bundle Root to verify
        target: PathBuf,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum FailureThreshold {
    Error,
    Warning,
    Suggestion,
}

impl From<FailureThreshold> for Severity {
    fn from(value: FailureThreshold) -> Self {
        match value {
            FailureThreshold::Error => Severity::Error,
            FailureThreshold::Warning => Severity::Warning,
            FailureThreshold::Suggestion => Severity::Suggestion,
        }
    }
}

pub fn run() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Some(Command::Check {
            target,
            failure_threshold,
        }) => check(target, failure_threshold.into()),
        None => ExitCode::SUCCESS,
    }
}

fn check(target: PathBuf, failure_threshold: Severity) -> ExitCode {
    if target.as_os_str() == "-" {
        let mut contents = String::new();
        if let Err(error) = std::io::stdin().read_to_string(&mut contents) {
            eprintln!("<stdin>: error: {error}");
            return ExitCode::from(2);
        }

        let report = verify_concept_document(std::path::Path::new("<stdin>"), &contents);
        print!(
            "{}",
            format_report_with_threshold(&report, failure_threshold)
        );
        return if report.passes_failure_threshold(failure_threshold) {
            ExitCode::SUCCESS
        } else {
            ExitCode::from(1)
        };
    }

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

    print!(
        "{}",
        format_report_with_threshold(&report, failure_threshold)
    );

    if report.passes_failure_threshold(failure_threshold) {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
