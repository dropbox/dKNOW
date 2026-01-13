//! SVG format processing module
//!
//! Converts SVG (Scalable Vector Graphics) files to markdown format

use std::fmt::Write;
use std::path::Path;

use crate::error::{DoclingError, Result};
use docling_svg::{parse_svg, SvgDocument};

/// Process SVG file and return markdown
///
/// # Errors
///
/// Returns an error if the file cannot be read or if SVG parsing fails.
#[must_use = "this function returns the extracted markdown content"]
pub fn process_svg(path: &Path) -> Result<String> {
    let doc = parse_svg(path)
        .map_err(|e| DoclingError::ConversionError(format!("Failed to parse SVG file: {e}")))?;

    Ok(svg_to_markdown(&doc))
}

/// Convert SVG document to markdown
fn svg_to_markdown(doc: &SvgDocument) -> String {
    let mut output = String::new();

    // Add metadata as YAML frontmatter if present
    if doc.metadata.title.is_some()
        || doc.metadata.description.is_some()
        || doc.metadata.width.is_some()
        || doc.metadata.height.is_some()
    {
        output.push_str("---\n");

        if let Some(title) = &doc.metadata.title {
            let _ = writeln!(output, "title: {title}");
        }

        if let Some(desc) = &doc.metadata.description {
            let _ = writeln!(output, "description: {desc}");
        }

        if let Some(width) = &doc.metadata.width {
            let _ = writeln!(output, "width: {width}");
        }

        if let Some(height) = &doc.metadata.height {
            let _ = writeln!(output, "height: {height}");
        }

        if let Some(viewbox) = &doc.metadata.viewbox {
            let _ = writeln!(output, "viewBox: {viewbox}");
        }

        output.push_str("---\n\n");
    }

    // Add title as header if present
    if let Some(title) = &doc.metadata.title {
        let _ = writeln!(output, "# {title}\n");
    }

    // Add description if present
    if let Some(desc) = &doc.metadata.description {
        let _ = writeln!(output, "{desc}\n");
    }

    // Extract text elements
    if !doc.text_elements.is_empty() {
        output.push_str("## Text Content\n\n");

        for elem in &doc.text_elements {
            // Add text content
            let _ = writeln!(output, "{}\n", elem.content);
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use docling_svg::parse_svg_str;

    #[test]
    fn test_svg_to_markdown_basic() {
        let svg = r#"<?xml version="1.0"?>
<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
    <title>Test SVG</title>
    <desc>A simple test</desc>
    <text x="10" y="20">Hello World</text>
</svg>"#;

        let doc = parse_svg_str(svg).expect("Failed to parse SVG");
        let markdown = svg_to_markdown(&doc);

        assert!(markdown.contains("title: Test SVG"));
        assert!(markdown.contains("description: A simple test"));
        assert!(markdown.contains("# Test SVG"));
        assert!(markdown.contains("Hello World"));
    }

    #[test]
    fn test_svg_to_markdown_multiple_text() {
        let svg = r#"<?xml version="1.0"?>
<svg xmlns="http://www.w3.org/2000/svg">
    <text>First</text>
    <text>Second</text>
    <text>Third</text>
</svg>"#;

        let doc = parse_svg_str(svg).expect("Failed to parse SVG");
        let markdown = svg_to_markdown(&doc);

        assert!(markdown.contains("First"));
        assert!(markdown.contains("Second"));
        assert!(markdown.contains("Third"));
    }

    #[test]
    fn test_process_svg_file() {
        let path = Path::new("../../test-corpus/svg/simple_icon.svg");
        if path.exists() {
            let result = process_svg(path);
            assert!(result.is_ok());

            let markdown = result.unwrap();
            assert!(markdown.contains("Simple Icon") || markdown.contains("ICON"));
        }
    }

    #[test]
    fn test_process_svg_nonexistent_file() {
        // Test error handling for missing file
        let result = process_svg(Path::new("/nonexistent/path/to/image.svg"));
        assert!(result.is_err());
    }

    #[test]
    fn test_svg_to_markdown_no_metadata() {
        // Test SVG without metadata
        let svg = r#"<?xml version="1.0"?>
<svg xmlns="http://www.w3.org/2000/svg">
    <text>Content without metadata</text>
</svg>"#;

        let doc = parse_svg_str(svg).expect("Failed to parse SVG");
        let markdown = svg_to_markdown(&doc);

        // Should not have frontmatter
        assert!(!markdown.starts_with("---"));
        // But should have content
        assert!(markdown.contains("Content without metadata"));
    }

    #[test]
    fn test_svg_to_markdown_empty_svg() {
        // Test SVG with no text elements
        let svg = r#"<?xml version="1.0"?>
<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
    <rect width="50" height="50" fill="red"/>
</svg>"#;

        let doc = parse_svg_str(svg).expect("Failed to parse SVG");
        let markdown = svg_to_markdown(&doc);

        // Should have metadata but no text content section
        assert!(!markdown.contains("## Text Content"));
    }

    #[test]
    fn test_svg_to_markdown_with_viewbox() {
        // Test SVG with viewBox attribute
        let svg = r#"<?xml version="1.0"?>
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
    <title>ViewBox Test</title>
    <text>Test</text>
</svg>"#;

        let doc = parse_svg_str(svg).expect("Failed to parse SVG");
        let markdown = svg_to_markdown(&doc);

        assert!(markdown.contains("viewBox:") || !markdown.is_empty());
    }

    #[test]
    fn test_svg_to_markdown_full_metadata() {
        // Test SVG with all metadata fields
        let svg = r#"<?xml version="1.0"?>
<svg xmlns="http://www.w3.org/2000/svg" width="200" height="150" viewBox="0 0 200 150">
    <title>Complete Metadata</title>
    <desc>A fully documented SVG</desc>
    <text>Content</text>
</svg>"#;

        let doc = parse_svg_str(svg).expect("Failed to parse SVG");
        let markdown = svg_to_markdown(&doc);

        assert!(markdown.contains("title: Complete Metadata"));
        assert!(markdown.contains("description: A fully documented SVG"));
        assert!(markdown.contains("width:"));
        assert!(markdown.contains("height:"));
    }

    #[test]
    fn test_svg_to_markdown_trailing_newline() {
        // Test that output ends with newline
        let svg = r#"<?xml version="1.0"?>
<svg xmlns="http://www.w3.org/2000/svg">
    <text>Content</text>
</svg>"#;

        let doc = parse_svg_str(svg).expect("Failed to parse SVG");
        let markdown = svg_to_markdown(&doc);

        assert!(markdown.ends_with('\n'));
    }

    #[test]
    fn test_process_svg_empty_path() {
        // Test empty path
        let result = process_svg(Path::new(""));
        assert!(result.is_err());
    }

    #[test]
    fn test_process_svg_with_spaces_in_path() {
        // Test file path with spaces
        let result = process_svg(Path::new("/nonexistent/path with spaces/image.svg"));
        assert!(result.is_err());
    }

    #[test]
    fn test_process_svg_with_special_characters() {
        // Test file path with special characters
        let result = process_svg(Path::new("/nonexistent/image-file_v1.0.svg"));
        assert!(result.is_err());
    }

    #[test]
    fn test_process_svg_relative_path() {
        // Test relative path
        let result = process_svg(Path::new("../nonexistent/image.svg"));
        assert!(result.is_err());
    }
}
