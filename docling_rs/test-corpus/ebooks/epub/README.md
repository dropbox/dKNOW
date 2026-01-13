# EPUB Test Files

Test corpus for EPUB (Electronic Publication) format parsing.

## Format Overview

**Format:** EPUB (Electronic Publication)
**Underlying:** ZIP archive with XHTML/HTML content + XML metadata
**Specification:** EPUB 2.0.1 (2010), EPUB 3.x (2011-2019)
**Complexity:** Medium (ZIP + XML parsing)

## Test Files

### 1. simple_novel.epub
- **Description:** Classic fiction novel (reflowable EPUB 2)
- **Content:**
  - Title: "Alice's Adventures in Wonderland"
  - Author: Lewis Carroll
  - Multiple chapters in XHTML
  - Simple text content (no images)
- **Purpose:** Test basic EPUB 2 parsing, metadata extraction, chapter navigation
- **Source:** Project Gutenberg (https://www.gutenberg.org/ebooks/11)
- **Download:** `wget https://www.gutenberg.org/ebooks/11.epub.images -O simple_novel.epub`
- **Status:** DRM-free, public domain

### 2. technical_book.epub
- **Description:** Technical documentation or programming book
- **Content:**
  - Technical content with code samples
  - Monospace formatting
  - Multiple sections and subsections
  - Code listings
- **Purpose:** Test code formatting, technical content, complex structure
- **Source:** "The Rust Programming Language" or similar open technical book
- **Download:** Manual download from https://doc.rust-lang.org/book/
- **Status:** DRM-free, open license

### 3. illustrated_book.epub
- **Description:** Children's book or illustrated novel
- **Content:**
  - Multiple images (PNG/JPEG)
  - Image captions
  - Simple narrative text
  - Cover image
- **Purpose:** Test image handling, figure captions, media files in manifest
- **Source:** Standard Ebooks (https://standardebooks.org/)
- **Example:** https://standardebooks.org/ebooks/charles-dickens/a-christmas-carol
- **Download:** Manual download from Standard Ebooks
- **Status:** DRM-free, public domain with modern formatting

### 4. magazine.epub
- **Description:** Digital magazine or journal (EPUB 3 fixed layout)
- **Content:**
  - Fixed layout pages
  - Multi-column layout
  - Mixed text and images
  - CSS styling
- **Purpose:** Test EPUB 3 features, fixed layout, advanced formatting
- **Source:** Create synthetic or find open magazine
- **Alternative:** Convert PDF magazine to EPUB using Calibre
- **Status:** May need to create custom test file

### 5. academic_book.epub
- **Description:** Academic textbook with footnotes and citations
- **Content:**
  - Academic writing
  - Footnotes and endnotes
  - Citations and bibliography
  - Tables and figures
- **Purpose:** Test footnotes, citations, complex academic structure
- **Source:** OpenStax (https://openstax.org/) - free textbooks
- **Example:** https://openstax.org/details/books/introduction-sociology-3e
- **Download:** OpenStax provides EPUB downloads
- **Status:** DRM-free, Creative Commons licensed

## How to Obtain EPUB Test Files

### Method 1: Project Gutenberg (Free Classic Books)
```bash
# Alice's Adventures in Wonderland
wget https://www.gutenberg.org/ebooks/11.epub.images -O simple_novel.epub

# More books: Browse https://www.gutenberg.org/
# Download format: EPUB with images
```

### Method 2: Standard Ebooks (High-Quality Free E-books)
```bash
# Visit https://standardebooks.org/
# Select a book
# Download EPUB (compatible)
# Example: Charles Dickens, Jane Austen, etc.
```

### Method 3: OpenStax (Free Textbooks)
```bash
# Visit https://openstax.org/
# Select a textbook
# Download EPUB format
# Example: Biology, Physics, Sociology textbooks
```

### Method 4: Calibre Conversion (For Testing)
```bash
# Install Calibre e-book management software
# Convert other formats to EPUB for testing

ebook-convert input.pdf output.epub
```

### Method 5: Create Synthetic EPUB (For Specific Tests)
Use `epub` Python library or manual ZIP construction for custom test cases.

## Expected Parsing Behavior

For each test file, the EPUB parser should extract:

**Essential Fields:**
- Title (from OPF metadata)
- Authors/Creators (from OPF metadata)
- Language (from OPF metadata)
- Publisher, Date, Identifier (from OPF metadata)
- Description/Summary (from OPF metadata)

**Content:**
- Chapters in reading order (spine order)
- Chapter titles (from headings or TOC)
- Chapter content (XHTML/HTML → plain text or markdown)

**Table of Contents:**
- TOC entries with labels and hrefs
- Hierarchical structure (if present)

**Markdown Output Format:**
```markdown
# [Book Title]

**Authors:** [Author 1, Author 2]
**Publisher:** [Publisher Name]
**Date:** [Publication Date]
**Language:** [en]

## Table of Contents

1. Chapter 1 - Introduction
2. Chapter 2 - Main Content
3. Chapter 3 - Conclusion

---

## Chapter 1 - Introduction

[Chapter content converted to plain text...]

---

## Chapter 2 - Main Content

[Chapter content...]

---

## Appendix

**Identifier:** ISBN 123-456-789
**Rights:** Public Domain / CC BY 4.0
```

## Known Limitations

1. **Fixed Layout EPUBs:** May not render correctly in markdown (complex CSS layouts)
2. **Images:** Image alt text extracted, but images not embedded in markdown
3. **MathML:** Math equations may not convert well to plain text
4. **JavaScript:** Interactive content not executed (EPUB 3 feature)
5. **Media Overlays:** Audio synchronization not supported (EPUB 3 feature)
6. **Complex CSS:** Styling information lost in plain text conversion
7. **Embedded Fonts:** Font information not extracted

## File Size Guidelines

- **Simple novel:** 200-500 KB (plain text)
- **Technical book:** 500 KB - 2 MB (with code samples)
- **Illustrated book:** 5-15 MB (with images)
- **Magazine:** 10-30 MB (high-res images, fixed layout)
- **Academic book:** 2-10 MB (tables, figures, references)

Keep test files under 50 MB for reasonable test execution times.

## Testing Strategy

### Unit Tests (docling-ebook/src/epub.rs)
- Test metadata extraction
- Test chapter parsing
- Test TOC extraction
- Test HTML title extraction
- Test HTML to plain text conversion

### Integration Tests (docling-core)
- Parse each EPUB file
- Verify title and author extraction
- Verify chapter count
- Verify TOC structure
- Compare output with expected markdown

### Edge Cases
- Empty EPUB (minimal structure)
- Corrupted EPUB (invalid ZIP or XML)
- Missing metadata (no title or author)
- DRM-protected EPUB (should error gracefully)
- Very large EPUB (> 100 MB)

## Validation Tools

To validate EPUB files and compare parsing:

### 1. EPUBCheck (Official Validator)
```bash
# Install: https://github.com/w3c/epubcheck
java -jar epubcheck.jar book.epub
```

### 2. Calibre E-book Viewer
```bash
# Install: https://calibre-ebook.com/
ebook-viewer book.epub
```

### 3. Python epub Library
```python
import epub
doc = epub.open_epub('book.epub')
print(doc.get_metadata('DC', 'title'))
```

### 4. Unzip and Inspect
```bash
# EPUBs are ZIP files
unzip book.epub -d extracted/
cat extracted/META-INF/container.xml
cat extracted/OEBPS/content.opf
```

## References

- **EPUB 3.2 Specification:** https://www.w3.org/publishing/epub32/
- **EPUB 2.0.1 Specification:** http://idpf.org/epub/20/spec/OPF_2.0.1_draft.htm
- **Project Gutenberg:** https://www.gutenberg.org/ (60,000+ free e-books)
- **Standard Ebooks:** https://standardebooks.org/ (high-quality public domain)
- **OpenStax:** https://openstax.org/ (free textbooks)
- **Rust `epub` crate:** https://crates.io/crates/epub

## Legal Notes

All test files should be:
- **DRM-free** (no encryption)
- **Legally distributable** (public domain, open license, or permission)
- **Properly attributed** (credit original authors and sources)

**Do not include:**
- DRM-protected commercial e-books
- Copyrighted content without permission
- Pirated or illegally obtained EPUBs

---

**Last Updated:** 2025-11-07
**Format Support:** EPUB 2.0.1, EPUB 3.x
**Test Files:** 0/5 (documented, need download)
