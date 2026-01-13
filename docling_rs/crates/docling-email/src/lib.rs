//! docling-email - Email and contact format support for docling
//!
//! This crate provides parsers for common email and communication formats:
//! - **EML** - Email message files (RFC 822/5322)
//! - **MBOX** - Unix mailbox archive format (RFC 4155)
//! - **VCF** - vCard contact card format (RFC 6350)
//! - **MSG** - Microsoft Outlook message format
//!
//! ## Examples
//!
//! Parse an EML email message:
//!
//! ```rust,no_run
//! use docling_email::{parse_eml, email_to_markdown};
//!
//! let eml_bytes = std::fs::read("message.eml")?;
//! let email = parse_eml(&eml_bytes)?;
//!
//! println!("From: {}", email.from);
//! println!("Subject: {}", email.subject);
//! println!("Body: {}", email.body_text);
//! println!("Attachments: {}", email.attachments.len());
//!
//! // Convert to markdown
//! let markdown = email_to_markdown(&email);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Parse an MBOX mailbox archive:
//!
//! ```rust,no_run
//! use docling_email::{parse_mbox, mbox_to_markdown};
//!
//! let mbox_bytes = std::fs::read("mailbox.mbox")?;
//! let mailbox = parse_mbox(&mbox_bytes)?;
//!
//! println!("Messages: {}", mailbox.messages.len());
//! for msg in &mailbox.messages {
//!     println!("  - {}: {}", msg.from, msg.subject);
//! }
//!
//! // Convert to markdown
//! let markdown = mbox_to_markdown(&mailbox);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Parse a VCF contact card:
//!
//! ```rust,no_run
//! use docling_email::{parse_vcf, vcf_to_markdown};
//!
//! let vcf_bytes = std::fs::read("contact.vcf")?;
//! let contacts = parse_vcf(&vcf_bytes)?;
//!
//! for contact in &contacts {
//!     println!("Name: {:?}", contact.name);
//!     println!("Email: {:?}", contact.emails);
//!     println!("Phone: {:?}", contact.phones);
//! }
//!
//! // Convert to markdown
//! let markdown = vcf_to_markdown(&contacts);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Parse an Outlook MSG file:
//!
//! ```rust,no_run
//! use docling_email::{parse_msg_from_path, msg_to_markdown};
//! use std::path::Path;
//!
//! let msg = parse_msg_from_path(Path::new("message.msg"))?;
//!
//! println!("From: {:?}", msg.sender);
//! println!("Subject: {}", msg.subject);
//! println!("Body: {:?}", msg.body);
//!
//! // Convert to markdown
//! let markdown = msg_to_markdown(&msg);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## Architecture
//!
//! All parsers follow the pure Rust architecture:
//! ```text
//! Email file → Parse headers/body → Extract content → Markdown
//! ```
//!
//! No external dependencies like Python are required.

/// EML (RFC 5322) email file parser
pub mod eml;
/// Error types for email parsing
pub mod error;
/// MBOX mailbox format parser
pub mod mbox;
/// Microsoft Outlook MSG file parser
pub mod msg;
/// vCard contact file parser
pub mod vcf;

pub use eml::{email_to_markdown, parse_eml, Attachment, EmailMessage, MimePart};
pub use mbox::{mbox_to_markdown, parse_mbox, Mailbox};
pub use msg::{msg_to_markdown, parse_msg_from_path, ParsedMsg};
pub use vcf::{parse_vcf, vcf_to_markdown, Contact};

#[cfg(test)]
mod tests {
    // Unit tests are located in each module file (eml.rs, mbox.rs, msg.rs, vcard.rs)
    // Integration tests are in crates/docling-core/tests/integration_tests.rs
}
