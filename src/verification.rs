use std::path::Path;

use crate::traversal::{self, BundleTraversal, OkfDocument, OkfDocumentKind};

mod rules;

use rules::Rule;

pub const KNOWN_OKF_VERSION: &str = "0.1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Suggestion,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    rule: Rule,
    message: String,
    document: String,
    line: Option<usize>,
    column: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationReport {
    pub findings: Vec<Finding>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationOptions {
    pub rule_set: RuleSet,
    pub suppressions: Vec<String>,
    pub exclusions: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleSet {
    Default,
    Conformance,
}

impl Default for VerificationOptions {
    fn default() -> Self {
        Self {
            rule_set: RuleSet::Default,
            suppressions: Vec::new(),
            exclusions: Vec::new(),
        }
    }
}

impl VerificationReport {
    pub fn is_success(&self) -> bool {
        self.passes_failure_threshold(Severity::Warning)
    }

    pub fn passes_failure_threshold(&self, failure_threshold: Severity) -> bool {
        !self
            .findings
            .iter()
            .any(|finding| finding.severity() >= failure_threshold)
    }

    pub fn is_conformant(&self) -> bool {
        !self.findings.iter().any(Finding::is_conformance_rule)
    }

    pub fn is_healthy(&self) -> bool {
        self.is_healthy_at(Severity::Warning)
    }

    pub fn is_healthy_at(&self, failure_threshold: Severity) -> bool {
        self.is_conformant()
            && !self.findings.iter().any(|finding| {
                !finding.is_conformance_rule() && finding.severity() >= failure_threshold
            })
    }

    pub fn extend(&mut self, other: VerificationReport) {
        self.findings.extend(other.findings);
    }
}

impl Finding {
    fn rule_code(&self) -> &'static str {
        self.rule.code()
    }

    fn severity(&self) -> Severity {
        self.rule.severity()
    }

    fn is_conformance_rule(&self) -> bool {
        self.rule.is_conformance_rule()
    }

    fn is_suppressed_by(&self, suppressions: &[String]) -> bool {
        suppressions.iter().any(|rule| rule == self.rule_code())
    }
}

pub fn verify_bundle_root(bundle_root: &Path) -> std::io::Result<VerificationReport> {
    verify_bundle_root_with_options(bundle_root, &VerificationOptions::default())
}

pub fn verify_bundle_root_with_options(
    bundle_root: &Path,
    options: &VerificationOptions,
) -> std::io::Result<VerificationReport> {
    let traversal = BundleTraversal::new(bundle_root);
    let documents = traversal.verification_scope(&options.exclusions)?;

    let mut report = VerificationReport {
        findings: Vec::new(),
    };

    for document in &documents {
        let contents = std::fs::read_to_string(document.path())?;

        match document.kind() {
            OkfDocumentKind::RootIndexFile => {
                report.extend(verify_index_file(document.path(), &contents, true));
            }
            OkfDocumentKind::IndexFile => {
                report.extend(verify_index_file(document.path(), &contents, false));
            }
            OkfDocumentKind::LogFile => {
                report.extend(verify_log_file(document.path(), &contents));
            }
            OkfDocumentKind::ConceptDocument => {
                report.extend(verify_concept_document(document.path(), &contents));
            }
        }
    }

    report.extend(verify_bundle_links(bundle_root, &documents)?);

    Ok(filter_report(report, options))
}

pub fn verify_concept_document(path: &Path, contents: &str) -> VerificationReport {
    let document = path.display().to_string();
    let mut findings = Vec::new();

    if !contents.starts_with("---\n") && contents.trim() != "---" {
        findings.push(Rule::ConceptFrontmatter.finding(
            document,
            "Concept Document must start with Frontmatter delimited by ---",
            Some(1),
            Some(1),
        ));
        return VerificationReport { findings };
    }

    let Some(frontmatter_end) = contents[4..].find("\n---") else {
        findings.push(Rule::ConceptFrontmatter.finding(
            document,
            "Concept Document Frontmatter must have a closing --- delimiter",
            Some(1),
            Some(1),
        ));
        return VerificationReport { findings };
    };

    let frontmatter = &contents[4..4 + frontmatter_end];
    let parsed_frontmatter = match serde_yaml::from_str::<serde_yaml::Mapping>(frontmatter) {
        Ok(frontmatter) => frontmatter,
        Err(error) => {
            let location = error.location();
            findings.push(Rule::ConceptFrontmatter.finding(
                document,
                format!("Concept Document Frontmatter must be parseable YAML: {error}"),
                location.as_ref().map(|location| location.line()),
                location.as_ref().map(|location| location.column()),
            ));
            return VerificationReport { findings };
        }
    };

    let type_key = serde_yaml::Value::String("type".to_string());
    let has_type = has_non_empty_string_field(&parsed_frontmatter, &type_key);

    if !has_type {
        findings.push(Rule::ConceptType.finding(
            document.clone(),
            "Concept Document Frontmatter must include a Concept Type",
            Some(2),
            Some(1),
        ));
    }

    let description_key = serde_yaml::Value::String("description".to_string());
    if !has_non_empty_string_field(&parsed_frontmatter, &description_key) {
        findings.push(Rule::ConceptDescription.finding(
            document.clone(),
            "Concept Document Frontmatter should include a Description",
            Some(2),
            Some(1),
        ));
    }

    if tags_are_unsorted(&parsed_frontmatter) {
        findings.push(Rule::ConceptTagsSorted.finding(
            document,
            "Concept Document Tags should be sorted",
            Some(2),
            Some(1),
        ));
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

fn tags_are_unsorted(frontmatter: &serde_yaml::Mapping) -> bool {
    let key = serde_yaml::Value::String("tags".to_string());
    let Some(tags) = frontmatter
        .get(&key)
        .and_then(serde_yaml::Value::as_sequence)
    else {
        return false;
    };
    let strings = tags
        .iter()
        .filter_map(serde_yaml::Value::as_str)
        .collect::<Vec<_>>();
    strings.windows(2).any(|pair| pair[0] > pair[1])
}

pub fn verify_index_file(path: &Path, contents: &str, is_root: bool) -> VerificationReport {
    let document = path.display().to_string();
    let mut findings = Vec::new();

    if let Some((frontmatter, body_start)) = split_frontmatter(contents) {
        if !is_root {
            findings.push(Rule::IndexNoFrontmatter.finding(
                document.clone(),
                "Index File must not contain frontmatter",
                Some(1),
                Some(1),
            ));
        } else {
            match serde_yaml::from_str::<serde_yaml::Mapping>(frontmatter) {
                Err(error) => {
                    let location = error.location();
                    findings.push(Rule::RootIndexFrontmatter.finding(
                        document.clone(),
                        format!("Root Index File frontmatter must be parseable YAML: {error}"),
                        location.as_ref().map(|location| location.line()),
                        location.as_ref().map(|location| location.column()),
                    ));
                }
                Ok(mapping) => {
                    let version_key = serde_yaml::Value::String("okf_version".to_string());
                    if let Some(value) = mapping.get(&version_key) {
                        if let Some(version) = value.as_str() {
                            if version != KNOWN_OKF_VERSION {
                                findings.push(Rule::RootIndexVersion.finding(
                                    document.clone(),
                                    format!(
                                        "Root Index File declares unknown OKF version {version}; \
                                         best-effort verification will be used"
                                    ),
                                    Some(2),
                                    Some(1),
                                ));
                            }
                        } else {
                            findings.push(Rule::RootIndexVersion.finding(
                                document.clone(),
                                "Root Index File okf_version should be a string",
                                Some(2),
                                Some(1),
                            ));
                        }
                    }
                }
            }
        }

        for (line_index, line) in contents.lines().enumerate().skip(body_start) {
            check_index_entry_line(&document, line, line_index + 1, &mut findings);
        }
    } else {
        for (line_index, line) in contents.lines().enumerate() {
            check_index_entry_line(&document, line, line_index + 1, &mut findings);
        }
    }

    VerificationReport { findings }
}

fn check_index_entry_line(
    document: &str,
    line: &str,
    line_number: usize,
    findings: &mut Vec<Finding>,
) {
    let trimmed = line.trim_start();
    if (trimmed.starts_with("- ") || trimmed.starts_with("* ")) && !trimmed.contains("](") {
        findings.push(Rule::IndexEntryLink.finding(
            document,
            "Index File entry is missing a markdown link",
            Some(line_number),
            Some(1),
        ));
    }
}

pub fn verify_log_file(path: &Path, contents: &str) -> VerificationReport {
    let document = path.display().to_string();
    let mut findings = Vec::new();

    if split_frontmatter(contents).is_some() {
        findings.push(Rule::LogNoFrontmatter.finding(
            document.clone(),
            "Log File must not contain frontmatter",
            Some(1),
            Some(1),
        ));
    }

    let mut dates = Vec::new();

    for (line_index, line) in contents.lines().enumerate() {
        if let Some(date) = line.strip_prefix("## ") {
            let line_number = line_index + 1;
            if !is_iso_8601_date(date) {
                findings.push(Rule::LogDateFormat.finding(
                    document.clone(),
                    format!("Log File date heading must use ISO 8601 date format: {date}"),
                    Some(line_number),
                    Some(4),
                ));
            } else {
                dates.push((date.to_string(), line_number));
            }
        }
    }

    for window in dates.windows(2) {
        let (current, next) = (&window[0], &window[1]);
        if current.0 < next.0 {
            findings.push(Rule::LogDateOrder.finding(
                document.clone(),
                "Log File dates should be ordered newest first",
                Some(current.1),
                Some(1),
            ));
        }
    }

    VerificationReport { findings }
}

fn split_frontmatter(contents: &str) -> Option<(&str, usize)> {
    if contents.trim() == "---" {
        return None;
    }

    if !contents.starts_with("---\n") {
        return None;
    }

    let after_opening = &contents[4..];
    let end = after_opening.find("\n---")?;
    let frontmatter = &after_opening[..end];
    let body_line_offset = contents[..4 + end].matches('\n').count().saturating_sub(1) + 1;

    Some((frontmatter, body_line_offset + 1))
}

fn is_iso_8601_date(value: &str) -> bool {
    if value.len() != 10 {
        return false;
    }

    let bytes = value.as_bytes();
    if !bytes.iter().enumerate().all(|(index, byte)| {
        if index == 4 || index == 7 {
            *byte == b'-'
        } else {
            byte.is_ascii_digit()
        }
    }) {
        return false;
    }

    let year: i32 = value[..4].parse().unwrap_or(0);
    let month: u32 = value[5..7].parse().unwrap_or(0);
    let day: u32 = value[8..10].parse().unwrap_or(0);

    (1..=12).contains(&month) && (1..=31).contains(&day) && year >= 0
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
        let location = match (finding.line, finding.column) {
            (Some(line), Some(column)) => format!(":{line}:{column}"),
            (Some(line), None) => format!(":{line}"),
            _ => String::new(),
        };
        output.push_str(&format!(
            "{}{}: {} [{}] {}\n",
            finding.document,
            location,
            severity_name(finding.severity()),
            finding.rule_code(),
            finding.message
        ));
    }
    output
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

pub fn format_report_json(report: &VerificationReport, failure_threshold: Severity) -> String {
    let findings = report
        .findings
        .iter()
        .map(format_finding_json)
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"conformant\":{},\"healthy\":{},\"status\":\"{}\",\"findings\":[{}]}}\n",
        report.is_conformant(),
        report.is_healthy_at(failure_threshold),
        if report.passes_failure_threshold(failure_threshold) {
            "pass"
        } else {
            "fail"
        },
        findings
    )
}

fn format_finding_json(finding: &Finding) -> String {
    format!(
        "{{\"rule_code\":\"{}\",\"severity\":\"{}\",\
         \"message\":\"{}\",\"document\":\"{}\",\"location\":{}}}",
        finding.rule_code(),
        severity_name(finding.severity()),
        json_escape(&finding.message),
        json_escape(&finding.document),
        format_location_json(finding)
    )
}

fn format_location_json(finding: &Finding) -> String {
    match (finding.line, finding.column) {
        (Some(line), Some(column)) => format!("{{\"line\":{line},\"column\":{column}}}"),
        (Some(line), None) => format!("{{\"line\":{line}}}"),
        _ => "null".to_string(),
    }
}

fn severity_name(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Suggestion => "suggestion",
    }
}

fn json_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

pub fn format_document(contents: &str) -> String {
    let mut output = contents
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n");
    output.push('\n');
    output
}

pub fn fix_document(contents: &str) -> String {
    sort_inline_tags(&format_document(contents))
}

fn sort_inline_tags(contents: &str) -> String {
    contents
        .lines()
        .map(sort_inline_tags_line)
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn sort_inline_tags_line(line: &str) -> String {
    let Some(values) = line
        .strip_prefix("tags: [")
        .and_then(|rest| rest.strip_suffix(']'))
    else {
        return line.to_string();
    };
    let mut tags = values
        .split(',')
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .collect::<Vec<_>>();
    tags.sort_unstable();
    format!("tags: [{}]", tags.join(", "))
}

fn filter_report(report: VerificationReport, options: &VerificationOptions) -> VerificationReport {
    VerificationReport {
        findings: report
            .findings
            .into_iter()
            .filter(|finding| include_finding(finding, options))
            .collect(),
    }
}

fn include_finding(finding: &Finding, options: &VerificationOptions) -> bool {
    if finding.is_suppressed_by(&options.suppressions) {
        return false;
    }
    options.rule_set != RuleSet::Conformance || finding.is_conformance_rule()
}

fn verify_bundle_links(
    bundle_root: &Path,
    documents: &[OkfDocument],
) -> std::io::Result<VerificationReport> {
    let mut findings = Vec::new();
    for document in documents {
        if !document.is_concept_document() {
            continue;
        }
        let contents = std::fs::read_to_string(document.path())?;
        findings.extend(find_broken_links(bundle_root, document.path(), &contents));
    }
    Ok(VerificationReport { findings })
}

fn find_broken_links(bundle_root: &Path, document: &Path, contents: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (line_index, line) in contents.lines().enumerate() {
        for target in markdown_link_targets(line) {
            if should_check_link(target) && !link_exists(bundle_root, document, target) {
                findings.push(broken_link_finding(document, line_index + 1, target));
            }
        }
    }
    findings
}

fn markdown_link_targets(line: &str) -> Vec<&str> {
    let mut targets = Vec::new();
    let mut rest = line;
    while let Some(start) = rest.find("](") {
        rest = &rest[start + 2..];
        let Some(end) = rest.find(')') else {
            break;
        };
        targets.push(&rest[..end]);
        rest = &rest[end + 1..];
    }
    targets
}

fn should_check_link(target: &str) -> bool {
    target.ends_with(".md") && !target.contains("://") && !target.starts_with('#')
}

fn link_exists(bundle_root: &Path, document: &Path, target: &str) -> bool {
    let target = target.split('#').next().unwrap_or(target);
    let path = if let Some(bundle_relative) = target.strip_prefix('/') {
        bundle_root.join(bundle_relative)
    } else {
        document.parent().unwrap_or(bundle_root).join(target)
    };
    path.is_file()
}

fn broken_link_finding(document: &Path, line: usize, target: &str) -> Finding {
    Rule::BrokenLink.finding(
        document.display().to_string(),
        format!("Broken Link target does not exist: {target}"),
        Some(line),
        Some(1),
    )
}

pub fn check_index_maintenance(bundle_root: &Path) -> std::io::Result<VerificationReport> {
    let mut findings = Vec::new();
    for directory in BundleTraversal::new(bundle_root).directories()? {
        let index = directory.join("index.md");
        let expected = build_index_file(&directory)?;
        let current = std::fs::read_to_string(&index).ok();
        if current.as_deref() != Some(expected.as_str()) {
            findings.push(Rule::IndexMaintenance.finding(
                index.display().to_string(),
                "Index File does not reflect the current Bundle Hierarchy",
                Some(1),
                Some(1),
            ));
        }
    }
    Ok(VerificationReport { findings })
}

pub fn fix_index_maintenance(bundle_root: &Path) -> std::io::Result<VerificationReport> {
    for directory in BundleTraversal::new(bundle_root).directories()? {
        let index = directory.join("index.md");
        let expected = build_index_file(&directory)?;
        std::fs::write(index, expected)?;
    }
    check_index_maintenance(bundle_root)
}

fn build_index_file(directory: &Path) -> std::io::Result<String> {
    let mut entries = std::fs::read_dir(directory)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.path());
    let mut lines = vec!["# Index".to_string(), String::new()];

    for entry in entries {
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            lines.push(format!(
                "* [{}]({}/)",
                title_from_path(&path),
                title_link(&path)
            ));
        } else if traversal::is_concept_document_file(&path) {
            lines.push(concept_index_entry(&path)?);
        }
    }

    Ok(lines.join("\n") + "\n")
}

fn concept_index_entry(path: &Path) -> std::io::Result<String> {
    let metadata = read_concept_metadata(path)?;
    let mut line = format!(
        "* [{}]({})",
        metadata.title,
        path.file_name().unwrap().to_string_lossy()
    );
    if let Some(description) = metadata.description {
        line.push_str(&format!(" - {description}"));
    }
    Ok(line)
}

struct ConceptMetadata {
    title: String,
    description: Option<String>,
}

fn read_concept_metadata(path: &Path) -> std::io::Result<ConceptMetadata> {
    let contents = std::fs::read_to_string(path)?;
    let mapping = split_frontmatter(&contents)
        .and_then(|(frontmatter, _)| serde_yaml::from_str::<serde_yaml::Mapping>(frontmatter).ok());
    Ok(ConceptMetadata {
        title: metadata_value(&mapping, "title").unwrap_or_else(|| title_from_path(path)),
        description: metadata_value(&mapping, "description"),
    })
}

fn metadata_value(mapping: &Option<serde_yaml::Mapping>, key: &str) -> Option<String> {
    let key = serde_yaml::Value::String(key.to_string());
    mapping
        .as_ref()
        .and_then(|mapping| mapping.get(&key))
        .and_then(serde_yaml::Value::as_str)
        .map(str::to_string)
}

fn title_from_path(path: &Path) -> String {
    let stem = path.file_stem().or_else(|| path.file_name()).unwrap();
    stem.to_string_lossy()
        .replace('-', " ")
        .split_whitespace()
        .map(capitalize)
        .collect::<Vec<_>>()
        .join(" ")
}

fn title_link(path: &Path) -> String {
    path.file_name().unwrap().to_string_lossy().to_string()
}

fn capitalize(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}
