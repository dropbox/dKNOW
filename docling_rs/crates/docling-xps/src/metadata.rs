//! XPS document metadata

/// XPS document metadata (from docProps/core.xml)
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct XpsMetadata {
    /// Document title
    pub title: Option<String>,

    /// Document author(s)
    pub author: Option<String>,

    /// Document subject
    pub subject: Option<String>,

    /// Document creator (application)
    pub creator: Option<String>,

    /// Document keywords
    pub keywords: Option<String>,

    /// Document description
    pub description: Option<String>,

    /// Creation date (ISO 8601)
    pub created: Option<String>,

    /// Last modified date (ISO 8601)
    pub modified: Option<String>,
}

impl XpsMetadata {
    /// Create empty metadata
    #[inline]
    #[must_use = "creates empty metadata"]
    pub const fn new() -> Self {
        Self {
            title: None,
            author: None,
            subject: None,
            creator: None,
            keywords: None,
            description: None,
            created: None,
            modified: None,
        }
    }
}
