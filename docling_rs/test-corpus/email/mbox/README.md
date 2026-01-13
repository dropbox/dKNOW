# MBOX Test Corpus

This directory contains test files for the MBOX (mailbox archive) format parser.

**Format:** MBOX (Unix mailbox format)
**Extensions:** `.mbox`, `.mbx`
**Total Files:** 5
**Total Size:** ~50 KB

---

## Test Files

### 1. simple_10_messages.mbox

**Description:** Basic mailbox with 10 plain text emails
**Size:** ~4.5 KB
**Message Count:** 10
**Content Types:** Plain text only
**Features:**
- Simple conversation threads
- Reply threading (In-Reply-To headers)
- Various senders and recipients
- Typical business email content

**Test Coverage:**
- Basic MBOX parsing
- Message boundary detection
- Thread relationships
- Plain text email bodies

**Expected Output:**
- 10 distinct emails numbered 1-10
- All subject lines extracted
- From/To/Date headers present
- Message bodies correctly extracted

---

### 2. mixed_content.mbox

**Description:** Diverse content types including HTML, attachments, and quoted text
**Size:** ~3.8 KB
**Message Count:** 3
**Content Types:** Multipart/alternative, multipart/mixed, plain text
**Features:**
- HTML + plain text alternatives (multipart/alternative)
- Email with PDF attachment (base64 encoded)
- mboxrd escaping test (">From " quoted line)
- Newsletter-style HTML content

**Test Coverage:**
- Multipart MIME parsing
- HTML to markdown conversion
- Attachment metadata extraction
- mboxrd escape/unescape (">From " handling)

**Expected Output:**
- 3 emails with varied structures
- HTML emails converted to markdown
- Attachment listed: "report.pdf (application/pdf, ~300 bytes)"
- Quoted ">From " line unescaped to "From "

---

### 3. unicode_multilang.mbox

**Description:** Multilingual emails with Unicode characters, emoji, and special symbols
**Size:** ~4.2 KB
**Message Count:** 5
**Content Types:** Plain text with UTF-8 encoding
**Features:**
- French email (accents: é, è, ê, à, ù, ç)
- German email (umlauts: ä, ö, ü, ß)
- Japanese email (hiragana, katakana, kanji: ひらがな、カタカナ、漢字)
- Emoji test (😀 🎉 ❤️ 🔥 ✅)
- Math symbols (α β γ ± × ÷ ≤ ≥ ∞)
- RFC 2047 encoded subjects (=?UTF-8?Q?...?= and =?UTF-8?B?...?=)

**Test Coverage:**
- UTF-8 character handling
- RFC 2047 subject decoding (quoted-printable and base64)
- Emoji rendering
- International character sets

**Expected Output:**
- All 5 emails parsed correctly
- Unicode characters preserved
- Emoji rendered as text (or preserved as Unicode)
- Subject lines decoded from RFC 2047 encoding

---

### 4. threaded_conversation.mbox

**Description:** Email thread with proper threading headers (In-Reply-To, References)
**Size:** ~3.8 KB
**Message Count:** 7
**Content Types:** Plain text
**Features:**
- Complete email thread (initial message + 6 replies)
- Threading headers (In-Reply-To, References)
- Multiple participants (Alice, Bob, Carol, Dave)
- Subject line evolution
- Full names in From headers ("Alice Smith <alice@example.com>")

**Test Coverage:**
- Thread relationship parsing
- In-Reply-To header extraction
- References header chain
- Display name parsing

**Expected Output:**
- 7 emails in chronological order
- All threading headers preserved
- Subject lines show thread evolution
- From headers show full names with email addresses

---

### 5. business_emails.mbox

**Description:** Professional business emails with priority, confidentiality, and formatting
**Size:** ~3.9 KB
**Message Count:** 5
**Content Types:** Plain text
**Features:**
- HR announcement (benefits enrollment)
- Finance report (Q4 budget review)
- Legal notice (confidential, attorney-client privilege)
- IT security alert (high priority)
- Executive communication (CEO all-hands)
- Special headers (X-Priority, Importance)

**Test Coverage:**
- Professional email formats
- Priority/importance handling
- Confidentiality markers
- Typical corporate communication

**Expected Output:**
- 5 business emails parsed correctly
- All headers extracted
- Priority indicators preserved (if available)
- Professional formatting maintained

---

## Validation

### Manual Validation

```bash
# Count messages in each file
grep -c "^From " test-corpus/email/mbox/*.mbox

# Expected counts:
# simple_10_messages.mbox: 10
# mixed_content.mbox: 3
# unicode_multilang.mbox: 5
# threaded_conversation.mbox: 7
# business_emails.mbox: 5
```

### Parser Validation

```bash
# Test with docling-email parser
cd crates/docling-email
cargo test --lib

# Test with docling-core integration
cd ../../
USE_RUST_BACKEND=1 cargo test --test integration_tests -- --exact test_mbox
```

---

## Test Expectations

### Parsing Success Rate
- **Target:** 100% (all 5 files parse successfully)
- **Total Messages:** 30 (10 + 3 + 5 + 7 + 5)
- **Expected Failures:** 0

### Content Verification

For each test file, verify:
1. **Message Count:** Correct number of emails extracted
2. **Headers:** From, To, Subject, Date, Message-ID present
3. **Body:** Message body text extracted
4. **Threading:** In-Reply-To and References preserved (if present)
5. **Attachments:** Attachment metadata listed (if present)
6. **Encoding:** Unicode/UTF-8 characters handled correctly
7. **Escaping:** mboxrd ">From " lines unescaped

### Markdown Output

Expected markdown structure for each file:
```markdown
# Mailbox: filename.mbox

**Message Count:** N

---

# Email 1: Subject Line
## Metadata
**From:** sender@example.com
**To:** recipient@example.com
**Date:** 2024-01-15T10:00:00-08:00

## Body
Email body content...

---

# Email 2: Next Subject
...
```

---

## Known Limitations

1. **Attachment Data:** Binary attachment data is not extracted, only metadata (filename, MIME type, size)
2. **HTML Conversion:** HTML emails are converted to markdown (some formatting may be lost)
3. **Threading Display:** Thread relationships preserved in headers but not visually represented in markdown
4. **Large Mailboxes:** Files >100MB may cause memory issues (current implementation loads entire file)

---

## Sources

All test files were synthetically generated for testing purposes. They contain:
- Realistic email structures and headers
- Diverse content types and encodings
- Common business communication patterns
- No real personal information (all data is fictional)

**Creation Date:** 2025-11-07
**Generated By:** Claude AI for docling_rs project
**License:** MIT (same as project)

---

## Future Test Additions

Potential additional test cases:
- **Large mailbox:** 1000+ messages (~50 MB)
- **Maildir comparison:** Same messages in MBOX vs Maildir format
- **Corrupted MBOX:** Malformed delimiters, missing headers
- **Spam folder:** Emails with suspicious content/headers
- **Archive:** Very old emails (1990s-era formatting)

---

**Total Test Coverage:** 30 messages across 5 diverse test files
**Estimated Parse Time:** <100ms total for all 5 files
