# MSG (Microsoft Outlook Message) Test Corpus

**Format:** Microsoft Outlook Message (.msg)
**Parser:** msg_parser v0.1.1
**Total Files:** 5 (to be created)
**Purpose:** Test MSG parsing, markdown conversion, and edge cases

---

## Test File Requirements

### 1. simple_email.msg (Basic plain text email)
**Size:** ~5-10 KB
**Features:**
- Plain text body (no HTML)
- Single sender
- Single recipient (To)
- Simple subject
- No attachments
- Basic headers

**Test Coverage:**
- Basic MSG parsing
- Subject extraction
- Sender/recipient parsing
- Plain text body extraction
- Date header parsing

**How to Create:**
1. Open Microsoft Outlook
2. Create new email:
   - From: Test Sender <sender@example.com>
   - To: recipient@example.com
   - Subject: Test Email
   - Body: "This is a simple test email message."
3. File → Save As → Outlook Message Format (.msg)
4. Save as `simple_email.msg`

---

### 2. html_email.msg (HTML formatted email)
**Size:** ~20-30 KB
**Features:**
- HTML body with formatting (bold, italics, lists)
- Embedded styles
- Multiple recipients (To and Cc)
- Rich subject line
- No attachments

**Test Coverage:**
- HTML body extraction
- HTML to markdown conversion (via html2md)
- Multiple recipient parsing
- Formatted content handling

**How to Create:**
1. Open Microsoft Outlook
2. Create new email with HTML formatting:
   - To: recipient1@example.com, recipient2@example.com
   - Cc: cc@example.com
   - Subject: HTML Email Test
   - Body: Use HTML formatting (bold, italics, bullet lists, links)
3. File → Save As → Outlook Message Format (.msg)
4. Save as `html_email.msg`

**Example Body:**
```html
<p>This is an <strong>HTML</strong> email with <em>formatting</em>.</p>
<ul>
  <li>Item 1</li>
  <li>Item 2</li>
  <li>Item 3</li>
</ul>
<p>Visit <a href="https://example.com">our website</a>.</p>
```

---

### 3. attachment_email.msg (Email with attachments)
**Size:** ~100-500 KB
**Features:**
- Plain or HTML body
- Multiple file attachments (PDF, DOCX, images)
- Attachment metadata (filename, size, MIME type)
- Test attachment extraction

**Test Coverage:**
- Attachment parsing
- Attachment metadata extraction (filename, size, MIME type)
- Multiple attachments handling
- Binary data handling

**How to Create:**
1. Open Microsoft Outlook
2. Create new email:
   - Subject: Email with Attachments
   - Body: "Please find the attached files."
   - Attach files:
     - document.pdf (~100 KB)
     - report.docx (~50 KB)
     - image.png (~20 KB)
3. File → Save As → Outlook Message Format (.msg)
4. Save as `attachment_email.msg`

---

### 4. meeting_request.msg (Calendar meeting invite)
**Size:** ~10-20 KB
**Features:**
- Meeting invitation
- Date and time information
- Location field
- Attendees list
- Optional: iCalendar (.ics) embedded data

**Test Coverage:**
- Special message type (meeting request)
- Calendar data extraction
- Attendee parsing
- Meeting metadata

**How to Create:**
1. Open Microsoft Outlook
2. Create new meeting invitation:
   - Calendar → New Meeting
   - Subject: Team Meeting
   - Location: Conference Room A
   - Date/Time: Future date, 1 hour duration
   - Attendees: attendee1@example.com, attendee2@example.com
   - Body: "Please join us for the team meeting."
3. Send invitation
4. Open Sent Items, find the meeting request
5. File → Save As → Outlook Message Format (.msg)
6. Save as `meeting_request.msg`

**Note:** Meeting requests have special properties that may not be fully supported by msg_parser v0.1.1. This file tests edge case handling.

---

### 5. thread_reply.msg (Reply in email thread)
**Size:** ~15-25 KB
**Features:**
- "Re:" subject prefix
- Quoted previous message in body
- Multiple recipients (To, Cc)
- In-Reply-To and References headers
- Thread relationship

**Test Coverage:**
- Thread detection (Re: prefix)
- Quoted text handling
- Reply metadata
- Multiple recipients

**How to Create:**
1. Create an initial email (simple_email.msg)
2. Open the email in Outlook
3. Click "Reply All"
4. Add response text above the quoted message:
   - Subject: Re: Test Email
   - Body: "Thank you for your message.\n\n> Original message quoted here..."
5. File → Save As → Outlook Message Format (.msg)
6. Save as `thread_reply.msg`

---

## Expected Output Validation

For each test file, the parser should produce markdown output in this format:

```markdown
# Email Message

**Subject:** [Email subject]
**From:** [Sender Name] <sender@example.com>
**To:** recipient1@example.com, recipient2@example.com
**Cc:** cc@example.com
**Date:** [Date string from headers]

---

## Body

[Plain text or converted HTML body]

---

## Attachments

- document.pdf (512 KB) [application/pdf]
- image.png (128 KB) [image/png]
```

---

## Test Execution

### Unit Tests (in msg.rs)

Run MSG-specific unit tests:
```bash
cargo test -p docling-email msg::
```

**Current Tests:**
- `test_parse_email_address_with_name` - Parse "Name <email>" format
- `test_parse_email_address_no_name` - Parse "email" format
- `test_format_email_address_with_name` - Format EmailAddress with name
- `test_format_email_address_no_name` - Format EmailAddress without name
- `test_format_email_list` - Format list of addresses
- `test_msg_to_markdown_basic` - Basic markdown conversion
- `test_msg_to_markdown_with_attachments` - Attachment formatting
- `test_msg_to_markdown_html_body` - HTML body conversion

### Integration Tests (with actual MSG files)

Once test files are created:
```bash
# Test parsing simple MSG
cargo test --test integration_tests test_msg_simple

# Test all MSG files
cargo test --test integration_tests test_msg

# Full test suite
cargo test --test integration_tests
```

---

## Known Limitations

### msg_parser v0.1.1 Limitations:

1. **No HTML Body Field:**
   - `outlook.body_html` doesn't exist
   - Only `outlook.body` (plain text) and `outlook.rtf_compressed` (RTF format)
   - HTML emails may lose formatting

2. **BCC Recipients:**
   - `outlook.bcc` is a String (DisplayBcc), not Vec<Person>
   - Requires manual parsing of comma-separated addresses

3. **Attachment Data:**
   - `att.payload` is base64-encoded string
   - Size calculation is approximate (base64 length)
   - Binary data not directly accessible

4. **Date Format:**
   - `outlook.headers.date` is a String
   - May require parsing for standardized format

5. **Meeting Requests:**
   - Special Outlook item types may not be fully parsed
   - Calendar data (iCalendar) may not be exposed

6. **No from_bytes() Method:**
   - Must use `Outlook::from_path()` with file path
   - Cannot parse from in-memory bytes directly

---

## Alternative Test File Sources

If you don't have Microsoft Outlook:

### Option 1: Use Python (Windows only)
```python
import win32com.client

outlook = win32com.client.Dispatch("Outlook.Application")
mail = outlook.CreateItem(0)  # 0 = MailItem
mail.Subject = "Test Email"
mail.Body = "This is a test email."
mail.To = "recipient@example.com"
mail.SaveAs("C:\\path\\to\\simple_email.msg", 3)  # 3 = olMSG format
```

### Option 2: Download Sample MSG Files
- Search for "sample .msg files" online
- Use msg-parser-rs test files (if available)
- Extract from email archives

### Option 3: Convert from EML
```bash
# If you have EML files, convert to MSG (requires Windows + Outlook)
# Or use online converters (not recommended for test corpus)
```

---

## Validation Checklist

For each test file:

- [ ] File exists and is valid MSG format (not corrupted)
- [ ] File size is reasonable (< 1 MB for test corpus)
- [ ] Parser doesn't crash on this file
- [ ] Subject extracted correctly
- [ ] Sender name and email extracted
- [ ] Recipients (To, Cc, Bcc) extracted correctly
- [ ] Body content extracted (plain text or HTML)
- [ ] Attachments listed with correct metadata
- [ ] Date extracted from headers
- [ ] Markdown output is readable and formatted correctly

---

## Future Enhancements

### Potential Improvements:

1. **HTML Body Support:**
   - Decode RTF body and convert to HTML/markdown
   - Use rtf-parser crate if needed

2. **Attachment Extraction:**
   - Decode base64 payload
   - Save attachments to disk
   - Provide binary data access

3. **Advanced Metadata:**
   - Extract all MAPI properties
   - Expose Outlook-specific fields
   - Parse meeting request data

4. **from_bytes() Implementation:**
   - Write bytes to temp file
   - Parse with Outlook::from_path()
   - Clean up temp file

---

## Contributing

To add test files to this corpus:

1. Create the MSG file using the instructions above
2. Verify the file with `msg_parser` (run simple parse test)
3. Add the file to this directory: `test-corpus/email/msg/`
4. Update this README with file details (size, features, SHA-256 hash)
5. Add integration test for the file

**Note:** MSG files are binary and may be large. Consider using git-lfs for version control, or documenting the files without committing them to the repository.

---

## File Inventory (To Be Created)

| File | Size | SHA-256 | Status |
|------|------|---------|--------|
| simple_email.msg | ~5-10 KB | TBD | ⏳ Not Created |
| html_email.msg | ~20-30 KB | TBD | ⏳ Not Created |
| attachment_email.msg | ~100-500 KB | TBD | ⏳ Not Created |
| meeting_request.msg | ~10-20 KB | TBD | ⏳ Not Created |
| thread_reply.msg | ~15-25 KB | TBD | ⏳ Not Created |

**Total Corpus Size:** ~150-580 KB (estimated)

**Creation Date:** 2025-11-07
**Last Updated:** 2025-11-07
**Maintainer:** docling_rs project

---

## References

- **MSG Format Specification:** [MS-OXMSG] Outlook Item (.msg) File Format
  https://docs.microsoft.com/en-us/openspecs/exchange_server_protocols/ms-oxmsg/
- **msg_parser crate:** https://github.com/marirs/msg-parser-rs
- **OLE/CFB Format:** [MS-CFB] Compound File Binary File Format
  https://docs.microsoft.com/en-us/openspecs/windows_protocols/ms-cfb/
