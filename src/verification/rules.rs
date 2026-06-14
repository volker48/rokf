use super::{Finding, Severity};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Rule {
    ConceptFrontmatter,
    ConceptType,
    ConceptDescription,
    ConceptTagsSorted,
    IndexNoFrontmatter,
    LogNoFrontmatter,
    RootIndexFrontmatter,
    RootIndexVersion,
    IndexEntryLink,
    LogDateFormat,
    LogDateOrder,
    BrokenLink,
    IndexMaintenance,
}

impl Rule {
    pub(super) fn code(self) -> &'static str {
        match self {
            Self::ConceptFrontmatter => "OKF001",
            Self::ConceptType => "OKF002",
            Self::ConceptDescription => "OKF101",
            Self::ConceptTagsSorted => "OKF102",
            Self::IndexNoFrontmatter => "OKF200",
            Self::LogNoFrontmatter => "OKF201",
            Self::RootIndexFrontmatter => "OKF202",
            Self::RootIndexVersion => "OKF203",
            Self::IndexEntryLink => "OKF204",
            Self::LogDateFormat => "OKF301",
            Self::LogDateOrder => "OKF302",
            Self::BrokenLink => "OKF400",
            Self::IndexMaintenance => "OKF500",
        }
    }

    pub(super) fn severity(self) -> Severity {
        match self {
            Self::ConceptFrontmatter
            | Self::ConceptType
            | Self::IndexNoFrontmatter
            | Self::LogNoFrontmatter
            | Self::RootIndexFrontmatter
            | Self::LogDateFormat => Severity::Error,
            Self::ConceptDescription
            | Self::RootIndexVersion
            | Self::LogDateOrder
            | Self::BrokenLink
            | Self::IndexMaintenance => Severity::Warning,
            Self::ConceptTagsSorted | Self::IndexEntryLink => Severity::Suggestion,
        }
    }

    pub(super) fn is_conformance_rule(self) -> bool {
        matches!(
            self,
            Self::ConceptFrontmatter
                | Self::ConceptType
                | Self::IndexNoFrontmatter
                | Self::LogNoFrontmatter
                | Self::RootIndexFrontmatter
                | Self::LogDateFormat
        )
    }

    pub(super) fn finding(
        self,
        document: impl Into<String>,
        message: impl Into<String>,
        line: Option<usize>,
        column: Option<usize>,
    ) -> Finding {
        Finding {
            rule: self,
            message: message.into(),
            document: document.into(),
            line,
            column,
        }
    }
}
