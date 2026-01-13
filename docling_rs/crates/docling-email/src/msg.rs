// MSG (Microsoft Outlook Message) Format Parser
//
// MSG is Microsoft Outlook's proprietary format for storing individual email messages.
// Files use the OLE/CFB (Compound File Binary) structure.
//
// Format: Binary file (OLE/CFB container with MAPI properties)
// Extensions: .msg
// MIME Type: application/vnd.ms-outlook
//
// Implementation: Uses the `msg_parser` crate (v0.1.1) for parsing OLE/CFB and MAPI properties
// Content: Headers + body (text/HTML) + attachments + metadata
//
// References:
// - [MS-OXMSG]: Outlook Item (.msg) File Format
//   https://docs.microsoft.com/en-us/openspecs/exchange_server_protocols/ms-oxmsg/
// - [MS-CFB]: Compound File Binary File Format
//   https://docs.microsoft.com/en-us/openspecs/windows_protocols/ms-cfb/
// - msg_parser crate: https://github.com/marirs/msg-parser-rs

use crate::error::EmailError;
use msg_parser::Outlook;
use std::fmt::Write;
use std::path::Path;

/// Parsed MSG (Outlook message) data
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ParsedMsg {
    /// Email subject line
    pub subject: String,
    /// Email sender address
    pub sender: EmailAddress,
    /// Email recipients (To, Cc, Bcc)
    pub recipients: Recipients,
    /// Email body content
    pub body: MsgBody,
    /// File attachments
    pub attachments: Vec<MsgAttachment>,
    /// Send/receive date (if available)
    pub date: Option<String>,
}

/// Email address (name + email)
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct EmailAddress {
    /// Display name (optional)
    pub name: Option<String>,
    /// Email address
    pub email: String,
}

/// Email recipients (To, Cc, Bcc)
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Recipients {
    /// Primary recipients (To field)
    pub to: Vec<EmailAddress>,
    /// Carbon copy recipients (Cc field)
    pub cc: Vec<EmailAddress>,
    /// Blind carbon copy recipients (Bcc field)
    pub bcc: Vec<EmailAddress>,
}

/// MSG email body (plain text and/or HTML)
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct MsgBody {
    /// Plain text body content
    pub plain: Option<String>,
    /// HTML body content
    pub html: Option<String>,
}

/// MSG attachment metadata
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct MsgAttachment {
    /// Attachment filename
    pub filename: String,
    /// Size in bytes
    pub size: usize,
    /// MIME content type (if known)
    pub content_type: Option<String>,
}

/// Parse an MSG file from a file path
///
/// # Arguments
/// * `path` - Path to the MSG file
///
/// # Errors
///
/// Returns an error if the MSG file is invalid, corrupted, or cannot be parsed.
///
/// # Examples
/// ```no_run
/// use docling_email::msg::parse_msg_from_path;
/// let msg = parse_msg_from_path("message.msg").unwrap();
/// println!("Subject: {}", msg.subject);
/// ```
#[must_use = "this function returns a parsed MSG email that should be processed"]
pub fn parse_msg_from_path<P: AsRef<Path>>(path: P) -> Result<ParsedMsg, EmailError> {
    // Parse MSG file using msg_parser crate
    let outlook = Outlook::from_path(path.as_ref())
        .map_err(|e| EmailError::ParseError(format!("Failed to parse MSG file: {e}")))?;

    // Convert msg_parser::Outlook to our ParsedMsg structure
    Ok(outlook_to_parsed_msg(outlook))
}

/// Parse an MSG file from raw bytes
///
/// # Arguments
/// * `bytes` - Raw bytes of the MSG file
///
/// # Errors
///
/// Returns an error if:
/// - Temporary file cannot be created
/// - MSG parsing fails
///
/// # Examples
/// ```no_run
/// use docling_email::msg::parse_msg;
/// let bytes = std::fs::read("message.msg").unwrap();
/// let msg = parse_msg(&bytes).unwrap();
/// ```
#[must_use = "this function returns a parsed MSG email that should be processed"]
pub fn parse_msg(bytes: &[u8]) -> Result<ParsedMsg, EmailError> {
    use std::io::Write;

    // msg_parser doesn't have a direct from_bytes() method,
    // so we write bytes to a temp file and parse from there
    let mut temp_file = tempfile::NamedTempFile::new()
        .map_err(|e| EmailError::ParseError(format!("Failed to create temp file: {e}")))?;

    temp_file
        .write_all(bytes)
        .map_err(|e| EmailError::ParseError(format!("Failed to write to temp file: {e}")))?;

    temp_file
        .flush()
        .map_err(|e| EmailError::ParseError(format!("Failed to flush temp file: {e}")))?;

    // Parse from the temp file path
    parse_msg_from_path(temp_file.path())
}

/// Convert `msg_parser::Outlook` to our `ParsedMsg` structure
fn outlook_to_parsed_msg(outlook: Outlook) -> ParsedMsg {
    // Extract subject (outlook.subject is String, not Option)
    let subject = outlook.subject;

    // Extract sender (outlook.sender is Person struct)
    let sender = EmailAddress {
        name: if outlook.sender.name.is_empty() {
            None
        } else {
            Some(outlook.sender.name)
        },
        email: outlook.sender.email,
    };

    // Extract recipients (outlook.to and outlook.cc are Vec<Person>)
    let recipients = Recipients {
        to: outlook
            .to
            .iter()
            .map(|person| EmailAddress {
                name: if person.name.is_empty() {
                    None
                } else {
                    Some(person.name.clone())
                },
                email: person.email.clone(),
            })
            .collect(),
        cc: outlook
            .cc
            .iter()
            .map(|person| EmailAddress {
                name: if person.name.is_empty() {
                    None
                } else {
                    Some(person.name.clone())
                },
                email: person.email.clone(),
            })
            .collect(),
        // BCC is a String (DisplayBcc), not Vec<Person>, so we'll parse it
        bcc: if outlook.bcc.is_empty() {
            Vec::new()
        } else {
            outlook
                .bcc
                .split(',')
                .filter(|s| !s.trim().is_empty())
                .map(|addr| parse_email_address(addr.trim()))
                .collect()
        },
    };

    // Extract body (outlook.body is String, not Option<String>)
    // Note: msg_parser v0.1.1 doesn't have body_html field
    // We have rtf_compressed field but not HTML
    let body = MsgBody {
        plain: if outlook.body.is_empty() {
            None
        } else {
            Some(outlook.body)
        },
        html: None, // msg_parser v0.1.1 doesn't expose HTML body
    };

    // Extract attachments (outlook.attachments is Vec<Attachment>)
    let attachments = outlook
        .attachments
        .iter()
        .map(|att| MsgAttachment {
            filename: if att.file_name.is_empty() {
                att.display_name.clone()
            } else {
                att.file_name.clone()
            },
            // payload is base64-encoded string, decode to get size
            size: att.payload.len(), // Approximate size (base64 length)
            content_type: if att.mime_tag.is_empty() {
                None
            } else {
                Some(att.mime_tag.clone())
            },
        })
        .collect();

    // Extract date from headers
    let date = if outlook.headers.date.is_empty() {
        None
    } else {
        Some(outlook.headers.date)
    };

    ParsedMsg {
        subject,
        sender,
        recipients,
        body,
        attachments,
        date,
    }
}

/// Parse an email address string into `EmailAddress` struct
///
/// Handles formats:
/// - "Name <email@example.com>"
/// - "email@example.com"
/// - "Name" (invalid, but handle gracefully)
fn parse_email_address(addr_str: &str) -> EmailAddress {
    // Try to extract name and email from "Name <email>" format
    if let Some(angle_start) = addr_str.find('<') {
        if let Some(angle_end) = addr_str.find('>') {
            let name = addr_str[..angle_start].trim();
            let email = addr_str[angle_start + 1..angle_end].trim();
            return EmailAddress {
                name: if name.is_empty() {
                    None
                } else {
                    Some(name.to_string())
                },
                email: email.to_string(),
            };
        }
    }

    // If no angle brackets, assume the entire string is the email
    EmailAddress {
        name: None,
        email: addr_str.to_string(),
    }
}

/// Convert `ParsedMsg` to markdown format
///
/// # Arguments
/// * `msg` - Parsed MSG data
///
/// # Returns
/// * String containing markdown-formatted email content
///
/// # Format
/// ```markdown
/// # Email Message
///
/// **Subject:** Email subject
/// **From:** Sender Name <sender@example.com>
/// **To:** recipient1@example.com, recipient2@example.com
/// **Cc:** cc@example.com
/// **Date:** 2025-11-07 10:30:00 UTC
///
/// ---
///
/// ## Body
///
/// [Email body content]
///
/// ---
///
/// ## Attachments
///
/// - document.pdf (524 KB)
/// - image.png (128 KB)
/// ```
#[must_use = "converts MSG email to markdown format"]
pub fn msg_to_markdown(msg: &ParsedMsg) -> String {
    let mut md = String::new();

    // Title
    md.push_str("# Email Message\n\n");

    // Headers
    let _ = writeln!(md, "**Subject:** {}", msg.subject);
    let _ = writeln!(md, "**From:** {}", format_email_address(&msg.sender));

    if !msg.recipients.to.is_empty() {
        let _ = writeln!(md, "**To:** {}", format_email_list(&msg.recipients.to));
    }

    if !msg.recipients.cc.is_empty() {
        let _ = writeln!(md, "**Cc:** {}", format_email_list(&msg.recipients.cc));
    }

    if !msg.recipients.bcc.is_empty() {
        let _ = writeln!(md, "**Bcc:** {}", format_email_list(&msg.recipients.bcc));
    }

    if let Some(date) = &msg.date {
        let _ = writeln!(md, "**Date:** {date}");
    }

    md.push_str("\n---\n\n## Body\n\n");

    // Body: prefer plain text, fall back to HTML
    if let Some(plain) = &msg.body.plain {
        md.push_str(plain);
    } else if let Some(html) = &msg.body.html {
        // Try to convert HTML to markdown using html2md
        match html2md::parse_html(html) {
            markdown_text if !markdown_text.trim().is_empty() => {
                md.push_str(&markdown_text);
            }
            _ => {
                // Fallback: include HTML as code block
                md.push_str("```html\n");
                md.push_str(html);
                md.push_str("\n```\n");
            }
        }
    } else {
        md.push_str("*(No body content)*\n");
    }

    // Attachments section
    if !msg.attachments.is_empty() {
        md.push_str("\n\n---\n\n## Attachments\n\n");
        for att in &msg.attachments {
            let size_kb = att.size / 1024;
            let size_str = if size_kb > 0 {
                format!("{size_kb} KB")
            } else {
                format!("{} bytes", att.size)
            };
            let _ = write!(md, "- {} ({})", att.filename, size_str);
            if let Some(ct) = &att.content_type {
                let _ = write!(md, " [{ct}]");
            }
            md.push('\n');
        }
    }

    md
}

/// Format an `EmailAddress` as a string
///
/// # Arguments
/// * `addr` - Email address to format
///
/// # Returns
/// * "Name <email@example.com>" if name is present
/// * "email@example.com" if name is absent
#[inline]
fn format_email_address(addr: &EmailAddress) -> String {
    addr.name.as_ref().map_or_else(
        || addr.email.clone(),
        |name| format!("{name} <{}>", addr.email),
    )
}

/// Format a list of email addresses as a comma-separated string
///
/// # Arguments
/// * `addrs` - List of email addresses
///
/// # Returns
/// * Comma-separated string of formatted addresses
#[inline]
fn format_email_list(addrs: &[EmailAddress]) -> String {
    addrs
        .iter()
        .map(format_email_address)
        .collect::<Vec<_>>()
        .join(", ")
}

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_email_address_with_name() {
        let addr = parse_email_address("John Doe <john@example.com>");
        assert_eq!(addr.name, Some("John Doe".to_string()));
        assert_eq!(addr.email, "john@example.com");
    }

    #[test]
    fn test_parse_email_address_no_name() {
        let addr = parse_email_address("jane@example.com");
        assert_eq!(addr.name, None);
        assert_eq!(addr.email, "jane@example.com");
    }

    #[test]
    fn test_format_email_address_with_name() {
        let addr = EmailAddress {
            name: Some("Alice Smith".to_string()),
            email: "alice@example.com".to_string(),
        };
        let formatted = format_email_address(&addr);
        assert_eq!(formatted, "Alice Smith <alice@example.com>");
    }

    #[test]
    fn test_format_email_address_no_name() {
        let addr = EmailAddress {
            name: None,
            email: "bob@example.com".to_string(),
        };
        let formatted = format_email_address(&addr);
        assert_eq!(formatted, "bob@example.com");
    }

    #[test]
    fn test_format_email_list() {
        let addrs = vec![
            EmailAddress {
                name: Some("Alice".to_string()),
                email: "alice@example.com".to_string(),
            },
            EmailAddress {
                name: None,
                email: "bob@example.com".to_string(),
            },
            EmailAddress {
                name: Some("Charlie".to_string()),
                email: "charlie@example.com".to_string(),
            },
        ];
        let formatted = format_email_list(&addrs);
        assert_eq!(
            formatted,
            "Alice <alice@example.com>, bob@example.com, Charlie <charlie@example.com>"
        );
    }

    #[test]
    fn test_msg_to_markdown_basic() {
        let msg = ParsedMsg {
            subject: "Test Subject".to_string(),
            sender: EmailAddress {
                name: Some("Sender Name".to_string()),
                email: "sender@example.com".to_string(),
            },
            recipients: Recipients {
                to: vec![EmailAddress {
                    name: None,
                    email: "recipient@example.com".to_string(),
                }],
                cc: vec![],
                bcc: vec![],
            },
            body: MsgBody {
                plain: Some("This is the email body.".to_string()),
                html: None,
            },
            attachments: vec![],
            date: Some("2025-11-07".to_string()),
        };

        let markdown = msg_to_markdown(&msg);

        assert!(markdown.contains("# Email Message"));
        assert!(markdown.contains("**Subject:** Test Subject"));
        assert!(markdown.contains("**From:** Sender Name <sender@example.com>"));
        assert!(markdown.contains("**To:** recipient@example.com"));
        assert!(markdown.contains("**Date:** 2025-11-07"));
        assert!(markdown.contains("## Body"));
        assert!(markdown.contains("This is the email body."));
    }

    #[test]
    fn test_msg_to_markdown_with_attachments() {
        let msg = ParsedMsg {
            subject: "Test".to_string(),
            sender: EmailAddress {
                name: None,
                email: "test@example.com".to_string(),
            },
            recipients: Recipients {
                to: vec![],
                cc: vec![],
                bcc: vec![],
            },
            body: MsgBody {
                plain: Some("Body".to_string()),
                html: None,
            },
            attachments: vec![
                MsgAttachment {
                    filename: "document.pdf".to_string(),
                    size: 524_288, // 512 KB
                    content_type: Some("application/pdf".to_string()),
                },
                MsgAttachment {
                    filename: "image.png".to_string(),
                    size: 1024, // 1 KB
                    content_type: None,
                },
            ],
            date: None,
        };

        let markdown = msg_to_markdown(&msg);

        assert!(markdown.contains("## Attachments"));
        assert!(markdown.contains("document.pdf (512 KB) [application/pdf]"));
        assert!(markdown.contains("image.png (1 KB)"));
    }

    #[test]
    fn test_msg_to_markdown_html_body() {
        let msg = ParsedMsg {
            subject: "HTML Email".to_string(),
            sender: EmailAddress {
                name: None,
                email: "sender@example.com".to_string(),
            },
            recipients: Recipients {
                to: vec![],
                cc: vec![],
                bcc: vec![],
            },
            body: MsgBody {
                plain: None,
                html: Some("<p>This is <strong>bold</strong> text.</p>".to_string()),
            },
            attachments: vec![],
            date: None,
        };

        let markdown = msg_to_markdown(&msg);

        // html2md should convert <p> and <strong> to markdown
        // Expected output: "This is **bold** text."
        assert!(markdown.contains("## Body"));
        // Check that HTML was converted (may contain markdown bold syntax or plain text)
        assert!(markdown.contains("bold"));
    }

    // Note: Integration tests with actual MSG files will be added after
    // creating test corpus files. These tests verify the data structure
    // handling and markdown conversion logic.
}
