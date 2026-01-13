//! # docling-adobe
//!
//! Adobe Creative Suite document parsing for docling-rs.
//!
//! This crate provides parsing support for Adobe Creative Suite file formats,
//! enabling text extraction and markdown conversion from professional publishing
//! and design documents.
//!
//! ## Supported Formats
//!
//! | Format | Extension | Description |
//! |--------|-----------|-------------|
//! | IDML | `.idml` | `InDesign` Markup Language (interchange format) |
//!
//! ## What is IDML?
//!
//! IDML (`InDesign` Markup Language) is Adobe `InDesign`'s XML-based interchange format.
//! Unlike the native `.indd` format, IDML files are ZIP archives containing XML files
//! that describe the document structure:
//!
//! - **Stories** - Text flows that can span multiple frames
//! - **Spreads** - Page layouts with positioned frames
//! - **Styles** - Paragraph and character formatting definitions
//! - **Resources** - Fonts, colors, and other assets
//!
//! IDML is commonly used for:
//! - Document exchange between different `InDesign` versions
//! - Programmatic document generation and modification
//! - Archival and backup purposes
//! - Cross-platform compatibility
//!
//! ## Quick Start
//!
//! ### Parse an IDML File
//!
//! ```rust,no_run
//! use docling_adobe::{IdmlParser, IdmlDocument};
//!
//! // Parse IDML file to structured document
//! let doc = IdmlParser::parse_file("brochure.idml")?;
//!
//! // Access metadata
//! if let Some(title) = &doc.metadata.title {
//!     println!("Document: {}", title);
//! }
//! if let Some(author) = &doc.metadata.author {
//!     println!("Author: {}", author);
//! }
//!
//! // Iterate through stories (text flows)
//! for story in &doc.stories {
//!     println!("Story ID: {}", story.id);
//!     for paragraph in &story.paragraphs {
//!         println!("  [{}] {}",
//!             paragraph.style.as_deref().unwrap_or("default"),
//!             paragraph.text
//!         );
//!     }
//! }
//! # Ok::<(), docling_adobe::IdmlError>(())
//! ```
//!
//! ### Convert to Markdown
//!
//! ```rust,no_run
//! use docling_adobe::{IdmlParser, IdmlSerializer};
//!
//! let doc = IdmlParser::parse_file("newsletter.idml")?;
//! let markdown = IdmlSerializer::to_markdown(&doc);
//!
//! // Markdown preserves document structure:
//! // - Headings from heading styles
//! // - Paragraphs with proper spacing
//! // - Document metadata as frontmatter
//! println!("{}", markdown);
//! # Ok::<(), docling_adobe::IdmlError>(())
//! ```
//!
//! ### Build Documents Programmatically
//!
//! ```rust
//! use docling_adobe::{IdmlDocument, Story, Paragraph, Metadata};
//!
//! // Create a new document
//! let mut doc = IdmlDocument::with_metadata(Metadata {
//!     title: Some("Annual Report".to_string()),
//!     author: Some("Finance Team".to_string()),
//! });
//!
//! // Create a story with styled paragraphs
//! let mut story = Story::new("main-story".to_string());
//! story.add_paragraph(Paragraph::with_style(
//!     "Heading1".to_string(),
//!     "Executive Summary".to_string()
//! ));
//! story.add_paragraph(Paragraph::new(
//!     "This year showed strong growth across all divisions.".to_string()
//! ));
//!
//! doc.add_story(story);
//! ```
//!
//! ## Document Structure
//!
//! ### `IdmlDocument`
//!
//! The top-level container representing an `InDesign` document:
//!
//! | Field | Type | Description |
//! |-------|------|-------------|
//! | `metadata` | `Metadata` | Document title and author |
//! | `stories` | `Vec<Story>` | Text flows in the document |
//!
//! ### Story
//!
//! A text flow that can span multiple frames on multiple pages:
//!
//! | Field | Type | Description |
//! |-------|------|-------------|
//! | `id` | `String` | Unique story identifier (e.g., "u1000") |
//! | `paragraphs` | `Vec<Paragraph>` | Paragraphs in reading order |
//!
//! ### Paragraph
//!
//! A single paragraph with optional style information:
//!
//! | Field | Type | Description |
//! |-------|------|-------------|
//! | `style` | `Option<String>` | Paragraph style name (e.g., "Heading1") |
//! | `text` | `String` | Plain text content |
//!
//! ## Style Mapping
//!
//! The serializer maps `InDesign` paragraph styles to markdown:
//!
//! | `InDesign` Style | Markdown Output |
//! |----------------|-----------------|
//! | `Heading1`, `Title` | `# Heading` |
//! | `Heading2`, `Subtitle` | `## Heading` |
//! | `Heading3` | `### Heading` |
//! | `BodyText`, `Normal` | Plain paragraph |
//! | `Quote`, `Blockquote` | `> Quote` |
//! | `BulletList` | `- Item` |
//!
//! ## IDML File Structure
//!
//! An IDML file is a ZIP archive containing:
//!
//! ```text
//! document.idml/
//! ├── mimetype                 # "application/vnd.adobe.indesign-idml-package"
//! ├── designmap.xml            # Document structure and metadata
//! ├── META-INF/
//! │   └── container.xml        # Package information
//! ├── Resources/
//! │   ├── Fonts.xml            # Font definitions
//! │   ├── Styles.xml           # Paragraph and character styles
//! │   └── Graphic.xml          # Graphics and colors
//! ├── Spreads/
//! │   ├── Spread_u123.xml      # Page layouts with frames
//! │   └── ...
//! └── Stories/
//!     ├── Story_u456.xml       # Text content flows
//!     └── ...
//! ```
//!
//! ## Use Cases
//!
//! - **Content extraction**: Extract text from marketing materials
//! - **Document conversion**: Convert `InDesign` layouts to web content
//! - **Template processing**: Parse and modify document templates
//! - **Archival**: Extract text for long-term preservation
//! - **Translation workflows**: Extract text for localization
//!
//! ## Limitations
//!
//! Current implementation focuses on text extraction:
//!
//! - **Text only**: Images and graphics are not extracted
//! - **Basic styles**: Complex formatting may be simplified
//! - **No layout**: Page positions and frames are not preserved
//! - **Stories only**: Master pages and layers are not processed
//!
//! ## Error Handling
//!
//! ```rust,no_run
//! use docling_adobe::{IdmlParser, IdmlError};
//!
//! match IdmlParser::parse_file("document.idml") {
//!     Ok(doc) => println!("Parsed {} stories", doc.stories.len()),
//!     Err(IdmlError::IoError(e)) => println!("File error: {}", e),
//!     Err(IdmlError::ParseError(e)) => println!("Parse error: {}", e),
//!     Err(IdmlError::InvalidStructure(e)) => println!("Structure error: {}", e),
//! }
//! ```
//!
//! ## Future Support
//!
//! Planned formats for future releases:
//!
//! - **AI** (Adobe Illustrator) - Vector graphics
//! - **PSD** (Adobe Photoshop) - Layer extraction and text
//! - **XFA** (PDF Forms) - Form field extraction
//! - **INDD** (`InDesign` native) - Direct format support

/// Error types for IDML parsing
pub mod error;
/// IDML (InDesign Markup Language) parser and serializer
pub mod idml;

pub use error::{IdmlError, Result};
pub use idml::{IdmlDocument, IdmlParser, IdmlSerializer, Metadata, Paragraph, Story};
