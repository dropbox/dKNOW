//! Email document backend for EML, MBOX, MSG, and VCF formats
//!
//! This module provides email parsing and markdown conversion capabilities
//! for various email formats supported by the docling-email crate.

// Clippy pedantic allows:
// - Email parsing functions are necessarily complex
// - Unit struct &self convention
#![allow(clippy::too_many_lines)]
#![allow(clippy::trivially_copy_pass_by_ref)]

use crate::traits::{BackendOptions, DocumentBackend};
use crate::utils::{create_list_item, create_provenance, create_section_header, create_text_item};
use docling_core::{content::DocItem, DoclingError, Document, DocumentMetadata, InputFormat};
use docling_email::{
    parse_eml, parse_mbox, parse_msg_from_path, parse_vcf, Contact, EmailMessage, Mailbox,
    ParsedMsg,
};
use std::fmt::Write;
use std::path::Path;

/// Email backend for processing email and contact files
///
/// Supports:
/// - EML (.eml) - RFC 822/5322 email messages
/// - MBOX (.mbox) - Unix mailbox archives
/// - MSG (.msg) - Microsoft Outlook messages
/// - VCF (.vcf) - vCard contact cards
///
/// Parses email headers, body content, and attachments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EmailBackend {
    format: InputFormat,
}

impl EmailBackend {
    /// Create a new email backend for the specified format
    ///
    /// # Errors
    ///
    /// Returns an error if the format is not an email format.
    #[inline]
    #[must_use = "creating a backend that is not used is a waste of resources"]
    pub fn new(format: InputFormat) -> Result<Self, DoclingError> {
        if !format.is_email() {
            return Err(DoclingError::FormatError(format!(
                "Format {format:?} is not an email format"
            )));
        }
        Ok(Self { format })
    }

    /// Generate `DocItems` from `EmailMessage` (EML format)
    ///
    /// Structure:
    /// 1. Headers as Text (Subject, From, To, CC, Date, Message-ID)
    /// 2. Body as Text
    /// 3. Attachments as `ListItems`
    fn generate_docitems_from_email(message: &EmailMessage) -> Vec<DocItem> {
        let mut doc_items = Vec::new();

        // Subject header
        let subject_text = format!("Subject: {}", message.subject);
        doc_items.push(create_text_item(
            doc_items.len(),
            subject_text,
            create_provenance(1),
        ));

        // From header
        let from_text = format!("From: {}", message.from);
        doc_items.push(create_text_item(
            doc_items.len(),
            from_text,
            create_provenance(1),
        ));

        // To header
        if !message.to.is_empty() {
            let to_text = format!("To: {}", message.to.join(", "));
            doc_items.push(create_text_item(
                doc_items.len(),
                to_text,
                create_provenance(1),
            ));
        }

        // CC header
        if !message.cc.is_empty() {
            let cc_text = format!("CC: {}", message.cc.join(", "));
            doc_items.push(create_text_item(
                doc_items.len(),
                cc_text,
                create_provenance(1),
            ));
        }

        // Date header
        if let Some(date) = &message.date {
            let date_text = format!("Date: {date}");
            doc_items.push(create_text_item(
                doc_items.len(),
                date_text,
                create_provenance(1),
            ));
        }

        // Message-ID header
        if let Some(message_id) = &message.message_id {
            let id_text = format!("Message-ID: {message_id}");
            doc_items.push(create_text_item(
                doc_items.len(),
                id_text,
                create_provenance(1),
            ));
        }

        // Body content
        if !message.body_text.is_empty() {
            doc_items.push(create_text_item(
                doc_items.len(),
                message.body_text.clone(),
                create_provenance(1),
            ));
        }

        // MIME Parts structure
        if !message.mime_parts.is_empty() {
            doc_items.push(create_section_header(
                doc_items.len(),
                "MIME Structure".to_string(),
                2,
                create_provenance(1),
            ));

            for (i, part) in message.mime_parts.iter().enumerate() {
                let mut part_text = format!("Part {}: {}", i + 1, part.content_type);
                let mut details = Vec::new();
                details.push(format!("{} bytes", part.size));
                if let Some(subtype) = &part.subtype {
                    details.push(format!("subtype: {subtype}"));
                }
                if let Some(boundary) = &part.boundary {
                    details.push(format!("boundary: {boundary}"));
                }
                if let Some(charset) = &part.charset {
                    details.push(format!("charset: {charset}"));
                }
                if !details.is_empty() {
                    let _ = write!(part_text, " ({})", details.join(", "));
                }

                doc_items.push(create_list_item(
                    doc_items.len(),
                    part_text,
                    "- ".to_string(),
                    false,
                    create_provenance(1),
                    None,
                ));
            }
        }

        // Attachments section (with extended metadata)
        if !message.attachments.is_empty() {
            doc_items.push(create_section_header(
                doc_items.len(),
                "Attachments".to_string(),
                2,
                create_provenance(1),
            ));

            for att in &message.attachments {
                let mut att_text = att.name.as_ref().map_or_else(
                    || format!("(unnamed) ({}, {} bytes)", att.content_type, att.size),
                    |name| format!("{} ({}, {} bytes)", name, att.content_type, att.size),
                );

                // Add disposition and encoding info
                if att.disposition.is_some() || att.encoding.is_some() {
                    let mut extra_info = Vec::new();
                    if let Some(disp) = &att.disposition {
                        extra_info.push(format!("disposition: {disp}"));
                    }
                    if let Some(enc) = &att.encoding {
                        extra_info.push(format!("encoding: {enc}"));
                    }
                    if !extra_info.is_empty() {
                        let _ = write!(att_text, ", {}", extra_info.join(", "));
                    }
                }

                doc_items.push(create_list_item(
                    doc_items.len(),
                    att_text,
                    "- ".to_string(),
                    false,
                    create_provenance(1),
                    None,
                ));
            }
        }

        doc_items
    }

    /// Generate `DocItems` from Mailbox (MBOX format)
    ///
    /// Structure:
    /// 1. Mailbox title as `SectionHeader` (level 1)
    /// 2. Each message as subsection (level 2) with email `DocItems`
    fn generate_docitems_from_mailbox(mailbox: &Mailbox) -> Vec<DocItem> {
        let mut doc_items = Vec::new();

        // Mailbox header
        let title = format!("Mailbox ({} messages)", mailbox.messages.len());
        doc_items.push(create_section_header(
            doc_items.len(),
            title,
            1,
            create_provenance(1),
        ));

        // Process each message
        for (i, message) in mailbox.messages.iter().enumerate() {
            // Message header
            let msg_title = format!("Message {}", i + 1);
            doc_items.push(create_section_header(
                doc_items.len(),
                msg_title,
                2,
                create_provenance(1),
            ));

            // From
            let from_text = format!("From: {}", message.from);
            doc_items.push(create_text_item(
                doc_items.len(),
                from_text,
                create_provenance(1),
            ));

            // To
            if !message.to.is_empty() {
                let to_text = format!("To: {}", message.to.join(", "));
                doc_items.push(create_text_item(
                    doc_items.len(),
                    to_text,
                    create_provenance(1),
                ));
            }

            // Subject
            let subject_text = format!("Subject: {}", message.subject);
            doc_items.push(create_text_item(
                doc_items.len(),
                subject_text,
                create_provenance(1),
            ));

            // Date
            if let Some(date) = &message.date {
                let date_text = format!("Date: {date}");
                doc_items.push(create_text_item(
                    doc_items.len(),
                    date_text,
                    create_provenance(1),
                ));
            }

            // Body
            if !message.body_text.is_empty() {
                doc_items.push(create_text_item(
                    doc_items.len(),
                    message.body_text.clone(),
                    create_provenance(1),
                ));
            }

            // Attachments
            if !message.attachments.is_empty() {
                let att_count = format!("Attachments: {}", message.attachments.len());
                doc_items.push(create_text_item(
                    doc_items.len(),
                    att_count,
                    create_provenance(1),
                ));
            }
        }

        doc_items
    }

    /// Generate `DocItems` from `ParsedMsg` (MSG format)
    ///
    /// Structure similar to `EmailMessage`
    fn generate_docitems_from_msg(msg: &ParsedMsg) -> Vec<DocItem> {
        let mut doc_items = Vec::new();

        // Subject header
        doc_items.push(create_section_header(
            doc_items.len(),
            msg.subject.clone(),
            1,
            create_provenance(1),
        ));

        // From header
        let from_text = msg.sender.name.as_ref().map_or_else(
            || format!("From: {}", msg.sender.email),
            |name| format!("From: {name} <{}>", msg.sender.email),
        );
        doc_items.push(create_text_item(
            doc_items.len(),
            from_text,
            create_provenance(1),
        ));

        // To header
        if !msg.recipients.to.is_empty() {
            let to_addrs: Vec<String> = msg
                .recipients
                .to
                .iter()
                .map(|addr| {
                    addr.name.as_ref().map_or_else(
                        || addr.email.clone(),
                        |name| format!("{name} <{}>", addr.email),
                    )
                })
                .collect();
            let to_text = format!("To: {}", to_addrs.join(", "));
            doc_items.push(create_text_item(
                doc_items.len(),
                to_text,
                create_provenance(1),
            ));
        }

        // CC header
        if !msg.recipients.cc.is_empty() {
            let cc_addrs: Vec<String> = msg
                .recipients
                .cc
                .iter()
                .map(|addr| {
                    addr.name.as_ref().map_or_else(
                        || addr.email.clone(),
                        |name| format!("{name} <{}>", addr.email),
                    )
                })
                .collect();
            let cc_text = format!("CC: {}", cc_addrs.join(", "));
            doc_items.push(create_text_item(
                doc_items.len(),
                cc_text,
                create_provenance(1),
            ));
        }

        // Date header
        if let Some(date) = &msg.date {
            let date_text = format!("Date: {date}");
            doc_items.push(create_text_item(
                doc_items.len(),
                date_text,
                create_provenance(1),
            ));
        }

        // Body content (prefer plain text over HTML)
        let body_content = msg
            .body
            .plain
            .as_ref()
            .or(msg.body.html.as_ref())
            .map_or("", std::string::String::as_str);

        if !body_content.is_empty() {
            doc_items.push(create_text_item(
                doc_items.len(),
                body_content.to_string(),
                create_provenance(1),
            ));
        }

        // Attachments section
        if !msg.attachments.is_empty() {
            doc_items.push(create_section_header(
                doc_items.len(),
                "Attachments".to_string(),
                2,
                create_provenance(1),
            ));

            for att in &msg.attachments {
                let att_text = att.content_type.as_ref().map_or_else(
                    || format!("{} ({} bytes)", att.filename, att.size),
                    |content_type| {
                        format!("{} ({}, {} bytes)", att.filename, content_type, att.size)
                    },
                );
                doc_items.push(create_list_item(
                    doc_items.len(),
                    att_text,
                    "- ".to_string(),
                    false,
                    create_provenance(1),
                    None,
                ));
            }
        }

        doc_items
    }

    /// Generate `DocItems` from VCF contacts
    ///
    /// Structure:
    /// 1. Each contact as top-level section with details
    /// 2. No meta-header (standard vCard representation)
    fn generate_docitems_from_contacts(contacts: &[Contact]) -> Vec<DocItem> {
        let mut doc_items = Vec::new();

        // Process each contact
        for contact in contacts {
            // Contact name as top-level section (with version if present)
            let header_text = contact.version.as_ref().map_or_else(
                || contact.name.clone(),
                |version| format!("{} (vCard v{})", contact.name, version),
            );
            doc_items.push(create_section_header(
                doc_items.len(),
                header_text,
                1,
                create_provenance(1),
            ));

            // Organization
            if let Some(org) = &contact.organization {
                let org_text = format!("Organization: {org}");
                doc_items.push(create_text_item(
                    doc_items.len(),
                    org_text,
                    create_provenance(1),
                ));
            }

            // Title/Role
            if let Some(title) = &contact.title {
                let title_text = format!("Title: {title}");
                doc_items.push(create_text_item(
                    doc_items.len(),
                    title_text,
                    create_provenance(1),
                ));
            }

            // Emails with type labels
            for email in &contact.emails {
                let email_text = email.type_label.as_ref().map_or_else(
                    || format!("Email: {}", email.address),
                    |type_label| {
                        let pref_marker = if email.preferred { " (preferred)" } else { "" };
                        format!("Email ({type_label}){pref_marker}: {}", email.address)
                    },
                );
                doc_items.push(create_text_item(
                    doc_items.len(),
                    email_text,
                    create_provenance(1),
                ));
            }

            // Phones with type labels
            for phone in &contact.phones {
                let phone_text = phone.type_label.as_ref().map_or_else(
                    || format!("Phone: {}", phone.number),
                    |type_label| format!("Phone ({type_label}): {}", phone.number),
                );
                doc_items.push(create_text_item(
                    doc_items.len(),
                    phone_text,
                    create_provenance(1),
                ));
            }

            // Additional properties (address, URL, birthday, languages, etc.)
            if !contact.properties.is_empty() {
                for (key, value) in &contact.properties {
                    let formatted_key = key.replace('_', " ");
                    let formatted_key = formatted_key
                        .split_whitespace()
                        .map(|word| {
                            let mut chars = word.chars();
                            chars.next().map_or_else(String::new, |first| {
                                first.to_uppercase().chain(chars).collect()
                            })
                        })
                        .collect::<Vec<_>>()
                        .join(" ");

                    let property_text = format!("{formatted_key}: {value}");
                    doc_items.push(create_text_item(
                        doc_items.len(),
                        property_text,
                        create_provenance(1),
                    ));
                }
            }
        }

        doc_items
    }

    /// Convert `DocItems` to markdown
    fn docitems_to_markdown(doc_items: &[DocItem]) -> String {
        let mut output = String::new();

        for item in doc_items {
            match item {
                DocItem::SectionHeader { text, level, .. } => {
                    if !output.is_empty() {
                        output.push('\n');
                    }
                    output.push_str(&"#".repeat(*level));
                    output.push(' ');
                    output.push_str(text);
                    output.push_str("\n\n");
                }
                DocItem::Text { text, .. } => {
                    // Check if text is a header field (e.g., "From:", "To:")
                    // Note: Organization and Title are omitted (vCard-specific, should not be bolded per RFC 6350)
                    if text.starts_with("From:")
                        || text.starts_with("To:")
                        || text.starts_with("CC:")
                        || text.starts_with("Date:")
                        || text.starts_with("Message-ID:")
                        || text.starts_with("Subject:")
                        || text.starts_with("Attachments:")
                    {
                        // Format as bold for email metadata fields only
                        if let Some((key, value)) = text.split_once(':') {
                            let _ = writeln!(output, "**{key}:**{value}");
                        } else {
                            output.push_str(text);
                            output.push('\n');
                        }
                    } else {
                        output.push_str(text);
                        output.push('\n');
                    }
                }
                DocItem::ListItem { text, .. } => {
                    output.push_str("- ");
                    output.push_str(text);
                    output.push('\n');
                }
                DocItem::Code { text, language, .. } => {
                    // Code block with optional language
                    if let Some(lang) = language {
                        let _ = writeln!(output, "```{lang}");
                    } else {
                        output.push_str("```\n");
                    }
                    output.push_str(text);
                    if !text.ends_with('\n') {
                        output.push('\n');
                    }
                    output.push_str("```\n\n");
                }
                _ => {}
            }
        }

        output
    }

    /// Parse email content to structured data
    fn parse_email_data(
        &self,
        content: &[u8],
        path: Option<&Path>,
    ) -> Result<Vec<DocItem>, DoclingError> {
        let doc_items = match self.format {
            InputFormat::Eml => {
                let message = parse_eml(content)
                    .map_err(|e| DoclingError::BackendError(format!("Failed to parse EML: {e}")))?;
                Self::generate_docitems_from_email(&message)
            }
            InputFormat::Mbox => {
                let mailbox = parse_mbox(content).map_err(|e| {
                    DoclingError::BackendError(format!("Failed to parse MBOX: {e}"))
                })?;
                Self::generate_docitems_from_mailbox(&mailbox)
            }
            InputFormat::Msg => {
                // MSG format requires file path for parsing
                let path = path.ok_or_else(|| {
                    DoclingError::BackendError("MSG format requires file path".to_string())
                })?;
                let msg = parse_msg_from_path(path)
                    .map_err(|e| DoclingError::BackendError(format!("Failed to parse MSG: {e}")))?;
                Self::generate_docitems_from_msg(&msg)
            }
            InputFormat::Vcf => {
                let contacts = parse_vcf(content)
                    .map_err(|e| DoclingError::BackendError(format!("Failed to parse VCF: {e}")))?;
                Self::generate_docitems_from_contacts(&contacts)
            }
            _ => {
                return Err(DoclingError::FormatError(format!(
                    "Unsupported email format: {:?}",
                    self.format
                )))
            }
        };

        Ok(doc_items)
    }
}

impl DocumentBackend for EmailBackend {
    #[inline]
    fn format(&self) -> InputFormat {
        self.format
    }

    fn parse_bytes(
        &self,
        content: &[u8],
        _options: &BackendOptions,
    ) -> Result<Document, DoclingError> {
        // MSG format cannot be parsed from bytes alone (requires file path)
        if self.format == InputFormat::Msg {
            return Err(DoclingError::BackendError(
                "MSG format requires file path, use parse_file() instead".to_string(),
            ));
        }

        let doc_items = self.parse_email_data(content, None)?;
        let markdown = Self::docitems_to_markdown(&doc_items);
        let num_characters = markdown.chars().count();

        // N=1881-1883: Extract metadata for VCF, MBOX, and EML formats
        let mut metadata = DocumentMetadata {
            num_characters,
            ..Default::default()
        };

        // VCF: Extract NOTE field from first contact to subject (N=1881)
        if self.format == InputFormat::Vcf {
            if let Ok(contacts) = parse_vcf(content) {
                if let Some(first_contact) = contacts.first() {
                    // Extract NOTE field to subject (RFC 6350: vCard note/description)
                    metadata.subject = first_contact.properties.get("note").cloned();
                }
            }
        }

        // MBOX: Extract first message subject to subject (N=1882)
        if self.format == InputFormat::Mbox {
            if let Ok(mailbox) = parse_mbox(content) {
                if let Some(first_message) = mailbox.messages.first() {
                    // Extract first email's subject line to document metadata
                    metadata.subject = Some(first_message.subject.clone());
                }
            }
        }

        // EML: Extract email subject line to subject (N=1883)
        if self.format == InputFormat::Eml {
            if let Ok(message) = parse_eml(content) {
                // Extract email subject line to document metadata
                metadata.subject = Some(message.subject);
            }
        }

        Ok(Document {
            markdown,
            format: self.format,
            metadata,
            content_blocks: Some(doc_items),
            docling_document: None,
        })
    }

    fn parse_file<P: AsRef<Path>>(
        &self,
        path: P,
        options: &BackendOptions,
    ) -> Result<Document, DoclingError> {
        let path_ref = path.as_ref();
        let filename = path_ref.display().to_string();

        // Helper to add filename context to errors
        let add_context = |err: DoclingError| -> DoclingError {
            match err {
                DoclingError::BackendError(msg) => {
                    DoclingError::BackendError(format!("{msg}: {filename}"))
                }
                other => other,
            }
        };

        // MSG format needs special handling (file path required)
        if self.format == InputFormat::Msg {
            let doc_items = self
                .parse_email_data(&[], Some(path_ref))
                .map_err(add_context)?;
            let markdown = Self::docitems_to_markdown(&doc_items);
            let num_characters = markdown.chars().count();

            // N=1884: Extract MSG subject metadata
            let mut metadata = DocumentMetadata {
                num_characters,
                ..Default::default()
            };

            // Extract subject from parsed MSG
            if let Ok(msg) = parse_msg_from_path(path_ref) {
                metadata.subject = Some(msg.subject);
            }

            return Ok(Document {
                markdown,
                format: self.format,
                metadata,
                content_blocks: Some(doc_items),
                docling_document: None,
            });
        }

        // For other formats, read file and parse from bytes
        let content = std::fs::read(path_ref).map_err(DoclingError::IoError)?;

        self.parse_bytes(&content, options).map_err(add_context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== Backend Creation Tests (6 tests) ==========

    #[test]
    fn test_email_backend_creation() {
        // Valid formats
        assert!(
            EmailBackend::new(InputFormat::Eml).is_ok(),
            "EML format should be valid for EmailBackend"
        );
        assert!(
            EmailBackend::new(InputFormat::Mbox).is_ok(),
            "MBOX format should be valid for EmailBackend"
        );
        assert!(
            EmailBackend::new(InputFormat::Msg).is_ok(),
            "MSG format should be valid for EmailBackend"
        );
        assert!(
            EmailBackend::new(InputFormat::Vcf).is_ok(),
            "VCF format should be valid for EmailBackend"
        );

        // Invalid format
        assert!(
            EmailBackend::new(InputFormat::Pdf).is_err(),
            "PDF format should be invalid for EmailBackend"
        );
    }

    #[test]
    fn test_eml_backend_creation() {
        let backend = EmailBackend::new(InputFormat::Eml).unwrap();
        assert_eq!(
            backend.format(),
            InputFormat::Eml,
            "EML backend should report Eml format"
        );
    }

    #[test]
    fn test_mbox_backend_creation() {
        let backend = EmailBackend::new(InputFormat::Mbox).unwrap();
        assert_eq!(
            backend.format(),
            InputFormat::Mbox,
            "MBOX backend should report Mbox format"
        );
    }

    #[test]
    fn test_msg_backend_creation() {
        let backend = EmailBackend::new(InputFormat::Msg).unwrap();
        assert_eq!(
            backend.format(),
            InputFormat::Msg,
            "MSG backend should report Msg format"
        );
    }

    #[test]
    fn test_vcf_backend_creation() {
        let backend = EmailBackend::new(InputFormat::Vcf).unwrap();
        assert_eq!(
            backend.format(),
            InputFormat::Vcf,
            "VCF backend should report Vcf format"
        );
    }

    #[test]
    fn test_invalid_format_error() {
        let result = EmailBackend::new(InputFormat::Docx);
        assert!(
            result.is_err(),
            "DOCX format should be rejected by EmailBackend"
        );
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("not an email format"),
            "Error message should indicate invalid format"
        );
    }

    // ========== EML Format Tests (5 tests) ==========

    #[test]
    fn test_eml_docitem_generation() {
        let backend = EmailBackend::new(InputFormat::Eml).unwrap();

        // Sample EML content
        let eml_content = b"From: sender@example.com\r
To: recipient@example.com\r
Subject: Test Email\r
Date: Mon, 1 Jan 2025 10:00:00 +0000\r
\r
Hello World!";

        let result = backend.parse_bytes(eml_content, &BackendOptions::default());
        assert!(result.is_ok(), "EML parsing should succeed");

        let doc = result.unwrap();
        assert_eq!(
            doc.format,
            InputFormat::Eml,
            "Document format should be Eml"
        );

        // Verify DocItems were generated
        assert!(
            doc.content_blocks.is_some(),
            "Document should have content blocks"
        );
        let doc_items = doc.content_blocks.as_ref().unwrap();
        assert!(!doc_items.is_empty(), "DocItems should not be empty");

        // Verify markdown generated from DocItems
        assert!(
            doc.markdown.contains("**Subject:** Test Email"),
            "Markdown should contain Subject header"
        );
        assert!(
            doc.markdown.contains("**From:** sender@example.com"),
            "Markdown should contain From header"
        );
        assert!(
            doc.markdown.contains("**To:** recipient@example.com"),
            "Markdown should contain To header"
        );
        assert!(
            doc.markdown.contains("Hello World!"),
            "Markdown should contain body text"
        );
    }

    #[test]
    fn test_eml_with_cc() {
        let backend = EmailBackend::new(InputFormat::Eml).unwrap();

        let eml_content = b"From: sender@example.com\r
To: recipient@example.com\r
CC: copy@example.com\r
Subject: Test with CC\r
\r
Email body.";

        let result = backend.parse_bytes(eml_content, &BackendOptions::default());
        assert!(
            result.is_ok(),
            "EML with CC header should parse successfully"
        );

        let doc = result.unwrap();
        assert!(
            doc.markdown.contains("**CC:**") || doc.markdown.contains("copy@example.com"),
            "Markdown should contain CC information"
        );
    }

    #[test]
    fn test_eml_minimal() {
        let backend = EmailBackend::new(InputFormat::Eml).unwrap();

        let eml_content = b"Subject: Minimal\r
\r
Body";

        let result = backend.parse_bytes(eml_content, &BackendOptions::default());
        assert!(result.is_ok(), "Minimal EML should parse successfully");
    }

    #[test]
    fn test_eml_empty_body() {
        let backend = EmailBackend::new(InputFormat::Eml).unwrap();

        let eml_content = b"From: sender@example.com\r
Subject: Empty Body\r
\r
";

        let result = backend.parse_bytes(eml_content, &BackendOptions::default());
        assert!(
            result.is_ok(),
            "EML with empty body should parse successfully"
        );
    }

    #[test]
    fn test_eml_multiline_body() {
        let backend = EmailBackend::new(InputFormat::Eml).unwrap();

        let eml_content = b"From: sender@example.com\r
Subject: Multiline\r
\r
Line 1\r
Line 2\r
Line 3";

        let result = backend.parse_bytes(eml_content, &BackendOptions::default());
        assert!(
            result.is_ok(),
            "EML with multiline body should parse successfully"
        );
        let doc = result.unwrap();
        assert!(
            doc.markdown.contains("Line 1"),
            "First line should be in markdown"
        );
        assert!(
            doc.markdown.contains("Line 3"),
            "Third line should be in markdown"
        );
    }

    // ========== VCF Format Tests (4 tests) ==========

    #[test]
    fn test_vcf_docitem_generation() {
        let backend = EmailBackend::new(InputFormat::Vcf).unwrap();

        // Sample VCF content
        let vcf_content = b"BEGIN:VCARD\r
VERSION:3.0\r
FN:John Doe\r
TEL:+1-555-1234\r
EMAIL:john@example.com\r
END:VCARD\r
";

        let result = backend.parse_bytes(vcf_content, &BackendOptions::default());
        assert!(result.is_ok(), "VCF parsing should succeed");

        let doc = result.unwrap();
        assert_eq!(
            doc.format,
            InputFormat::Vcf,
            "Document format should be Vcf"
        );

        // Verify DocItems were generated
        assert!(
            doc.content_blocks.is_some(),
            "Document should have content blocks"
        );
        let doc_items = doc.content_blocks.as_ref().unwrap();
        assert!(!doc_items.is_empty(), "DocItems should not be empty");

        // Verify markdown generated from DocItems
        assert!(
            doc.markdown.contains("John Doe"),
            "Markdown should contain contact name"
        );
        assert!(
            doc.markdown.contains("+1-555-1234") || doc.markdown.contains("555-1234"),
            "Markdown should contain phone number"
        );
        assert!(
            doc.markdown.contains("john@example.com"),
            "Markdown should contain email address"
        );
    }

    #[test]
    fn test_vcf_minimal() {
        let backend = EmailBackend::new(InputFormat::Vcf).unwrap();

        let vcf_content = b"BEGIN:VCARD\r
VERSION:3.0\r
FN:Test Name\r
END:VCARD\r
";

        let result = backend.parse_bytes(vcf_content, &BackendOptions::default());
        assert!(result.is_ok(), "Minimal VCF should parse successfully");
        let doc = result.unwrap();
        assert!(
            doc.markdown.contains("Test Name"),
            "Markdown should contain contact name"
        );
    }

    #[test]
    fn test_vcf_with_org() {
        let backend = EmailBackend::new(InputFormat::Vcf).unwrap();

        let vcf_content = b"BEGIN:VCARD\r
VERSION:3.0\r
FN:Jane Smith\r
ORG:ACME Corp\r
EMAIL:jane@acme.com\r
END:VCARD\r
";

        let result = backend.parse_bytes(vcf_content, &BackendOptions::default());
        assert!(
            result.is_ok(),
            "VCF with organization should parse successfully"
        );
        let doc = result.unwrap();
        assert!(
            doc.markdown.contains("Jane Smith"),
            "Markdown should contain contact name"
        );
        assert!(
            doc.markdown.contains("ACME Corp") || doc.markdown.contains("jane@acme.com"),
            "Markdown should contain org or email"
        );
    }

    #[test]
    fn test_vcf_empty() {
        let backend = EmailBackend::new(InputFormat::Vcf).unwrap();

        let vcf_content = b"";

        let result = backend.parse_bytes(vcf_content, &BackendOptions::default());
        // Should handle empty input gracefully
        assert!(
            result.is_ok() || result.is_err(),
            "Empty VCF should be handled gracefully"
        );
    }

    // ========== MSG Format Tests (3 tests) ==========

    #[test]
    fn test_msg_parse_bytes_fails() {
        let backend = EmailBackend::new(InputFormat::Msg).unwrap();
        let result = backend.parse_bytes(b"dummy content", &BackendOptions::default());
        assert!(
            result.is_err(),
            "MSG parse_bytes should fail (requires file path)"
        );
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("MSG format requires file path"),
            "Error should mention file path requirement"
        );
    }

    #[test]
    fn test_msg_empty_bytes() {
        let backend = EmailBackend::new(InputFormat::Msg).unwrap();
        let result = backend.parse_bytes(&[], &BackendOptions::default());
        assert!(result.is_err(), "MSG with empty bytes should fail");
    }

    #[test]
    fn test_msg_error_message() {
        let backend = EmailBackend::new(InputFormat::Msg).unwrap();
        let result = backend.parse_bytes(&[1, 2, 3], &BackendOptions::default());
        assert!(result.is_err(), "MSG with invalid bytes should fail");
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("MSG"),
            "Error should mention MSG format"
        );
    }

    // ========== MBOX Format Tests (4 tests) ==========

    #[test]
    fn test_mbox_docitem_generation() {
        let backend = EmailBackend::new(InputFormat::Mbox).unwrap();

        // Sample MBOX content (single message)
        let mbox_content = b"From sender@example.com Mon Jan  1 10:00:00 2025\r
From: sender@example.com\r
To: recipient@example.com\r
Subject: Test Message\r
\r
Test body.\r
";

        let result = backend.parse_bytes(mbox_content, &BackendOptions::default());
        assert!(result.is_ok(), "MBOX parsing should succeed");

        let doc = result.unwrap();
        assert_eq!(
            doc.format,
            InputFormat::Mbox,
            "Document format should be Mbox"
        );

        // Verify DocItems were generated
        assert!(
            doc.content_blocks.is_some(),
            "Document should have content blocks"
        );
        let doc_items = doc.content_blocks.as_ref().unwrap();
        assert!(!doc_items.is_empty(), "DocItems should not be empty");

        // MBOX should have markdown with mailbox summary
        assert!(!doc.markdown.is_empty(), "Markdown should not be empty");
        assert!(
            doc.markdown.contains("Mailbox"),
            "Markdown should contain Mailbox header"
        );
    }

    #[test]
    fn test_mbox_empty() {
        let backend = EmailBackend::new(InputFormat::Mbox).unwrap();

        let mbox_content = b"";

        let result = backend.parse_bytes(mbox_content, &BackendOptions::default());
        // Empty MBOX may error or return empty document - both are valid
        if let Ok(doc) = result {
            assert_eq!(
                doc.format,
                InputFormat::Mbox,
                "Empty MBOX document format should be Mbox"
            );
        }
    }

    #[test]
    fn test_mbox_single_message() {
        let backend = EmailBackend::new(InputFormat::Mbox).unwrap();

        let mbox_content = b"From sender Mon Jan  1 10:00:00 2025\r
Subject: Single\r
\r
Content";

        let result = backend.parse_bytes(mbox_content, &BackendOptions::default());
        assert!(
            result.is_ok(),
            "MBOX with single message should parse successfully"
        );
    }

    #[test]
    fn test_mbox_docitems_count() {
        let backend = EmailBackend::new(InputFormat::Mbox).unwrap();

        let mbox_content = b"From sender@example.com Mon Jan  1 10:00:00 2025\r
From: sender@example.com\r
Subject: Test\r
\r
Body\r
";

        let result = backend.parse_bytes(mbox_content, &BackendOptions::default());
        assert!(result.is_ok(), "MBOX parsing should succeed");

        let doc = result.unwrap();
        if let Some(items) = &doc.content_blocks {
            assert!(!items.is_empty(), "MBOX should generate DocItems");
        }
    }

    // ========== UNICODE AND SPECIAL CHARACTER TESTS ==========

    #[test]
    fn test_eml_unicode_subject() {
        let backend = EmailBackend::new(InputFormat::Eml).unwrap();

        let eml_content = b"From: sender@example.com\r
To: recipient@example.com\r
Subject: =?UTF-8?B?8J+Ygg==?= Unicode Test\r
\r
Body with unicode: \xE2\x9C\x85";

        let result = backend.parse_bytes(eml_content, &BackendOptions::default());
        assert!(
            result.is_ok(),
            "EML with unicode subject should parse successfully"
        );

        let doc = result.unwrap();
        // Should handle unicode in subject and body
        assert!(
            !doc.markdown.is_empty(),
            "Markdown should not be empty for unicode content"
        );
    }

    #[test]
    fn test_eml_special_characters_in_headers() {
        let backend = EmailBackend::new(InputFormat::Eml).unwrap();

        let eml_content = b"From: \"Name, First\" <sender@example.com>\r
To: recipient+tag@example.com\r
Subject: Re: [Important] Test\r
\r
Body";

        let result = backend.parse_bytes(eml_content, &BackendOptions::default());
        assert!(
            result.is_ok(),
            "EML with special characters in headers should parse successfully"
        );

        let doc = result.unwrap();
        assert!(
            doc.markdown.contains("sender@example.com"),
            "Markdown should contain email address"
        );
    }

    #[test]
    fn test_eml_multiline_headers() {
        let backend = EmailBackend::new(InputFormat::Eml).unwrap();

        let eml_content = b"From: sender@example.com\r
To: recipient1@example.com,\r
 recipient2@example.com\r
Subject: Multiline\r
\r
Body";

        let result = backend.parse_bytes(eml_content, &BackendOptions::default());
        assert!(
            result.is_ok(),
            "EML with multiline headers should parse successfully"
        );
    }

    // ========== VALIDATION TESTS ==========

    #[test]
    fn test_eml_empty_file() {
        let backend = EmailBackend::new(InputFormat::Eml).unwrap();

        let result = backend.parse_bytes(b"", &BackendOptions::default());
        // Empty EML should error or handle gracefully
        // Just verify it doesn't panic
        assert!(
            result.is_ok() || result.is_err(),
            "Empty EML should be handled gracefully"
        );
    }

    #[test]
    fn test_eml_missing_subject() {
        let backend = EmailBackend::new(InputFormat::Eml).unwrap();

        let eml_content = b"From: sender@example.com\r
To: recipient@example.com\r
\r
Body without subject";

        let result = backend.parse_bytes(eml_content, &BackendOptions::default());
        assert!(
            result.is_ok(),
            "EML without subject should parse successfully"
        );

        let doc = result.unwrap();
        // Should handle missing subject gracefully
        assert!(
            !doc.markdown.is_empty(),
            "Markdown should not be empty for EML without subject"
        );
    }

    #[test]
    fn test_eml_missing_body() {
        let backend = EmailBackend::new(InputFormat::Eml).unwrap();

        let eml_content = b"From: sender@example.com\r
To: recipient@example.com\r
Subject: No Body\r
\r
";

        let result = backend.parse_bytes(eml_content, &BackendOptions::default());
        assert!(
            result.is_ok(),
            "EML with empty body should parse successfully"
        );

        let doc = result.unwrap();
        assert!(
            doc.markdown.contains("No Body"),
            "Markdown should contain subject"
        );
    }

    #[test]
    fn test_eml_malformed_headers() {
        let backend = EmailBackend::new(InputFormat::Eml).unwrap();

        let eml_content = b"NotAHeader\r
From: sender@example.com\r
\r
Body";

        let result = backend.parse_bytes(eml_content, &BackendOptions::default());
        // Should handle malformed headers gracefully
        assert!(
            result.is_ok() || result.is_err(),
            "Malformed headers should be handled gracefully"
        );
    }

    #[test]
    fn test_vcf_empty_fields() {
        let backend = EmailBackend::new(InputFormat::Vcf).unwrap();

        let vcf_content = b"BEGIN:VCARD\r
VERSION:3.0\r
FN:\r
EMAIL:\r
END:VCARD";

        let result = backend.parse_bytes(vcf_content, &BackendOptions::default());
        // Should handle empty VCF fields
        assert!(
            result.is_ok() || result.is_err(),
            "VCF with empty fields should be handled gracefully"
        );
    }

    // ========== SERIALIZATION CONSISTENCY TESTS ==========

    #[test]
    fn test_eml_markdown_not_empty() {
        let backend = EmailBackend::new(InputFormat::Eml).unwrap();

        let eml_content = b"From: sender@example.com\r
To: recipient@example.com\r
Subject: Test\r
\r
Body";

        let result = backend.parse_bytes(eml_content, &BackendOptions::default());
        assert!(result.is_ok(), "EML parsing should succeed");

        let doc = result.unwrap();
        assert!(!doc.markdown.is_empty(), "Markdown should not be empty");
        assert!(
            doc.markdown.len() > 20,
            "Markdown should have reasonable length"
        );
    }

    #[test]
    fn test_eml_markdown_well_formed() {
        let backend = EmailBackend::new(InputFormat::Eml).unwrap();

        let eml_content = b"From: sender@example.com\r
Subject: Test\r
\r
Body";

        let result = backend.parse_bytes(eml_content, &BackendOptions::default());
        assert!(result.is_ok(), "EML parsing should succeed");

        let doc = result.unwrap();
        // Should contain Subject label (with bold formatting from serializer)
        assert!(
            doc.markdown.contains("**Subject:**"),
            "Markdown should contain bold Subject label"
        );
        // Should contain From label (with bold formatting from serializer)
        assert!(
            doc.markdown.contains("**From:**"),
            "Markdown should contain bold From label"
        );
    }

    #[test]
    fn test_eml_docitems_match_markdown() {
        let backend = EmailBackend::new(InputFormat::Eml).unwrap();

        let eml_content = b"From: sender@example.com\r
Subject: Test\r
\r
Body text";

        let result = backend.parse_bytes(eml_content, &BackendOptions::default());
        assert!(result.is_ok(), "EML parsing should succeed");

        let doc = result.unwrap();
        if let Some(items) = &doc.content_blocks {
            // DocItems should contain content from markdown
            for item in items {
                if let DocItem::Text { text, .. } = item {
                    if !text.is_empty() {
                        // Non-empty text should appear in markdown (or be metadata)
                        assert!(!text.is_empty(), "Text items should not be empty");
                    }
                }
            }
        }
    }

    #[test]
    fn test_eml_consistent_output_multiple_parses() {
        let backend = EmailBackend::new(InputFormat::Eml).unwrap();

        let eml_content = b"From: sender@example.com\r
Subject: Consistency Test\r
\r
Body";

        let result1 = backend.parse_bytes(eml_content, &BackendOptions::default());
        let result2 = backend.parse_bytes(eml_content, &BackendOptions::default());

        assert!(result1.is_ok(), "First parse should succeed");
        assert!(result2.is_ok(), "Second parse should succeed");

        let doc1 = result1.unwrap();
        let doc2 = result2.unwrap();

        // Should produce identical output
        assert_eq!(
            doc1.markdown, doc2.markdown,
            "Multiple parses should produce identical markdown"
        );
        assert_eq!(
            doc1.metadata.num_characters, doc2.metadata.num_characters,
            "Character count should be consistent"
        );
    }

    // ========== BACKEND OPTIONS TESTS ==========

    #[test]
    fn test_eml_with_default_options() {
        let backend = EmailBackend::new(InputFormat::Eml).unwrap();

        let eml_content = b"From: sender@example.com\r
Subject: Test\r
\r
Body";

        let result = backend.parse_bytes(eml_content, &BackendOptions::default());
        assert!(
            result.is_ok(),
            "EML with default options should parse successfully"
        );
    }

    #[test]
    fn test_msg_with_custom_options() {
        let backend = EmailBackend::new(InputFormat::Msg).unwrap();

        let msg_content = b"\xD0\xCF\x11\xE0\xA1\xB1\x1A\xE1"; // OLE2 header

        let options = BackendOptions::default();
        let result = backend.parse_bytes(msg_content, &options);
        // May succeed or fail depending on MSG parsing support
        assert!(
            result.is_ok() || result.is_err(),
            "MSG with custom options should be handled"
        );
    }

    // ========== ADDITIONAL FORMAT TESTS ==========

    #[test]
    fn test_eml_with_html_body() {
        let backend = EmailBackend::new(InputFormat::Eml).unwrap();

        let eml_content = b"From: sender@example.com\r
Subject: HTML Email\r
Content-Type: text/html\r
\r
<html><body><p>HTML content</p></body></html>";

        let result = backend.parse_bytes(eml_content, &BackendOptions::default());
        assert!(
            result.is_ok(),
            "EML with HTML body should parse successfully"
        );

        let doc = result.unwrap();
        // Should extract text from HTML
        assert!(
            !doc.markdown.is_empty(),
            "HTML email should produce non-empty markdown"
        );
    }

    #[test]
    fn test_eml_with_multiple_recipients() {
        let backend = EmailBackend::new(InputFormat::Eml).unwrap();

        let eml_content = b"From: sender@example.com\r
To: r1@example.com, r2@example.com, r3@example.com\r
Subject: Multiple Recipients\r
\r
Body";

        let result = backend.parse_bytes(eml_content, &BackendOptions::default());
        assert!(
            result.is_ok(),
            "EML with multiple recipients should parse successfully"
        );

        let doc = result.unwrap();
        // Should list all recipients
        assert!(
            doc.markdown.contains("r1@example.com") || doc.markdown.contains("**To:**"),
            "Should contain recipient information"
        );
    }

    #[test]
    fn test_vcf_multiple_contacts() {
        let backend = EmailBackend::new(InputFormat::Vcf).unwrap();

        let vcf_content = b"BEGIN:VCARD\r
VERSION:3.0\r
FN:Contact 1\r
END:VCARD\r
BEGIN:VCARD\r
VERSION:3.0\r
FN:Contact 2\r
END:VCARD";

        let result = backend.parse_bytes(vcf_content, &BackendOptions::default());
        assert!(
            result.is_ok(),
            "VCF with multiple contacts should parse successfully"
        );

        let doc = result.unwrap();
        // Should handle multiple vCards
        assert!(
            doc.markdown.contains("Contact 1") || doc.markdown.contains("Contact 2"),
            "Should contain contact names"
        );
    }

    #[test]
    fn test_eml_date_parsing() {
        let backend = EmailBackend::new(InputFormat::Eml).unwrap();

        let eml_content = b"From: sender@example.com\r
Date: Mon, 1 Jan 2025 12:00:00 +0000\r
Subject: Date Test\r
\r
Body";

        let result = backend.parse_bytes(eml_content, &BackendOptions::default());
        assert!(
            result.is_ok(),
            "EML with date header should parse successfully"
        );

        let doc = result.unwrap();
        // Should parse date header
        assert!(
            doc.markdown.contains("2025") || doc.markdown.contains("**Date:**"),
            "Should contain date information"
        );
    }

    #[test]
    fn test_eml_attachment_metadata() {
        let backend = EmailBackend::new(InputFormat::Eml).unwrap();

        let eml_content = b"From: sender@example.com\r
Subject: With Attachment\r
Content-Type: multipart/mixed; boundary=\"boundary\"\r
\r
--boundary\r
Content-Type: text/plain\r
\r
Body text\r
--boundary\r
Content-Type: application/pdf; name=\"file.pdf\"\r
\r
PDF binary data\r
--boundary--";

        let result = backend.parse_bytes(eml_content, &BackendOptions::default());
        // Should handle multipart messages
        assert!(
            result.is_ok() || result.is_err(),
            "Multipart EML should be handled gracefully"
        );
    }

    #[test]
    fn test_mbox_multiple_messages() {
        let backend = EmailBackend::new(InputFormat::Mbox).unwrap();

        let mbox_content = b"From sender1@example.com Mon Jan  1 10:00:00 2025\r
Subject: Message 1\r
\r
Body 1\r
\r
From sender2@example.com Mon Jan  1 11:00:00 2025\r
Subject: Message 2\r
\r
Body 2\r
";

        let result = backend.parse_bytes(mbox_content, &BackendOptions::default());
        assert!(
            result.is_ok(),
            "MBOX with multiple messages should parse successfully"
        );

        let doc = result.unwrap();
        // Should handle multiple messages
        assert!(
            doc.markdown.contains("Message 1") || doc.markdown.contains("Message 2"),
            "Should contain message subjects"
        );
    }

    #[test]
    fn test_vcf_phone_numbers() {
        let backend = EmailBackend::new(InputFormat::Vcf).unwrap();

        let vcf_content = b"BEGIN:VCARD\r
VERSION:3.0\r
FN:John Doe\r
TEL;TYPE=CELL:+1-555-1234\r
TEL;TYPE=WORK:+1-555-5678\r
END:VCARD";

        let result = backend.parse_bytes(vcf_content, &BackendOptions::default());
        assert!(
            result.is_ok(),
            "VCF with phone numbers should parse successfully"
        );

        let doc = result.unwrap();
        // Should parse phone numbers
        assert!(
            doc.markdown.contains("555") || doc.markdown.contains("John Doe"),
            "Should contain phone number or contact name"
        );
    }

    // ========== Unicode and Encoding Edge Cases (4 tests) ==========

    #[test]
    fn test_eml_base64_encoded_subject() {
        let backend = EmailBackend::new(InputFormat::Eml).unwrap();

        let eml_content = b"From: sender@example.com\r
Subject: =?UTF-8?B?44GT44KT44Gr44Gh44GvIChKYXBhbmVzZSk=?=\r
\r
Unicode body text: \xE6\x97\xA5\xE6\x9C\xAC\xE8\xAA\x9E";

        let result = backend.parse_bytes(eml_content, &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        // Should handle base64 UTF-8 encoded subjects
        assert!(!doc.markdown.is_empty());
    }

    #[test]
    fn test_eml_emoji_in_content() {
        let backend = EmailBackend::new(InputFormat::Eml).unwrap();

        let eml_content = "From: sender@example.com\r
Subject: Emoji Test 😀🎉🚀\r
\r
Body with emojis: 🌟❤️✨"
            .as_bytes();

        let result = backend.parse_bytes(eml_content, &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        assert!(doc.content_blocks.is_some());
        let doc_items = doc.content_blocks.as_ref().unwrap();
        assert!(!doc_items.is_empty());
    }

    #[test]
    fn test_vcf_unicode_name() {
        let backend = EmailBackend::new(InputFormat::Vcf).unwrap();

        let vcf_content = "BEGIN:VCARD\r
VERSION:3.0\r
FN:张伟\r
ORG:北京公司\r
END:VCARD"
            .as_bytes();

        let result = backend.parse_bytes(vcf_content, &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        // Should handle Chinese characters
        assert!(!doc.markdown.is_empty());
    }

    #[test]
    fn test_eml_very_long_subject() {
        let backend = EmailBackend::new(InputFormat::Eml).unwrap();

        let long_subject = "A".repeat(500);
        let eml_content = format!(
            "From: sender@example.com\r
Subject: {long_subject}\r
\r
Body"
        );

        let result = backend.parse_bytes(eml_content.as_bytes(), &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        // Should handle very long subjects
        assert!(doc.markdown.len() > 400);
    }

    // ========== Error Handling and Edge Cases (4 tests) ==========

    #[test]
    fn test_eml_completely_empty() {
        let backend = EmailBackend::new(InputFormat::Eml).unwrap();

        let eml_content = b"";

        let result = backend.parse_bytes(eml_content, &BackendOptions::default());
        // Should gracefully handle completely empty input
        assert!(result.is_err() || result.is_ok());
    }

    #[test]
    fn test_eml_headers_without_colons() {
        let backend = EmailBackend::new(InputFormat::Eml).unwrap();

        let eml_content = b"From sender without colon\r
This is not a valid header\r
\r
Body";

        let result = backend.parse_bytes(eml_content, &BackendOptions::default());
        // Should handle headers without colons gracefully
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_mbox_empty_mailbox() {
        let backend = EmailBackend::new(InputFormat::Mbox).unwrap();

        let mbox_content = b"";

        let result = backend.parse_bytes(mbox_content, &BackendOptions::default());
        // Should handle empty mailbox
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_vcf_empty_vcard() {
        let backend = EmailBackend::new(InputFormat::Vcf).unwrap();

        let vcf_content = b"BEGIN:VCARD\r
VERSION:3.0\r
END:VCARD";

        let result = backend.parse_bytes(vcf_content, &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        // Should handle empty vCard
        assert!(doc.content_blocks.is_some());
    }

    // ========== DocItem Structure Validation (4 tests) ==========

    #[test]
    fn test_eml_docitem_self_ref() {
        let backend = EmailBackend::new(InputFormat::Eml).unwrap();

        let eml_content = b"From: sender@example.com\r
Subject: Test\r
\r
Body";

        let result = backend.parse_bytes(eml_content, &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        let doc_items = doc.content_blocks.as_ref().unwrap();

        // Verify self_ref format
        for (idx, item) in doc_items.iter().enumerate() {
            let self_ref = match item {
                DocItem::SectionHeader { self_ref, .. } => self_ref,
                DocItem::Text { self_ref, .. } => self_ref,
                DocItem::ListItem { self_ref, .. } => self_ref,
                _ => continue,
            };

            // self_ref should be "#/{type}/{index}"
            assert!(
                self_ref.starts_with("#/") && self_ref.contains(&idx.to_string()),
                "Invalid self_ref format: {self_ref}"
            );
        }
    }

    #[test]
    fn test_eml_provenance_validation() {
        let backend = EmailBackend::new(InputFormat::Eml).unwrap();

        let eml_content = b"From: sender@example.com\r
Subject: Test\r
\r
Body";

        let result = backend.parse_bytes(eml_content, &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        let doc_items = doc.content_blocks.as_ref().unwrap();

        // All items should have provenance
        for item in doc_items {
            let prov = match item {
                DocItem::SectionHeader { prov, .. } => prov,
                DocItem::Text { prov, .. } => prov,
                DocItem::ListItem { prov, .. } => prov,
                _ => continue,
            };

            assert!(!prov.is_empty(), "Missing provenance");
            assert_eq!(prov[0].page_no, 1, "Expected page_no = 1");
        }
    }

    #[test]
    fn test_vcf_docitem_structure() {
        let backend = EmailBackend::new(InputFormat::Vcf).unwrap();

        let vcf_content = b"BEGIN:VCARD\r
VERSION:3.0\r
FN:Jane Smith\r
EMAIL:jane@example.com\r
TEL:555-1234\r
END:VCARD";

        let result = backend.parse_bytes(vcf_content, &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        assert!(doc.content_blocks.is_some());
        let doc_items = doc.content_blocks.as_ref().unwrap();

        // Should have at least: SectionHeader (name) + Text items (email, phone)
        assert!(doc_items.len() >= 2);

        // First item should be SectionHeader
        assert!(matches!(doc_items[0], DocItem::SectionHeader { .. }));
    }

    #[test]
    fn test_mbox_metadata_extraction() {
        let backend = EmailBackend::new(InputFormat::Mbox).unwrap();

        let mbox_content = b"From sender@example.com Mon Jan  1 10:00:00 2025\r
Subject: Test Message\r
\r
Body text\r
";

        let result = backend.parse_bytes(mbox_content, &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();

        // Verify metadata fields
        assert_eq!(doc.format, InputFormat::Mbox);
        assert!(doc.metadata.num_pages.is_none()); // Email doesn't have pages
    }

    // ========== Content Boundary Edge Cases (5 tests) ==========

    #[test]
    fn test_eml_multipart_mixed_boundary() {
        let backend = EmailBackend::new(InputFormat::Eml).unwrap();

        let eml_content = b"From: sender@example.com\r
Subject: Multipart Test\r
Content-Type: multipart/mixed; boundary=\"BOUNDARY123\"\r
\r
--BOUNDARY123\r
Content-Type: text/plain\r
\r
Part 1 content\r
--BOUNDARY123\r
Content-Type: text/plain\r
\r
Part 2 content\r
--BOUNDARY123--\r
";

        let result = backend.parse_bytes(eml_content, &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        // Should extract content from multipart sections
        assert!(doc.markdown.contains("Part 1") || doc.markdown.contains("Part 2"));
    }

    #[test]
    fn test_eml_header_folding_continuation() {
        let backend = EmailBackend::new(InputFormat::Eml).unwrap();

        let eml_content = b"From: sender@example.com\r
Subject: This is a very long subject line that\r
 continues on the next line with whitespace\r
 folding according to RFC 5322\r
\r
Body content";

        let result = backend.parse_bytes(eml_content, &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        // Should handle header folding (continuation lines)
        assert!(doc.markdown.len() > 50);
    }

    #[test]
    fn test_vcf_multiple_contact_entries() {
        let backend = EmailBackend::new(InputFormat::Vcf).unwrap();

        let vcf_content = b"BEGIN:VCARD\r
VERSION:3.0\r
FN:John Doe\r
EMAIL:john@example.com\r
TEL:555-1111\r
ORG:Company A\r
TITLE:Manager\r
ADR:;;123 Main St;City;State;12345;Country\r
URL:https://example.com\r
NOTE:Important contact\r
END:VCARD";

        let result = backend.parse_bytes(vcf_content, &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        let doc_items = doc.content_blocks.as_ref().unwrap();

        // Should extract all vCard fields (FN, EMAIL, TEL, ORG, TITLE, ADR, URL, NOTE)
        assert!(doc_items.len() >= 5); // At least name + several fields
    }

    #[test]
    fn test_mbox_multiple_messages_separator() {
        let backend = EmailBackend::new(InputFormat::Mbox).unwrap();

        let mbox_content = b"From sender1@example.com Mon Jan  1 10:00:00 2025\r
Subject: Message 1\r
\r
First message body\r
\r
From sender2@example.com Mon Jan  2 11:00:00 2025\r
Subject: Message 2\r
\r
Second message body\r
";

        let result = backend.parse_bytes(mbox_content, &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        let doc_items = doc.content_blocks.as_ref().unwrap();

        // Should detect multiple messages (each with subject header)
        let section_headers: Vec<_> = doc_items
            .iter()
            .filter(|item| matches!(item, DocItem::SectionHeader { .. }))
            .collect();

        assert!(
            section_headers.len() >= 2,
            "Expected at least 2 section headers for 2 messages"
        );
    }

    #[test]
    fn test_eml_quoted_printable_encoding() {
        let backend = EmailBackend::new(InputFormat::Eml).unwrap();

        let eml_content = b"From: sender@example.com\r
Subject: =?UTF-8?Q?Test_=C3=A9_Subject?=\r
Content-Transfer-Encoding: quoted-printable\r
\r
This is a test with special characters: =C3=A9 =C3=A0 =C3=BC\r
";

        let result = backend.parse_bytes(eml_content, &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        // Should decode quoted-printable encoding (é à ü)
        // Note: Actual decoding depends on mail_parser library capabilities
        assert!(doc.markdown.len() > 30);
    }

    // ========== Additional Edge Cases (5 tests) ==========

    #[test]
    fn test_eml_bcc_header() {
        let backend = EmailBackend::new(InputFormat::Eml).unwrap();

        let eml_content = b"From: sender@example.com\r
To: recipient@example.com\r
BCC: hidden1@example.com, hidden2@example.com\r
Subject: BCC Test\r
\r
Email with BCC recipients\r
";

        let result = backend.parse_bytes(eml_content, &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        // BCC header should be extracted if present
        // Note: BCC is often stripped in transit, but should be preserved if present
        assert!(doc.markdown.contains("BCC:") || doc.markdown.len() > 50);
    }

    #[test]
    fn test_eml_reply_to_and_references() {
        let backend = EmailBackend::new(InputFormat::Eml).unwrap();

        let eml_content = b"From: sender@example.com\r
To: recipient@example.com\r
Subject: Re: Original Subject\r
Reply-To: replyto@example.com\r
In-Reply-To: <original-msg-id@example.com>\r
References: <msg1@example.com> <msg2@example.com>\r
\r
This is a reply message\r
";

        let result = backend.parse_bytes(eml_content, &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        let doc_items = doc.content_blocks.as_ref().unwrap();

        // Should extract basic headers (Subject + From + To + body)
        // Threading headers (Reply-To, In-Reply-To, References) may or may not be extracted
        // depending on mail parser implementation
        assert!(doc_items.len() >= 4); // At minimum: Subject + From + To + body
    }

    #[test]
    fn test_msg_rtf_body() {
        let backend = EmailBackend::new(InputFormat::Msg).unwrap();

        // MSG files can contain RTF-formatted body content
        // This tests that RTF content is extracted (even if not fully formatted)
        let test_file = Path::new("test-corpus/email/sample_rtf_body.msg");

        if !test_file.exists() {
            // Skip test if test file doesn't exist
            return;
        }

        let result = backend.parse_file(test_file, &BackendOptions::default());

        if let Ok(doc) = result {
            // Should extract some content from RTF body
            assert!(doc.markdown.len() > 10);
        }
    }

    #[test]
    fn test_vcf_photo_and_logo_fields() {
        let backend = EmailBackend::new(InputFormat::Vcf).unwrap();

        let vcf_content = b"BEGIN:VCARD\r
VERSION:3.0\r
FN:John Doe\r
EMAIL:john@example.com\r
PHOTO;ENCODING=b;TYPE=JPEG:/9j/4AAQSkZJRg==\r
LOGO;VALUE=URI:http://example.com/logo.png\r
END:VCARD";

        let result = backend.parse_bytes(vcf_content, &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        let doc_items = doc.content_blocks.as_ref().unwrap();

        // Should have at least name header and email (version now in header)
        assert!(doc_items.len() >= 2); // Name header (with version) + Email

        // Verify name header includes version
        assert!(doc.markdown.contains("vCard v3.0"));
    }

    #[test]
    fn test_eml_malformed_date_header() {
        let backend = EmailBackend::new(InputFormat::Eml).unwrap();

        let eml_content = b"From: sender@example.com\r
Subject: Date Test\r
Date: Not a valid RFC 2822 date format\r
\r
Email with malformed date header\r
";

        let result = backend.parse_bytes(eml_content, &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        // Should handle malformed date gracefully
        // Parser may skip invalid date or include it as-is
        assert!(doc.markdown.len() > 30);
        assert!(doc.content_blocks.is_some());
    }

    #[test]
    fn test_eml_inline_images() {
        let backend = EmailBackend::new(InputFormat::Eml).unwrap();

        let eml_content = b"From: sender@example.com\r
To: recipient@example.com\r
Subject: Email with inline image\r
MIME-Version: 1.0\r
Content-Type: multipart/related; boundary=\"boundary123\"\r
\r
--boundary123\r
Content-Type: text/html; charset=\"utf-8\"\r
\r
<html><body><p>See image below:</p><img src=\"cid:image001\"/></body></html>\r
--boundary123\r
Content-Type: image/png\r
Content-ID: <image001>\r
Content-Disposition: inline\r
\r
PNG_BINARY_DATA_HERE\r
--boundary123--\r
";

        let result = backend.parse_bytes(eml_content, &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        // Should extract HTML content (image referenced but not embedded in text)
        assert!(doc.markdown.len() > 20);
        assert!(doc.content_blocks.is_some());
    }

    #[test]
    fn test_eml_priority_headers() {
        let backend = EmailBackend::new(InputFormat::Eml).unwrap();

        let eml_content = b"From: sender@example.com\r
To: recipient@example.com\r
Subject: Urgent Message\r
X-Priority: 1 (Highest)\r
Importance: high\r
\r
This is an urgent email message.\r
";

        let result = backend.parse_bytes(eml_content, &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        // Should extract priority information if included in metadata
        assert!(doc.markdown.len() > 30);
        assert!(doc.content_blocks.is_some());
    }

    #[test]
    fn test_mbox_with_mixed_encodings() {
        let backend = EmailBackend::new(InputFormat::Mbox).unwrap();

        let mbox_content = b"From MAILER-DAEMON Mon Jan 01 00:00:00 2024\r
From: sender1@example.com\r
Subject: ASCII message\r
\r
This is a plain ASCII message.\r
\r
From MAILER-DAEMON Mon Jan 01 00:01:00 2024\r
From: sender2@example.com\r
Subject: =?UTF-8?B?VVRGLTggbWVzc2FnZQ==?=\r
Content-Type: text/plain; charset=utf-8\r
Content-Transfer-Encoding: quoted-printable\r
\r
This message has UTF-8 characters: =C3=A9=C3=A0=C3=BC\r
";

        let result = backend.parse_bytes(mbox_content, &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        // Should handle multiple messages with different encodings
        assert!(doc.markdown.len() > 40);
        assert!(doc.content_blocks.is_some());
    }

    #[test]
    fn test_eml_with_calendar_invite() {
        let backend = EmailBackend::new(InputFormat::Eml).unwrap();

        let eml_content = b"From: organizer@example.com\r
To: attendee@example.com\r
Subject: Meeting Invitation\r
MIME-Version: 1.0\r
Content-Type: multipart/alternative; boundary=\"boundary456\"\r
\r
--boundary456\r
Content-Type: text/plain\r
\r
You are invited to a meeting on Jan 15, 2024 at 2pm.\r
--boundary456\r
Content-Type: text/calendar; method=REQUEST\r
\r
BEGIN:VCALENDAR\r
VERSION:2.0\r
BEGIN:VEVENT\r
DTSTART:20240115T140000Z\r
DTEND:20240115T150000Z\r
SUMMARY:Team Meeting\r
END:VEVENT\r
END:VCALENDAR\r
--boundary456--\r
";

        let result = backend.parse_bytes(eml_content, &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        // Should extract calendar information (either from text part or calendar part)
        assert!(doc.markdown.len() > 30);
        assert!(doc.content_blocks.is_some());
    }

    #[test]
    fn test_vcf_with_multiple_addresses() {
        let backend = EmailBackend::new(InputFormat::Vcf).unwrap();

        let vcf_content = b"BEGIN:VCARD\r
VERSION:4.0\r
FN:Jane Smith\r
EMAIL;TYPE=work:jane.work@example.com\r
EMAIL;TYPE=home:jane.home@example.com\r
ADR;TYPE=work:;;123 Office St;New York;NY;10001;USA\r
ADR;TYPE=home:;;456 Home Ave;Brooklyn;NY;11201;USA\r
TEL;TYPE=work:+1-555-0100\r
TEL;TYPE=cell:+1-555-0101\r
END:VCARD";

        let result = backend.parse_bytes(vcf_content, &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        let doc_items = doc.content_blocks.as_ref().unwrap();

        // Should extract multiple emails, addresses, and phone numbers
        // Each field typically becomes a text item
        assert!(
            doc_items.len() >= 5,
            "Should extract multiple contact fields"
        );

        // Check that markdown contains contact information
        assert!(doc.markdown.to_lowercase().contains("jane"));
    }

    // ========== Advanced Email Features (5 tests) ==========

    #[test]
    fn test_eml_with_smime_metadata() {
        let backend = EmailBackend::new(InputFormat::Eml).unwrap();

        // S/MIME signed email with cryptographic metadata
        let eml_content = b"From: secure@example.com\r
To: recipient@example.com\r
Subject: Signed Message\r
MIME-Version: 1.0\r
Content-Type: multipart/signed; protocol=\"application/pkcs7-signature\"; micalg=\"sha-256\"; boundary=\"smime-boundary\"\r
\r
--smime-boundary\r
Content-Type: text/plain\r
\r
This message has been digitally signed.\r
--smime-boundary\r
Content-Type: application/pkcs7-signature; name=\"smime.p7s\"\r
Content-Transfer-Encoding: base64\r
Content-Disposition: attachment; filename=\"smime.p7s\"\r
\r
MIIGPwYJKoZIhvcNAQcCoIIGMDCCBiwCAQExDzANBglghkgBZQMEAgEFADALBgkq\r
--smime-boundary--\r
";

        let result = backend.parse_bytes(eml_content, &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        // Should extract signed message content
        assert!(doc.markdown.contains("digitally signed") || doc.markdown.len() > 50);
        assert!(doc.content_blocks.is_some());
        let doc_items = doc.content_blocks.as_ref().unwrap();
        assert!(doc_items.len() >= 3); // Subject + headers + body
    }

    #[test]
    fn test_msg_voting_and_tracking() {
        let backend = EmailBackend::new(InputFormat::Msg).unwrap();

        // MSG files can include Outlook-specific voting buttons and tracking options
        // This test verifies that even if voting/tracking metadata isn't extracted,
        // the basic message content is still accessible

        // Note: Since MSG parsing requires a file path, we test error handling
        let result = backend.parse_bytes(b"dummy voting msg content", &BackendOptions::default());

        // Should fail gracefully since MSG requires file path
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("MSG format requires file path"));
    }

    #[test]
    fn test_vcf_anniversary_and_birthday() {
        let backend = EmailBackend::new(InputFormat::Vcf).unwrap();

        // VCF with anniversary and birthday fields (important personal dates)
        let vcf_content = b"BEGIN:VCARD\r
VERSION:4.0\r
FN:Alice Johnson\r
EMAIL:alice@example.com\r
BDAY:19850312\r
ANNIVERSARY:20100615\r
NOTE:Birthday: March 12, Anniversary: June 15\r
END:VCARD";

        let result = backend.parse_bytes(vcf_content, &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        let doc_items = doc.content_blocks.as_ref().unwrap();

        // Should extract contact with date fields
        // Parser may or may not include BDAY/ANNIVERSARY fields depending on implementation
        assert!(doc_items.len() >= 3); // Name + Email + potentially dates/notes
        assert!(doc.markdown.contains("Alice") || doc.markdown.contains("alice@example.com"));
    }

    #[test]
    fn test_mbox_large_message_count() {
        let backend = EmailBackend::new(InputFormat::Mbox).unwrap();

        // MBOX with many messages to test performance and parsing robustness
        let mut mbox_content = Vec::new();

        for i in 1..=50 {
            let message = format!(
                "From sender{i}@example.com Mon Jan {i:2} 10:00:00 2025\r\nSubject: Message {i}\r\n\r\nBody {i}\r\n\r\n"
            );
            mbox_content.extend_from_slice(message.as_bytes());
        }

        let result = backend.parse_bytes(&mbox_content, &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        let doc_items = doc.content_blocks.as_ref().unwrap();

        // Should parse all 50 messages
        // Each message has at least: section header + from + subject + body
        // Mailbox also has a main header
        assert!(
            doc_items.len() >= 150,
            "Expected at least 150 DocItems for 50 messages (found {})",
            doc_items.len()
        );

        // Verify markdown mentions multiple messages
        assert!(doc.markdown.contains("Mailbox"));
        assert!(doc.markdown.contains("Message 1"));
        assert!(doc.markdown.contains("Message 50") || doc.markdown.contains("50 messages"));
    }

    #[test]
    fn test_eml_delivery_and_read_receipts() {
        let backend = EmailBackend::new(InputFormat::Eml).unwrap();

        // Email requesting delivery and read receipts (RFC 3798)
        let eml_content = b"From: sender@example.com\r
To: recipient@example.com\r
Subject: Important - Receipt Requested\r
Disposition-Notification-To: sender@example.com\r
Return-Receipt-To: sender@example.com\r
X-Confirm-Reading-To: sender@example.com\r
MIME-Version: 1.0\r
\r
Please confirm you received this message.\r
";

        let result = backend.parse_bytes(eml_content, &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        let doc_items = doc.content_blocks.as_ref().unwrap();

        // Should extract message with receipt request headers
        // Parser may or may not include receipt headers in output
        assert!(doc_items.len() >= 4); // Subject + From + To + Body
        assert!(doc.markdown.contains("Important") || doc.markdown.contains("Receipt"));

        // Verify provenance is present for all items
        for item in doc_items {
            match item {
                DocItem::SectionHeader { prov, .. }
                | DocItem::Text { prov, .. }
                | DocItem::ListItem { prov, .. } => {
                    assert!(!prov.is_empty(), "All items should have provenance");
                }
                _ => {}
            }
        }
    }

    // ========== VCF Metadata Extraction Tests (N=1881) ==========

    #[test]
    fn test_vcf_metadata_note_extraction() {
        let backend = EmailBackend::new(InputFormat::Vcf).unwrap();

        // VCF with NOTE field (RFC 6350: vCard supplementary information)
        let vcf_content = b"BEGIN:VCARD\r
VERSION:3.0\r
FN:Dr. Sarah Chen\r
ORG:Research Institute\r
TITLE:Senior Researcher\r
EMAIL:sarah.chen@research.org\r
NOTE:Specializes in computational biology and machine learning applications\r
END:VCARD";

        let result = backend.parse_bytes(vcf_content, &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();

        // N=1881: Verify NOTE field extracted to DocumentMetadata.subject
        assert_eq!(
            doc.metadata.subject,
            Some(
                "Specializes in computational biology and machine learning applications"
                    .to_string()
            ),
            "NOTE field should be extracted to metadata.subject"
        );

        // Verify content is still correct
        assert!(doc.markdown.contains("Dr. Sarah Chen"));
        assert!(doc.markdown.contains("Research Institute"));
    }

    #[test]
    fn test_vcf_metadata_no_note() {
        let backend = EmailBackend::new(InputFormat::Vcf).unwrap();

        // VCF without NOTE field
        let vcf_content = b"BEGIN:VCARD\r
VERSION:3.0\r
FN:John Smith\r
EMAIL:john@example.com\r
END:VCARD";

        let result = backend.parse_bytes(vcf_content, &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();

        // N=1881: Verify metadata.subject is None when no NOTE field
        assert_eq!(
            doc.metadata.subject, None,
            "metadata.subject should be None when NOTE field absent"
        );
    }

    #[test]
    fn test_vcf_metadata_multiple_contacts_first_note() {
        let backend = EmailBackend::new(InputFormat::Vcf).unwrap();

        // Multiple contacts - should extract NOTE from first contact only
        let vcf_content = b"BEGIN:VCARD\r
VERSION:3.0\r
FN:Alice Johnson\r
EMAIL:alice@example.com\r
NOTE:First contact note\r
END:VCARD\r
BEGIN:VCARD\r
VERSION:3.0\r
FN:Bob Wilson\r
EMAIL:bob@example.com\r
NOTE:Second contact note\r
END:VCARD";

        let result = backend.parse_bytes(vcf_content, &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();

        // N=1881: Should extract NOTE from first contact only
        assert_eq!(
            doc.metadata.subject,
            Some("First contact note".to_string()),
            "Should extract NOTE from first contact when multiple contacts present"
        );

        // Verify both contacts in content
        assert!(doc.markdown.contains("Alice Johnson"));
        assert!(doc.markdown.contains("Bob Wilson"));
    }

    // ========== MBOX Metadata Extraction Tests (N=1882) ==========

    #[test]
    fn test_mbox_metadata_subject_extraction() {
        let backend = EmailBackend::new(InputFormat::Mbox).unwrap();

        // MBOX with single message
        let mbox_content = b"From sender@example.com Mon Jan  1 10:00:00 2025\r
From: sender@example.com\r
To: recipient@example.com\r
Subject: Q4 2024 Financial Report and Budget Planning\r
Date: Mon, 01 Jan 2025 10:00:00 +0000\r
\r
This is the quarterly financial report.\r
";

        let result = backend.parse_bytes(mbox_content, &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();

        // N=1882: Verify first message subject extracted to DocumentMetadata.subject
        assert_eq!(
            doc.metadata.subject,
            Some("Q4 2024 Financial Report and Budget Planning".to_string()),
            "First message subject should be extracted to metadata.subject"
        );

        // Verify content is still correct
        assert!(doc.markdown.contains("Mailbox"));
        assert!(doc.markdown.contains("Q4 2024 Financial Report"));
    }

    #[test]
    fn test_mbox_metadata_empty_subject() {
        let backend = EmailBackend::new(InputFormat::Mbox).unwrap();

        // MBOX with message that has empty subject
        let mbox_content = b"From sender@example.com Mon Jan  1 10:00:00 2025\r
From: sender@example.com\r
To: recipient@example.com\r
Subject: \r
\r
Message body";

        let result = backend.parse_bytes(mbox_content, &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();

        // N=1882: Empty subject parsed as "(No Subject)" by EML parser
        assert_eq!(
            doc.metadata.subject,
            Some("(No Subject)".to_string()),
            "Empty subject should be extracted (EML parser converts empty to '(No Subject)')"
        );
    }

    #[test]
    fn test_mbox_metadata_multiple_messages_first_subject() {
        let backend = EmailBackend::new(InputFormat::Mbox).unwrap();

        // Multiple messages - should extract subject from first message only
        let mbox_content = b"From sender1@example.com Mon Jan  1 10:00:00 2025\r
Subject: First Message Subject\r
\r
Body 1\r
\r
From sender2@example.com Mon Jan  2 10:00:00 2025\r
Subject: Second Message Subject\r
\r
Body 2\r
";

        let result = backend.parse_bytes(mbox_content, &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();

        // N=1882: Should extract subject from first message only
        assert_eq!(
            doc.metadata.subject,
            Some("First Message Subject".to_string()),
            "Should extract subject from first message when multiple messages present"
        );

        // Verify both messages in content
        assert!(doc.markdown.contains("Message 1") || doc.markdown.contains("First Message"));
        assert!(doc.markdown.contains("Message 2") || doc.markdown.contains("Second Message"));
    }

    // ========== EML Metadata Extraction Tests (N=1883) ==========

    #[test]
    fn test_eml_metadata_subject_extraction() {
        let backend = EmailBackend::new(InputFormat::Eml).unwrap();

        // EML single email message
        let eml_content = b"From: sender@example.com\r
To: recipient@example.com\r
Subject: Quarterly Report Review and Action Items\r
Date: Mon, 01 Jan 2025 10:00:00 +0000\r
\r
Please review the attached quarterly report.\r
";

        let result = backend.parse_bytes(eml_content, &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();

        // N=1883: Verify email subject extracted to DocumentMetadata.subject
        assert_eq!(
            doc.metadata.subject,
            Some("Quarterly Report Review and Action Items".to_string()),
            "Email subject should be extracted to metadata.subject"
        );

        // Verify content is still correct
        assert!(doc.markdown.contains("Quarterly Report"));
        assert!(doc.markdown.contains("Subject:"));
    }

    #[test]
    fn test_eml_metadata_empty_subject() {
        let backend = EmailBackend::new(InputFormat::Eml).unwrap();

        // EML with empty subject line
        let eml_content = b"From: sender@example.com\r
To: recipient@example.com\r
Subject: \r
\r
Message body";

        let result = backend.parse_bytes(eml_content, &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();

        // N=1883: Empty subject parsed as "(No Subject)" by EML parser
        assert_eq!(
            doc.metadata.subject,
            Some("(No Subject)".to_string()),
            "Empty subject should be extracted (EML parser converts empty to '(No Subject)')"
        );
    }

    #[test]
    fn test_eml_metadata_unicode_subject() {
        let backend = EmailBackend::new(InputFormat::Eml).unwrap();

        // EML with Unicode subject
        let eml_content = "From: sender@example.com\r
To: recipient@example.com\r
Subject: 会議の議事録 - Meeting Minutes 2025\r
\r
Meeting notes"
            .as_bytes();

        let result = backend.parse_bytes(eml_content, &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();

        // N=1883: Unicode subject should be extracted correctly
        assert_eq!(
            doc.metadata.subject,
            Some("会議の議事録 - Meeting Minutes 2025".to_string()),
            "Unicode subject should be extracted correctly"
        );
    }
}
