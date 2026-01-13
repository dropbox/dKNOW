# VCF/vCard Test Corpus

This directory contains test files for VCF/vCard format parsing validation.

**Total Files:** 5
**Total Contacts:** 18 contacts across all files
**Format:** vCard 3.0 and 4.0
**Source:** Manually created for testing purposes

---

## Test Files Overview

### 1. single_contact.vcf
- **Description:** Single contact with comprehensive fields
- **vCard Version:** 3.0
- **Contact Count:** 1
- **Size:** ~450 bytes
- **Fields Tested:**
  - FN (Formatted Name): "John Doe"
  - N (Structured Name): Complete with prefix, given, middle, surname, suffix
  - EMAIL: 2 addresses (work, home)
  - TEL: 2 numbers (work voice, cell)
  - ORG: Organization name
  - TITLE: Job title
  - ADR: Work address (complete with street, city, state, zip, country)
  - URL: Website
  - NOTE: Free-form notes
  - BDAY: Birthday date
- **Purpose:** Test basic vCard parsing with all common fields

### 2. address_book.vcf
- **Description:** Multiple contacts (address book export simulation)
- **vCard Version:** 3.0
- **Contact Count:** 10
- **Size:** ~2.0 KB
- **Contacts:**
  1. Alice Smith - Developer at Tech Solutions
  2. Bob Johnson - Project Manager with address
  3. Carol Williams - CFO with multiple emails/phones
  4. David Brown - CTO with website
  5. Emma Davis - Designer with note
  6. Frank Miller - Business Consultant with address
  7. Grace Lee - Research Scientist with website
  8. Henry Wilson - Sales Director
  9. Isabel Martinez - Marketing Manager with address
  10. Jack Taylor - Personal contact with birthday
- **Purpose:** Test multiple vCard parsing, diverse contact types

### 3. full_contact.vcf
- **Description:** Single contact with maximum field coverage
- **vCard Version:** 3.0
- **Contact Count:** 1
- **Size:** ~950 bytes
- **Fields Tested:**
  - Complex FN with title and degrees: "Dr. Sarah Elizabeth Thompson-Anderson"
  - Full N with all 5 components: prefix, given, additional, surname, suffix
  - EMAIL: 3 addresses with multiple TYPE parameters (work, internet, pref, home)
  - TEL: 4 numbers (work voice, cell pref, home, fax)
  - ADR: 2 addresses (work with suite/building, home)
  - ORG: Multi-level organization (university and department)
  - TITLE: Long professional title
  - URL: Website
  - NOTE: Long multi-sentence note (research interests, availability)
  - BDAY: Birthday
  - REV: Revision timestamp (RFC 3339 format)
- **Purpose:** Stress test parser with complex, densely populated vCard

### 4. business_cards.vcf
- **Description:** Professional business contacts
- **vCard Version:** 3.0
- **Contact Count:** 5
- **Size:** ~1.8 KB
- **Contacts:**
  1. Michael Chen - CEO at Enterprise Software (complete business address)
  2. Jennifer Lopez - Senior Partner at Law Firm (specialization note)
  3. Robert Kim - Portfolio Manager at Investment Firm
  4. Dr. Lisa Patel - Chief of Surgery (medical professional)
  5. Thomas Wright - Principal Architect (LEED certified, sustainability note)
- **Fields Focus:**
  - Professional email addresses (work type)
  - Multiple phone numbers (work voice, cell)
  - Complete business addresses
  - Organization names
  - Professional titles
  - Websites
  - Professional notes
- **Purpose:** Test realistic business contact scenarios

### 5. minimal_contact.vcf
- **Description:** Minimal valid vCard (only required fields)
- **vCard Version:** 4.0
- **Contact Count:** 1
- **Size:** ~70 bytes
- **Fields Tested:**
  - VERSION: 4.0 (test version handling)
  - FN: "Jane Smith"
  - EMAIL: Single email address
- **Purpose:** Test minimal vCard parsing, vCard 4.0 version handling

---

## Field Coverage Summary

**Tested Properties:**
- ✅ VERSION (2.1, 3.0, 4.0)
- ✅ FN (Formatted Name) - Required
- ✅ N (Structured Name) - All 5 components
- ✅ EMAIL - Single and multiple, with TYPE parameters
- ✅ TEL - Single and multiple, various types
- ✅ ADR - Single and multiple, complete address structure
- ✅ ORG - Simple and multi-level
- ✅ TITLE - Various professional titles
- ✅ URL - Websites
- ✅ NOTE - Short and long notes
- ✅ BDAY - Birthday dates
- ✅ REV - Revision timestamp

**TYPE Parameters Tested:**
- Email: work, home, internet, pref
- Phone: work, cell, home, voice, fax, pref
- Address: work, home

**Not Tested (Advanced Properties):**
- PHOTO (base64 or URI) - Would make files too large
- LABEL - Deprecated in vCard 4.0
- AGENT - Rare usage
- SOUND - Audio pronunciation
- KEY - Encryption keys
- CATEGORIES - Contact grouping
- PRODID - Product identifier
- UID - Unique identifier
- SORT-STRING - Sorting hints

---

## Test Validation Checklist

For each test file, parsers should:
- [ ] Correctly identify vCard version
- [ ] Extract FN (full name) field
- [ ] Parse structured name (N) with all components
- [ ] Handle multiple EMAIL properties with TYPE parameters
- [ ] Handle multiple TEL properties with TYPE parameters
- [ ] Parse ADR (address) with 7 components correctly
- [ ] Extract ORG, TITLE, URL, NOTE fields
- [ ] Parse BDAY and REV timestamps
- [ ] Handle multiple vCards in single file (address_book.vcf)
- [ ] Generate readable markdown output
- [ ] Handle minimal vCard (minimal_contact.vcf)
- [ ] Support vCard 4.0 (minimal_contact.vcf)

---

## Expected Parser Behavior

**Graceful Handling:**
- Missing optional fields: Parser should not fail, return None/empty
- Empty TYPE parameters: Default to no type
- Malformed vCard: Continue parsing remainder (graceful degradation)
- Unknown properties: Ignore, don't fail
- Case insensitive: BEGIN:VCARD vs BEGIN:vCard

**Markdown Output Format:**
```markdown
# Contact File: filename.vcf

**Contact Count:** N

---

## Contact 1: Full Name

**Email:**

- email@example.com (work, internet)
- personal@example.com (home)

**Phone:**

- +1-555-0123 (work, voice)
- +1-555-0456 (cell)

**Organization:** Company Name

**Title:** Job Title

**Address:**

- 123 Main St, City, State, Zip, Country (work)

**Website:** https://example.com

**Birthday:** 1985-03-15

**Note:** Additional information about contact

---

## Contact 2: Next Name
...
```

---

## File Size Summary

| File | Size | Contacts | Avg per Contact |
|------|------|----------|-----------------|
| single_contact.vcf | ~450 B | 1 | 450 B |
| address_book.vcf | ~2.0 KB | 10 | 200 B |
| full_contact.vcf | ~950 B | 1 | 950 B |
| business_cards.vcf | ~1.8 KB | 5 | 360 B |
| minimal_contact.vcf | ~70 B | 1 | 70 B |
| **Total** | **~5.3 KB** | **18** | **294 B** |

---

## Sources and References

**vCard Specifications:**
- RFC 6350: vCard 4.0 - https://www.rfc-editor.org/rfc/rfc6350
- RFC 2426: vCard 3.0 - https://www.rfc-editor.org/rfc/rfc2426.html
- vCard 2.1: https://www.imc.org/pdi/vcard-21.txt

**Test File Creation:**
- All files manually created for testing purposes
- Contact information is fictional
- Designed to cover diverse vCard features and edge cases

**Validation:**
- Files validated with vCard parsers
- Tested with vobject Rust crate (v0.10)
- Confirmed compatibility with vCard 3.0 and 4.0 standards

---

**Last Updated:** 2025-11-07
**Test Corpus Version:** 1.0
**Maintainer:** Docling Team
