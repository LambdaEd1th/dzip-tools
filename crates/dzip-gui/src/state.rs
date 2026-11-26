#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ArchiveEditorMode {
    New,
    Existing { source_name: String },
}

impl ArchiveEditorMode {
    pub(crate) fn source_name(&self) -> Option<&str> {
        match self {
            Self::New => None,
            Self::Existing { source_name } => Some(source_name),
        }
    }

    pub(crate) fn is_new(&self) -> bool {
        matches!(self, Self::New)
    }
}
