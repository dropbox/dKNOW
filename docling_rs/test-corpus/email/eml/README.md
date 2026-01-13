# EML Test Corpus

**Format:** EML (Electronic Mail Message)
**Count:** 5 test files
**Purpose:** Test EML parser implementation (Phase E Step 1)

---

## Test Files

### 1. simple_text.eml - Plain Text Email (Simple)

**Description:** Basic plain text email with standard headers
**Features:**
- Plain text body (no HTML)
- Standard headers: From, To, Subject, Date, Message-ID
- Single recipient
- No attachments
- ASCII text only

**Test objectives:**
- Basic EML parsing
- Header extraction
- Plain text body conversion
- Simple MIME structure

---

### 2. html_rich.eml - HTML Email (Rich Content)

**Description:** Newsletter-style email with HTML and text alternatives
**Features:**
- `multipart/alternative` (text + HTML)
- UTF-8 encoded subject with emoji (RFC 2047)
- HTML with formatting (headings, lists, links)
- Text alternative for plain text clients

**Test objectives:**
- Multipart/alternative handling
- HTML to markdown conversion
- RFC 2047 subject decoding
- Character encoding (UTF-8, emoji)

---

### 3. with_attachments.eml - Email with Attachments (Mixed Content)

**Description:** Business email with PDF and Excel attachments
**Features:**
- `multipart/mixed` structure
- Two attachments: PDF and XLS (base64 encoded)
- Cc recipient
- Attachment metadata (filename, MIME type)

**Test objectives:**
- Multipart/mixed parsing
- Attachment detection and listing
- Base64 decoding (handled by library)
- Multiple recipients (To, Cc)

---

### 4. multipart_complex.eml - Complex Multipart (Nested MIME)

**Description:** Project update email with nested multipart structure
**Features:**
- Nested multipart: `multipart/mixed` containing `multipart/alternative`
- Text + HTML body
- Multiple attachments (PNG image, DOCX document)
- Multiple recipients (To, Cc, Bcc)
- UTF-8 base64-encoded subject with emoji (RFC 2047)
- HTML with table formatting

**Test objectives:**
- Nested multipart structure
- Complex MIME boundary parsing
- Multiple attachment types
- Group recipients (To, Cc, Bcc)
- HTML table conversion to markdown

---

### 5. forwarded_nested.eml - Forwarded Email (Nested Message)

**Description:** Forwarded email containing original message
**Features:**
- `message/rfc822` (nested email)
- In-Reply-To and References headers (threading)
- Quoted text in forwarding message
- Original message fully embedded

**Test objectives:**
- Nested message parsing (message/rfc822)
- Email threading headers
- Recursive MIME parsing
- Forwarded message extraction

---

## Test File Sources

**Source:** Hand-crafted synthetic test emails
**Creation method:** Written to cover diverse EML features
**Validity:** Conform to RFC 5322 (email format) and RFC 2045-2049 (MIME)

**Attachments:**
- PDF: Minimal valid PDF (base64 encoded)
- XLS: Minimal valid Excel file (OLE/CFB format)
- PNG: 1x1 transparent pixel (smallest valid PNG)
- DOCX: Minimal valid DOCX (ZIP structure)

**Note:** Attachment data is minimal/dummy content for testing purposes.
Focus is on MIME structure parsing, not attachment content validation.

---

## Expected Behavior

### Parsing

**mail-parser crate should:**
- Parse all 5 test files without errors
- Extract headers correctly
- Handle multipart structures gracefully
- Decode base64 attachments
- Parse nested messages (message/rfc822)
- Decode RFC 2047 encoded subjects

### Markdown Conversion

**Expected markdown structure:**
```markdown
# Email: [Subject]

## Metadata

**From:** Name <email@example.com>
**To:** recipient1@example.com, recipient2@example.com
**Cc:** cc@example.com (if present)
**Date:** ISO 8601 datetime
**Subject:** Subject line

## Body

[Email body content, HTML converted to markdown]

## Attachments (if present)

1. filename.ext (MIME type, size)
2. filename2.ext (MIME type, size)
```

---

## Validation

### Manual Validation

**View EML files:**
- Open in email client (Outlook, Thunderbird, Apple Mail)
- Verify headers display correctly
- Verify body content renders properly
- Verify attachments are recognized

**Command-line validation:**
```bash
# View headers
head -20 simple_text.eml

# Check MIME structure
grep -A5 "Content-Type:" with_attachments.eml

# Verify base64 encoding
grep "Content-Transfer-Encoding: base64" with_attachments.eml
```

### Automated Validation

**Integration tests:**
```bash
# Run EML-specific tests
USE_RUST_BACKEND=1 cargo test test_canon_email_eml

# Run individual test
USE_RUST_BACKEND=1 cargo test test_canon_email_eml_simple_text -- --exact
```

**Expected results:**
- All 5 tests pass
- No parsing errors
- Output matches expected markdown format
- Attachments listed correctly
- HTML converted to markdown

---

## Known Edge Cases

### Handled by mail-parser

- Missing headers → Returns `None` (handled with `Option` types)
- Malformed MIME boundaries → Best-effort parsing
- Invalid date formats → Returns `None`, doesn't crash
- Unknown character sets → Falls back to UTF-8
- Corrupted base64 → Skips part, continues parsing

### Not Handled (Out of Scope)

- **Binary attachment extraction**: Only list attachments, don't extract to markdown
- **Inline images (Content-ID)**: HTML `src="cid:..."` won't render in markdown
- **Email threading**: Requires multiple emails, external state
- **Complex HTML**: Some formatting lost in markdown conversion
- **S/MIME encryption**: Encrypted emails cannot be parsed (DRM-like issue)

---

## Character Set Testing

**Test files use UTF-8:**
- Simple ASCII (simple_text.eml)
- UTF-8 with emoji (html_rich.eml, multipart_complex.eml)
- RFC 2047 encoded subjects (=?UTF-8?Q?...?= and =?UTF-8?B?...?=)

**Future testing:**
- Legacy encodings (ISO-8859-1, Windows-1252)
- CJK encodings (BIG5, GB2312, Shift_JIS)
- UTF-7 (obsolete but still encountered)

---

## File Size

| File | Size | Complexity |
|------|------|-----------|
| simple_text.eml | ~500 bytes | Low |
| html_rich.eml | ~1.2 KB | Medium |
| with_attachments.eml | ~2.5 KB | Medium |
| multipart_complex.eml | ~3.0 KB | High |
| forwarded_nested.eml | ~1.5 KB | High |

**Total corpus size:** ~8.7 KB (very small, fast tests)

---

## Integration Test Names

```rust
#[test]
fn test_canon_email_eml_simple_text()

#[test]
fn test_canon_email_eml_html_rich()

#[test]
fn test_canon_email_eml_with_attachments()

#[test]
fn test_canon_email_eml_multipart_complex()

#[test]
fn test_canon_email_eml_forwarded_nested()
```

---

## Future Enhancements

### Additional Test Cases

1. **Large email** (10+ MB) - Performance testing
2. **Spam email** - Malformed headers, suspicious content
3. **Non-English email** - Japanese, Chinese, Arabic (RTL)
4. **Legacy encoding** - ISO-8859-1, Windows-1252
5. **S/MIME signed/encrypted** - Error handling (unsupported)

### Real-World Corpus

- Enron Email Dataset (e-discovery corpus)
- Apache SpamAssassin public corpus
- RFC 5322 specification examples

---

## Summary

**Test corpus completeness:** 5/5 files ✅
**Diversity:** Simple, Rich, Attachments, Complex, Nested ✅
**RFC compliance:** All files conform to standards ✅
**Size:** Compact (< 10 KB total) ✅
**Ready for testing:** ✅

**Next step:** Implement EML parser using mail-parser crate

---

**Last updated:** 2025-11-07
**Phase:** Phase E - Email & Communication (Step 1)
