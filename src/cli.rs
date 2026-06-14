use std::io::Read;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};

use crate::verification::{
    RuleSet, Severity, VerificationOptions, check_index_maintenance, discover_bundle_root,
    fix_document, fix_index_maintenance, format_document, format_report_json,
    format_report_with_threshold, verify_bundle_root_with_options, verify_concept_document,
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
    let target = match target {
        Some(target) => target,
        None => match discover_bundle_root() {
            Some(root) => root,
            None => {
                eprintln!(
                    "rokf check: error: could not discover a Bundle Root from the current directory"
                );
                return ExitCode::from(2);
            }
        },
    };
    let config = match load_config(config_path.as_deref(), &target) {
        Ok(config) => config,
        Err(code) => return code,
    };
    let failure_threshold = threshold_arg
        .map(Severity::from)
        .or(config.failure_threshold)
        .unwrap_or(Severity::Warning);

    if target.as_os_str() == "-" {
        return check_stdin(failure_threshold, output, fix);
    }

    let report = if target.is_dir() {
        match check_bundle(&target, &config.options) {
            Ok(report) => report,
            Err(code) => return code,
        }
    } else {
        match check_document(&target, fix) {
            Ok(report) => report,
            Err(code) => return code,
        }
    };

    print_report(&report, failure_threshold, output);

    exit_for_report(&report, failure_threshold)
}

fn check_stdin(failure_threshold: Severity, output: OutputFormat, fix: bool) -> ExitCode {
    let mut contents = String::new();
    if let Err(error) = std::io::stdin().read_to_string(&mut contents) {
        eprintln!("<stdin>: error: {error}");
        return ExitCode::from(2);
    }

    if fix {
        print!("{}", fix_document(&contents));
        return ExitCode::SUCCESS;
    }

    let report = verify_concept_document(std::path::Path::new("<stdin>"), &contents);
    print_report(&report, failure_threshold, output);
    exit_for_report(&report, failure_threshold)
}

fn check_bundle(
    target: &std::path::Path,
    options: &VerificationOptions,
) -> Result<crate::verification::VerificationReport, ExitCode> {
    verify_bundle_root_with_options(target, options).map_err(|error| {
        eprintln!("{}: error: {error}", target.display());
        ExitCode::from(2)
    })
}

fn check_document(
    target: &std::path::Path,
    fix: bool,
) -> Result<crate::verification::VerificationReport, ExitCode> {
    let contents = read_target(target)?;
    if fix {
        std::fs::write(target, fix_document(&contents)).map_err(|error| {
            eprintln!("{}: error: {error}", target.display());
            ExitCode::from(2)
        })?;
        return Ok(verify_concept_document(target, &read_target(target)?));
    }
    Ok(verify_concept_document(target, &contents))
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

#[derive(Default)]
struct Config {
    failure_threshold: Option<Severity>,
    options: VerificationOptions,
}

fn load_config(
    config_path: Option<&std::path::Path>,
    target: &std::path::Path,
) -> Result<Config, ExitCode> {
    let path = config_path
        .map(PathBuf::from)
        .or_else(|| discover_config(target));
    let Some(path) = path else {
        return Ok(Config::default());
    };
    let contents = std::fs::read_to_string(&path).map_err(|error| {
        eprintln!("{}: error: {error}", path.display());
        ExitCode::from(2)
    })?;
    parse_config(&path, &contents)
}

fn discover_config(target: &std::path::Path) -> Option<PathBuf> {
    let start = if target.is_dir() {
        target
    } else {
        target.parent()?
    };
    let config = start.join("rokf.yml");
    config.is_file().then_some(config)
}

fn parse_config(path: &std::path::Path, contents: &str) -> Result<Config, ExitCode> {
    let mapping = serde_yaml::from_str::<serde_yaml::Mapping>(contents).map_err(|error| {
        eprintln!(
            "{}: error: Configuration must be parseable YAML: {error}",
            path.display()
        );
        ExitCode::from(2)
    })?;
    Ok(Config {
        failure_threshold: config_string(&mapping, "failure_threshold").and_then(parse_severity),
        options: VerificationOptions {
            rule_set: parse_rule_set(config_string(&mapping, "rule_set").as_deref()),
            suppressions: config_list(&mapping, "suppressions"),
            exclusions: config_list(&mapping, "exclusions"),
        },
    })
}

fn config_string(mapping: &serde_yaml::Mapping, key: &str) -> Option<String> {
    let key = serde_yaml::Value::String(key.to_string());
    mapping
        .get(&key)
        .and_then(serde_yaml::Value::as_str)
        .map(str::to_string)
}

fn config_list(mapping: &serde_yaml::Mapping, key: &str) -> Vec<String> {
    let key = serde_yaml::Value::String(key.to_string());
    mapping
        .get(&key)
        .and_then(serde_yaml::Value::as_sequence)
        .into_iter()
        .flatten()
        .filter_map(serde_yaml::Value::as_str)
        .map(str::to_string)
        .collect()
}

fn parse_severity(value: String) -> Option<Severity> {
    match value.as_str() {
        "error" => Some(Severity::Error),
        "warning" => Some(Severity::Warning),
        "suggestion" => Some(Severity::Suggestion),
        _ => None,
    }
}

fn parse_rule_set(value: Option<&str>) -> RuleSet {
    match value {
        Some("conformance") => RuleSet::Conformance,
        _ => RuleSet::Default,
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
    let mut documents = Vec::new();
    if let Err(error) = collect_markdown_documents(directory, &mut documents) {
        eprintln!("{}: error: {error}", directory.display());
        return ExitCode::from(2);
    }
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

fn collect_markdown_documents(
    directory: &std::path::Path,
    documents: &mut Vec<PathBuf>,
) -> std::io::Result<()> {
    let mut entries = std::fs::read_dir(directory)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.path());
    for entry in entries {
        if entry.file_type()?.is_dir() {
            collect_markdown_documents(&entry.path(), documents)?;
        } else if entry
            .path()
            .extension()
            .is_some_and(|extension| extension == "md")
        {
            documents.push(entry.path());
        }
    }
    Ok(())
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
