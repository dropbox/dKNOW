# FB2 Test Corpus

FictionBook (FB2) format test files for docling_rs.

## Format Overview

FB2 (FictionBook 2.0) is an XML-based e-book format popular in Russia and Eastern Europe. It stores books as structured XML documents with semantic markup for chapters, paragraphs, poems, and metadata.

## Test Files

### 1. simple.fb2
**Purpose:** Basic FB2 structure testing
**Features:**
- Simple 2-chapter structure
- Basic metadata (author, title, genre)
- Plain text paragraphs
- No special formatting

### 2. fiction_novel.fb2
**Purpose:** Complex fiction book with rich features
**Features:**
- Multi-level nested sections (parts, chapters)
- Rich metadata (author, annotation, keywords, ISBN)
- Epigraphs and text-author attribution
- Poem with stanzas and verses
- Empty lines for spacing
- Text emphasis (italic)
- Subtitles within chapters
- Multiple bodies (main text + author notes)
- Publishing information

### 3. technical_book.fb2
**Purpose:** Non-fiction technical book with code
**Features:**
- Technical content (programming)
- Code blocks with syntax preservation
- Inline code elements
- Numbered sections (hierarchical)
- Strong emphasis (bold)
- Special characters (&amp; entity encoding)
- Multi-level section nesting

### 4. poetry.fb2
**Purpose:** Poetry collection with specialized markup
**Features:**
- Multiple poems with titles
- Stanzas and verses (v elements)
- Epigraphs with attribution
- Multiple parts/sections
- Emphasis on poem structure

### 5. multilingual.fb2
**Purpose:** Unicode and international text support
**Features:**
- Multiple languages: Russian, English, Spanish, Chinese, French, Arabic, Greek, German
- Cyrillic characters (русский)
- Accented characters (español, français)
- CJK characters (中文)
- Right-to-left text (العربية)
- Greek alphabet (Ελληνικά)
- Special symbols and emojis
- Unicode special characters (№, ™, ©, ®, €, £, ¥, ₽)
- Mathematical symbols (∑, ∫, √, ≈, ≠, ±, ∞)

## Rust Library

Using `fb2` crate v0.4.4:
- GitHub: https://github.com/r-glazkov/fb2
- Crates.io: https://crates.io/crates/fb2
- Docs: https://docs.rs/fb2/0.4.4
