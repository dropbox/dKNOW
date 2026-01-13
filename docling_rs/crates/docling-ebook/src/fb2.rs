/// `FictionBook` (FB2) format parser
///
/// FB2 is an XML-based e-book format popular in Russia and Eastern Europe.
/// Unlike EPUB which uses a ZIP archive, FB2 is a single XML file containing
/// all metadata, content, and images (base64-encoded).
///
/// Format structure:
/// - Single XML file (or .fb2.zip compressed)
/// - <description>: metadata (title-info, document-info, publish-info)
/// - <body>: main content (hierarchical sections)
/// - <body name="notes">: footnotes/endnotes
/// - <binary>: base64-encoded images
///
/// References:
/// - Official XSD schema: <https://github.com/gribuser/fb2>
/// - Format spec: `FictionBook` 2.0/2.1
use crate::error::{EbookError, Result};
use crate::types::{Chapter, EbookMetadata, ParsedEbook, TocEntry};
use quick_xml::events::Event;
use quick_xml::Reader;
use std::collections::HashMap;
use std::fmt::Write;
use std::fs::File;
use std::io::Read;
use std::path::Path;

/// FB2 author information
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
struct Fb2Author {
    first_name: Option<String>,
    middle_name: Option<String>,
    last_name: Option<String>,
    nickname: Option<String>,
    email: Option<String>,
}

impl std::fmt::Display for Fb2Author {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(nickname) = &self.nickname {
            return write!(f, "{nickname}");
        }

        let mut parts = Vec::new();
        if let Some(first) = &self.first_name {
            parts.push(first.clone());
        }
        if let Some(middle) = &self.middle_name {
            parts.push(middle.clone());
        }
        if let Some(last) = &self.last_name {
            parts.push(last.clone());
        }

        if parts.is_empty() {
            if let Some(email) = &self.email {
                write!(f, "{email}")
            } else {
                write!(f, "Unknown Author")
            }
        } else {
            write!(f, "{}", parts.join(" "))
        }
    }
}

/// FB2 section (recursive content structure)
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
struct Fb2Section {
    /// Section ID for cross-reference linking (reserved for future use)
    _id: Option<String>,
    title: Option<String>,
    content: Vec<String>, // Paragraphs
    subsections: Vec<Fb2Section>,
}

impl Fb2Section {
    #[inline]
    const fn new() -> Self {
        Self {
            _id: None,
            title: None,
            content: Vec::new(),
            subsections: Vec::new(),
        }
    }
}

/// FB2 image data
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
struct Fb2Image {
    id: String,
    /// MIME type for future image embedding/export
    _content_type: String,
    /// Binary image data for future image embedding/export
    _data: Vec<u8>,
}

/// Internal parsed FB2 structure
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ParsedFb2 {
    metadata: EbookMetadata,
    body_title: Option<String>,
    sections: Vec<Fb2Section>,
    notes: Vec<Fb2Section>,
    images: HashMap<String, Fb2Image>,
}

/// Parse FB2 file from path
///
/// Supports both plain .fb2 XML files and .fb2.zip compressed files.
///
/// # Arguments
/// * `path` - Path to .fb2 or .fb2.zip file
///
/// # Returns
/// * `ParsedEbook` with metadata, chapters, and table of contents
///
/// # Errors
/// * File I/O errors
/// * XML parsing errors
/// * Invalid FB2 structure
#[must_use = "this function returns a parsed ebook that should be processed"]
pub fn parse_fb2<P: AsRef<Path>>(path: P) -> Result<ParsedEbook> {
    let xml_content = load_fb2_file(path.as_ref())?;
    parse_fb2_xml(&xml_content)
}

/// Load FB2 file (handle both .fb2 and .fb2.zip)
fn load_fb2_file(path: &Path) -> Result<String> {
    let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    if extension == "zip" {
        // Compressed .fb2.zip file
        load_fb2_from_zip(path)
    } else {
        // Plain .fb2 XML file
        std::fs::read_to_string(path)
            .map_err(|e| EbookError::IoError(format!("Failed to read FB2 file: {e}")))
    }
}

/// Extract FB2 XML from ZIP archive
fn load_fb2_from_zip(path: &Path) -> Result<String> {
    let file = File::open(path)
        .map_err(|e| EbookError::IoError(format!("Failed to open ZIP file: {e}")))?;

    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| EbookError::ParseError(format!("Invalid ZIP archive: {e}")))?;

    // Find .fb2 file inside ZIP (usually only one)
    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| EbookError::ParseError(format!("Failed to read ZIP entry: {e}")))?;

        if std::path::Path::new(file.name())
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("fb2"))
        {
            let mut content = String::new();
            file.read_to_string(&mut content)
                .map_err(|e| EbookError::IoError(format!("Failed to read FB2 from ZIP: {e}")))?;
            return Ok(content);
        }
    }

    Err(EbookError::ParseError(
        "No .fb2 file found in ZIP archive".to_string(),
    ))
}

/// Parse FB2 XML content
fn parse_fb2_xml(xml_content: &str) -> Result<ParsedEbook> {
    let mut reader = Reader::from_str(xml_content);
    // Commented out: method `config_mut` not available on Reader
    // Consider alternative configuration if needed

    let mut parsed = ParsedFb2 {
        metadata: EbookMetadata::new(),
        body_title: None,
        sections: Vec::new(),
        notes: Vec::new(),
        images: HashMap::new(),
    };

    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = e.name();
                let tag_name = std::str::from_utf8(name.as_ref()).unwrap_or("");

                match tag_name {
                    "description" => {
                        parse_description(&mut reader, &mut parsed.metadata)?;
                    }
                    "body" => {
                        // Check if this is the notes body
                        let is_notes = e.attributes().any(|attr| {
                            if let Ok(attr) = attr {
                                if let Ok(key) = std::str::from_utf8(attr.key.as_ref()) {
                                    if key == "name" {
                                        if let Ok(value) = std::str::from_utf8(&attr.value) {
                                            return value == "notes";
                                        }
                                    }
                                }
                            }
                            false
                        });

                        if is_notes {
                            let (_, notes_sections) = parse_body(&mut reader)?;
                            parsed.notes = notes_sections;
                        } else {
                            let (body_title, sections) = parse_body(&mut reader)?;
                            parsed.body_title = body_title;
                            parsed.sections = sections;
                        }
                    }
                    "binary" => {
                        if let Some(image) = parse_binary(&mut reader, &e)? {
                            parsed.images.insert(image.id.clone(), image);
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(EbookError::ParseError(format!("XML parsing error: {e}"))),
            _ => {}
        }
        buf.clear();
    }

    // Convert internal structure to ParsedEbook
    Ok(convert_to_parsed_ebook(parsed))
}

/// Parse `<description>` element (metadata)
fn parse_description(reader: &mut Reader<&[u8]>, metadata: &mut EbookMetadata) -> Result<()> {
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = e.name();
                let tag_name = std::str::from_utf8(name.as_ref()).unwrap_or("");

                match tag_name {
                    "title-info" => parse_title_info(reader, metadata)?,
                    "document-info" => parse_document_info(reader, metadata)?,
                    "publish-info" => parse_publish_info(reader, metadata)?,
                    _ => skip_element(reader, tag_name)?,
                }
            }
            Ok(Event::End(e)) => {
                let name = e.name();
                let tag_name = std::str::from_utf8(name.as_ref()).unwrap_or("");
                if tag_name == "description" {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(EbookError::ParseError(format!(
                    "Error parsing description: {e}"
                )))
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(())
}

/// Parse `<title-info>` element (book metadata)
#[allow(clippy::too_many_lines)] // Complex XML metadata parsing - keeping together for clarity
fn parse_title_info(reader: &mut Reader<&[u8]>, metadata: &mut EbookMetadata) -> Result<()> {
    let mut buf = Vec::new();
    let mut current_author = Fb2Author::default();
    let mut current_translator = Fb2Author::default();
    let mut in_author = false;
    let mut in_translator = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = e.name();
                let tag_name = std::str::from_utf8(name.as_ref()).unwrap_or("");

                match tag_name {
                    "author" => {
                        in_author = true;
                        current_author = Fb2Author::default();
                    }
                    "translator" => {
                        in_translator = true;
                        current_translator = Fb2Author::default();
                    }
                    "genre" => {
                        if let Some(text) = read_text_content(reader, "genre")? {
                            // FB2 uses specific genre codes (e.g., sf_fantasy, prose_contemporary)
                            // Store the genre code for better classification
                            metadata.subjects.push(format!("Genre: {text}"));
                        }
                    }
                    "book-title" => {
                        metadata.title = read_text_content(reader, "book-title")?;
                    }
                    "annotation" => {
                        let annotation = parse_annotation(reader)?;
                        metadata.description = Some(annotation);
                    }
                    "keywords" => {
                        if let Some(keywords) = read_text_content(reader, "keywords")? {
                            let keyword_list: Vec<String> = keywords
                                .split(',')
                                .map(|s| s.trim().to_string())
                                .filter(|s| !s.is_empty())
                                .collect();
                            metadata.subjects.extend(keyword_list);
                        }
                    }
                    "date" => {
                        metadata.date = read_text_content(reader, "date")?;
                    }
                    "lang" => {
                        metadata.language = read_text_content(reader, "lang")?;
                    }
                    "src-lang" => {
                        // Source language (for translations)
                        if let Some(src_lang) = read_text_content(reader, "src-lang")? {
                            metadata
                                .subjects
                                .push(format!("Source language: {src_lang}"));
                        }
                    }
                    "first-name" if in_author => {
                        current_author.first_name = read_text_content(reader, "first-name")?;
                    }
                    "middle-name" if in_author => {
                        current_author.middle_name = read_text_content(reader, "middle-name")?;
                    }
                    "last-name" if in_author => {
                        current_author.last_name = read_text_content(reader, "last-name")?;
                    }
                    "nickname" if in_author => {
                        current_author.nickname = read_text_content(reader, "nickname")?;
                    }
                    "email" if in_author => {
                        current_author.email = read_text_content(reader, "email")?;
                    }
                    "first-name" if in_translator => {
                        current_translator.first_name = read_text_content(reader, "first-name")?;
                    }
                    "middle-name" if in_translator => {
                        current_translator.middle_name = read_text_content(reader, "middle-name")?;
                    }
                    "last-name" if in_translator => {
                        current_translator.last_name = read_text_content(reader, "last-name")?;
                    }
                    "nickname" if in_translator => {
                        current_translator.nickname = read_text_content(reader, "nickname")?;
                    }
                    "email" if in_translator => {
                        current_translator.email = read_text_content(reader, "email")?;
                    }
                    _ => skip_element(reader, tag_name)?,
                }
            }
            Ok(Event::End(e)) => {
                let name = e.name();
                let tag_name = std::str::from_utf8(name.as_ref()).unwrap_or("");

                if tag_name == "author" {
                    in_author = false;
                    metadata.creators.push(current_author.to_string());
                } else if tag_name == "translator" {
                    in_translator = false;
                    // Add translator to contributors with prefix
                    metadata
                        .contributors
                        .push(format!("Translator: {current_translator}"));
                } else if tag_name == "title-info" {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(EbookError::ParseError(format!(
                    "Error parsing title-info: {e}"
                )))
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(())
}

/// Parse `<document-info>` element (document metadata)
fn parse_document_info(reader: &mut Reader<&[u8]>, metadata: &mut EbookMetadata) -> Result<()> {
    let mut buf = Vec::new();
    let mut doc_author = Fb2Author::default();
    let mut in_author = false;
    let mut doc_info_parts: Vec<String> = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = e.name();
                let tag_name = std::str::from_utf8(name.as_ref()).unwrap_or("");

                match tag_name {
                    "author" => {
                        in_author = true;
                        doc_author = Fb2Author::default();
                    }
                    "program-used" => {
                        if let Some(program) = read_text_content(reader, "program-used")? {
                            doc_info_parts.push(format!("Created with: {program}"));
                        }
                    }
                    "date" => {
                        // Document creation date (may have 'value' attribute for machine-readable date)
                        if let Some(date_text) = read_text_content(reader, "date")? {
                            doc_info_parts.push(format!("Document date: {date_text}"));
                        }
                    }
                    "src-url" => {
                        if let Some(url) = read_text_content(reader, "src-url")? {
                            doc_info_parts.push(format!("Source: {url}"));
                        }
                    }
                    "id" => {
                        metadata.identifier = read_text_content(reader, "id")?;
                    }
                    "version" => {
                        if let Some(version) = read_text_content(reader, "version")? {
                            doc_info_parts.push(format!("Version: {version}"));
                        }
                    }
                    "history" => {
                        let history = parse_history(reader)?;
                        if !history.is_empty() {
                            doc_info_parts.push(format!("History:\n{history}"));
                        }
                    }
                    "first-name" if in_author => {
                        doc_author.first_name = read_text_content(reader, "first-name")?;
                    }
                    "middle-name" if in_author => {
                        doc_author.middle_name = read_text_content(reader, "middle-name")?;
                    }
                    "last-name" if in_author => {
                        doc_author.last_name = read_text_content(reader, "last-name")?;
                    }
                    "nickname" if in_author => {
                        doc_author.nickname = read_text_content(reader, "nickname")?;
                    }
                    "email" if in_author => {
                        doc_author.email = read_text_content(reader, "email")?;
                    }
                    _ => skip_element(reader, tag_name)?,
                }
            }
            Ok(Event::End(e)) => {
                let name = e.name();
                let tag_name = std::str::from_utf8(name.as_ref()).unwrap_or("");

                if tag_name == "author" {
                    in_author = false;
                    let author_str = doc_author.to_string();
                    if author_str != "Unknown Author" {
                        metadata
                            .contributors
                            .push(format!("Document author: {author_str}"));
                    }
                } else if tag_name == "document-info" {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(EbookError::ParseError(format!(
                    "Error parsing document-info: {e}"
                )))
            }
            _ => {}
        }
        buf.clear();
    }

    // Append document-info parts to description (as plain text, not markdown)
    if !doc_info_parts.is_empty() {
        let doc_info = format!("\n\n{}", doc_info_parts.join("\n"));
        if let Some(ref mut desc) = metadata.description {
            desc.push_str(&doc_info);
        } else {
            metadata.description = Some(doc_info.trim().to_string());
        }
    }

    Ok(())
}

/// Parse `<history>` element (version history)
fn parse_history(reader: &mut Reader<&[u8]>) -> Result<String> {
    let mut paragraphs = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = e.name();
                let tag_name = std::str::from_utf8(name.as_ref()).unwrap_or("");

                if tag_name == "p" {
                    if let Some(text) = read_text_content(reader, "p")? {
                        paragraphs.push(format!("  - {text}"));
                    }
                } else {
                    skip_element(reader, tag_name)?;
                }
            }
            Ok(Event::End(e)) => {
                let name = e.name();
                let tag_name = std::str::from_utf8(name.as_ref()).unwrap_or("");
                if tag_name == "history" {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(EbookError::ParseError(format!(
                    "Error parsing history: {e}"
                )))
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(paragraphs.join("\n"))
}

/// Parse `<publish-info>` element (publication metadata)
fn parse_publish_info(reader: &mut Reader<&[u8]>, metadata: &mut EbookMetadata) -> Result<()> {
    let mut buf = Vec::new();
    let mut publisher_name = None;
    let mut city = None;
    let mut year = None;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = e.name();
                let tag_name = std::str::from_utf8(name.as_ref()).unwrap_or("");

                match tag_name {
                    "book-name" => {
                        // Alternative book title in publish-info
                        // We already have title, so skip or could compare
                        skip_element(reader, "book-name")?;
                    }
                    "publisher" => {
                        publisher_name = read_text_content(reader, "publisher")?;
                    }
                    "city" => {
                        city = read_text_content(reader, "city")?;
                    }
                    "year" => {
                        year = read_text_content(reader, "year")?;
                    }
                    "isbn" => {
                        // Store ISBN in identifier if not already set
                        if metadata.identifier.is_none() {
                            metadata.identifier = read_text_content(reader, "isbn")?;
                        } else {
                            skip_element(reader, "isbn")?;
                        }
                    }
                    _ => skip_element(reader, tag_name)?,
                }
            }
            Ok(Event::End(e)) => {
                let name = e.name();
                let tag_name = std::str::from_utf8(name.as_ref()).unwrap_or("");
                if tag_name == "publish-info" {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(EbookError::ParseError(format!(
                    "Error parsing publish-info: {e}"
                )))
            }
            _ => {}
        }
        buf.clear();
    }

    // Construct detailed publisher string
    let mut publisher_parts = Vec::new();
    if let Some(name) = publisher_name {
        publisher_parts.push(name);
    }
    if let Some(c) = city {
        publisher_parts.push(c);
    }
    if let Some(y) = year {
        publisher_parts.push(y);
    }

    if !publisher_parts.is_empty() {
        metadata.publisher = Some(publisher_parts.join(", "));
    }

    Ok(())
}

/// Parse `<annotation>` element (book description with paragraphs)
fn parse_annotation(reader: &mut Reader<&[u8]>) -> Result<String> {
    let mut paragraphs = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = e.name();
                let tag_name = std::str::from_utf8(name.as_ref()).unwrap_or("");

                if tag_name == "p" {
                    if let Some(text) = read_text_content(reader, "p")? {
                        paragraphs.push(text);
                    }
                } else {
                    skip_element(reader, tag_name)?;
                }
            }
            Ok(Event::End(e)) => {
                let name = e.name();
                let tag_name = std::str::from_utf8(name.as_ref()).unwrap_or("");
                if tag_name == "annotation" {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(EbookError::ParseError(format!(
                    "Error parsing annotation: {e}"
                )))
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(paragraphs.join("\n\n"))
}

/// Parse `<body>` element (content sections)
/// Returns (`body_title`, sections)
fn parse_body(reader: &mut Reader<&[u8]>) -> Result<(Option<String>, Vec<Fb2Section>)> {
    let mut buf = Vec::new();
    let mut body_title: Option<String> = None;
    let mut body_epigraph: Option<String> = None;
    let mut sections = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = e.name();
                let tag_name = std::str::from_utf8(name.as_ref()).unwrap_or("");

                match tag_name {
                    "title" => {
                        // Parse body title (title page, may include subtitle)
                        body_title = parse_section_title(reader)?;
                    }
                    "epigraph" => {
                        let epigraph = parse_epigraph(reader)?;
                        if !epigraph.is_empty() {
                            body_epigraph = Some(epigraph);
                        }
                    }
                    "section" => {
                        let section = parse_section(reader)?;
                        sections.push(section);
                    }
                    _ => skip_element(reader, tag_name)?,
                }
            }
            Ok(Event::End(e)) => {
                let name = e.name();
                let tag_name = std::str::from_utf8(name.as_ref()).unwrap_or("");
                if tag_name == "body" {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(EbookError::ParseError(format!("Error parsing body: {e}"))),
            _ => {}
        }
        buf.clear();
    }

    // If body has epigraph (without title), create a prologue section
    if body_epigraph.is_some() {
        let mut prologue = Fb2Section::new();
        prologue.title = Some("Prologue".to_string());

        if let Some(epigraph) = body_epigraph {
            prologue.content.push(format!("> {epigraph}"));
        }

        // Insert prologue at the beginning
        sections.insert(0, prologue);
    }

    Ok((body_title, sections))
}

/// Parse `<section>` element (recursive)
fn parse_section(reader: &mut Reader<&[u8]>) -> Result<Fb2Section> {
    let mut section = Fb2Section::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = e.name();
                let tag_name = std::str::from_utf8(name.as_ref()).unwrap_or("");

                match tag_name {
                    "title" => {
                        section.title = parse_section_title(reader)?;
                    }
                    "subtitle" => {
                        // Subtitle is like a smaller heading
                        if let Some(text) = read_text_content(reader, "subtitle")? {
                            section.content.push(format!("### {text}"));
                        }
                    }
                    "epigraph" => {
                        let epigraph = parse_epigraph(reader)?;
                        if !epigraph.is_empty() {
                            section.content.push(format!("> {epigraph}"));
                        }
                    }
                    "poem" => {
                        let poem = parse_poem(reader)?;
                        if !poem.is_empty() {
                            section.content.push(poem);
                        }
                    }
                    "p" => {
                        if let Some(text) = parse_paragraph(reader)? {
                            section.content.push(text);
                        }
                    }
                    "section" => {
                        let subsection = parse_section(reader)?;
                        section.subsections.push(subsection);
                    }
                    "empty-line" => {
                        section.content.push(String::new());
                    }
                    _ => skip_element(reader, tag_name)?,
                }
            }
            Ok(Event::End(e)) => {
                let name = e.name();
                let tag_name = std::str::from_utf8(name.as_ref()).unwrap_or("");
                if tag_name == "section" {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(EbookError::ParseError(format!(
                    "Error parsing section: {e}"
                )))
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(section)
}

/// Parse `<title>` element within section
fn parse_section_title(reader: &mut Reader<&[u8]>) -> Result<Option<String>> {
    let mut title_parts = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = e.name();
                let tag_name = std::str::from_utf8(name.as_ref()).unwrap_or("");

                if tag_name == "p" {
                    if let Some(text) = read_text_content(reader, "p")? {
                        title_parts.push(text);
                    }
                } else {
                    skip_element(reader, tag_name)?;
                }
            }
            Ok(Event::End(e)) => {
                let name = e.name();
                let tag_name = std::str::from_utf8(name.as_ref()).unwrap_or("");
                if tag_name == "title" {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(EbookError::ParseError(format!("Error parsing title: {e}"))),
            _ => {}
        }
        buf.clear();
    }

    if title_parts.is_empty() {
        Ok(None)
    } else {
        Ok(Some(title_parts.join(" ")))
    }
}

/// Parse `<p>` element with inline formatting
fn parse_paragraph(reader: &mut Reader<&[u8]>) -> Result<Option<String>> {
    let mut text = String::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = e.name();
                let tag_name = std::str::from_utf8(name.as_ref()).unwrap_or("");

                match tag_name {
                    "strong" => {
                        if let Some(content) = read_text_content(reader, "strong")? {
                            let _ = write!(text, "**{content}**");
                        }
                    }
                    "emphasis" => {
                        if let Some(content) = read_text_content(reader, "emphasis")? {
                            let _ = write!(text, "*{content}*");
                        }
                    }
                    "strikethrough" => {
                        if let Some(content) = read_text_content(reader, "strikethrough")? {
                            let _ = write!(text, "~~{content}~~");
                        }
                    }
                    "code" => {
                        if let Some(content) = read_text_content(reader, "code")? {
                            let _ = write!(text, "`{content}`");
                        }
                    }
                    "sub" => {
                        if let Some(content) = read_text_content(reader, "sub")? {
                            let _ = write!(text, "_{{{content}}}");
                        }
                    }
                    "sup" => {
                        if let Some(content) = read_text_content(reader, "sup")? {
                            let _ = write!(text, "^{{{content}}}");
                        }
                    }
                    _ => skip_element(reader, tag_name)?,
                }
            }
            Ok(Event::Text(e)) => {
                if let Ok(content) = e.unescape() {
                    text.push_str(&content);
                }
            }
            Ok(Event::End(e)) => {
                let name = e.name();
                let tag_name = std::str::from_utf8(name.as_ref()).unwrap_or("");
                if tag_name == "p" {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(EbookError::ParseError(format!(
                    "Error parsing paragraph: {e}"
                )))
            }
            _ => {}
        }
        buf.clear();
    }

    let trimmed = text.trim();
    if trimmed.is_empty() {
        Ok(None)
    } else {
        Ok(Some(trimmed.to_string()))
    }
}

/// Parse `<binary>` element (base64 image)
fn parse_binary(
    reader: &mut Reader<&[u8]>,
    start_tag: &quick_xml::events::BytesStart,
) -> Result<Option<Fb2Image>> {
    // Extract attributes
    let mut id = None;
    let mut content_type = None;

    for attr in start_tag.attributes().flatten() {
        let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
        let value = std::str::from_utf8(&attr.value).unwrap_or("");

        match key {
            "id" => id = Some(value.to_string()),
            "content-type" => content_type = Some(value.to_string()),
            _ => {}
        }
    }

    // Extract required fields or skip element
    let (Some(id), Some(content_type)) = (id, content_type) else {
        skip_element(reader, "binary")?;
        return Ok(None);
    };

    // Read base64 content
    let mut base64_content = String::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Text(e)) => {
                if let Ok(text) = e.unescape() {
                    base64_content.push_str(&text);
                }
            }
            Ok(Event::End(e)) => {
                let name = e.name();
                let tag_name = std::str::from_utf8(name.as_ref()).unwrap_or("");
                if tag_name == "binary" {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(EbookError::ParseError(format!("Error parsing binary: {e}"))),
            _ => {}
        }
        buf.clear();
    }

    // Decode base64
    let data = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        base64_content.trim(),
    )
    .map_err(|e| EbookError::ParseError(format!("Invalid base64 image data: {e}")))?;

    Ok(Some(Fb2Image {
        id,
        _content_type: content_type,
        _data: data,
    }))
}

/// Read text content of an element
fn read_text_content(reader: &mut Reader<&[u8]>, tag_name: &str) -> Result<Option<String>> {
    let mut text = String::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Text(e)) => {
                if let Ok(content) = e.unescape() {
                    text.push_str(&content);
                }
            }
            Ok(Event::End(e)) => {
                let name = e.name();
                let end_tag = std::str::from_utf8(name.as_ref()).unwrap_or("");
                if end_tag == tag_name {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(EbookError::ParseError(format!(
                    "Error reading text content: {e}"
                )))
            }
            _ => {}
        }
        buf.clear();
    }

    let trimmed = text.trim();
    if trimmed.is_empty() {
        Ok(None)
    } else {
        Ok(Some(trimmed.to_string()))
    }
}

/// Parse `<epigraph>` element (quotation with optional author)
fn parse_epigraph(reader: &mut Reader<&[u8]>) -> Result<String> {
    let mut parts = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = e.name();
                let tag_name = std::str::from_utf8(name.as_ref()).unwrap_or("");

                match tag_name {
                    "p" => {
                        if let Some(text) = read_text_content(reader, "p")? {
                            parts.push(text);
                        }
                    }
                    "poem" => {
                        let poem = parse_poem(reader)?;
                        if !poem.is_empty() {
                            parts.push(poem);
                        }
                    }
                    "text-author" => {
                        if let Some(author) = read_text_content(reader, "text-author")? {
                            parts.push(format!("\n— {author}"));
                        }
                    }
                    _ => skip_element(reader, tag_name)?,
                }
            }
            Ok(Event::End(e)) => {
                let name = e.name();
                let tag_name = std::str::from_utf8(name.as_ref()).unwrap_or("");
                if tag_name == "epigraph" {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(EbookError::ParseError(format!(
                    "Error parsing epigraph: {e}"
                )))
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(parts.join("\n> "))
}

/// Parse `<poem>` element (poetry with stanzas and verses)
fn parse_poem(reader: &mut Reader<&[u8]>) -> Result<String> {
    let mut parts = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = e.name();
                let tag_name = std::str::from_utf8(name.as_ref()).unwrap_or("");

                match tag_name {
                    "title" => {
                        if let Some(title) = parse_section_title(reader)? {
                            parts.push(format!("**{title}**\n"));
                        }
                    }
                    "stanza" => {
                        let stanza = parse_stanza(reader)?;
                        if !stanza.is_empty() {
                            parts.push(stanza);
                        }
                    }
                    "text-author" => {
                        if let Some(author) = read_text_content(reader, "text-author")? {
                            parts.push(format!("\n— {author}"));
                        }
                    }
                    "p" => {
                        // Some poems have paragraph descriptions
                        if let Some(text) = read_text_content(reader, "p")? {
                            parts.push(text);
                        }
                    }
                    _ => skip_element(reader, tag_name)?,
                }
            }
            Ok(Event::End(e)) => {
                let name = e.name();
                let tag_name = std::str::from_utf8(name.as_ref()).unwrap_or("");
                if tag_name == "poem" {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(EbookError::ParseError(format!("Error parsing poem: {e}"))),
            _ => {}
        }
        buf.clear();
    }

    Ok(parts.join("\n\n"))
}

/// Parse `<stanza>` element (group of verses in a poem)
fn parse_stanza(reader: &mut Reader<&[u8]>) -> Result<String> {
    let mut verses = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = e.name();
                let tag_name = std::str::from_utf8(name.as_ref()).unwrap_or("");

                if tag_name == "v" {
                    // Verse line
                    if let Some(verse) = read_text_content(reader, "v")? {
                        verses.push(verse);
                    }
                } else {
                    skip_element(reader, tag_name)?;
                }
            }
            Ok(Event::End(e)) => {
                let name = e.name();
                let tag_name = std::str::from_utf8(name.as_ref()).unwrap_or("");
                if tag_name == "stanza" {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(EbookError::ParseError(format!("Error parsing stanza: {e}"))),
            _ => {}
        }
        buf.clear();
    }

    Ok(verses.join("\n"))
}

/// Skip an element and all its children
fn skip_element(reader: &mut Reader<&[u8]>, tag_name: &str) -> Result<()> {
    let mut depth = 1;
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = e.name();
                let start_tag = std::str::from_utf8(name.as_ref()).unwrap_or("");
                if start_tag == tag_name {
                    depth += 1;
                }
            }
            Ok(Event::End(e)) => {
                let name = e.name();
                let end_tag = std::str::from_utf8(name.as_ref()).unwrap_or("");
                if end_tag == tag_name {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(EbookError::ParseError(format!(
                    "Error skipping element: {e}"
                )))
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(())
}

/// Convert internal FB2 structure to `ParsedEbook`
fn convert_to_parsed_ebook(parsed: ParsedFb2) -> ParsedEbook {
    let mut ebook = ParsedEbook::new(parsed.metadata);

    // Set body title (title page content, may include subtitle)
    ebook.body_title = parsed.body_title;

    // Convert sections to chapters
    let mut chapter_order = 0;

    for section in &parsed.sections {
        flatten_sections(
            section,
            &mut ebook.chapters,
            &mut ebook.toc,
            &mut chapter_order,
        );
    }

    // Add notes as final chapter if present
    if !parsed.notes.is_empty() {
        let notes_content = serialize_sections(&parsed.notes, 0);
        ebook.chapters.push(Chapter {
            title: Some("Notes".to_string()),
            content: notes_content,
            href: "notes".to_string(),
            spine_order: chapter_order,
        });
    }

    ebook
}

/// Flatten FB2 sections into chapters and TOC
fn flatten_sections(
    section: &Fb2Section,
    chapters: &mut Vec<Chapter>,
    toc: &mut Vec<TocEntry>,
    chapter_order: &mut usize,
) {
    // Create chapter from this section
    let title = section
        .title
        .clone()
        .or_else(|| Some(format!("Chapter {}", *chapter_order + 1)));
    let href = format!("section_{chapter_order}");

    // Serialize section content
    let mut content = String::new();

    // Note: Title is NOT added to content here - it's added by the backend
    // as a SectionHeader DocItem for consistency across all ebook formats.
    // This prevents duplicate titles in the output.

    // Add paragraphs
    for paragraph in &section.content {
        if paragraph.is_empty() {
            content.push('\n');
        } else {
            content.push_str(paragraph);
            content.push_str("\n\n");
        }
    }

    // Add subsections
    if !section.subsections.is_empty() {
        let subsections_md = serialize_sections(&section.subsections, 2);
        content.push_str(&subsections_md);
    }

    // Create chapter
    chapters.push(Chapter {
        title: title.clone(),
        content,
        href: href.clone(),
        spine_order: *chapter_order,
    });

    // Create TOC entry
    toc.push(TocEntry {
        label: title.unwrap_or_else(|| format!("Chapter {}", *chapter_order + 1)),
        href,
        play_order: Some(*chapter_order),
        children: Vec::new(),
    });

    *chapter_order += 1;
}

/// Serialize sections to markdown (recursive)
fn serialize_sections(sections: &[Fb2Section], level: usize) -> String {
    let mut md = String::new();
    let heading_prefix = "#".repeat(level.max(1));

    for section in sections {
        // Section title
        if let Some(ref title) = section.title {
            let _ = writeln!(md, "{heading_prefix} {title}\n");
        }

        // Section content
        for paragraph in &section.content {
            if paragraph.is_empty() {
                md.push('\n');
            } else {
                md.push_str(paragraph);
                md.push_str("\n\n");
            }
        }

        // Subsections
        if !section.subsections.is_empty() {
            let subsections_md = serialize_sections(&section.subsections, level + 1);
            md.push_str(&subsections_md);
        }
    }

    md
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fb2_author_to_string() {
        let author = Fb2Author {
            first_name: Some("Leo".to_string()),
            middle_name: None,
            last_name: Some("Tolstoy".to_string()),
            nickname: None,
            email: None,
        };
        assert_eq!(author.to_string(), "Leo Tolstoy");

        let author_with_nickname = Fb2Author {
            first_name: Some("Leo".to_string()),
            last_name: Some("Tolstoy".to_string()),
            nickname: Some("Count".to_string()),
            middle_name: None,
            email: None,
        };
        assert_eq!(author_with_nickname.to_string(), "Count");
    }

    #[test]
    fn test_parse_simple_fb2() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<FictionBook xmlns="http://www.gribuser.ru/xml/fictionbook/2.0">
  <description>
    <title-info>
      <genre>prose</genre>
      <author>
        <first-name>Test</first-name>
        <last-name>Author</last-name>
      </author>
      <book-title>Test Book</book-title>
      <lang>en</lang>
    </title-info>
    <document-info>
      <id>test-123</id>
    </document-info>
  </description>
  <body>
    <section>
      <title><p>Chapter 1</p></title>
      <p>First paragraph.</p>
      <p>Second paragraph.</p>
    </section>
  </body>
</FictionBook>"#;

        let result = parse_fb2_xml(xml).unwrap();

        assert_eq!(result.metadata.title, Some("Test Book".to_string()));
        assert_eq!(result.metadata.creators, vec!["Test Author"]);
        assert_eq!(result.metadata.language, Some("en".to_string()));
        assert_eq!(result.metadata.identifier, Some("test-123".to_string()));
        assert_eq!(result.chapters.len(), 1);
        assert_eq!(result.chapters[0].title, Some("Chapter 1".to_string()));
    }

    #[test]
    fn test_parse_inline_formatting() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<FictionBook xmlns="http://www.gribuser.ru/xml/fictionbook/2.0">
  <description>
    <title-info>
      <book-title>Formatting Test</book-title>
      <lang>en</lang>
      <author><first-name>Test</first-name></author>
    </title-info>
    <document-info><id>test</id></document-info>
  </description>
  <body>
    <section>
      <p>Text with <strong>bold</strong> and <emphasis>italic</emphasis>.</p>
    </section>
  </body>
</FictionBook>"#;

        let result = parse_fb2_xml(xml).unwrap();
        assert_eq!(result.chapters.len(), 1);
        assert!(result.chapters[0].content.contains("**bold**"));
        assert!(result.chapters[0].content.contains("*italic*"));
    }

    #[test]
    fn test_parse_nested_sections() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<FictionBook xmlns="http://www.gribuser.ru/xml/fictionbook/2.0">
  <description>
    <title-info>
      <book-title>Nested Test</book-title>
      <lang>en</lang>
      <author><first-name>Test</first-name></author>
    </title-info>
    <document-info><id>test</id></document-info>
  </description>
  <body>
    <section>
      <title><p>Chapter 1</p></title>
      <p>Chapter content.</p>
      <section>
        <title><p>Section 1.1</p></title>
        <p>Subsection content.</p>
      </section>
    </section>
  </body>
</FictionBook>"#;

        let result = parse_fb2_xml(xml).unwrap();
        assert_eq!(result.chapters.len(), 1);
        // Top-level title is stored in chapter.title (not in content) to avoid duplication
        assert_eq!(result.chapters[0].title, Some("Chapter 1".to_string()));
        // Subsections still have their titles in content
        assert!(result.chapters[0].content.contains("## Section 1.1"));
    }
}
