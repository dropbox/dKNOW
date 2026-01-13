# docling-email

Email and contact format parsers for docling-rs, providing extraction of messages, attachments, and metadata from email archives and contact cards.

## Supported Formats

| Format | Extensions | Status | Description |
|--------|-----------|--------|-------------|
| EML | `.eml` | ✅ Full Support | Email message files (RFC 822/5322) |
| MBOX | `.mbox` | ✅ Full Support | Unix mailbox archive format (RFC 4155) |
| MSG | `.msg` | ✅ Full Support | Microsoft Outlook message format (OLE/CFB) |
| VCF | `.vcf`, `.vcard` | ✅ Full Support | vCard contact card format (RFC 6350) |

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
docling-email = "2.58.0"
```

Or use cargo:

```bash
cargo add docling-email
```

## Quick Start

### Parse EML (Email Message)

```rust
use docling_email::{parse_eml, EmailMessage};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new("message.eml");
    let email: EmailMessage = parse_eml(path)?;

    println!("From: {}", email.from);
    println!("To: {}", email.to.join(", "));
    println!("Subject: {}", email.subject);
    println!("Date: {}", email.date);
    println!("\n{}", email.body);

    // List attachments
    for attachment in &email.attachments {
        println!("Attachment: {} ({} bytes)", attachment.filename, attachment.data.len());
    }

    Ok(())
}
```

### Parse MBOX (Mailbox Archive)

```rust
use docling_email::{parse_mbox, Mailbox};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new("archive.mbox");
    let mailbox: Mailbox = parse_mbox(path)?;

    println!("Total messages: {}", mailbox.messages.len());

    for (idx, message) in mailbox.messages.iter().enumerate() {
        println!("\n--- Message {} ---", idx + 1);
        println!("From: {}", message.from);
        println!("Subject: {}", message.subject);
        println!("Date: {}", message.date);
    }

    Ok(())
}
```

### Parse MSG (Outlook Message)

```rust
use docling_email::{parse_msg_from_path, ParsedMsg};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new("message.msg");
    let msg: ParsedMsg = parse_msg_from_path(path)?;

    println!("From: {}", msg.from.unwrap_or_default());
    println!("To: {}", msg.to.join(", "));
    println!("Subject: {}", msg.subject.unwrap_or_default());
    println!("\nBody:\n{}", msg.body.unwrap_or_default());

    Ok(())
}
```

### Parse VCF (vCard Contact)

```rust
use docling_email::{parse_vcf, Contact};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new("contact.vcf");
    let contacts: Vec<Contact> = parse_vcf(path)?;

    for contact in contacts {
        println!("Name: {}", contact.name);
        if let Some(email) = contact.email {
            println!("Email: {}", email);
        }
        if let Some(phone) = contact.phone {
            println!("Phone: {}", phone);
        }
        if let Some(org) = contact.organization {
            println!("Organization: {}", org);
        }
        println!();
    }

    Ok(())
}
```

## Data Structures

### EmailMessage

```rust
pub struct EmailMessage {
    pub from: String,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub bcc: Vec<String>,
    pub subject: String,
    pub date: String,
    pub body: String,
    pub html_body: Option<String>,
    pub attachments: Vec<Attachment>,
}
```

### Attachment

```rust
pub struct Attachment {
    pub filename: String,
    pub content_type: String,
    pub data: Vec<u8>,
}
```

### Mailbox

```rust
pub struct Mailbox {
    pub messages: Vec<EmailMessage>,
}
```

### ParsedMsg

```rust
pub struct ParsedMsg {
    pub from: Option<String>,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub subject: Option<String>,
    pub date: Option<String>,
    pub body: Option<String>,
    pub attachments: Vec<Attachment>,
}
```

### Contact (VCF)

```rust
pub struct Contact {
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub organization: Option<String>,
    pub title: Option<String>,
    pub address: Option<String>,
    pub url: Option<String>,
    pub note: Option<String>,
}
```

## Features

### EML Support
- RFC 822/5322 compliant email parsing
- Multipart message handling (multipart/mixed, multipart/alternative)
- MIME type detection
- Base64 and quoted-printable decoding
- HTML and plain text body extraction
- Attachment extraction with filename and content type
- Header parsing (From, To, CC, BCC, Subject, Date, Message-ID)

### MBOX Support
- Unix mbox format (mboxrd, mboxo, mboxcl)
- Multiple message parsing from single archive
- From-line parsing
- Full EML parsing for each message
- Large file streaming (memory-efficient)

### MSG Support
- Microsoft Outlook binary format (OLE/CFB container)
- MAPI property extraction
- Subject, sender, recipient parsing
- Body text extraction (plain text and HTML)
- Attachment extraction
- Date and message ID parsing

### VCF Support
- vCard 2.1, 3.0, 4.0 parsing
- Multiple contact parsing from single file
- Standard properties (N, FN, EMAIL, TEL, ORG, TITLE, ADR, URL, NOTE)
- Photo extraction (base64-encoded)

## Advanced Usage

### Convert Email to Markdown

```rust
use docling_email::email_to_markdown;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let markdown = email_to_markdown(Path::new("message.eml"))?;
    println!("{}", markdown);
    Ok(())
}
```

Output format:
```markdown
**From:** sender@example.com
**To:** recipient@example.com
**Subject:** Meeting Notes
**Date:** 2024-01-15 14:30:00

Body content here...

**Attachments:**
- presentation.pdf (1.2 MB)
- notes.txt (5.4 KB)
```

### Convert MBOX to Markdown

```rust
use docling_email::mbox_to_markdown;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let markdown = mbox_to_markdown(Path::new("archive.mbox"))?;
    println!("{}", markdown);
    Ok(())
}
```

### Convert MSG to Markdown

```rust
use docling_email::msg_to_markdown;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let markdown = msg_to_markdown(Path::new("message.msg"))?;
    println!("{}", markdown);
    Ok(())
}
```

### Convert VCF to Markdown

```rust
use docling_email::vcf_to_markdown;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let markdown = vcf_to_markdown(Path::new("contacts.vcf"))?;
    println!("{}", markdown);
    Ok(())
}
```

### Extract All Attachments

```rust
use docling_email::parse_eml;
use std::path::Path;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let email = parse_eml(Path::new("message.eml"))?;

    for (idx, attachment) in email.attachments.iter().enumerate() {
        let filename = if attachment.filename.is_empty() {
            format!("attachment_{}.bin", idx)
        } else {
            attachment.filename.clone()
        };

        fs::write(&filename, &attachment.data)?;
        println!("Extracted: {} ({} bytes, {})", filename, attachment.data.len(), attachment.content_type);
    }

    Ok(())
}
```

### Parse HTML Body

```rust
use docling_email::parse_eml;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let email = parse_eml(Path::new("message.eml"))?;

    if let Some(html) = email.html_body {
        println!("HTML Body:\n{}", html);
    } else {
        println!("Plain Text Body:\n{}", email.body);
    }

    Ok(())
}
```

### Filter MBOX Messages

```rust
use docling_email::parse_mbox;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mailbox = parse_mbox(Path::new("archive.mbox"))?;

    // Find all messages from specific sender
    let from_sender: Vec<_> = mailbox
        .messages
        .iter()
        .filter(|msg| msg.from.contains("@example.com"))
        .collect();

    println!("Messages from example.com: {}", from_sender.len());

    // Find messages with attachments
    let with_attachments: Vec<_> = mailbox
        .messages
        .iter()
        .filter(|msg| !msg.attachments.is_empty())
        .collect();

    println!("Messages with attachments: {}", with_attachments.len());

    Ok(())
}
```

## Error Handling

All parsing functions return appropriate error types:

```rust
use docling_email::{parse_eml, error::EmailError};
use std::path::Path;

fn main() {
    match parse_eml(Path::new("message.eml")) {
        Ok(email) => {
            println!("Subject: {}", email.subject);
        }
        Err(e) => {
            eprintln!("Failed to parse email: {}", e);
        }
    }
}
```

## Performance

Email parsing is optimized for speed and memory efficiency:

| Format | Typical Size | Parse Time | Memory Usage |
|--------|--------------|------------|--------------|
| EML | 10 KB - 1 MB | 5-50 ms | 2-20 MB |
| MBOX | 10 MB - 1 GB | 100ms-10s | 50-500 MB (streaming) |
| MSG | 10 KB - 5 MB | 10-100 ms | 5-50 MB |
| VCF | 1 KB - 100 KB | 1-10 ms | 1-10 MB |

Benchmarked on Apple M1, 16GB RAM.

## Dependencies

- `mail-parser` - RFC-compliant email parsing
- `mailparse` - MIME message parsing
- `mbox-reader` - MBOX archive reading
- `msg_parser` - Microsoft MSG format parsing
- `vcard` - vCard parsing
- `html2md` - HTML to Markdown conversion
- `chrono` - Date/time handling

## Integration with docling-core

This crate is automatically used by `docling-core` when processing email files:

```rust
use docling_backend::{DocumentConverter, ConversionOptions};  // Note: DocumentConverter is in docling-backend crate
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let converter = DocumentConverter::new()?;

    // Convert email to structured document
    let doc = converter.convert(Path::new("message.eml"), ConversionOptions::default())?;

    println!("From: {}", doc.metadata.get("from").unwrap_or("Unknown"));
    println!("Subject: {}", doc.metadata.get("subject").unwrap_or("(no subject)"));

    Ok(())
}
```

## Testing

Run tests:

```bash
cargo test -p docling-email
```

Run with test files (requires test corpus):

```bash
# EML tests
cargo test -p docling-email test_eml

# MBOX tests
cargo test -p docling-email test_mbox

# MSG tests
cargo test -p docling-email test_msg

# VCF tests
cargo test -p docling-email test_vcf
```

## Examples

See `examples/` directory for complete working examples:

- `examples/eml_parser.rs` - Basic EML parsing
- `examples/mbox_parser.rs` - MBOX archive parsing
- `examples/msg_parser.rs` - Outlook MSG parsing
- `examples/vcf_parser.rs` - vCard contact parsing
- `examples/extract_attachments.rs` - Extract all attachments

Run examples:

```bash
cargo run --example eml_parser -- message.eml
cargo run --example mbox_parser -- archive.mbox
cargo run --example msg_parser -- message.msg
cargo run --example vcf_parser -- contacts.vcf
```

## Format Specifications

### EML
- **Specification**: RFC 822 (1982), RFC 5322 (2008)
- **MIME**: RFC 2045-2049
- **MIME Type**: `message/rfc822`
- **Structure**: Plain text with headers and body

### MBOX
- **Specification**: RFC 4155 (2005)
- **Variants**: mboxrd (recommended), mboxo, mboxcl, mboxcl2
- **MIME Type**: `application/mbox`
- **Structure**: Concatenated email messages with From-line separators

### MSG
- **Specification**: Proprietary Microsoft format
- **Container**: OLE/CFB (Compound File Binary Format)
- **MIME Type**: `application/vnd.ms-outlook`
- **Structure**: Binary property stream with MAPI properties

### VCF
- **Specification**: RFC 6350 (vCard 4.0), RFC 2426 (vCard 3.0)
- **MIME Type**: `text/vcard`, `text/x-vcard`
- **Structure**: Plain text key-value format

## Known Limitations

### EML
- S/MIME encrypted messages are not decrypted (show as encrypted)
- PGP encrypted messages are not decrypted
- Complex nested multipart messages may lose structure

### MBOX
- Very large MBOX files (>10 GB) may require streaming API (future)
- From-line escaping variations across mbox formats

### MSG
- Requires complete MSG file (streaming not supported)
- RTF body format converted to plain text (formatting lost)
- Some MAPI properties may not be parsed

### VCF
- vCard 2.1 charset handling is basic
- Some extension properties (X-*) may not be parsed
- Photo extraction limited to base64-encoded images

## Roadmap

- [ ] S/MIME signature verification
- [ ] PGP signature verification (requires gpg integration)
- [ ] Streaming MBOX parser for very large archives
- [ ] EML/MBOX message threading (In-Reply-To, References)
- [ ] Better RTF body parsing for MSG
- [ ] vCard photo export to files
- [ ] PST (Outlook data file) support

## License

MIT License - see LICENSE file for details

## Contributing

Contributions welcome! Please see the main docling-rs repository for contribution guidelines.

## Related Crates

- `docling-core` - Main document conversion library
- `docling-backend` - Backend orchestration for all formats
- `docling-cli` - Command-line interface
- `docling-ebook` - E-book format support (EPUB, MOBI, etc.)
- `docling-archive` - Archive format support (ZIP, TAR, etc.)

## References

- [RFC 5322 - Internet Message Format](https://tools.ietf.org/html/rfc5322)
- [RFC 4155 - MBOX Format](https://tools.ietf.org/html/rfc4155)
- [RFC 6350 - vCard 4.0](https://tools.ietf.org/html/rfc6350)
- [Microsoft MSG Format Documentation](https://docs.microsoft.com/en-us/openspecs/exchange_server_protocols/ms-oxmsg/)
