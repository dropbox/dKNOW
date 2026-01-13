//! MBOX (Mailbox Archive) parser
//!
//! Parses Unix mailbox format files.
//! MBOX files contain multiple email messages separated by "From " lines.

use crate::eml::{parse_eml, EmailMessage};
use crate::error::{EmailError, Result};
use std::fmt::Write;

/// Parsed MBOX mailbox
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Mailbox {
    /// Messages in the mailbox
    pub messages: Vec<EmailMessage>,
}

/// Parse an MBOX file from bytes
///
/// # Errors
///
/// Returns an error if:
/// - The content is not valid UTF-8
/// - Message parsing fails
#[must_use = "this function returns a parsed mailbox that should be processed"]
pub fn parse_mbox(content: &[u8]) -> Result<Mailbox> {
    let content_str = std::str::from_utf8(content)
        .map_err(|e| EmailError::MboxError(format!("Invalid UTF-8: {e}")))?;

    let messages = split_mbox_messages(content_str)?;

    Ok(Mailbox { messages })
}

/// Split MBOX content into individual messages
fn split_mbox_messages(content: &str) -> Result<Vec<EmailMessage>> {
    let mut messages = Vec::new();
    let mut current_message = String::new();
    let mut in_message = false;

    for line in content.lines() {
        // MBOX format: messages separated by lines starting with "From "
        // (note the space after "From")
        if line.starts_with("From ") {
            // Save previous message if exists
            if in_message && !current_message.is_empty() {
                if let Ok(msg) = parse_eml(current_message.as_bytes()) {
                    messages.push(msg);
                }
            }
            // Start new message
            current_message.clear();
            in_message = true;
            continue;
        }

        if in_message {
            current_message.push_str(line);
            current_message.push('\n');
        }
    }

    // Don't forget the last message
    if in_message && !current_message.is_empty() {
        if let Ok(msg) = parse_eml(current_message.as_bytes()) {
            messages.push(msg);
        }
    }

    if messages.is_empty() {
        return Err(EmailError::MboxError(
            "No messages found in MBOX file".to_string(),
        ));
    }

    Ok(messages)
}

/// Convert a Mailbox to markdown
#[must_use = "converts mailbox to markdown format"]
pub fn mbox_to_markdown(mailbox: &Mailbox) -> String {
    let mut output = String::new();

    let _ = writeln!(output, "# Mailbox ({} messages)\n", mailbox.messages.len());

    for (i, message) in mailbox.messages.iter().enumerate() {
        if i > 0 {
            output.push_str("\n---\n\n");
        }
        let _ = writeln!(output, "## Message {}\n", i + 1);

        // Add message metadata
        let _ = writeln!(output, "From: {}", message.from);
        if !message.to.is_empty() {
            let _ = writeln!(output, "To: {}", message.to.join(", "));
        }
        let _ = writeln!(output, "Subject: {}", message.subject);
        if let Some(date) = &message.date {
            let _ = writeln!(output, "Date: {date}");
        }
        output.push('\n');

        // Add body
        output.push_str(&message.body_text);
        output.push('\n');

        // Add attachments if any
        if !message.attachments.is_empty() {
            output.push_str("\nAttachments:\n");
            for att in &message.attachments {
                output.push_str("- ");
                if let Some(name) = &att.name {
                    output.push_str(name);
                } else {
                    output.push_str("(unnamed)");
                }
                let _ = writeln!(output, " ({}, {} bytes)", att.content_type, att.size);
            }
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_mbox_single_message() {
        let mbox = b"From sender@example.com Mon Nov 07 10:00:00 2025\r\n\
                     From: sender@example.com\r\n\
                     To: recipient@example.com\r\n\
                     Subject: Test Message\r\n\
                     \r\n\
                     This is a test message.\r\n";

        let mailbox = parse_mbox(mbox).unwrap();
        assert_eq!(mailbox.messages.len(), 1);
        assert_eq!(mailbox.messages[0].subject, "Test Message");
    }

    #[test]
    fn test_parse_mbox_multiple_messages() {
        let mbox = b"From sender1@example.com Mon Nov 07 10:00:00 2025\r\n\
                     From: sender1@example.com\r\n\
                     Subject: Message 1\r\n\
                     \r\n\
                     First message.\r\n\
                     From sender2@example.com Mon Nov 07 11:00:00 2025\r\n\
                     From: sender2@example.com\r\n\
                     Subject: Message 2\r\n\
                     \r\n\
                     Second message.\r\n";

        let mailbox = parse_mbox(mbox).unwrap();
        assert_eq!(mailbox.messages.len(), 2);
        assert_eq!(mailbox.messages[0].subject, "Message 1");
        assert_eq!(mailbox.messages[1].subject, "Message 2");
    }

    #[test]
    fn test_parse_empty_mbox_fails() {
        let result = parse_mbox(b"");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_mbox_no_messages_fails() {
        let mbox = b"This is not an MBOX file\r\n\
                     Just some random text.\r\n";
        let result = parse_mbox(mbox);
        assert!(result.is_err());
    }

    /// Test mbox with messages from different senders
    #[test]
    fn test_parse_mbox_different_senders() {
        let mbox = b"From alice@example.com Mon Nov 07 10:00:00 2025\r\n\
                     From: alice@example.com\r\n\
                     To: bob@example.com\r\n\
                     Subject: Hello Bob\r\n\
                     \r\n\
                     Hello!\r\n\
                     From bob@example.com Mon Nov 07 11:00:00 2025\r\n\
                     From: bob@example.com\r\n\
                     To: alice@example.com\r\n\
                     Subject: Re: Hello Bob\r\n\
                     \r\n\
                     Hi Alice!\r\n";

        let mailbox = parse_mbox(mbox).unwrap();
        assert_eq!(mailbox.messages.len(), 2);
        assert!(mailbox.messages[0].from.contains("alice"));
        assert!(mailbox.messages[1].from.contains("bob"));
    }

    /// Test markdown output for mbox
    #[test]
    fn test_mbox_to_markdown_structure() {
        let mbox = b"From sender@example.com Mon Nov 07 10:00:00 2025\r\n\
                     From: sender@example.com\r\n\
                     To: recipient@example.com\r\n\
                     Subject: Test Subject\r\n\
                     \r\n\
                     Test body.\r\n";

        let mailbox = parse_mbox(mbox).unwrap();
        let markdown = mbox_to_markdown(&mailbox);

        assert!(
            markdown.contains("# Mailbox (1 messages)"),
            "Should have mailbox title with count"
        );
        assert!(
            markdown.contains("## Message 1"),
            "Should have message heading"
        );
        assert!(
            markdown.contains("From: sender@example.com"),
            "Should have From field"
        );
        assert!(
            markdown.contains("Subject: Test Subject"),
            "Should have Subject field"
        );
    }

    /// Test mbox with many messages
    #[test]
    fn test_parse_mbox_many_messages() {
        let mbox = b"From user1@test.com Mon Nov 07 10:00:00 2025\r\n\
                     From: user1@test.com\r\n\
                     Subject: Msg 1\r\n\
                     \r\n\
                     Body 1.\r\n\
                     From user2@test.com Mon Nov 07 10:01:00 2025\r\n\
                     From: user2@test.com\r\n\
                     Subject: Msg 2\r\n\
                     \r\n\
                     Body 2.\r\n\
                     From user3@test.com Mon Nov 07 10:02:00 2025\r\n\
                     From: user3@test.com\r\n\
                     Subject: Msg 3\r\n\
                     \r\n\
                     Body 3.\r\n";

        let mailbox = parse_mbox(mbox).unwrap();
        assert_eq!(mailbox.messages.len(), 3);
        assert_eq!(mailbox.messages[0].subject, "Msg 1");
        assert_eq!(mailbox.messages[1].subject, "Msg 2");
        assert_eq!(mailbox.messages[2].subject, "Msg 3");
    }

    /// Test mbox markdown separators for multiple messages
    #[test]
    fn test_mbox_markdown_separators() {
        let mbox = b"From sender1@test.com Mon Nov 07 10:00:00 2025\r\n\
                     From: sender1@test.com\r\n\
                     Subject: First\r\n\
                     \r\n\
                     First body.\r\n\
                     From sender2@test.com Mon Nov 07 11:00:00 2025\r\n\
                     From: sender2@test.com\r\n\
                     Subject: Second\r\n\
                     \r\n\
                     Second body.\r\n";

        let mailbox = parse_mbox(mbox).unwrap();
        let markdown = mbox_to_markdown(&mailbox);

        assert!(
            markdown.contains("## Message 1"),
            "Should have message 1 heading"
        );
        assert!(
            markdown.contains("## Message 2"),
            "Should have message 2 heading"
        );
        assert!(
            markdown.contains("---"),
            "Should have separator between messages"
        );
    }

    /// Test mbox with message containing multiline body
    #[test]
    fn test_parse_mbox_multiline_body() {
        let mbox = b"From sender@test.com Mon Nov 07 10:00:00 2025\r\n\
                     From: sender@test.com\r\n\
                     Subject: Multiline\r\n\
                     \r\n\
                     Line 1.\r\n\
                     Line 2.\r\n\
                     Line 3.\r\n";

        let mailbox = parse_mbox(mbox).unwrap();
        assert_eq!(mailbox.messages.len(), 1);
        let body = &mailbox.messages[0].body_text;
        assert!(body.contains("Line 1"));
        assert!(body.contains("Line 2"));
        assert!(body.contains("Line 3"));
    }
}
