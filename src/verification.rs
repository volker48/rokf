use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Severity {
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
        self.findings.is_empty()
    }
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
    let has_type = parsed_frontmatter
        .get(&type_key)
        .and_then(serde_yaml::Value::as_str)
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false);

    if !has_type {
        findings.push(Finding {
            rule_code: "OKF002",
            severity: Severity::Error,
            message: "Concept Document Frontmatter must include a Concept Type".to_string(),
            document,
            line: Some(2),
            column: Some(1),
        });
    }

    VerificationReport { findings }
}

pub fn format_report(report: &VerificationReport) -> String {
    if report.findings.is_empty() {
        return "OK\n".to_string();
    }

    let mut output = String::new();
    for finding in &report.findings {
        let severity = match finding.severity {
            Severity::Error => "error",
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
