//! Microsoft `OneNote` (.one) format support
//!
//! `OneNote` files use proprietary `OneNote` File Format with complex structure.
//!
//! **STATUS: UNSUPPORTED - Desktop format parsing not available**
//!
//! Rationale:
//! - `onenote_parser` crate (v0.3.1) only supports OneDrive/cloud format
//! - Desktop format (.one files from `OneNote` 2016 app) NOT YET SUPPORTED
//! - This affects majority of users who use desktop `OneNote`
//! - Parsing requires reverse engineering proprietary Microsoft format
//!
//! Decision: Return clear error message until Rust library matures (v0.4.0+)
//! Monitor: <https://github.com/msiemens/onenote.rs>

use anyhow::Result;

/// Backend for Microsoft `OneNote` files
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct OneNoteBackend;

impl OneNoteBackend {
    /// Create a new `OneNote` backend instance
    #[inline]
    #[must_use = "creates OneNote backend instance"]
    pub const fn new() -> Self {
        Self
    }

    /// Return error explaining `OneNote` is unsupported
    /// Desktop `OneNote` format (.one) is not supported by available Rust libraries
    ///
    /// # Errors
    ///
    /// This function always returns an error (`OneNote` format is not supported).
    #[must_use = "this function returns a Result that should be handled"]
    pub fn parse_error(&self) -> Result<Vec<u8>> {
        anyhow::bail!(
            "OneNote desktop format (.one) is not yet supported. \
            Reason: Available Rust library (onenote_parser v0.3.1) only supports \
            OneDrive/cloud format, not desktop OneNote 2016 files. \
            This format requires proprietary Microsoft APIs or reverse engineering. \
            Status: Deferred until library matures (monitor: https://github.com/msiemens/onenote.rs)"
        )
    }

    /// Get the backend name
    #[inline]
    #[must_use = "returns backend name string"]
    pub const fn name(&self) -> &'static str {
        "OneNote"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_onenote_backend_creation() {
        let backend = OneNoteBackend::new();
        assert_eq!(backend.name(), "OneNote");
    }

    #[test]
    #[allow(
        clippy::default_constructed_unit_structs,
        reason = "testing Default trait impl"
    )]
    fn test_onenote_backend_default_equals_new() {
        // Verify derived Default produces same result as new()
        assert_eq!(OneNoteBackend::default(), OneNoteBackend::new());
    }
}
