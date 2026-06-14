#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FrontmatterBlock<'a> {
    raw: &'a str,
    body_start_index: usize,
}

impl<'a> FrontmatterBlock<'a> {
    pub(crate) fn body_start_index(&self) -> usize {
        self.body_start_index
    }

    pub(crate) fn parse_mapping(&self) -> Result<serde_yaml::Mapping, serde_yaml::Error> {
        serde_yaml::from_str::<serde_yaml::Mapping>(self.raw)
    }
}

pub(crate) struct OkfDocumentText<'a> {
    contents: &'a str,
}

impl<'a> OkfDocumentText<'a> {
    pub(crate) fn new(contents: &'a str) -> Self {
        Self { contents }
    }

    pub(crate) fn has_frontmatter_opening(&self) -> bool {
        self.contents.starts_with("---\n") || self.contents.trim() == "---"
    }

    pub(crate) fn closed_frontmatter(&self) -> Option<FrontmatterBlock<'a>> {
        if self.contents.trim() == "---" || !self.contents.starts_with("---\n") {
            return None;
        }

        let after_opening = &self.contents[4..];
        let end = after_opening.find("\n---")?;
        let closing_start = 4 + end;
        let closing_line = self.contents[..closing_start].matches('\n').count();

        Some(FrontmatterBlock {
            raw: &after_opening[..end],
            body_start_index: closing_line + 2,
        })
    }

    pub(crate) fn metadata_value(&self, key: &str) -> Option<String> {
        let mapping = self.closed_frontmatter()?.parse_mapping().ok()?;
        mapping
            .get(serde_yaml::Value::String(key.to_string()))
            .and_then(serde_yaml::Value::as_str)
            .map(str::to_string)
    }

    pub(crate) fn lines(&self) -> std::str::Lines<'a> {
        self.contents.lines()
    }
}

#[cfg(test)]
mod tests {
    use super::OkfDocumentText;

    #[test]
    fn missing_frontmatter_opening_has_no_closed_frontmatter() {
        let document = OkfDocumentText::new("# Customers\n");

        assert!(!document.has_frontmatter_opening());
        assert!(document.closed_frontmatter().is_none());
    }

    #[test]
    fn unclosed_frontmatter_has_opening_but_no_closed_frontmatter() {
        let document = OkfDocumentText::new("---");

        assert!(document.has_frontmatter_opening());
        assert!(document.closed_frontmatter().is_none());
    }

    #[test]
    fn closed_frontmatter_tracks_first_body_line_index() {
        let document = OkfDocumentText::new("---\ntype: Table\n---\n\n# Customers\n");
        let frontmatter = document.closed_frontmatter().expect("closed frontmatter");
        let body_lines = document
            .lines()
            .skip(frontmatter.body_start_index())
            .collect::<Vec<_>>();

        assert_eq!(body_lines, vec!["", "# Customers"]);
    }

    #[test]
    fn metadata_value_reads_string_fields() {
        let document = OkfDocumentText::new("---\ntitle: Customers\ncount: 1\n---\n");

        assert_eq!(
            document.metadata_value("title"),
            Some("Customers".to_string())
        );
        assert_eq!(document.metadata_value("count"), None);
        assert_eq!(document.metadata_value("missing"), None);
    }

    #[test]
    fn malformed_yaml_returns_a_parse_error() {
        let document = OkfDocumentText::new("---\ntags: [customers\n---\n");
        let frontmatter = document.closed_frontmatter().expect("closed frontmatter");

        assert!(frontmatter.parse_mapping().is_err());
    }
}
