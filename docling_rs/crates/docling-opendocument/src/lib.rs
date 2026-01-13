//! `OpenDocument` Format (ODF) parsers for docling
//!
//! This crate provides parsers for the `OpenDocument` Format family:
//! - **ODT** (`OpenDocument` Text) - Word processor documents
//! - **ODS** (`OpenDocument` Spreadsheet) - Spreadsheet documents
//! - **ODP** (`OpenDocument` Presentation) - Presentation documents
//!
//! ## Format Overview
//!
//! `OpenDocument` Format (ODF) is an open standard (ISO/IEC 26300) for office
//! documents. All ODF files are ZIP archives containing XML files for content,
//! styles, and metadata.
//!
//! ## Usage
//!
//! ### ODT (Text Documents)
//!
//! ```no_run
//! use docling_opendocument::odt::parse_odt_file;
//!
//! let doc = parse_odt_file("document.odt")?;
//! println!("Title: {:?}", doc.title);
//! println!("Content: {}", doc.text);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ### ODS (Spreadsheets)
//!
//! ```no_run
//! use docling_opendocument::ods::parse_ods_file;
//!
//! let doc = parse_ods_file("spreadsheet.ods")?;
//! println!("Sheets: {:?}", doc.sheet_names);
//! println!("Content: {}", doc.text);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ### ODP (Presentations)
//!
//! ```no_run
//! use docling_opendocument::odp::parse_odp_file;
//!
//! let doc = parse_odp_file("presentation.odp")?;
//! println!("Slides: {}", doc.slide_count);
//! println!("Content: {}", doc.text);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## Implementation Notes
//!
//! - **ODT** and **ODP** use custom XML parsers with `quick-xml`
//! - **ODS** uses the `calamine` crate for robust spreadsheet parsing
//! - All formats extract text content and basic metadata
//! - Complex formatting, images, and styles are not preserved

pub mod error;
pub mod odp;
pub mod ods;
pub mod odt;
pub mod xml;

// Re-export main types
pub use error::{OdfError, Result};
pub use odp::{parse_odp_file, parse_odp_reader, parse_odp_slides, OdpDocument, OdpSlide};
pub use ods::{parse_ods_file, parse_ods_sheets, OdsDocument, OdsSheet};
pub use odt::{parse_odt_file, parse_odt_reader, OdtDocument};

/// `OpenDocument` format variant
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OdfFormat {
    /// `OpenDocument` Text (.odt)
    Text,
    /// `OpenDocument` Spreadsheet (.ods)
    Spreadsheet,
    /// `OpenDocument` Presentation (.odp)
    Presentation,
}

impl OdfFormat {
    /// Detect format from file extension
    #[inline]
    #[must_use = "detects format from file extension"]
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "odt" => Some(Self::Text),
            "ods" => Some(Self::Spreadsheet),
            "odp" => Some(Self::Presentation),
            _ => None,
        }
    }

    /// Get file extension for this format
    #[inline]
    #[must_use = "returns file extension for format"]
    pub const fn extension(&self) -> &str {
        match self {
            Self::Text => "odt",
            Self::Spreadsheet => "ods",
            Self::Presentation => "odp",
        }
    }
}

impl std::fmt::Display for OdfFormat {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Text => "text",
            Self::Spreadsheet => "spreadsheet",
            Self::Presentation => "presentation",
        };
        write!(f, "{s}")
    }
}

impl std::str::FromStr for OdfFormat {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        // Strip leading dot if present (e.g., ".odt" -> "odt")
        let s = s.strip_prefix('.').unwrap_or(s);
        match s.to_lowercase().as_str() {
            // Accept extensions and display names
            "odt" | "text" => Ok(Self::Text),
            "ods" | "spreadsheet" => Ok(Self::Spreadsheet),
            "odp" | "presentation" => Ok(Self::Presentation),
            _ => Err(format!(
                "unknown ODF format: '{s}' (expected: text/odt, spreadsheet/ods, presentation/odp)"
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_from_extension() {
        assert_eq!(OdfFormat::from_extension("odt"), Some(OdfFormat::Text));
        assert_eq!(OdfFormat::from_extension("ODT"), Some(OdfFormat::Text));
        assert_eq!(
            OdfFormat::from_extension("ods"),
            Some(OdfFormat::Spreadsheet)
        );
        assert_eq!(
            OdfFormat::from_extension("odp"),
            Some(OdfFormat::Presentation)
        );
        assert_eq!(OdfFormat::from_extension("pdf"), None);
    }

    #[test]
    fn test_format_extension() {
        assert_eq!(OdfFormat::Text.extension(), "odt");
        assert_eq!(OdfFormat::Spreadsheet.extension(), "ods");
        assert_eq!(OdfFormat::Presentation.extension(), "odp");
    }

    #[test]
    fn test_format_display() {
        assert_eq!(format!("{}", OdfFormat::Text), "text");
        assert_eq!(format!("{}", OdfFormat::Spreadsheet), "spreadsheet");
        assert_eq!(format!("{}", OdfFormat::Presentation), "presentation");
    }

    #[test]
    fn test_format_from_str() {
        use std::str::FromStr;

        // Extensions (lowercase)
        assert_eq!(OdfFormat::from_str("odt").unwrap(), OdfFormat::Text);
        assert_eq!(OdfFormat::from_str("ods").unwrap(), OdfFormat::Spreadsheet);
        assert_eq!(OdfFormat::from_str("odp").unwrap(), OdfFormat::Presentation);

        // Extensions with dot
        assert_eq!(OdfFormat::from_str(".odt").unwrap(), OdfFormat::Text);
        assert_eq!(OdfFormat::from_str(".ODS").unwrap(), OdfFormat::Spreadsheet);

        // Display names
        assert_eq!(OdfFormat::from_str("text").unwrap(), OdfFormat::Text);
        assert_eq!(
            OdfFormat::from_str("SPREADSHEET").unwrap(),
            OdfFormat::Spreadsheet
        );
        assert_eq!(
            OdfFormat::from_str("Presentation").unwrap(),
            OdfFormat::Presentation
        );

        // Error case
        assert!(OdfFormat::from_str("pdf").is_err());
        assert!(OdfFormat::from_str("docx").is_err());
    }

    #[test]
    fn test_format_roundtrip() {
        use std::str::FromStr;

        for fmt in [
            OdfFormat::Text,
            OdfFormat::Spreadsheet,
            OdfFormat::Presentation,
        ] {
            let s = fmt.to_string();
            let parsed = OdfFormat::from_str(&s).unwrap();
            assert_eq!(fmt, parsed, "roundtrip failed for {s}");
        }
    }
}
