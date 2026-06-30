use std::io::Read;
use std::path::{Path, PathBuf};

use crate::traversal;
use crate::verification::{
    RuleSet, Severity, VerificationOptions, VerificationReport, filter_report, fix_document,
    verify_bundle_root_with_options, verify_concept_document,
};

pub(crate) struct CheckInput {
    pub(crate) target: Option<PathBuf>,
    pub(crate) failure_threshold: Option<Severity>,
    pub(crate) config_path: Option<PathBuf>,
    pub(crate) fix: bool,
}

pub(crate) enum CheckOutcome {
    Report {
        report: VerificationReport,
        failure_threshold: Severity,
    },
    FixedStdin {
        contents: String,
    },
}

#[derive(Debug)]
pub(crate) struct CheckError {
    message: String,
}

impl CheckError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    fn io(path: &Path, error: std::io::Error) -> Self {
        Self::new(format!("{}: error: {error}", path.display()))
    }
}

impl std::fmt::Display for CheckError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for CheckError {}

#[derive(Default)]
struct Config {
    failure_threshold: Option<Severity>,
    options: VerificationOptions,
}

pub(crate) fn run(input: CheckInput) -> Result<CheckOutcome, CheckError> {
    let target = resolve_target(input.target)?;
    let config = load_config(input.config_path.as_deref(), &target)?;
    let failure_threshold = input
        .failure_threshold
        .or(config.failure_threshold)
        .unwrap_or(Severity::Warning);

    if target.as_os_str() == "-" {
        return check_stdin(failure_threshold, &config.options, input.fix);
    }

    let report = if target.is_dir() {
        check_bundle(&target, &config.options)?
    } else {
        check_document(&target, &config.options, input.fix)?
    };

    Ok(CheckOutcome::Report {
        report,
        failure_threshold,
    })
}

fn resolve_target(target: Option<PathBuf>) -> Result<PathBuf, CheckError> {
    match target {
        Some(target) => Ok(target),
        None => traversal::discover_bundle_root().ok_or_else(|| {
            CheckError::new(
                "rokf check: error: could not discover a Bundle Root from the current directory",
            )
        }),
    }
}

fn check_stdin(
    failure_threshold: Severity,
    options: &VerificationOptions,
    fix: bool,
) -> Result<CheckOutcome, CheckError> {
    let mut contents = String::new();
    std::io::stdin()
        .read_to_string(&mut contents)
        .map_err(|error| CheckError::new(format!("<stdin>: error: {error}")))?;

    if fix {
        return Ok(CheckOutcome::FixedStdin {
            contents: fix_document(&contents),
        });
    }

    Ok(CheckOutcome::Report {
        report: filter_report(
            verify_concept_document(Path::new("<stdin>"), &contents),
            options,
        ),
        failure_threshold,
    })
}

fn check_bundle(
    target: &Path,
    options: &VerificationOptions,
) -> Result<VerificationReport, CheckError> {
    verify_bundle_root_with_options(target, options).map_err(|error| CheckError::io(target, error))
}

fn check_document(
    target: &Path,
    options: &VerificationOptions,
    fix: bool,
) -> Result<VerificationReport, CheckError> {
    let contents = read_target(target)?;
    if fix {
        std::fs::write(target, fix_document(&contents))
            .map_err(|error| CheckError::io(target, error))?;
        return Ok(filter_report(
            verify_concept_document(target, &read_target(target)?),
            options,
        ));
    }
    Ok(filter_report(
        verify_concept_document(target, &contents),
        options,
    ))
}

fn read_target(target: &Path) -> Result<String, CheckError> {
    std::fs::read_to_string(target).map_err(|error| CheckError::io(target, error))
}

fn load_config(config_path: Option<&Path>, target: &Path) -> Result<Config, CheckError> {
    let path = config_path
        .map(PathBuf::from)
        .or_else(|| discover_config(target));
    let Some(path) = path else {
        return Ok(Config::default());
    };
    let contents = std::fs::read_to_string(&path).map_err(|error| CheckError::io(&path, error))?;
    parse_config(&path, &contents)
}

fn discover_config(target: &Path) -> Option<PathBuf> {
    let start = if target.is_dir() {
        target
    } else {
        target.parent()?
    };
    let config = start.join("rokf.yml");
    config.is_file().then_some(config)
}

fn parse_config(path: &Path, contents: &str) -> Result<Config, CheckError> {
    let mapping = serde_yaml::from_str::<serde_yaml::Mapping>(contents).map_err(|error| {
        CheckError::new(format!(
            "{}: error: Configuration must be parseable YAML: {error}",
            path.display()
        ))
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
