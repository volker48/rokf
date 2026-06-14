use std::io::Read;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};

use crate::check::{self, CheckInput, CheckOutcome};
use crate::traversal;
use crate::verification::{
    Severity, check_index_maintenance, fix_index_maintenance, format_document, format_report_json,
    format_report_with_threshold,
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
        /// Apply safe Fixes for Fixable Findings
        #[arg(long)]
        fix: bool,

        /// Output format for Verification results
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        output: OutputFormat,

        /// Explicit Configuration file
        #[arg(long)]
        config: Option<PathBuf>,

        /// Minimum Finding Severity that causes Verification to fail
        #[arg(long, value_enum)]
        failure_threshold: Option<FailureThreshold>,

        /// Concept Document, explicit Bundle Root, or `-` for stdin.
        /// When omitted, rokf attempts Bundle Discovery from the current directory.
        target: Option<PathBuf>,
    },

    /// Normalize OKF Document presentation
    Format {
        /// Report formatting drift without writing changes
        #[arg(long)]
        check: bool,

        /// OKF Document or `-` for stdin
        target: PathBuf,
    },

    /// Create Document Templates for Assisted Authoring
    Template {
        #[command(subcommand)]
        kind: TemplateKind,
    },

    /// Maintain Index Files for Progressive Disclosure
    Index {
        /// Report Index Maintenance Findings without writing changes
        #[arg(long, conflicts_with = "fix")]
        check: bool,

        /// Apply safe Index Maintenance updates
        #[arg(long)]
        fix: bool,

        /// Bundle Root
        bundle_root: PathBuf,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
}

#[derive(Debug, Subcommand)]
enum TemplateKind {
    /// Create a Concept Document template
    Concept {
        /// Producer-defined Concept Type
        #[arg(long = "type")]
        concept_type: String,

        /// Output path
        path: PathBuf,
    },

    /// Create an Index File template
    Index { path: PathBuf },

    /// Create a Log File template
    Log { path: PathBuf },
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
            fix,
            output,
            config,
            failure_threshold,
        }) => check(target, failure_threshold, config, output, fix),
        Some(Command::Format { check, target }) => format_command(target, check),
        Some(Command::Template { kind }) => template(kind),
        Some(Command::Index {
            check,
            fix,
            bundle_root,
        }) => index(bundle_root, check, fix),
        None => ExitCode::SUCCESS,
    }
}

fn check(
    target: Option<PathBuf>,
    threshold_arg: Option<FailureThreshold>,
    config_path: Option<PathBuf>,
    output: OutputFormat,
    fix: bool,
) -> ExitCode {
    let input = CheckInput {
        target,
        failure_threshold: threshold_arg.map(Severity::from),
        config_path,
        fix,
    };

    match check::run(input) {
        Ok(CheckOutcome::Report {
            report,
            failure_threshold,
        }) => {
            print_report(&report, failure_threshold, output);
            exit_for_report(&report, failure_threshold)
        }
        Ok(CheckOutcome::FixedStdin { contents }) => {
            print!("{contents}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}

fn read_target(target: &std::path::Path) -> Result<String, ExitCode> {
    std::fs::read_to_string(target).map_err(|error| {
        eprintln!("{}: error: {error}", target.display());
        ExitCode::from(2)
    })
}

fn exit_for_report(
    report: &crate::verification::VerificationReport,
    failure_threshold: Severity,
) -> ExitCode {
    if report.passes_failure_threshold(failure_threshold) {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn print_report(
    report: &crate::verification::VerificationReport,
    failure_threshold: Severity,
    output: OutputFormat,
) {
    match output {
        OutputFormat::Text => print!(
            "{}",
            format_report_with_threshold(report, failure_threshold)
        ),
        OutputFormat::Json => print!("{}", format_report_json(report, failure_threshold)),
    }
}

fn format_command(target: PathBuf, check: bool) -> ExitCode {
    if target.as_os_str() == "-" {
        return format_stdin();
    }
    if target.is_dir() {
        return format_directory(&target, check);
    }

    let contents = match std::fs::read_to_string(&target) {
        Ok(contents) => contents,
        Err(error) => {
            eprintln!("{}: error: {error}", target.display());
            return ExitCode::from(2);
        }
    };
    let formatted = format_document(&contents);
    if check {
        return format_check(&target, &contents, &formatted);
    }
    match std::fs::write(&target, formatted) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{}: error: {error}", target.display());
            ExitCode::from(2)
        }
    }
}

fn format_directory(directory: &std::path::Path, check: bool) -> ExitCode {
    let mut drift = false;
    let documents = match traversal::markdown_documents(directory) {
        Ok(documents) => documents,
        Err(error) => {
            eprintln!("{}: error: {error}", directory.display());
            return ExitCode::from(2);
        }
    };
    for document in documents {
        match format_document_path(&document, check) {
            Ok(changed) => drift |= changed,
            Err(code) => return code,
        }
    }
    if check && drift {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn format_document_path(target: &std::path::Path, check: bool) -> Result<bool, ExitCode> {
    let contents = read_target(target)?;
    let formatted = format_document(&contents);
    if check {
        if contents == formatted {
            return Ok(false);
        }
        println!("{}: would reformat", target.display());
        return Ok(true);
    }
    std::fs::write(target, formatted).map_err(|error| {
        eprintln!("{}: error: {error}", target.display());
        ExitCode::from(2)
    })?;
    Ok(false)
}

fn format_stdin() -> ExitCode {
    let mut contents = String::new();
    if let Err(error) = std::io::stdin().read_to_string(&mut contents) {
        eprintln!("<stdin>: error: {error}");
        return ExitCode::from(2);
    }
    print!("{}", format_document(&contents));
    ExitCode::SUCCESS
}

fn format_check(target: &std::path::Path, contents: &str, formatted: &str) -> ExitCode {
    if contents == formatted {
        ExitCode::SUCCESS
    } else {
        println!("{}: would reformat", target.display());
        ExitCode::from(1)
    }
}

fn template(kind: TemplateKind) -> ExitCode {
    let (path, contents) = match kind {
        TemplateKind::Concept { concept_type, path } => {
            (path, format!("---\ntype: {concept_type}\n---\n\n# \n"))
        }
        TemplateKind::Index { path } => (path, "# Index\n".to_string()),
        TemplateKind::Log { path } => (path, "# Directory Update Log\n".to_string()),
    };

    if path.exists() {
        eprintln!(
            "{}: error: refusing to overwrite existing OKF Document",
            path.display()
        );
        return ExitCode::from(2);
    }
    match std::fs::write(&path, contents) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{}: error: {error}", path.display());
            ExitCode::from(2)
        }
    }
}

fn index(bundle_root: PathBuf, check: bool, fix: bool) -> ExitCode {
    let report = if fix || !check {
        fix_index_maintenance(&bundle_root)
    } else {
        check_index_maintenance(&bundle_root)
    };
    let report = match report {
        Ok(report) => report,
        Err(error) => {
            eprintln!("{}: error: {error}", bundle_root.display());
            return ExitCode::from(2);
        }
    };
    print!(
        "{}",
        format_report_with_threshold(&report, Severity::Warning)
    );
    if report.findings.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
