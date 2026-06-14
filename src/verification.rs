use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Suggestion,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub rule_code: &'static str,
    pub severity: Severity,
    pub message: String,
    pub document: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationReport {
    pub findings: Vec<Finding>,
}

impl VerificationReport {
    pub fn is_success(&self) -> bool {
        self.passes_failure_threshold(Severity::Warning)
    }

    pub fn passes_failure_threshold(&self, failure_threshold: Severity) -> bool {
        !self
            .findings
            .iter()
            .any(|finding| finding.severity >= failure_threshold)
    }

    pub fn is_conformant(&self) -> bool {
        !self
            .findings
            .iter()
            .any(|finding| finding.severity == Severity::Error)
    }

    pub fn is_healthy(&self) -> bool {
        self.is_healthy_at(Severity::Warning)
    }

    pub fn is_healthy_at(&self, failure_threshold: Severity) -> bool {
        self.is_conformant()
            && !self.findings.iter().any(|finding| {
                finding.severity != Severity::Error && finding.severity >= failure_threshold
            })
    }

    fn extend(&mut self, other: VerificationReport) {
        self.findings.extend(other.findings);
    }
}

pub fn discover_bundle_root() -> Option<PathBuf> {
    let mut current = std::env::current_dir().ok()?;

    loop {
        if current.join("index.md").is_file() {
            return Some(current);
        }
        match current.parent() {
            Some(parent) => current = parent.to_path_buf(),
            None => return None,
        }
    }
}

pub fn verify_bundle_root(bundle_root: &Path) -> std::io::Result<VerificationReport> {
    let mut documents = Vec::new();
    collect_okf_documents(bundle_root, &mut documents)?;
    documents.sort();

    let mut report = VerificationReport {
        findings: Vec::new(),
    };

    for document in documents {
        if is_reserved_file(&document) {
            continue;
        }

        let contents = std::fs::read_to_string(&document)?;
        report.extend(verify_concept_document(&document, &contents));
    }

    Ok(report)
}

fn collect_okf_documents(directory: &Path, documents: &mut Vec<PathBuf>) -> std::io::Result<()> {
    let mut entries = std::fs::read_dir(directory)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            collect_okf_documents(&path, documents)?;
        } else if file_type.is_file() && path.extension().is_some_and(|extension| extension == "md")
        {
            documents.push(path);
        }
    }

    Ok(())
}

fn is_reserved_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|file_name| file_name.to_str())
        .is_some_and(|file_name| matches!(file_name, "index.md" | "log.md"))
}

pub fn verify_concept_document(path: &Path, contents: &str) -> VerificationReport {
    let document = path.display().to_string();
    let mut findings = Vec::new();

    if !contents.starts_with("---\n") && contents.trim() != "---" {
        findings.push(Finding {
            rule_code: "OKF001",
            severity: Severity::Error,
            message: "Concept Document must start with Frontmatter delimited by ---".to_string(),
            document,
            line: Some(1),
            column: Some(1),
        });
        return VerificationReport { findings };
    }

    let Some(frontmatter_end) = contents[4..].find("\n---") else {
        findings.push(Finding {
            rule_code: "OKF001",
            severity: Severity::Error,
            message: "Concept Document Frontmatter must have a closing --- delimiter".to_string(),
            document,
            line: Some(1),
            column: Some(1),
        });
        return VerificationReport { findings };
    };

    let frontmatter = &contents[4..4 + frontmatter_end];
    let parsed_frontmatter = match serde_yaml::from_str::<serde_yaml::Mapping>(frontmatter) {
        Ok(frontmatter) => frontmatter,
        Err(error) => {
            let location = error.location();
            findings.push(Finding {
                rule_code: "OKF001",
                severity: Severity::Error,
                message: format!("Concept Document Frontmatter must be parseable YAML: {error}"),
                document,
                line: location.as_ref().map(|location| location.line()),
                column: location.as_ref().map(|location| location.column()),
            });
            return VerificationReport { findings };
        }
    };

    let type_key = serde_yaml::Value::String("type".to_string());
    let has_type = has_non_empty_string_field(&parsed_frontmatter, &type_key);

    if !has_type {
        findings.push(Finding {
            rule_code: "OKF002",
            severity: Severity::Error,
            message: "Concept Document Frontmatter must include a Concept Type".to_string(),
            document: document.clone(),
            line: Some(2),
            column: Some(1),
        });
    }

    let description_key = serde_yaml::Value::String("description".to_string());
    if !has_non_empty_string_field(&parsed_frontmatter, &description_key) {
        findings.push(Finding {
            rule_code: "OKF101",
            severity: Severity::Warning,
            message: "Concept Document Frontmatter should include a Description".to_string(),
            document,
            line: Some(2),
            column: Some(1),
        });
    }

    VerificationReport { findings }
}

fn has_non_empty_string_field(frontmatter: &serde_yaml::Mapping, key: &serde_yaml::Value) -> bool {
    frontmatter
        .get(key)
        .and_then(serde_yaml::Value::as_str)
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
}

pub fn format_report(report: &VerificationReport) -> String {
    format_report_with_threshold(report, Severity::Warning)
}

pub fn format_report_with_threshold(
    report: &VerificationReport,
    failure_threshold: Severity,
) -> String {
    let mut output = format!(
        "conformant: {}\nhealthy: {}\n",
        yes_no(report.is_conformant()),
        yes_no(report.is_healthy_at(failure_threshold))
    );

    if report.findings.is_empty() {
        output.push_str("OK\n");
        return output;
    }

    for finding in &report.findings {
        let severity = match finding.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Suggestion => "suggestion",
        };
        let location = match (finding.line, finding.column) {
            (Some(line), Some(column)) => format!(":{line}:{column}"),
            (Some(line), None) => format!(":{line}"),
            _ => String::new(),
        };
        output.push_str(&format!(
            "{}{}: {} [{}] {}\n",
            finding.document, location, severity, finding.rule_code, finding.message
        ));
    }
    output
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}
