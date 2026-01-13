//! Common XML parsing utilities for `OpenDocument` formats

use crate::error::{OdfError, Result};
use quick_xml::events::Event;
use quick_xml::Reader;
use std::io::Read;
use zip::ZipArchive;

/// Extract a file from the ZIP archive
///
/// # Errors
///
/// Returns an error if the file is not found in the archive (`MissingFile`)
/// or if reading the file content fails (I/O error).
#[must_use = "this function returns extracted file bytes that should be processed"]
pub fn extract_file<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
    filename: &str,
) -> Result<Vec<u8>> {
    let mut file = archive
        .by_name(filename)
        .map_err(|_| OdfError::MissingFile(filename.to_string()))?;

    let mut content = Vec::new();
    file.read_to_end(&mut content)?;
    Ok(content)
}

/// Extract a file as a UTF-8 string
///
/// # Errors
///
/// Returns an error if the file is not found in the archive (`MissingFile`),
/// if reading the file content fails (I/O error), or if the content is not valid UTF-8.
#[must_use = "this function returns extracted file content that should be processed"]
pub fn extract_file_as_string<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
    filename: &str,
) -> Result<String> {
    let bytes = extract_file(archive, filename)?;
    String::from_utf8(bytes).map_err(std::convert::Into::into)
}

/// Parse XML and call handler for each element
///
/// # Errors
///
/// Returns an error if the XML is malformed or if the handler function returns an error.
#[must_use = "this function returns a Result that should be checked for errors"]
pub fn parse_xml<F>(xml_content: &str, mut handler: F) -> Result<()>
where
    F: FnMut(&Event) -> Result<()>,
{
    let mut reader = Reader::from_str(xml_content);
    reader.trim_text(true);

    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Eof) => break,
            Ok(ref e) => handler(e)?,
            Err(e) => return Err(e.into()),
        }
        buf.clear();
    }
    Ok(())
}

/// Get text content from XML events
///
/// # Errors
///
/// Returns an error if the XML is malformed or if text unescaping fails.
#[must_use = "this function returns extracted text that should be used"]
pub fn get_text_content(reader: &mut Reader<&[u8]>, buf: &mut Vec<u8>) -> Result<String> {
    let mut text = String::new();
    loop {
        match reader.read_event_into(buf) {
            Ok(Event::Text(e)) => {
                text.push_str(&e.unescape()?);
            }
            Ok(Event::End(_) | Event::Eof) => break,
            Ok(_) => {}
            Err(e) => return Err(e.into()),
        }
        buf.clear();
    }
    Ok(text)
}

/// Check if element has a specific qualified name (namespace:localname)
#[inline]
#[must_use = "checks if event matches qualified name"]
pub fn is_element(event: &Event, qualified_name: &[u8]) -> bool {
    match event {
        Event::Start(e) | Event::Empty(e) => e.name().as_ref() == qualified_name,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_xml_basic() {
        let xml = r"<root><child>text</child></root>";
        let mut count = 0;
        parse_xml(xml, |_| {
            count += 1;
            Ok(())
        })
        .unwrap();
        assert!(count > 0);
    }

    #[test]
    fn test_is_element() {
        let xml = r"<text:p>Hello</text:p>";
        let mut reader = Reader::from_str(xml);
        let mut buf = Vec::new();
        let event = reader.read_event_into(&mut buf).unwrap();
        assert!(is_element(&event, b"text:p"));
    }
}
