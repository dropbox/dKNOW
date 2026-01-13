/// Email format backend for docling-core
///
/// Processes EML and other email formats into markdown documents
use std::path::Path;

use docling_email::{
    email_to_markdown, mbox_to_markdown, msg_to_markdown, parse_eml, parse_mbox,
    parse_msg_from_path, parse_vcf, vcf_to_markdown,
};

use crate::error::{DoclingError, Result};

/// Process an EML file into markdown
///
/// # Errors
/// Returns an error if the file cannot be read or parsed as valid EML.
#[must_use = "this function returns the extracted markdown content"]
pub fn process_eml<P: AsRef<Path>>(path: P) -> Result<String> {
    // Read EML file bytes
    let bytes = std::fs::read(path.as_ref())?;

    // Parse EML using docling-email
    let parsed = parse_eml(&bytes)
        .map_err(|e| DoclingError::ConversionError(format!("Failed to parse EML: {e}")))?;

    // Convert to markdown
    Ok(email_to_markdown(&parsed))
}

/// Process an MBOX file into markdown
///
/// # Errors
/// Returns an error if the file cannot be read or parsed as valid MBOX.
#[must_use = "this function returns the extracted markdown content"]
pub fn process_mbox<P: AsRef<Path>>(path: P) -> Result<String> {
    // Read MBOX file bytes
    let bytes = std::fs::read(path.as_ref())?;

    // Parse MBOX using docling-email
    let parsed = parse_mbox(&bytes)
        .map_err(|e| DoclingError::ConversionError(format!("Failed to parse MBOX: {e}")))?;

    // Convert to markdown
    Ok(mbox_to_markdown(&parsed))
}

/// Process a VCF (vCard) file into markdown
///
/// # Errors
/// Returns an error if the file cannot be read or parsed as valid VCF.
#[must_use = "this function returns the extracted markdown content"]
pub fn process_vcf<P: AsRef<Path>>(path: P) -> Result<String> {
    // Read VCF file bytes
    let bytes = std::fs::read(path.as_ref())?;

    // Parse VCF using docling-email
    let parsed = parse_vcf(&bytes)
        .map_err(|e| DoclingError::ConversionError(format!("Failed to parse VCF: {e}")))?;

    // Convert to markdown
    Ok(vcf_to_markdown(&parsed))
}

/// Process an MSG (Outlook message) file into markdown
///
/// # Errors
/// Returns an error if the file cannot be parsed as valid MSG.
#[must_use = "this function returns the extracted markdown content"]
pub fn process_msg<P: AsRef<Path>>(path: P) -> Result<String> {
    // Parse MSG using docling-email (msg_parser crate internally)
    let parsed = parse_msg_from_path(path.as_ref())
        .map_err(|e| DoclingError::ConversionError(format!("Failed to parse MSG: {e}")))?;

    // Convert to markdown
    Ok(msg_to_markdown(&parsed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_process_eml_basic() {
        // Create a temporary EML file with unique name (avoids race conditions)
        let eml_content = br#"From: sender@example.com
To: recipient@example.com
Subject: Test Email
Date: Mon, 15 Jan 2024 10:00:00 -0800
MIME-Version: 1.0
Content-Type: text/plain; charset="UTF-8"

Hello, this is a test email.
"#;

        let mut temp_file = NamedTempFile::with_suffix(".eml").unwrap();
        temp_file.write_all(eml_content).unwrap();

        let result = process_eml(temp_file.path());
        assert!(result.is_ok());

        let markdown = result.unwrap();
        assert!(markdown.contains("# Test Email"));
        assert!(markdown.contains("sender@example.com"));
        assert!(markdown.contains("recipient@example.com"));
        assert!(markdown.contains("Hello, this is a test email"));
        // temp_file automatically cleaned up on drop
    }

    #[test]
    fn test_process_eml_nonexistent_file() {
        // Test error handling for missing file
        let result = process_eml("/nonexistent/path/to/email.eml");
        assert!(result.is_err());
    }

    #[test]
    fn test_process_eml_with_html_body() {
        // Test EML with HTML content
        let eml_content = br#"From: sender@example.com
To: recipient@example.com
Subject: HTML Email
Date: Mon, 15 Jan 2024 10:00:00 -0800
MIME-Version: 1.0
Content-Type: text/html; charset="UTF-8"

<html><body><h1>Hello HTML</h1><p>This is HTML content.</p></body></html>
"#;

        let mut temp_file = NamedTempFile::with_suffix(".eml").unwrap();
        temp_file.write_all(eml_content).unwrap();

        let result = process_eml(temp_file.path());
        assert!(result.is_ok());

        let markdown = result.unwrap();
        assert!(markdown.contains("# HTML Email"));
        assert!(!markdown.is_empty());
        // temp_file automatically cleaned up on drop
    }

    #[test]
    fn test_process_eml_with_attachments() {
        // Test EML with multipart/mixed (attachments)
        let eml_content = br#"From: sender@example.com
To: recipient@example.com
Subject: Email with Attachment
Date: Mon, 15 Jan 2024 10:00:00 -0800
MIME-Version: 1.0
Content-Type: multipart/mixed; boundary="boundary123"

--boundary123
Content-Type: text/plain; charset="UTF-8"

Email body with attachment.
--boundary123
Content-Type: application/pdf; name="document.pdf"
Content-Disposition: attachment; filename="document.pdf"

[PDF binary data]
--boundary123--
"#;

        let mut temp_file = NamedTempFile::with_suffix(".eml").unwrap();
        temp_file.write_all(eml_content).unwrap();

        let result = process_eml(temp_file.path());
        assert!(result.is_ok());

        let markdown = result.unwrap();
        assert!(markdown.contains("# Email with Attachment"));
        assert!(!markdown.is_empty());
        // temp_file automatically cleaned up on drop
    }

    #[test]
    fn test_process_mbox_basic() {
        // Test MBOX format (multiple emails)
        let mbox_content = br"From sender@example.com Mon Jan 15 10:00:00 2024
From: sender@example.com
To: recipient@example.com
Subject: First Email
Date: Mon, 15 Jan 2024 10:00:00 -0800

First email body.

From sender2@example.com Mon Jan 15 11:00:00 2024
From: sender2@example.com
To: recipient2@example.com
Subject: Second Email
Date: Mon, 15 Jan 2024 11:00:00 -0800

Second email body.
";

        let mut temp_file = NamedTempFile::with_suffix(".mbox").unwrap();
        temp_file.write_all(mbox_content).unwrap();

        let result = process_mbox(temp_file.path());
        assert!(result.is_ok());

        let markdown = result.unwrap();
        // Should contain both emails
        assert!(markdown.contains("First Email") || markdown.contains("sender@example.com"));
        assert!(!markdown.is_empty());
        // temp_file automatically cleaned up on drop
    }

    #[test]
    fn test_process_mbox_nonexistent_file() {
        // Test error handling for missing MBOX file
        let result = process_mbox("/nonexistent/path/to/mailbox.mbox");
        assert!(result.is_err());
    }

    #[test]
    fn test_process_vcf_basic() {
        // Test vCard format (contact card)
        let vcf_content = br"BEGIN:VCARD
VERSION:3.0
FN:John Doe
N:Doe;John;;;
EMAIL:john.doe@example.com
TEL:+1-555-1234
ORG:Example Corp
TITLE:Software Engineer
END:VCARD
";

        let mut temp_file = NamedTempFile::with_suffix(".vcf").unwrap();
        temp_file.write_all(vcf_content).unwrap();

        let result = process_vcf(temp_file.path());
        assert!(result.is_ok());

        let markdown = result.unwrap();
        assert!(markdown.contains("John Doe") || markdown.contains("john.doe@example.com"));
        assert!(!markdown.is_empty());
        // temp_file automatically cleaned up on drop
    }

    #[test]
    fn test_process_vcf_nonexistent_file() {
        // Test error handling for missing VCF file
        let result = process_vcf("/nonexistent/path/to/contact.vcf");
        assert!(result.is_err());
    }

    #[test]
    fn test_process_vcf_multiple_contacts() {
        // Test vCard with multiple contact cards
        let vcf_content = br"BEGIN:VCARD
VERSION:3.0
FN:John Doe
EMAIL:john@example.com
END:VCARD
BEGIN:VCARD
VERSION:3.0
FN:Jane Smith
EMAIL:jane@example.com
END:VCARD
";

        let mut temp_file = NamedTempFile::with_suffix(".vcf").unwrap();
        temp_file.write_all(vcf_content).unwrap();

        let result = process_vcf(temp_file.path());
        assert!(result.is_ok());

        let markdown = result.unwrap();
        // Should contain both contacts
        assert!(!markdown.is_empty());
        // temp_file automatically cleaned up on drop
    }

    #[test]
    fn test_process_msg_nonexistent_file() {
        // Test error handling for missing MSG file
        let result = process_msg("/nonexistent/path/to/message.msg");
        assert!(result.is_err());
    }

    #[test]
    fn test_eml_trailing_newline() {
        // Test that EML output ends with newline (may have multiple)
        let eml_content = br#"From: sender@example.com
To: recipient@example.com
Subject: Trailing Newline Test
Date: Mon, 15 Jan 2024 10:00:00 -0800
MIME-Version: 1.0
Content-Type: text/plain; charset="UTF-8"

Test content.
"#;

        let mut temp_file = NamedTempFile::with_suffix(".eml").unwrap();
        temp_file.write_all(eml_content).unwrap();

        let result = process_eml(temp_file.path());
        assert!(result.is_ok());

        let markdown = result.unwrap();
        // Check that output ends with at least one newline (markdown convention)
        assert!(
            markdown.ends_with('\n'),
            "Email markdown should end with newline"
        );
        // Check output is not empty
        assert!(
            !markdown.trim().is_empty(),
            "Email markdown should have content"
        );
        // temp_file automatically cleaned up on drop
    }

    #[test]
    fn test_eml_markdown_structure() {
        // Test that EML output follows expected markdown structure
        let eml_content = br#"From: sender@example.com
To: recipient@example.com
Subject: Structure Test
Date: Mon, 15 Jan 2024 10:00:00 -0800
MIME-Version: 1.0
Content-Type: text/plain; charset="UTF-8"

Body content here.
"#;

        let mut temp_file = NamedTempFile::with_suffix(".eml").unwrap();
        temp_file.write_all(eml_content).unwrap();

        let result = process_eml(temp_file.path());
        assert!(result.is_ok());

        let markdown = result.unwrap();
        // Subject should be a heading
        assert!(markdown.contains("# Structure Test"));
        // Should contain metadata
        assert!(markdown.contains("From:") || markdown.contains("sender@example.com"));
        // Should contain body
        assert!(markdown.contains("Body content here"));
        // temp_file automatically cleaned up on drop
    }
}
