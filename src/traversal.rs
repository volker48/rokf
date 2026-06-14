use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OkfDocumentKind {
    ConceptDocument,
    RootIndexFile,
    IndexFile,
    LogFile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OkfDocument {
    path: PathBuf,
    kind: OkfDocumentKind,
}

impl OkfDocument {
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn kind(&self) -> OkfDocumentKind {
        self.kind
    }

    pub(crate) fn is_concept_document(&self) -> bool {
        self.kind == OkfDocumentKind::ConceptDocument
    }
}

pub(crate) struct BundleTraversal<'a> {
    root: &'a Path,
}

impl<'a> BundleTraversal<'a> {
    pub(crate) fn new(root: &'a Path) -> Self {
        Self { root }
    }

    pub(crate) fn documents(&self) -> std::io::Result<Vec<OkfDocument>> {
        markdown_documents(self.root).map(|documents| {
            documents
                .into_iter()
                .map(|path| self.classify_document(path))
                .collect()
        })
    }

    pub(crate) fn verification_scope(
        &self,
        exclusions: &[String],
    ) -> std::io::Result<Vec<OkfDocument>> {
        self.documents().map(|documents| {
            documents
                .into_iter()
                .filter(|document| !is_excluded(self.root, document.path(), exclusions))
                .collect()
        })
    }

    pub(crate) fn directories(&self) -> std::io::Result<Vec<PathBuf>> {
        let mut directories = Vec::new();
        collect_directories(self.root, &mut directories)?;
        directories.sort();
        Ok(directories)
    }

    fn classify_document(&self, path: PathBuf) -> OkfDocument {
        let root_index = self.root.join("index.md");
        let kind = if path == root_index {
            OkfDocumentKind::RootIndexFile
        } else if has_file_name(&path, "index.md") {
            OkfDocumentKind::IndexFile
        } else if has_file_name(&path, "log.md") {
            OkfDocumentKind::LogFile
        } else {
            OkfDocumentKind::ConceptDocument
        };
        OkfDocument { path, kind }
    }
}

pub(crate) fn discover_bundle_root() -> Option<PathBuf> {
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

pub(crate) fn markdown_documents(directory: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut documents = Vec::new();
    collect_markdown_documents(directory, &mut documents)?;
    documents.sort();
    Ok(documents)
}

pub(crate) fn is_concept_document_file(path: &Path) -> bool {
    path.extension().is_some_and(|extension| extension == "md")
        && !has_file_name(path, "index.md")
        && !has_file_name(path, "log.md")
}

fn collect_markdown_documents(
    directory: &Path,
    documents: &mut Vec<PathBuf>,
) -> std::io::Result<()> {
    for entry in std::fs::read_dir(directory)?.collect::<Result<Vec<_>, _>>()? {
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            collect_markdown_documents(&path, documents)?;
        } else if file_type.is_file() && path.extension().is_some_and(|extension| extension == "md")
        {
            documents.push(path);
        }
    }
    Ok(())
}

fn collect_directories(directory: &Path, directories: &mut Vec<PathBuf>) -> std::io::Result<()> {
    directories.push(directory.to_path_buf());
    for entry in std::fs::read_dir(directory)?.collect::<Result<Vec<_>, _>>()? {
        if entry.file_type()?.is_dir() {
            collect_directories(&entry.path(), directories)?;
        }
    }
    Ok(())
}

fn has_file_name(path: &Path, expected: &str) -> bool {
    path.file_name()
        .and_then(|file_name| file_name.to_str())
        .is_some_and(|file_name| file_name == expected)
}

fn is_excluded(bundle_root: &Path, document: &Path, exclusions: &[String]) -> bool {
    let relative = document.strip_prefix(bundle_root).unwrap_or(document);
    let relative = relative.to_string_lossy();
    exclusions
        .iter()
        .any(|exclusion| relative == exclusion.as_str() || relative.starts_with(exclusion))
}
