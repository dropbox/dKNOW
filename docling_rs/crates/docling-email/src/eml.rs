//! EML (Email Message) parser
//!
//! Parses RFC 5322 email messages using the mail-parser crate.
//! Extracts headers, body content, and attachments.

use crate::error::{EmailError, Result};
use mail_parser::{Message, MessageParser, MimeHeaders};
use std::fmt::Write;

/// Parsed email message
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct EmailMessage {
    /// Message-ID header
    pub message_id: Option<String>,
    /// Subject header
    pub subject: String,
    /// From address
    pub from: String,
    /// To addresses
    pub to: Vec<String>,
    /// CC addresses
    pub cc: Vec<String>,
    /// Date header
    pub date: Option<String>,
    /// Plain text body
    pub body_text: String,
    /// HTML body
    pub body_html: Option<String>,
    /// Attachments (name, content-type, size)
    pub attachments: Vec<Attachment>,
    /// MIME parts structure (for multipart messages)
    pub mime_parts: Vec<MimePart>,
}

/// Email attachment metadata
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Attachment {
    /// Filename
    pub name: Option<String>,
    /// Content-Type
    pub content_type: String,
    /// Size in bytes
    pub size: usize,
    /// Content-Disposition (inline, attachment, or None)
    pub disposition: Option<String>,
    /// Content-Transfer-Encoding (base64, quoted-printable, 7bit, 8bit, binary)
    pub encoding: Option<String>,
}

/// MIME part metadata (for multipart messages)
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct MimePart {
    /// Content-Type
    pub content_type: String,
    /// Part size in bytes
    pub size: usize,
    /// Subtype (e.g., "plain" for text/plain, "alternative" for multipart/alternative)
    pub subtype: Option<String>,
    /// Boundary string (for multipart types)
    pub boundary: Option<String>,
    /// Charset (for text types)
    pub charset: Option<String>,
}

/// Parse an EML file from bytes
///
/// # Errors
///
/// Returns an error if the email content is malformed or cannot be parsed.
#[must_use = "this function returns a parsed email that should be processed"]
pub fn parse_eml(content: &[u8]) -> Result<EmailMessage> {
    let parser = MessageParser::default();
    let message = parser
        .parse(content)
        .ok_or_else(|| EmailError::ParseError("Failed to parse email message".to_string()))?;

    Ok(extract_message_data(&message))
}

/// Extract structured data from parsed message
fn extract_message_data(message: &Message) -> EmailMessage {
    // Extract Message-ID
    let message_id = message.message_id().map(std::string::ToString::to_string);

    // Extract subject
    let subject = message.subject().unwrap_or("(No Subject)").to_string();

    // Extract From
    let from = message
        .from()
        .and_then(|addrs| addrs.first())
        .map_or_else(|| "(Unknown)".to_string(), |addr| format_address(addr));

    // Extract To addresses
    let to = message
        .to()
        .map(|addrs| addrs.iter().map(format_address).collect())
        .unwrap_or_default();

    // Extract CC addresses
    let cc = message
        .cc()
        .map(|addrs| addrs.iter().map(format_address).collect())
        .unwrap_or_default();

    // Extract date (preserve original format from header)
    let date = message
        .header_raw("Date")
        .map(std::string::ToString::to_string)
        .or_else(|| message.date().map(mail_parser::DateTime::to_rfc3339));

    // Extract body (plain text)
    let body_text = message
        .body_text(0)
        .map(|s| s.to_string())
        .unwrap_or_default();

    // Extract body (HTML)
    let body_html = message.body_html(0).map(|s| s.to_string());

    // Extract attachments with extended metadata
    let attachments = message
        .attachments()
        .map(|att| Attachment {
            name: att.attachment_name().map(std::string::ToString::to_string),
            content_type: att.content_type().map_or_else(
                || "application/octet-stream".to_string(),
                |ct| ct.ctype().to_string(),
            ),
            size: att.len(),
            disposition: att.content_disposition().map(|cd| cd.ctype().to_string()),
            encoding: att
                .content_transfer_encoding()
                .map(|e| format!("{e:?}").to_lowercase()),
        })
        .collect();

    // Extract MIME parts structure
    let mime_parts = extract_mime_parts(message);

    EmailMessage {
        message_id,
        subject,
        from,
        to,
        cc,
        date,
        body_text,
        body_html,
        attachments,
        mime_parts,
    }
}

/// Extract MIME parts structure from message
fn extract_mime_parts(message: &Message) -> Vec<MimePart> {
    let mut parts = Vec::new();

    // Iterate through all parts in the message
    for part in &message.parts {
        if let Some(ct) = part.content_type() {
            let content_type = ct.ctype().to_string();

            // Skip empty parts and parts without meaningful content
            if content_type == "multipart/alternative"
                || content_type == "multipart/mixed"
                || content_type == "multipart/related"
                || content_type.starts_with("text/")
                || content_type.starts_with("application/")
            {
                parts.push(MimePart {
                    content_type: content_type.clone(),
                    size: part.len(),
                    subtype: ct.subtype().map(std::string::ToString::to_string),
                    boundary: ct
                        .attribute("boundary")
                        .map(std::string::ToString::to_string),
                    charset: ct
                        .attribute("charset")
                        .map(std::string::ToString::to_string),
                });
            }
        }
    }

    parts
}

/// Format an address for display
#[inline]
fn format_address(addr: &mail_parser::Addr) -> String {
    match (addr.name(), addr.address()) {
        (Some(name), Some(address)) => format!("{name} <{address}>"),
        (Some(name), None) => name.to_string(),
        (None, address) => address.unwrap_or("").to_string(),
    }
}

/// Convert an `EmailMessage` to markdown
#[must_use = "converts email to markdown format"]
pub fn email_to_markdown(message: &EmailMessage) -> String {
    let mut output = String::new();

    // Title (subject as H1)
    output.push_str("# ");
    output.push_str(&message.subject);
    output.push_str("\n\n");

    // Metadata
    output.push_str("Subject: ");
    output.push_str(&message.subject);
    output.push('\n');

    output.push_str("From: ");
    output.push_str(&message.from);
    output.push('\n');

    if !message.to.is_empty() {
        output.push_str("To: ");
        output.push_str(&message.to.join(", "));
        output.push('\n');
    }

    if !message.cc.is_empty() {
        output.push_str("CC: ");
        output.push_str(&message.cc.join(", "));
        output.push('\n');
    }

    if let Some(date) = &message.date {
        output.push_str("Date: ");
        output.push_str(date);
        output.push('\n');
    }

    if let Some(msg_id) = &message.message_id {
        output.push_str("Message-ID: ");
        output.push_str(msg_id);
        output.push('\n');
    }

    output.push('\n');

    // Body
    output.push_str("## Message\n\n");
    output.push_str(&message.body_text);
    output.push_str("\n\n");

    // MIME Parts structure
    if !message.mime_parts.is_empty() {
        output.push_str("## MIME Structure\n\n");
        for (i, part) in message.mime_parts.iter().enumerate() {
            let _ = writeln!(output, "**Part {}**: {}", i + 1, part.content_type);
            let _ = writeln!(output, "- Size: {} bytes", part.size);
            if let Some(subtype) = &part.subtype {
                let _ = writeln!(output, "- Subtype: {subtype}");
            }
            if let Some(boundary) = &part.boundary {
                let _ = writeln!(output, "- Boundary: {boundary}");
            }
            if let Some(charset) = &part.charset {
                let _ = writeln!(output, "- Charset: {charset}");
            }
            output.push('\n');
        }
        output.push('\n');
    }

    // Attachments
    if !message.attachments.is_empty() {
        output.push_str("## Attachments\n\n");
        for att in &message.attachments {
            output.push_str("- ");
            if let Some(name) = &att.name {
                output.push_str(name);
            } else {
                output.push_str("(unnamed)");
            }
            let _ = write!(output, " ({}, {} bytes)", att.content_type, att.size);
            if let Some(disp) = &att.disposition {
                let _ = write!(output, ", disposition: {disp}");
            }
            if let Some(enc) = &att.encoding {
                let _ = write!(output, ", encoding: {enc}");
            }
            output.push('\n');
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_email() {
        let eml = b"From: sender@example.com\r\n\
                    To: recipient@example.com\r\n\
                    Subject: Test Message\r\n\
                    Date: Mon, 7 Nov 2025 10:00:00 +0000\r\n\
                    \r\n\
                    This is a test message.\r\n";

        let message = parse_eml(eml).unwrap();
        assert_eq!(message.subject, "Test Message");
        assert!(message.from.contains("sender@example.com"));
        assert_eq!(message.to.len(), 1);
        assert!(message.body_text.contains("test message"));
    }

    #[test]
    fn test_parse_empty_fails() {
        let result = parse_eml(b"");
        assert!(result.is_err());
    }

    /// Test email with CC recipients
    #[test]
    fn test_parse_email_with_cc() {
        let eml = b"From: sender@example.com\r\n\
                    To: recipient@example.com\r\n\
                    Cc: cc1@example.com, cc2@example.com\r\n\
                    Subject: CC Test\r\n\
                    \r\n\
                    Email with CC.\r\n";

        let message = parse_eml(eml).unwrap();
        assert_eq!(message.cc.len(), 2);
        assert!(message.cc.iter().any(|a| a.contains("cc1@example.com")));
        assert!(message.cc.iter().any(|a| a.contains("cc2@example.com")));
    }

    /// Test email with Message-ID
    #[test]
    fn test_parse_email_with_message_id() {
        let eml = b"From: sender@example.com\r\n\
                    To: recipient@example.com\r\n\
                    Subject: Message ID Test\r\n\
                    Message-ID: <12345@example.com>\r\n\
                    \r\n\
                    Email with Message-ID.\r\n";

        let message = parse_eml(eml).unwrap();
        assert!(message.message_id.is_some());
        assert!(message.message_id.unwrap().contains("12345@example.com"));
    }

    /// Test email with multiple To recipients
    #[test]
    fn test_parse_email_with_multiple_recipients() {
        let eml = b"From: sender@example.com\r\n\
                    To: alice@example.com, bob@example.com, carol@example.com\r\n\
                    Subject: Multiple Recipients\r\n\
                    \r\n\
                    Email to multiple recipients.\r\n";

        let message = parse_eml(eml).unwrap();
        assert_eq!(message.to.len(), 3);
    }

    /// Test email with named sender
    #[test]
    fn test_parse_email_with_named_sender() {
        let eml = b"From: \"John Doe\" <john@example.com>\r\n\
                    To: recipient@example.com\r\n\
                    Subject: Named Sender\r\n\
                    \r\n\
                    Email from named sender.\r\n";

        let message = parse_eml(eml).unwrap();
        assert!(
            message.from.contains("John Doe"),
            "From should include name: {}",
            message.from
        );
        assert!(message.from.contains("john@example.com"));
    }

    /// Test email without subject
    #[test]
    fn test_parse_email_without_subject() {
        let eml = b"From: sender@example.com\r\n\
                    To: recipient@example.com\r\n\
                    \r\n\
                    Email without subject.\r\n";

        let message = parse_eml(eml).unwrap();
        assert_eq!(message.subject, "(No Subject)");
    }

    /// Test email with multiline body
    #[test]
    fn test_parse_email_multiline_body() {
        let eml = b"From: sender@example.com\r\n\
                    To: recipient@example.com\r\n\
                    Subject: Multiline Body\r\n\
                    \r\n\
                    First paragraph.\r\n\
                    \r\n\
                    Second paragraph.\r\n\
                    \r\n\
                    Third paragraph.\r\n";

        let message = parse_eml(eml).unwrap();
        assert!(message.body_text.contains("First paragraph"));
        assert!(message.body_text.contains("Second paragraph"));
        assert!(message.body_text.contains("Third paragraph"));
    }

    /// Test markdown output contains required fields
    #[test]
    fn test_eml_to_markdown_structure() {
        let eml = b"From: sender@example.com\r\n\
                    To: recipient@example.com\r\n\
                    Subject: Markdown Test\r\n\
                    Date: Mon, 7 Nov 2025 10:00:00 +0000\r\n\
                    \r\n\
                    Test body content.\r\n";

        let message = parse_eml(eml).unwrap();
        let markdown = email_to_markdown(&message);

        assert!(
            markdown.contains("# Markdown Test"),
            "Should have subject as H1"
        );
        assert!(
            markdown.contains("From: sender@example.com"),
            "Should have From field"
        );
        assert!(
            markdown.contains("To: recipient@example.com"),
            "Should have To field"
        );
        assert!(
            markdown.contains("## Message"),
            "Should have Message section"
        );
        assert!(
            markdown.contains("Test body content"),
            "Should have body content"
        );
    }

    /// Test email with special characters in subject
    #[test]
    fn test_parse_email_special_chars_in_subject() {
        let eml = b"From: sender@example.com\r\n\
                    To: recipient@example.com\r\n\
                    Subject: Test: Special! @#$% Characters\r\n\
                    \r\n\
                    Body.\r\n";

        let message = parse_eml(eml).unwrap();
        assert!(message.subject.contains("Special"));
        assert!(message.subject.contains("Characters"));
    }

    /// Test email with date header
    #[test]
    fn test_parse_email_with_date() {
        let eml = b"From: sender@example.com\r\n\
                    To: recipient@example.com\r\n\
                    Subject: Date Test\r\n\
                    Date: Thu, 21 Nov 2025 14:30:00 -0500\r\n\
                    \r\n\
                    Email with date.\r\n";

        let message = parse_eml(eml).unwrap();
        assert!(message.date.is_some());
        assert!(
            message.date.as_ref().unwrap().contains("Nov"),
            "Date should contain month"
        );
    }

    /// Test comprehensive email with all fields
    #[test]
    fn test_parse_comprehensive_email() {
        let eml = b"From: \"John Doe\" <john@example.com>\r\n\
                    To: alice@example.com, bob@example.com\r\n\
                    Cc: manager@example.com\r\n\
                    Subject: Comprehensive Test Email\r\n\
                    Date: Mon, 25 Nov 2025 09:15:00 +0000\r\n\
                    Message-ID: <unique-id-123@example.com>\r\n\
                    \r\n\
                    Dear Team,\r\n\
                    \r\n\
                    This is a comprehensive test email.\r\n\
                    \r\n\
                    Best regards,\r\n\
                    John\r\n";

        let message = parse_eml(eml).unwrap();

        assert!(message.from.contains("John Doe"));
        assert_eq!(message.to.len(), 2);
        assert_eq!(message.cc.len(), 1);
        assert_eq!(message.subject, "Comprehensive Test Email");
        assert!(message.date.is_some());
        assert!(message.message_id.is_some());
        assert!(message.body_text.contains("Dear Team"));
        assert!(message.body_text.contains("Best regards"));
    }
}
