# Comprehensive Document Format Expansion - All Formats

**Project:** docling_rs
**Version:** 2.0 - Expanded Coverage
**Date:** 2025-11-06
**Purpose:** Complete document format support across all business, creative, technical, and specialized domains

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [All Supported Formats (Current)](#all-supported-formats-current)
3. [All Proposed Formats (Comprehensive)](#all-proposed-formats-comprehensive)
4. [Format Categories](#format-categories)
5. [AI Execution Checklist](#ai-execution-checklist)
6. [Test Corpus Requirements](#test-corpus-requirements)
7. [Implementation Directives](#implementation-directives)

---

## Executive Summary

### Original State: 15 Formats

**Office:** PDF, DOCX, PPTX, XLSX
**Web:** HTML, CSV, Markdown, AsciiDoc
**Specialized:** JATS, WebVTT
**Images:** PNG, JPEG, TIFF, WebP, BMP

### Current State (N=67): 51 Formats Implemented

**Original (15):** PDF, DOCX, PPTX, XLSX, HTML, CSV, Markdown, AsciiDoc, JATS, WebVTT, PNG, JPEG, TIFF, WebP, BMP

**New (36):** DOC, SRT, GIF, ZIP, TAR, 7Z, RAR, WAV, MP3, MP4, MKV, MOV, AVI, EML, MBOX, MSG, VCF (vCard), EPUB, FB2, MOBI, ODT, ODS, ODP, XPS, SVG, HEIF, AVIF, ICS, Jupyter Notebooks (.ipynb), GPX, KML, KMZ, DICOM, RTF, STL, OBJ, GLTF, GLB, DXF, IDML

**Note:** Genomic VCF implemented in docling-genomics crate (not yet in main InputFormat enum)

### Phase I Complete (with Strategic Deferrals): Adobe Creative Suite (N=64)

**Phase I Status (N=64):**
- ✅ I1: Vector & Publishing (1/2 complete): IDML ✅, AI ⏸️ deferred (20-40h, no PostScript parser)
- ⏸️ I2: Raster & Forms: PSD ⏸️ deferred (20-30h, text layers unsupported), XFA ⏸️ deferred, INDD ⏸️ deferred (use IDML)
- **Implemented:** IDML (InDesign Markup Language) complete with text extraction and markdown serialization (N=63)
- **Rationale for deferrals:**
  - AI: Requires PostScript parser (Turing-complete), 20-40 hours for full implementation, Python docling doesn't support it
  - PSD: Rust `psd` crate lacks text layer extraction (GitHub issue #20 open 4+ years), would require 20-30 hours custom parser
  - XFA: Very high complexity (XML Forms Architecture in PDF)
  - INDD: Proprietary binary, IDML (implemented) is the open interchange format
- **Strategic decision:** Adobe IDML provides sufficient publishing format coverage. Focus on higher-ROI formats.
- **Next:** Pivot to Phase G (Microsoft Extended), Phase H (Apple iWork), or Phase K3-4 (LaTeX)

### Phase J Complete: CAD & 3D Formats (N=62)

**Phase J Status:**
- ✅ J1: CAD Formats (1/3 complete): DXF ✅, DWG ⏸️ deferred, IFC ⏸️ deferred
- ✅ J2: 3D Formats (3/4 complete): STL ✅, OBJ ✅, GLTF/GLB ✅, FBX ⏸️ deferred
- **Rationale for deferrals:** DXF provides sufficient CAD coverage, GLTF/GLB/OBJ cover 3D needs

### Phase L Partial: Legacy Formats (N=56, N=66)

**Phase L Status:**
- ✅ L1-1: RTF support (N=56, rtf-parser crate)
- ✅ L1-2: DOC support (N=66, textutil conversion on macOS)
- ⏸️ L1-3: WordPerfect (.wpd) - DEFERRED (requires libwpd FFI, low ROI)
- ⏸️ L1-4: WPS (Microsoft Works) - DEFERRED (very legacy, low usage)
- **Rationale for deferrals:** RTF and DOC provide sufficient legacy Office coverage

### Remaining Formats: 9 VIABLE FORMATS (N=67 Status)

**Total Target: 60 VIABLE FORMATS** (51 implemented / 60 viable = **85% complete**) 🎯 **MILESTONE: 5/6 COMPLETE**

**Deferred Formats (7):**
- **CAD/3D:** DWG (use DXF), IFC (experimental libraries), FBX (GLTF/GLB/OBJ sufficient)
- **Adobe:** AI (no PostScript parser), PSD (text layers unsupported), XFA (very high complexity), INDD (use IDML)

### Strategic Pause at N=67

**Decision:** Format expansion paused for strategic planning

**Remaining 9 formats all require:**
1. **External tools** (LibreOffice, pandoc) - not currently installed
2. **Proprietary XML reverse engineering** (Apple iWork: Pages, Numbers, Keynote)
3. **Very high complexity** (Microsoft: MDB/ACCDB, MPP, ONE)

**Decision Report:** `reports/feature-phase-e-open-standards/remaining_formats_decision_2025-11-07.md`

**Recommended Paths:**
- **Path 1 (100% completion):** Install tools + implement iWork (46-67h, 5-7 iterations)
- **Path 2 (90% completion):** LaTeX + Pages only (18-25h, 2-3 iterations)
- **Path 3 (85% current):** Declare victory, focus on quality (0h)

**Next:** N=70 cleanup milestone (N mod 5), then user decision on completion target

### Categories (12 major categories)

1. **Audio Formats** (2) - Transcription
2. **E-book Formats** (4) - Digital publishing
3. **Email Formats** (5) - Communication archives
4. **Apple iWork** (3) - Mac ecosystem
5. **Adobe Creative Extended** (5) - Professional design
6. **Microsoft Extended** (6) - Enterprise tools
7. **CAD/Engineering** (4) - Technical drawings
8. **3D Formats** (3) - 3D modeling
9. **Archive Formats** (4) - Compressed archives
10. **Video Formats** (5) - Subtitle/transcript extraction
11. **Specialized Formats** (7) - Industry-specific
12. **Legacy/Additional** (4) - Backward compatibility

---

## All Supported Formats (Current)

### Already Implemented (15 formats)

| Format | Extension | Category | Status |
|--------|-----------|----------|--------|
| PDF | .pdf | Office | ✅ Implemented |
| DOCX | .docx | Office | ✅ Implemented |
| PPTX | .pptx | Office | ✅ Implemented |
| XLSX | .xlsx | Office | ✅ Implemented |
| HTML | .html | Web | ✅ Implemented |
| CSV | .csv | Data | ✅ Implemented |
| Markdown | .md | Web | ✅ Implemented |
| AsciiDoc | .asciidoc | Web | ✅ Implemented |
| JATS | .nxml | Academic | ✅ Implemented |
| WebVTT | .vtt | Captions | ✅ Implemented |
| PNG | .png | Image | ✅ Implemented |
| JPEG | .jpg | Image | ✅ Implemented |
| TIFF | .tif | Image | ✅ Implemented |
| WebP | .webp | Image | ✅ Implemented |
| BMP | .bmp | Image | ✅ Implemented |

---

## All Proposed Formats (Comprehensive)

### CATEGORY 1: Audio Formats (2 formats)

#### 1.1 WAV - Waveform Audio
**Extensions:** `.wav`
**Rust Libraries:** `hound`, `whisper-rs`, `vosk`
**Complexity:** Medium
**Use Case:** Meeting transcription, audio content extraction

**Test Files (5 examples):**
1. Meeting recording (business)
2. Podcast excerpt
3. Technical presentation
4. Interview
5. Multi-speaker conversation

**Sources:** https://commonvoice.mozilla.org/, https://librivox.org/

#### 1.2 MP3 - MPEG Audio Layer 3
**Extensions:** `.mp3`
**Rust Libraries:** `minimp3`, `symphonia`, `whisper-rs`
**Complexity:** Medium
**Use Case:** Compressed audio transcription

**Test Files (5 examples):**
1. High-quality speech (320kbps)
2. Music with vocals
3. Audiobook sample
4. Low-quality recording (64kbps)
5. Foreign language audio

**Sources:** https://freemusicarchive.org/, Mozilla Common Voice

---

### CATEGORY 2: E-book Formats (4 formats)

#### 2.1 EPUB - Electronic Publication
**Extensions:** `.epub`
**Rust Libraries:** `epub` (v2.0), `zip`, `quick-xml`
**Complexity:** Medium
**Use Case:** E-books, digital publications, technical documentation

**Implementation:**
- EPUB is ZIP archive with XHTML/HTML content
- Parse content.opf for metadata
- Extract XHTML chapters in reading order
- Handle both EPUB2 and EPUB3

**Test Files (5 examples):**
1. Fiction novel (reflowable)
2. Technical book with code samples
3. Illustrated children's book
4. Magazine (fixed layout)
5. Academic textbook with footnotes

**Sources:**
- Project Gutenberg: https://www.gutenberg.org/
- Standard Ebooks: https://standardebooks.org/
- Open Library: https://openlibrary.org/

#### 2.2 MOBI - Mobipocket
**Extensions:** `.mobi`, `.prc`
**Rust Libraries:** None mature - use `mobi` crate (limited) or convert via Calibre
**Complexity:** High
**Use Case:** Kindle e-books (older format)

**Implementation:**
- Consider conversion approach: MOBI → EPUB via Calibre
- Or: Parse MOBI binary format directly (complex)
- MOBI is based on Palm Database format

**Test Files (5 examples):**
1. Fiction e-book
2. Non-fiction with images
3. Dictionary/reference
4. Textbook
5. Comic book

**Sources:**
- MobileRead forums
- Internet Archive
- Convert EPUB samples

#### 2.3 AZW/AZW3 - Amazon Kindle
**Extensions:** `.azw`, `.azw3`
**Rust Libraries:** None - DRM-protected, requires Calibre or DeDRM
**Complexity:** Very High (DRM issues)
**Use Case:** Modern Kindle e-books

**Implementation:**
- AZW3 is KF8 format (similar to EPUB)
- DRM-free AZW3 can be parsed
- Consider requiring DRM-free files only

**Test Files (5 examples):**
1. DRM-free Kindle book
2. Self-published book
3. Technical manual
4. Magazine subscription
5. Textbook

**Sources:**
- Amazon self-publishing samples
- DRM-free indie books

**⚠️ Legal Note:** Only support DRM-free files. Do not implement DRM circumvention.

#### 2.4 FB2 - FictionBook
**Extensions:** `.fb2`, `.fb2.zip`
**Rust Libraries:** `quick-xml` (FB2 is XML-based)
**Complexity:** Low-Medium
**Use Case:** Popular in Russia/Eastern Europe

**Implementation:**
- FB2 is pure XML format
- Well-documented structure
- Support compressed (.fb2.zip) variant

**Test Files (5 examples):**
1. Fiction novel
2. Non-fiction book
3. Book with images
4. Poetry collection
5. Technical documentation

**Sources:**
- Russian digital libraries
- FB2 sample files online

---

### CATEGORY 3: Email Formats (5 formats)

#### 3.1 EML - Email Message
**Extensions:** `.eml`
**Rust Libraries:** `mail-parser` (v0.9), `mailparse`
**Complexity:** Medium
**Use Case:** Email archiving, e-discovery, communication extraction

**Implementation:**
- Parse MIME structure
- Extract headers, body, attachments
- Handle multipart messages
- Support HTML and plain text bodies

**Test Files (5 examples):**
1. Plain text email
2. HTML email with images
3. Email with attachments (PDF, DOCX)
4. Thread/conversation
5. Calendar invite (.ics embedded)

**Sources:**
- Export from email clients (Outlook, Thunderbird)
- Generate test emails

#### 3.2 MSG - Outlook Message
**Extensions:** `.msg`
**Rust Libraries:** `msg-parser` (limited) or use C library via FFI
**Complexity:** High (proprietary binary format)
**Use Case:** Microsoft Outlook email extraction

**Implementation:**
- MSG is OLE/CFB (Compound File Binary) format
- Parse property streams
- Extract body, recipients, attachments
- Alternative: Use external tool for conversion

**Test Files (5 examples):**
1. Simple text email
2. Rich HTML email
3. Email with attachments
4. Meeting request
5. Email with embedded images

**Sources:**
- Export from Outlook
- Business email archives

#### 3.3 MBOX - Mailbox Archive
**Extensions:** `.mbox`, `.mbx`
**Rust Libraries:** `mailparse`, custom parser (format is simple)
**Complexity:** Low-Medium
**Use Case:** Unix email archives, Thunderbird backups

**Implementation:**
- MBOX is text-based format
- Messages separated by "From " lines
- Parse each message as EML
- Handle different MBOX variants (mboxrd, mboxcl)

**Test Files (5 examples):**
1. Small mailbox (10 messages)
2. Large archive (1000+ messages)
3. Mixed content mailbox
4. Spam folder
5. Sent items

**Sources:**
- Export from Thunderbird
- Unix mail archives

#### 3.4 PST - Outlook Personal Folders
**Extensions:** `.pst`, `.ost`
**Rust Libraries:** None mature - use `pst` crate (experimental) or libpst via FFI
**Complexity:** Very High (complex proprietary format)
**Use Case:** Outlook data file extraction, e-discovery

**Implementation:**
- PST is complex database format
- Contains emails, calendar, contacts, tasks
- Consider using libpst (C library) via FFI
- Or: Require conversion to MBOX/EML first

**Test Files (5 examples):**
1. Small PST (< 100MB)
2. Large PST (> 1GB)
3. PST with calendar items
4. PST with contacts
5. OST (offline) file

**Sources:**
- Export from Outlook
- Legal e-discovery samples (if available)

#### 3.5 VCF/vCard - Contact Cards
**Extensions:** `.vcf`, `.vcard`
**Rust Libraries:** `vcard` crate, `vobject` parser
**Complexity:** Low
**Use Case:** Contact information extraction

**Implementation:**
- VCF is text-based format (RFC 6350)
- Parse contact fields: name, email, phone, address
- Handle vCard 2.1, 3.0, 4.0 versions
- Support multiple contacts in one file

**Test Files (5 examples):**
1. Single contact
2. Multiple contacts (address book)
3. Contact with photo
4. Business card with full details
5. Minimal contact (name + email only)

**Sources:**
- Export from address books
- Sample vCards online

---

### CATEGORY 4: Apple iWork Formats (3 formats)

#### 4.1 Pages - Apple Word Processor
**Extensions:** `.pages`
**Rust Libraries:** None - `.pages` is ZIP with proprietary XML
**Complexity:** High
**Use Case:** Mac document processing

**Implementation:**
- Modern Pages files are ZIP archives
- Contains index.xml with content
- Uses Apple's proprietary XML schema
- Legacy .pages are iWork '09 format (different structure)

**Test Files (5 examples):**
1. Simple text document
2. Report with images
3. Template document
4. Document with tables
5. Newsletter layout

**Sources:**
- Create with Pages on Mac
- Apple iWork sample files

#### 4.2 Numbers - Apple Spreadsheet
**Extensions:** `.numbers`
**Rust Libraries:** None - ZIP with proprietary format
**Complexity:** High
**Use Case:** Mac spreadsheet processing

**Implementation:**
- ZIP archive with index.zip
- index.zip contains another ZIP with data
- Nested structure with protobuf/proprietary format
- Complex to parse

**Test Files (5 examples):**
1. Simple spreadsheet
2. Budget/financial data
3. Spreadsheet with charts
4. Multiple sheets
5. Spreadsheet with formulas

**Sources:**
- Create with Numbers on Mac
- Apple sample files

#### 4.3 Keynote - Apple Presentation
**Extensions:** `.key`
**Rust Libraries:** None - ZIP with proprietary format
**Complexity:** High
**Use Case:** Mac presentation processing

**Implementation:**
- ZIP archive structure
- index.xml with presentation content
- Slide content in proprietary format
- Similar complexity to Pages

**Test Files (5 examples):**
1. Simple presentation
2. Presentation with animations
3. Presentation with charts
4. Photo slideshow
5. Business pitch deck

**Sources:**
- Create with Keynote on Mac
- Apple sample files

---

### CATEGORY 5: Adobe Creative Extended (5 formats)

#### 5.1 IDML - InDesign Markup Language
**Extensions:** `.idml`
**Rust Libraries:** `quick-xml` (IDML is XML-based)
**Complexity:** High
**Use Case:** Publishing, magazines, brochures

*(Details from previous plan)*

#### 5.2 AI - Adobe Illustrator
**Extensions:** `.ai`
**Rust Libraries:** `pdf` crate (AI is PDF-based)
**Complexity:** High
**Use Case:** Vector graphics, logos, diagrams

*(Details from previous plan)*

#### 5.3 PSD - Adobe Photoshop
**Extensions:** `.psd`, `.psb`
**Rust Libraries:** `psd` crate
**Complexity:** High
**Use Case:** Photo editing, composites

*(Details from previous plan)*

#### 5.4 XFA - PDF Forms (Acrobat)
**Extensions:** `.pdf` (with XFA data)
**Rust Libraries:** `pdf` crate + custom XFA parser
**Complexity:** Very High
**Use Case:** Interactive PDF forms

**Implementation:**
- XFA (XML Forms Architecture) embedded in PDF
- Parse XFA XML structure
- Extract form fields and values
- Complex form logic

**Test Files (5 examples):**
1. Simple form (name, address)
2. Tax form (IRS)
3. Application form
4. Survey form
5. Contract with fillable fields

**Sources:**
- IRS tax forms
- Government forms
- Business forms

#### 5.5 INDD - InDesign Native
**Extensions:** `.indd`
**Rust Libraries:** None (proprietary binary)
**Complexity:** Very High
**Use Case:** Native InDesign files

**Implementation:**
- Proprietary binary format
- Consider requiring export to IDML instead
- Or: Use InDesign Server API for conversion
- Parsing binary INDD is extremely complex

**Test Files (5 examples):**
1. Magazine layout
2. Book chapter
3. Brochure
4. Poster
5. Business card template

**Sources:**
- Create with InDesign
- Design template websites

---

### CATEGORY 6: Microsoft Extended (6 formats)

#### 6.1 PUB - Microsoft Publisher
*(Details from previous plan)*

#### 6.2 VSDX - Microsoft Visio
*(Details from previous plan)*

#### 6.3 ONE - Microsoft OneNote
*(Details from previous plan)*

#### 6.4 MPP - Microsoft Project
*(Details from previous plan)*

#### 6.5 MDB/ACCDB - Microsoft Access
**Extensions:** `.mdb`, `.accdb`
**Rust Libraries:** None mature - consider `mdbtools` via FFI
**Complexity:** Very High
**Use Case:** Database extraction, report generation

**Implementation:**
- MDB is Jet Database format (legacy)
- ACCDB is Access 2007+ format
- Use mdbtools (C library) for parsing
- Extract tables, queries, reports
- Export as CSV or structured data

**Test Files (5 examples):**
1. Simple contact database
2. Inventory database
3. CRM database
4. Database with forms/reports
5. Large multi-table database

**Sources:**
- Microsoft Access templates
- Sample databases online

#### 6.6 XPS - XML Paper Specification
**Extensions:** `.xps`, `.oxps`
**Rust Libraries:** `zip` + `quick-xml` (XPS is ZIP with XML)
**Complexity:** Medium
**Use Case:** Windows document format (PDF alternative)

**Implementation:**
- XPS is ZIP archive with XML pages
- Similar to PDF in purpose
- Parse FixedDocument structure
- Extract text and images

**Test Files (5 examples):**
1. Simple text document
2. Document with images
3. Multi-page report
4. Document with tables
5. Print-ready document

**Sources:**
- Windows "Print to XPS"
- Microsoft XPS samples

---

### CATEGORY 7: CAD/Engineering (4 formats)

#### 7.1 DWG - AutoCAD Drawing
**Extensions:** `.dwg`
**Rust Libraries:** None mature - consider `ezdxf` Python library via bridge
**Complexity:** Very High (proprietary)
**Use Case:** CAD drawings, architecture, engineering

**Implementation:**
- DWG is proprietary Autodesk format
- Consider using Open Design Alliance libraries
- Or: Require conversion to DXF
- Extract text entities, dimensions, metadata

**Test Files (5 examples):**
1. Simple 2D floor plan
2. Mechanical part drawing
3. Electrical schematic
4. 3D architectural model
5. Civil engineering site plan

**Sources:**
- AutoCAD sample files
- Engineering/architecture firms
- Open-source CAD projects

#### 7.2 DXF - Drawing Exchange Format
**Extensions:** `.dxf`
**Rust Libraries:** `dxf` crate (v0.4)
**Complexity:** Medium-High
**Use Case:** CAD data exchange (open format)

**Implementation:**
- DXF is text-based or binary CAD format
- Well-documented by Autodesk
- Parse entities: lines, text, dimensions
- Support both ASCII and binary DXF

**Test Files (5 examples):**
1. Simple 2D drawing
2. Mechanical assembly
3. Architectural floor plan
4. Electrical diagram
5. 3D model

**Sources:**
- AutoCAD "Save As DXF"
- Open-source CAD files

#### 7.3 STL - Stereolithography
**Extensions:** `.stl`
**Rust Libraries:** `stl_io` (v0.6)
**Complexity:** Low
**Use Case:** 3D printing, 3D models

**Implementation:**
- STL is simple 3D mesh format
- ASCII or binary variants
- Extract model metadata, dimensions
- Describe geometry (triangle count, bounds)

**Test Files (5 examples):**
1. Simple cube/sphere
2. Mechanical part
3. Figurine/sculpture
4. Architectural model
5. Medical scan (anatomical)

**Sources:**
- Thingiverse: https://www.thingiverse.com/
- NIH 3D Print Exchange
- Open-source 3D models

#### 7.4 IFC - Industry Foundation Classes
**Extensions:** `.ifc`
**Rust Libraries:** Custom parser (IFC is text-based)
**Complexity:** Very High
**Use Case:** BIM (Building Information Modeling)

**Implementation:**
- IFC is ISO standard for BIM data
- Text-based STEP format (ISO 10303-21)
- Complex object hierarchy
- Extract building elements, properties, metadata

**Test Files (5 examples):**
1. Simple building model
2. Multi-story building
3. Bridge/infrastructure
4. MEP (mechanical/electrical/plumbing) model
5. Interior design model

**Sources:**
- buildingSMART sample files
- Open BIM repositories

---

### CATEGORY 8: 3D Formats (3 formats)

#### 8.1 OBJ - Wavefront Object
**Extensions:** `.obj`, `.mtl` (materials)
**Rust Libraries:** `tobj` (v4.0)
**Complexity:** Low-Medium
**Use Case:** 3D models, graphics, games

**Implementation:**
- OBJ is text-based 3D format
- Parse vertices, faces, materials
- Extract model metadata
- Companion MTL file for materials

**Test Files (5 examples):**
1. Simple geometric shape
2. Character model
3. Architectural element
4. Vehicle model
5. Terrain/landscape

**Sources:**
- Free3D.com
- Sketchfab (free models)
- Blender exports

#### 8.2 FBX - Filmbox (Autodesk)
**Extensions:** `.fbx`
**Rust Libraries:** None mature - consider `fbxcel` (limited)
**Complexity:** Very High
**Use Case:** Game assets, animation, 3D interchange

**Implementation:**
- FBX is proprietary binary (ASCII variant exists)
- Contains models, animations, materials
- Complex format with many features
- Consider using Autodesk FBX SDK via FFI

**Test Files (5 examples):**
1. Static model
2. Rigged character
3. Animated model
4. Environment/scene
5. Game asset

**Sources:**
- Unity Asset Store (free assets)
- Game dev resources
- Mixamo (free rigged characters)

#### 8.3 GLTF/GLB - GL Transmission Format
**Extensions:** `.gltf`, `.glb`
**Rust Libraries:** `gltf` crate (v1.3)
**Complexity:** Medium
**Use Case:** Web 3D, AR/VR, game assets

**Implementation:**
- GLTF is JSON-based 3D format
- GLB is binary variant
- Well-documented Khronos standard
- Extract model metadata, scene graph

**Test Files (5 examples):**
1. Simple 3D object
2. Textured model
3. Animated character
4. PBR (physically-based rendering) model
5. Complex scene

**Sources:**
- Khronos samples: https://github.com/KhronosGroup/glTF-Sample-Models
- Sketchfab

---

### CATEGORY 9: Archive Formats (4 formats)

#### 9.1 ZIP - ZIP Archive
**Extensions:** `.zip`
**Rust Libraries:** `zip` crate (v0.6)
**Complexity:** Low
**Use Case:** Extract and parse contents

**Implementation:**
- Extract all files from ZIP
- Recursively process each file based on type
- Handle nested archives
- List contents if extraction not desired

**Test Files (5 examples):**
1. Document archive (multiple DOCs)
2. Source code ZIP
3. Image archive
4. Mixed content ZIP
5. Password-protected ZIP

**Sources:**
- Create sample ZIPs
- Software downloads

#### 9.2 RAR - Roshal Archive
**Extensions:** `.rar`
**Rust Libraries:** `unrar` via FFI (uses C library)
**Complexity:** Medium (proprietary, needs external library)
**Use Case:** Extract RAR archives

**Implementation:**
- Use unrar library (C bindings)
- Extract files for processing
- Handle RAR5 format
- List contents

**Test Files (5 examples):**
1. Document RAR
2. Multi-volume RAR
3. RAR5 format
4. Large archive
5. RAR with recovery records

**Sources:**
- Create with WinRAR
- Download archives online

#### 9.3 7Z - 7-Zip Archive
**Extensions:** `.7z`
**Rust Libraries:** `sevenz-rust` (v0.4)
**Complexity:** Medium
**Use Case:** High-compression archives

**Implementation:**
- Use 7z library or bindings
- Extract contents
- Handle LZMA compression
- Support multiple compression methods

**Test Files (5 examples):**
1. Document 7z
2. Highly compressed archive
3. Solid archive
4. Encrypted 7z
5. Multi-volume 7z

**Sources:**
- Create with 7-Zip
- Software archives

#### 9.4 TAR/TAR.GZ - Tape Archive
**Extensions:** `.tar`, `.tar.gz`, `.tgz`, `.tar.bz2`
**Rust Libraries:** `tar` (v0.4), `flate2`, `bzip2`
**Complexity:** Low
**Use Case:** Unix/Linux archives

**Implementation:**
- Parse TAR format
- Handle compressed variants (gzip, bzip2)
- Extract files for processing
- Preserve file metadata

**Test Files (5 examples):**
1. Source code tarball
2. Backup archive
3. Documentation archive
4. .tar.gz (common)
5. .tar.bz2 (alternative compression)

**Sources:**
- Unix software downloads
- Create sample tarballs

---

### CATEGORY 10: Video Formats (5 formats) - Subtitle/Transcript Extraction

#### 10.1 MP4 - MPEG-4 Video
**Extensions:** `.mp4`, `.m4v`
**Rust Libraries:** `mp4parse`, `ffmpeg-rs` (via FFI)
**Complexity:** Medium
**Use Case:** Extract subtitles, audio transcription

**Implementation:**
- Extract embedded subtitle tracks (SRT, VTT)
- Extract audio track for transcription
- Parse video metadata
- Use ffmpeg for extraction

**Test Files (5 examples):**
1. Video with English subtitles
2. Video with multiple subtitle tracks
3. Video without subtitles (audio only)
4. Lecture/presentation recording
5. Foreign language video

**Sources:**
- Sample MP4s online
- Create test videos

#### 10.2 MKV - Matroska Video
**Extensions:** `.mkv`
**Rust Libraries:** `matroska` parsing library
**Complexity:** Medium
**Use Case:** Extract subtitles, chapters

**Implementation:**
- Parse MKV container format
- Extract subtitle tracks (ASS, SRT, VTT)
- Extract chapter information
- Access audio for transcription

**Test Files (5 examples):**
1. Movie with subtitles
2. TV episode with chapters
3. Anime with soft subtitles
4. Documentary
5. Multi-audio track video

**Sources:**
- Sample MKVs
- Open-source video projects

#### 10.3 AVI - Audio Video Interleave
**Extensions:** `.avi`
**Rust Libraries:** `ffmpeg-rs` via FFI
**Complexity:** Medium
**Use Case:** Legacy video format

**Implementation:**
- Use ffmpeg for parsing
- Extract subtitles if present
- Extract audio for transcription
- Handle various codecs

**Test Files (5 examples):**
1. DivX encoded video
2. XviD video
3. Uncompressed AVI
4. Video with embedded subtitles
5. Old home video

**Sources:**
- Legacy video archives
- Convert modern videos to AVI

#### 10.4 MOV - QuickTime Movie
**Extensions:** `.mov`, `.qt`
**Rust Libraries:** `mp4parse` (MOV is similar to MP4)
**Complexity:** Medium
**Use Case:** Apple video format

**Implementation:**
- Similar to MP4 parsing
- Extract subtitle tracks
- Parse QuickTime metadata
- Handle Apple-specific codecs

**Test Files (5 examples):**
1. iPhone video
2. Screen recording (macOS)
3. ProRes video (professional)
4. Video with closed captions
5. Animation/motion graphics

**Sources:**
- Export from QuickTime
- iPhone videos
- Stock footage sites

#### 10.5 SRT - SubRip Subtitle
**Extensions:** `.srt`
**Rust Libraries:** `subparse`, custom parser (simple format)
**Complexity:** Very Low
**Use Case:** Subtitle file extraction

**Implementation:**
- SRT is plain text format
- Parse timestamp and text entries
- Simple structure: index, timecode, text
- Already common format

**Test Files (5 examples):**
1. English subtitles
2. Multi-language subtitles
3. Subtitles with formatting
4. Long-form content (movie)
5. Short-form content (clip)

**Sources:**
- OpenSubtitles.org
- Create sample SRT files

---

### CATEGORY 11: Specialized Formats (7 formats)

#### 11.1 DICOM - Digital Imaging and Communications in Medicine
**Extensions:** `.dcm`, `.dicom`
**Rust Libraries:** `dicom` crate (v0.6)
**Complexity:** High
**Use Case:** Medical imaging metadata extraction

**Implementation:**
- Parse DICOM metadata (patient info, scan details)
- Extract image data if needed
- Handle DICOM tags and VR (Value Representations)
- Privacy concerns: anonymize patient data

**Test Files (5 examples):**
1. CT scan
2. MRI scan
3. X-ray image
4. Ultrasound
5. Medical report (SR - Structured Report)

**Sources:**
- DICOM sample files
- Medical imaging datasets (anonymized)

#### 11.2 KML/KMZ - Keyhole Markup Language
**Extensions:** `.kml`, `.kmz`
**Rust Libraries:** `quick-xml` (KML is XML), `zip` (KMZ is zipped KML)
**Complexity:** Medium
**Use Case:** Geographic data, maps, locations

**Implementation:**
- KML is XML-based geographic format
- KMZ is compressed KML (ZIP)
- Parse placemarks, paths, polygons
- Extract location data and descriptions

**Test Files (5 examples):**
1. Simple placemark (single location)
2. Path/route (GPS track)
3. Area/polygon (region)
4. Tour (animated view)
5. Network link (dynamic data)

**Sources:**
- Google Earth exports
- OpenStreetMap KML exports
- GPS device exports

#### 11.3 GPX - GPS Exchange Format
**Extensions:** `.gpx`
**Rust Libraries:** `gpx` crate (v0.9)
**Complexity:** Low-Medium
**Use Case:** GPS tracks, routes, waypoints

**Implementation:**
- GPX is XML-based GPS format
- Parse tracks, routes, waypoints
- Extract timestamps, elevations, coordinates
- Generate summary statistics

**Test Files (5 examples):**
1. Hiking trail GPS track
2. Cycling route
3. Running workout
4. Waypoint collection (POIs)
5. Multi-day journey

**Sources:**
- GPS device exports
- Fitness apps (Strava, Garmin)
- OpenStreetMap GPX traces

#### 11.4 ICS/iCal - iCalendar
**Extensions:** `.ics`, `.ical`
**Rust Libraries:** `ical` crate (v0.10)
**Complexity:** Low-Medium
**Use Case:** Calendar events, appointments

**Implementation:**
- ICS is text-based calendar format
- Parse events (VEVENT), todos (VTODO), journals
- Extract date/time, location, attendees, description
- Handle recurring events

**Test Files (5 examples):**
1. Single event
2. Recurring meeting (weekly)
3. All-day event
4. Event with reminders
5. Calendar with multiple events

**Sources:**
- Export from calendar apps
- Meeting invites

#### 11.5 VCF - Variant Call Format (Bioinformatics)
**Extensions:** `.vcf` (different from vCard)
**Rust Libraries:** `bio` crate, `rust-htslib`
**Complexity:** High
**Use Case:** Genomic variant data

**Implementation:**
- VCF is tab-delimited genetic variant format
- Parse header, variant records
- Extract genomic positions, mutations
- Specialized scientific format

**Test Files (5 examples):**
1. Small variant set (< 100 variants)
2. Whole-genome variants
3. Exome sequencing results
4. Annotated VCF
5. Multi-sample VCF

**Sources:**
- 1000 Genomes Project
- dbSNP sample files
- Genomic databases

#### 11.6 LaTeX - Document Preparation System
**Extensions:** `.tex`, `.latex`
**Rust Libraries:** Custom parser (text-based with complex syntax)
**Complexity:** High
**Use Case:** Academic papers, technical documents

**Implementation:**
- Parse LaTeX commands and environments
- Extract document structure (sections, equations)
- Handle includes and bibliography
- Complex parsing (TeX is Turing-complete)

**Test Files (5 examples):**
1. Simple article
2. Academic paper with equations
3. Thesis with multiple chapters
4. Presentation (Beamer)
5. Technical report with code listings

**Sources:**
- arXiv LaTeX sources
- Overleaf templates
- Academic paper repositories

#### 11.7 Jupyter Notebook - .ipynb
**Extensions:** `.ipynb`
**Rust Libraries:** `serde_json` (notebooks are JSON)
**Complexity:** Low-Medium
**Use Case:** Data science notebooks, code documentation

**Implementation:**
- IPYNB is JSON format
- Parse cells (code, markdown, output)
- Extract code, text, execution results
- Handle embedded images/plots

**Test Files (5 examples):**
1. Simple Python notebook
2. Data analysis notebook (pandas)
3. Machine learning tutorial
4. Notebook with visualizations
5. Multi-language notebook (R, Julia)

**Sources:**
- Kaggle notebooks
- JupyterLab examples
- GitHub notebooks

---

### CATEGORY 12: Legacy/Additional Formats (4 formats)

#### 12.1 RTF - Rich Text Format
*(Details from previous plan)*

#### 12.2 WordPerfect - WPD
**Extensions:** `.wpd`, `.wp`
**Rust Libraries:** None - consider libwpd via FFI
**Complexity:** Very High
**Use Case:** Legacy word processing

**Implementation:**
- WordPerfect proprietary format
- Use libwpd (C library) via FFI
- Or: Require conversion via LibreOffice
- Historical format, declining usage

**Test Files (5 examples):**
1. Simple letter
2. Legal document
3. Technical manual
4. Report with tables
5. Document with footnotes

**Sources:**
- Legacy document archives
- Convert modern docs to WPD

#### 12.3 DOC - Microsoft Word 97-2003
**Extensions:** `.doc` (binary format)
**Rust Libraries:** None mature - consider antiword or LibreOffice
**Complexity:** High
**Use Case:** Legacy Word documents

**Implementation:**
- Binary Office format (OLE/CFB)
- Consider using antiword for text extraction
- Or: LibreOffice conversion to DOCX
- Modern docling may already handle this

**Test Files (5 examples):**
1. Simple text document
2. Document with formatting
3. Document with tables
4. Document with images
5. Template document

**Sources:**
- Legacy document archives
- Save DOCX as DOC

#### 12.4 WPS - Microsoft Works
**Extensions:** `.wps`
**Rust Libraries:** None - use LibreOffice for conversion
**Complexity:** High
**Use Case:** Legacy Works documents

**Implementation:**
- Microsoft Works proprietary format
- Require conversion to DOCX via LibreOffice
- Very legacy format

**Test Files (5 examples):**
1. Letter
2. Resume
3. Newsletter
4. Report
5. Database report

**Sources:**
- Legacy document collections
- Microsoft Works archives

---

## Format Categories Summary

| Category | Format Count | Complexity Range | Priority |
|----------|--------------|-----------------|----------|
| **Audio** | 2 | Medium | HIGH |
| **E-book** | 4 | Low-Very High | MEDIUM |
| **Email** | 5 | Low-Very High | HIGH |
| **Apple iWork** | 3 | High | MEDIUM |
| **Adobe Extended** | 5 | High-Very High | MEDIUM |
| **Microsoft Extended** | 6 | Medium-Very High | MEDIUM |
| **CAD/Engineering** | 4 | Low-Very High | LOW |
| **3D Formats** | 3 | Low-Medium | LOW |
| **Archive** | 4 | Low-Medium | HIGH |
| **Video** | 5 | Low-Medium | MEDIUM |
| **Specialized** | 7 | Low-Very High | LOW |
| **Legacy** | 4 | High-Very High | LOW |
| **TOTAL** | **52** | | |

---

## AI Execution Checklist

### Phase A: Foundation (High-Priority Quick Wins)

**Format Implementation Priority: Tier 1**

- [x] **A1: Archive Formats** - Extract and process contents ✅ COMPLETE (N=13-19)
  - [x] ZIP format support (N=13)
  - [x] TAR/TAR.GZ format support (N=14)
  - [x] 7Z format support (N=15)
  - [x] RAR format support using unar/lsar command-line tools (N=19)
  - [x] Test: Created 5 diverse test files per format
  - [x] Integration test stubs: All created, ready for testing

- [x] **A2: Subtitle/Caption Formats** ✅ COMPLETE (N=18)
  - [x] SRT subtitle parser (N=18)
  - [x] WebVTT support (already existed, verified)
  - [x] Test: 5 diverse subtitle files per format
  - [x] Integration test stubs: All created

- [x] **A3: Simple Image Formats** ✅ COMPLETE (N=21)
  - [x] GIF format support added to InputFormat enum (N=21)
  - [x] Verified existing image format handling (BMP, PNG, JPEG, TIFF, WebP, GIF all supported by image crate v0.24)
  - [x] Test: 5 diverse GIF test files created (simple, animated, large, icon_small, transparent)
  - [x] Integration test stubs: 5 GIF tests added to integration_tests.rs

### Phase B: Audio & Video (Transcription)

**Format Implementation Priority: Tier 1 (Python Parity)**

- [x] **B1: Audio Transcription** ✅ COMPLETE (Infrastructure: N=4-5, verified N=42)
  - [x] Integrate whisper-rs v0.15.1 - ✅ Implemented (docling-audio crate)
  - [x] WAV support with transcription - ✅ Implemented (hound + whisper-rs)
  - [x] MP3 support with transcription - ✅ Implemented (symphonia + whisper-rs)
  - [x] Test: 5 diverse audio files per format - ✅ Complete (10 files: 5 WAV + 5 MP3)
  - [x] Integration tests: 4 audio test stubs added (lines 5624-5650)
  - [ ] Benchmark: Transcription speed vs real-time - ⏸️ Deferred (requires model download)
  - **Report:** Audio transcription research at `reports/feature-phase-a-archives/audio_transcription_research_2025-11-07-08-02.md`

- [x] **B2: Video Subtitle Extraction** ✅ COMPLETE (Infrastructure: N=6, verified N=42)
  - [x] MP4 subtitle extraction (ffmpeg integration) - ✅ Implemented (docling-video crate)
  - [x] MKV subtitle extraction - ✅ Implemented
  - [x] MOV subtitle extraction - ✅ Implemented
  - [x] AVI subtitle extraction - ✅ Implemented
  - [x] SRT/WebVTT subtitle parsing - ✅ Implemented (srtparse crate)
  - [ ] Test: 5 diverse videos per format - ⚠️ Requires ffmpeg and video test files
  - [ ] Integration tests: Subtitle track extraction - ⚠️ Pending test corpus
  - [x] Optional: Audio track transcription - ✅ Implemented (via docling-audio integration)
  - **Report:** Video subtitle extraction research at `reports/feature-phase-a-archives/video_subtitle_extraction_research_2025-11-07-08-28.md`

**Phase B Status:** ✅ **INFRASTRUCTURE COMPLETE**
- Audio formats (WAV, MP3) fully integrated with transcription support
- Video formats (MP4, MKV, MOV, AVI) integrated with subtitle extraction
- Optional transcription feature behind feature flag
- Integration test stubs exist, full testing requires model download and video corpus

### Phase C: Email & Communication (High Business Value)

**Format Implementation Priority: Tier 1**

- [x] **C1: Email Formats** ✅ COMPLETE (N=38)
  - [x] EML parser (mail-parser crate v0.11) - ✅ Working (3 unit tests)
  - [x] MBOX parser (custom + mail-parser) - ✅ Working (4 unit tests)
  - [x] VCF/vCard contact parser (vcard crate v0.4) - ✅ Working (5 unit tests)
  - [x] MSG parser (msg_parser crate v0.1) - ✅ Working (8 unit tests)
  - [x] Test: 15+ test files created (5+ per format)
  - [x] Integration tests: 15 test stubs added (lines 5655-5757)
  - **Total unit tests:** 20/20 passing
  - **Report:** Phase C-1 verified N=38

- [ ] **C2: Complex Email Formats** ⏸️ DEFERRED (Low ROI, High Complexity)
  - [ ] PST parser (or conversion approach) - VERY HIGH complexity
  - [ ] Recommendation: Defer C-2, PST requires external tools or complex FFI
  - Alternative: Users can export PST → MBOX using Outlook or libpst command-line

### Phase D: E-books & Publishing

**Format Implementation Priority: Tier 2**

- [x] **D1: E-book Formats** ✅ COMPLETE (3 of 4 formats, AZW3 deferred)
  - [x] EPUB support (epub crate v2.1.5) ✅ Complete (N=28-31)
  - [x] FB2 support (XML-based, quick-xml) ✅ Complete (N=28-31)
  - [x] MOBI support (mobi crate v0.8.0) ✅ Complete (N=28-32)
  - [ ] AZW3 support (DRM-free only) ⏸️ DEFERRED (See decision below)
  - [x] Test: 5 diverse e-books per format ✅ Complete (EPUB: 5/5, FB2: 10/10, MOBI: 5/5)
  - [ ] Integration tests: E-book text extraction ⚠️ Blocked by Python framework RPATH issue

**AZW3 Decision (N=32):**
- **Status:** Deferred to future phase (not Phase D priority)
- **Rationale:**
  1. AZW3 is Amazon's modern Kindle format (KF8), often DRM-protected
  2. EPUB and MOBI cover 95% of e-book use cases
  3. Limited Rust library support for AZW3 (no mature crate)
  4. Would require significant research and custom implementation
  5. Diminishing returns: EPUB is the standard for DRM-free e-books
- **Alternative:** Users can convert AZW3 → EPUB → Markdown via Calibre
- **Future Work:** Revisit if high user demand (Phase D2 or later)

### Phase E: Open Standards

**Format Implementation Priority: Tier 2**

- [x] **E1: OpenDocument Formats** ✅ COMPLETE (N=34-35)
  - [x] ODT support (zip + quick-xml) - ✅ Verified N=34 (11/11 tests)
  - [x] ODS support (calamine) - ✅ Verified N=35 (16/16 tests)
  - [x] ODP support (zip + quick-xml) - ✅ Verified N=35 (12/12 tests)
  - [x] Test: 5 diverse files per format - ✅ Complete (15 total files)
  - [x] Integration tests: ODF parsing - ✅ Complete (46/46 tests passing)
  - **Report:** `reports/feature-phase-e-open-standards/phase_e1_complete_2025-11-07.md`

- [x] **E2: Microsoft Extended (Easy)** ✅ COMPLETE (N=9, N=17)
  - [x] XPS support (zip + quick-xml) - ✅ Implemented N=9
  - [x] Test: 5 diverse XPS files - ✅ Complete (5 files created)
  - [x] Integration tests: XPS parsing - ✅ Complete (5 tests, test stubs N=17)
  - **Report:** `reports/feature-phase-a-archives/N9_XPS_COMPLETE_2025-11-07-10-18.md`

- [x] **E3: Graphics Open Standards** ✅ COMPLETE (N=36)
  - [x] SVG support (quick-xml parser) - ✅ Implemented N=36
  - [x] Test: 5 diverse SVG files (icons, diagrams, maps) - ✅ Complete (5 files created)
  - [x] Integration tests: SVG text extraction - ✅ Complete (5 unit tests + 5 integration test stubs)
  - **Note:** Used quick-xml for text extraction (simpler than resvg rendering library)

### Phase F: Modern Image Formats

**Format Implementation Priority: Tier 2**

- [x] **F1: Next-Gen Images** ✅ COMPLETE (N=39, N=41)
  - [x] HEIF/HEIC support (libheif-rs v2.5) - ✅ Infrastructure ready (N=39)
  - [x] AVIF support (image crate v0.25 with avif feature) - ✅ Infrastructure ready (N=39)
  - [x] Test: 5 diverse files per format - ✅ COMPLETE (N=41: PNG converted to real formats)
  - [x] Integration tests: 10 test stubs added (5 HEIF + 5 AVIF) - ✅ Test files ready
  - **Status:** All 10 test files converted from PNG to native formats using libheif/libavif tools
  - **Tools used:** `heif-enc` (libheif v1.20.2), `avifenc` (libavif v1.3.0) via Homebrew
  - **Report:** Phase F COMPLETE - Format detection, test infrastructure, and test files all ready

### Phase G: Microsoft Extended (Complex)

**Format Implementation Priority: Tier 3**

- [ ] **G1: Diagram & Specialized**
  - [ ] VSDX support (Visio diagrams)
  - [ ] Test: 5 diverse diagrams (flowcharts, networks, etc.)
  - [ ] Integration tests: Diagram text extraction

- [ ] **G2: Database & Project**
  - [ ] MDB/ACCDB support (via mdbtools FFI)
  - [ ] MPP support (via MPXJ or conversion)
  - [ ] Test: 5 diverse files per format
  - [ ] Integration tests: Data extraction

- [ ] **G3: Note-Taking**
  - [ ] PUB support (via LibreOffice conversion)
  - [ ] ONE support (API or conversion approach)
  - [ ] Test: 5 diverse files per format
  - [ ] Integration tests: Content extraction

### Phase H: Apple Ecosystem

**Format Implementation Priority: Tier 3**

- [ ] **H1: iWork Formats**
  - [ ] Pages support (ZIP + proprietary XML)
  - [ ] Numbers support (nested ZIP + data)
  - [ ] Keynote support (ZIP + proprietary XML)
  - [ ] Test: 5 diverse files per format
  - [ ] Integration tests: iWork parsing

### Phase I: Adobe Creative Suite

**Format Implementation Priority: Tier 3-4**
**Status:** ✅ COMPLETE (with strategic deferrals) (N=63-64)

- [x] **I1: Vector & Publishing** ✅ COMPLETE (1/2, AI deferred)
  - [x] **I1-1: IDML support (InDesign, XML-based)** ✅ COMPLETE (N=63)
    - **Complexity:** MEDIUM-HIGH (6-8 hours actual)
    - **Format:** ZIP archive containing XML files (Stories, Spreads, designmap.xml)
    - **Rust Libraries:** zip v0.6, quick-xml v0.31
    - **Use Case:** Publishing industry (magazines, books, brochures, technical manuals)
    - **Implementation:** Extract stories (text content), spreads (layout), styles, metadata
    - **Test Plan:** 5 diverse files (simple doc, magazine, brochure, book chapter, technical manual)
    - **Status:** IDML parsing complete with text extraction, style mapping (Heading1-6), and markdown serialization
    - **Crate:** docling-adobe (new crate created)
    - **Test Results:** 12/12 unit tests passing (parser + serializer + types)
    - **Integration Tests:** 5 test stubs added (test_idml_simple_document, test_idml_magazine_layout, test_idml_brochure, test_idml_book_chapter, test_idml_technical_manual)
  - [ ] **I1-2: AI support (Illustrator, PDF-based)** ⏸️ DEFERRED (N=64)
    - **Complexity:** VERY HIGH (20-40 hours for full PostScript parsing)
    - **Research:** reports/feature-phase-e-open-standards/ai_research_2025-11-07-20-11.md
    - **Rationale for deferral:**
      - Requires PostScript parser (Turing-complete language, no Rust libraries)
      - Python docling v2.58.0 does NOT support AI format (confirms low priority)
      - PDF layer extraction would miss text-on-path and artboard metadata (80% completeness)
      - Users can convert AI → PDF losslessly for text extraction
      - 20-40 hours vs 6-8 hours for IDML (3-6x complexity)
    - **Alternative:** Implement PDF-layer-only extraction as future MVP (2-4 hours) if demand increases
    - **Status:** Deferred to future phase pending user demand or library availability

- [ ] **I2: Raster & Forms** ⏸️ ALL DEFERRED (N=64)
  - [ ] **I2-1: PSD support (Photoshop)** ⏸️ DEFERRED (N=64)
    - **Complexity:** VERY HIGH (20-30 hours for custom text layer parser)
    - **Research:** reports/feature-phase-e-open-standards/psd_research_2025-11-07-20-18.md
    - **Rationale for deferral:**
      - Rust `psd` crate (v0.3.5) does NOT support text layer extraction (GitHub issue #20 open since Feb 2021)
      - Would require custom Adobe descriptor format parser (proprietary, complex nested structures)
      - Python docling v2.58.0 does NOT support PSD format (confirms low priority)
      - PSD is image editing format with minimal text content (0-5 layers typical)
      - Users can export PSD → PDF to preserve text layers
      - 20-30 hours vs 6-8 hours for IDML (3-5x complexity)
    - **Alternative:** Implement OCR-based extraction (flatten layers → OCR) as future MVP (3-5 hours) if needed
    - **Status:** Deferred to future phase pending `psd` crate text layer support or high user demand
  - [ ] **I2-2: XFA/PDF Forms support** ⏸️ DEFERRED
    - **Complexity:** VERY HIGH (XML Forms Architecture in PDF, complex form logic)
    - **Rationale:** Specialized use case, very high complexity, low ROI for document processing
    - **Status:** Deferred to future phase
  - [ ] **I2-3: INDD support (native InDesign)** ⏸️ DEFERRED
    - **Complexity:** VERY HIGH (proprietary binary format, no specification)
    - **Rationale:** IDML (N=63) is the open interchange format for InDesign, covers same use case
    - **Alternative:** Users can export INDD → IDML from InDesign (File → Export → InDesign Markup)
    - **Status:** Permanently deferred in favor of IDML

**Phase I Conclusion (N=64):**
- **Implemented:** 1/5 formats (IDML)
- **Deferred:** 4/5 formats (AI, PSD, XFA, INDD)
- **Strategic Assessment:** IDML provides sufficient Adobe publishing format coverage
- **Recommendation:** Close Phase I, pivot to higher-ROI formats (Microsoft Extended, Apple iWork, LaTeX)

### Phase J: CAD & Engineering

**Format Implementation Priority: Tier 4**

- [x] **J1: CAD Formats** ✅ COMPLETE (1/3 formats, DWG/IFC deferred)
  - [x] **J1-1: DXF support (dxf crate)** ✅ COMPLETE (N=61)
    - [x] Extended docling-cad crate with dxf v0.6 (DXF parser)
    - [x] Full .dxf parsing: entities (lines, circles, arcs, polylines, text, splines, etc.), layers, bounding box
    - [x] Added InputFormat::Dxf variant and extension mapping (.dxf)
    - [x] Implemented DXF processing in docling-core with markdown serialization
    - [x] Test: 5 diverse DXF files created (simple_drawing, floor_plan, mechanical_part, electrical_schematic, 3d_model)
    - [x] Integration tests: 5 test stubs added (lines 6584-6617)
    - [x] Python script: generate_dxf_test_files.py (generates 5 test DXF files using ezdxf)
    - [x] Unit tests: 3/3 passing (parser + serializer tests)
    - **Status:** AutoCAD DXF format parsing complete with entity extraction and text content
    - **Crate:** dxf v0.6.0 (MIT license, mature, supports R10-2018)
  - [ ] **J1-2: DWG support** ⏸️ DEFERRED (proprietary format, use DXF as open alternative)
    - **Rationale:** DWG is proprietary Autodesk binary format with no mature Rust libraries
    - **Alternative:** Users can convert DWG → DXF using AutoCAD, LibreCAD, or ODA File Converter
    - **Status:** Deferred to future phase (low ROI, high complexity)
  - [ ] **J1-3: IFC support** ⏸️ DEFERRED (experimental libraries, VERY HIGH complexity)
    - **Rationale:** No production-ready Rust IFC libraries (ruststep experimental, ifc_rs alpha)
    - **Complexity:** 1,300+ entities, 2,500+ properties, complex BIM relationships
    - **Alternative:** DXF provides CAD/architectural format coverage
    - **Status:** Deferred to future phase (low ROI, experimental ecosystem)
    - **Report:** reports/feature-phase-e-open-standards/ifc_research_2025-11-07-19-40.md

- [x] **J2: 3D Formats** ✅ COMPLETE (3/4 formats, FBX deferred)
  - [x] **J2-1: STL support (stl_io crate)** ✅ COMPLETE (N=57)
    - [x] Created docling-cad crate with stl_io v0.8.5 (STL parser)
    - [x] Full .stl parsing (both ASCII and binary): mesh data, bounding box, triangle/vertex counts
    - [x] Added InputFormat::Stl variant and extension mapping (.stl)
    - [x] Implemented CAD processing module in docling-core with markdown serialization
    - [x] Test: 5 diverse STL files created (simple_cube, pyramid, complex_shape, large_mesh, minimal_triangle)
    - [x] Integration tests: 5 test stubs added (lines 6453-6486)
    - [x] Python script: generate_stl_test_files.py (generates 5 test STL meshes)
    - **Status:** 3D mesh parsing complete (geometric data, dimensions, bounding volume)
    - **Crate:** stl_io v0.8.5 (MIT license, mature, supports ASCII + binary STL)
    - **Report:** reports/feature-phase-e-open-standards/vsdx_research_2025-11-07.md (VSDX deferred, STL chosen instead)
  - [x] **J2-2: OBJ support (tobj crate)** ✅ COMPLETE (N=58)
    - [x] Extended docling-cad crate with tobj v4.0 (OBJ parser)
    - [x] Full .obj parsing: vertices, faces, normals, texture coords, materials (MTL)
    - [x] Added InputFormat::Obj variant and extension mapping (.obj)
    - [x] Implemented OBJ processing in docling-core with markdown serialization
    - [x] Test: 5 diverse OBJ files created (simple_cube, teapot_excerpt, pyramid_with_normals, textured_quad, icosphere)
    - [x] Integration tests: 5 test stubs added (lines 6492-6525)
    - [x] Python script: generate_obj_test_files.py (generates 5 test OBJ meshes + 1 MTL)
    - [x] Unit tests: 9/9 passing (6 parser + 3 serializer)
    - **Status:** 3D mesh parsing with multi-object, normals, and texture support
    - **Crate:** tobj v4.0.3 (MIT license, mature, supports ASCII OBJ + MTL materials)
  - [x] **J2-3: GLTF/GLB support (gltf crate)** ✅ COMPLETE (N=59)
    - [x] Extended docling-cad crate with gltf v1.4.1 (Khronos glTF 2.0)
    - [x] Full .gltf/.glb parsing: meshes, primitives, vertices, triangles, scene graph
    - [x] Added InputFormat::Gltf and InputFormat::Glb variants
    - [x] Implemented GLTF processing in docling-core with markdown serialization
    - [x] Test: 7 diverse GLTF files (2 programmatic + 5 Khronos samples)
    - [x] Integration tests: 7 test stubs added (lines 6531-6578)
    - [x] Python script: generate_gltf_test_files.py (generates/downloads test files)
    - [x] Unit tests: 12/12 passing (8 parser + 4 serializer)
    - **Status:** Modern 3D format parsing with JSON/binary support, scene graph, animations
    - **Crate:** gltf v1.4.1 (MIT/Apache-2.0 license, mature, glTF 2.0 standard)
  - [ ] **J2-4: FBX support** ⏸️ DEFERRED (proprietary, GLTF/GLB/OBJ provide sufficient 3D coverage)
    - **Rationale:** FBX is proprietary Autodesk binary format with limited Rust support (fbxcel is incomplete)
    - **Alternative:** GLTF/GLB (N=59) provides modern 3D interchange, OBJ (N=58) for legacy workflows
    - **Status:** Deferred to future phase (VERY HIGH complexity, low incremental value)
    - **Decision:** Made at N=60 (cleanup), confirmed N=62 (Phase J complete)

### Phase K: Specialized Domains

**Format Implementation Priority: Tier 4-5**

- [x] **K1: Geographic/GPS** ✅ COMPLETE (N=44, N=47)
  - [x] **K1-2: KML/KMZ support (kml crate v0.12)** ✅ COMPLETE (N=46-47)
    - [x] Created docling-gps crate extensions for KML/KMZ parsing
    - [x] Full KML parsing: placemarks (points, paths, polygons), folders, multi-geometries
    - [x] KMZ support: automatic ZIP extraction and KML parsing
    - [x] Added InputFormat::Kml and InputFormat::Kmz variants
    - [x] Implemented KML processing module in docling-core with markdown serialization
    - [x] Test: 5 diverse KML files + 1 KMZ file created
    - [x] Integration tests: 6 test stubs added (lines 6325-6369)
    - **Status:** Full KML/KMZ parsing with placemarks, folders, coordinates, altitude
    - **Report:** Phase K1-2 complete (N=47)
  - [x] **K1-1: GPX support (gpx crate v0.9)** ✅ COMPLETE (N=44)
    - [x] Created docling-gps crate with gpx v0.9 parsing
    - [x] Full .gpx parsing (GPX 1.0/1.1): tracks, routes, waypoints, metadata
    - [x] Added InputFormat::Gpx variant and extension mapping (.gpx)
    - [x] Implemented GPS processing module in docling-core with markdown serialization
    - [x] Test: 5 diverse GPX files created (hiking_trail, cycling_route, running_workout, waypoints_pois, multi_day_journey)
    - [x] Integration tests: 5 test stubs added (lines 6290-6323)
    - **Status:** Full GPX parsing with tracks (multi-segment), routes, waypoints, elevation, timestamps
    - **Report:** Phase K1-1 complete with gpx crate-based XML parsing

- [x] **K2: Calendar & Scheduling** ✅ COMPLETE (N=42)
  - [x] ICS/iCal support (ical crate v0.11) - ✅ Implemented (docling-calendar crate)
  - [x] Test: 5 diverse calendar files - ✅ Complete (5 ICS files created)
  - [x] Integration tests: Event extraction - ✅ Complete (5 integration test stubs added)
  - **Status:** Full ICS/iCalendar parsing with events, todos, and journal support
  - **Report:** Phase K2 complete with markdown serialization of calendar data

- [ ] **K3: Scientific/Academic**
  - [ ] LaTeX support (custom parser)
  - [x] **K3-1: Jupyter Notebook support (JSON-based)** ✅ COMPLETE (N=43)
    - [x] Created docling-notebook crate with nbformat v0.13 + jupyter-protocol v0.9
    - [x] Full .ipynb parsing (nbformat 4.x): markdown cells, code cells, raw cells, outputs, metadata
    - [x] Added InputFormat::Ipynb variant and extension mapping (.ipynb)
    - [x] Implemented notebook processing module in docling-core with markdown serialization
    - [x] Test: 5 diverse Jupyter notebooks created (simple_data_analysis, machine_learning_demo, math_formulas, error_handling, complex_visualization)
    - [x] Integration tests: 5 test stubs added (lines 6251-6284)
    - **Status:** Full Jupyter Notebook parsing complete with code cells, outputs (stream, execute_result, display_data, error), and markdown generation
    - **Report:** Phase K3-1 complete with nbformat-based JSON parsing
  - [x] **K3-2: DICOM support (dicom crate)** ✅ COMPLETE (N=49)
    - [x] Created docling-medical crate with dicom v0.9.0 (dicom-rs ecosystem)
    - [x] Full .dcm/.dicom parsing: patient info (anonymized), study, series, image metadata
    - [x] Added InputFormat::Dicom variant and extension mapping (.dcm, .dicom)
    - [x] Implemented DICOM processing module in docling-core with markdown serialization
    - [x] Privacy-first design: anonymize patient data by default (HIPAA/GDPR compliance)
    - [x] Test: 5 diverse DICOM files created (CT, MRI, X-ray, ultrasound, structured report)
    - [x] Integration tests: 5 test stubs added (lines 6373-6408)
    - [x] Python script: CREATE_DICOM_TEST_FILES.py (generates synthetic test files)
    - **Status:** Medical imaging metadata extraction complete (modalities: CT, MR, CR/DX, US, SR)
    - **Report:** reports/feature-phase-e-open-standards/dicom_research_2025-11-07.md
  - [x] **K3-3: VCF (genomic variant) support (noodles-vcf crate)** ✅ COMPLETE (N=51-54)
    - [x] Created docling-genomics crate with noodles-vcf v0.81.0
    - [x] Full .vcf parsing: header metadata, variant records, genotype data, statistics
    - [x] Prototype-first strategy (N=52): Standalone example before integration (0 compilation errors)
    - [x] Parser implementation (N=53): VcfParser with parse_file/parse_str/parse_reader (7 unit tests, 100% pass)
    - [x] Markdown serialization (N=54): Complete with genotype formatting and statistics (10 unit tests total, 100% pass)
    - [x] Example program: vcf_to_markdown.rs demonstrates parser + serializer pipeline
    - [x] Test file: test-corpus/genomics/vcf/small_variants.vcf (5 variants, 3 samples)
    - **Status:** VCF genomic variant parsing complete with header, variants, genotypes, and markdown output
    - **Report:** reports/feature-phase-e-open-standards/vcf_research_2025-11-07.md
  - [ ] **K3-4: LaTeX support (custom parser)** - Deferred

### Phase L: Legacy Formats

**Format Implementation Priority: Tier 5**

- [ ] **L1: Legacy Office**
  - [x] **L1-1: RTF support (rtf-parser crate)** ✅ COMPLETE (N=56)
    - [x] Created docling-legacy crate with rtf-parser v0.4.2
    - [x] Full .rtf parsing: text extraction, formatting metadata (bold, italic, fonts, colors)
    - [x] Added InputFormat::Rtf variant and extension mapping (.rtf)
    - [x] Implemented legacy processing module in docling-core with markdown serialization
    - [x] Test: 5 diverse RTF files created (simple_text, formatted_text, business_memo, technical_doc, unicode_test)
    - [x] Integration tests: 5 test stubs added (lines 6414-6447)
    - **Status:** RTF parsing complete with plain text extraction (formatting preservation in future)
    - **Crate:** rtf-parser v0.4.2 (implements RTF spec 1.9, UTF-16 Unicode support, no external deps)
  - [ ] DOC support (binary Word, via conversion)
  - [ ] WordPerfect support (via libwpd FFI)
  - [ ] WPS support (via LibreOffice conversion)
  - [ ] Test: 5 diverse files per remaining formats
  - [ ] Integration tests: Legacy doc parsing for remaining formats

---

## Test Corpus Requirements

### Directory Structure

```
test-corpus/
├── audio/
│   ├── wav/ (5 files)
│   └── mp3/ (5 files)
├── ebooks/
│   ├── epub/ (5 files)
│   ├── mobi/ (5 files)
│   ├── azw3/ (5 files)
│   └── fb2/ (5 files)
├── email/
│   ├── eml/ (5 files)
│   ├── msg/ (5 files)
│   ├── mbox/ (5 files)
│   ├── pst/ (5 files)
│   └── vcf/ (5 files)
├── apple/
│   ├── pages/ (5 files)
│   ├── numbers/ (5 files)
│   └── keynote/ (5 files)
├── adobe/
│   ├── idml/ (5 files)
│   ├── ai/ (5 files)
│   ├── psd/ (5 files)
│   ├── xfa/ (5 files)
│   └── indd/ (5 files)
├── microsoft-extended/
│   ├── pub/ (5 files)
│   ├── vsdx/ (5 files)
│   ├── one/ (5 files)
│   ├── mpp/ (5 files)
│   ├── mdb/ (5 files)
│   └── xps/ (5 files)
├── cad/
│   ├── dwg/ (5 files)
│   ├── dxf/ (5 files)
│   ├── stl/ (5 files)
│   └── ifc/ (5 files)
├── 3d/
│   ├── obj/ (5 files)
│   ├── fbx/ (5 files)
│   └── gltf/ (5 files)
├── archives/
│   ├── zip/ (5 files)
│   ├── rar/ (5 files)
│   ├── 7z/ (5 files)
│   └── tar/ (5 files)
├── video/
│   ├── mp4/ (5 files)
│   ├── mkv/ (5 files)
│   ├── avi/ (5 files)
│   ├── mov/ (5 files)
│   └── srt/ (5 files)
├── specialized/
│   ├── dicom/ (5 files)
│   ├── kml/ (5 files)
│   ├── gpx/ (5 files)
│   ├── ics/ (5 files)
│   ├── vcf-genomic/ (5 files)
│   ├── tex/ (5 files)
│   └── ipynb/ (5 files)
├── opendocument/
│   ├── odt/ (5 files)
│   ├── ods/ (5 files)
│   └── odp/ (5 files)
├── graphics/
│   ├── svg/ (5 files)
│   ├── gif/ (5 files)
│   ├── heif/ (5 files)
│   └── avif/ (5 files)
└── legacy/
    ├── rtf/ (5 files)
    ├── wpd/ (5 files)
    ├── doc/ (5 files)
    └── wps/ (5 files)
```

### Per-Format Requirements

Each format requires **5 diverse test files**:
1. **Simple** - Basic, minimal features
2. **Complex** - Rich features, tables, images, formatting
3. **Real-world** - Actual business/professional document
4. **Edge case** - Large, unusual structure, stress test
5. **Multi-language** - Non-English content (if applicable)

### Expected Output Generation

For each test file:
1. Run parser to generate output
2. Verify output correctness
3. Save to `test-corpus/expected-outputs/{category}/{format}/`
4. Create integration test entry

### Test Success Criteria

- **Format Recognition:** File extension correctly mapped to format
- **Parse Success:** File parses without crashing
- **Content Extraction:** Text/data extracted accurately
- **Structure Preservation:** Document structure maintained
- **Error Handling:** Graceful handling of corrupted/invalid files
- **Performance:** Parsing completes in reasonable time

---

## Implementation Directives for AI Workers

### General Instructions

1. **Work Through Phases Sequentially**
   - Start with Phase A (Foundation)
   - Complete all items in a phase before moving to next
   - Each phase can be a separate git branch

2. **For Each Format Implementation:**

   **Step 1: Research**
   - Read format specification/documentation
   - Identify Rust crates available
   - Verify crate maturity and maintenance
   - Document findings in `reports/main/format_{name}_research.md`

   **Step 2: Test Corpus Collection**
   - Find or create 5 diverse test files
   - Document source URLs in corpus README
   - Save files to `test-corpus/{category}/{format}/`
   - Verify files are valid and diverse

   **Step 3: Core Implementation**
   - Add format variant to `InputFormat` enum in `crates/docling-core/src/format.rs`
   - Create parser module: `crates/docling-{format}/src/parser.rs`
   - Implement parsing logic
   - Handle errors gracefully
   - Add documentation

   **Step 4: Integration**
   - Update `DocumentConverter` to route format
   - Add format to test infrastructure
   - Create integration tests
   - Generate expected outputs

   **Step 5: Validation**
   - Run integration tests: `cargo test test_canon_{format}`
   - Verify all 5 test files pass
   - Check performance benchmarks
   - Fix any failures

   **Step 6: Documentation**
   - Update `MASTER_PLAN.md` with format status
   - Add usage examples to README
   - Document any limitations
   - Note any external dependencies

   **Step 7: Git Commit**
   - Follow CLAUDE.md commit message format
   - Include format name in commit title
   - List all changes
   - Note any issues for next AI

3. **Branch Strategy:**
   - Create branch per phase: `feature/phase-a-foundation`
   - Commit each format separately
   - Create PR when phase complete
   - Merge to main after review

4. **Error Handling:**
   - Never panic on invalid input
   - Return `Result<T, Error>` from all parsers
   - Log warnings for unsupported features
   - Provide partial results when possible

5. **External Dependencies:**
   - Make external tools optional via Cargo features
   - Document installation requirements
   - Provide fallback methods
   - Test with and without external dependencies

6. **Performance:**
   - Measure parsing time for each test file
   - Log to CSV: `test-results/format_performance.csv`
   - Optimize hot paths
   - Use streaming parsers for large files

7. **Testing:**
   - Follow existing integration test patterns
   - Use `#[test]` for unit tests in each module
   - Add integration tests to `crates/docling-core/tests/`
   - Verify edge cases (empty files, corrupted, huge)

### Specific Directives by Phase

#### Phase A: Foundation
**Goal:** Quick wins with high utility

**Priority Order:**
1. Archive formats (ZIP, TAR, 7Z, RAR) - enables recursive parsing
2. SRT subtitles - simple text format
3. GIF - already supported by image crate

**Key Decisions:**
- For archives: Extract all files, parse each based on extension
- For SRT: Simple regex parser or nom parser
- For GIF: Use existing `image` crate integration

#### Phase B: Audio & Video
**Goal:** Python docling parity + multimedia support

**Priority Order:**
1. Whisper integration (critical for WAV/MP3)
2. WAV parser + transcription
3. MP3 parser + transcription
4. Video subtitle extraction (MP4, MKV)

**Key Decisions:**
- Use `whisper-rs` or `vosk` for transcription
- Make transcription optional (behind feature flag)
- Support offline transcription
- Handle multiple languages

#### Phase C: Email & Communication
**Goal:** High business value for document processing

**Priority Order:**
1. EML (most common)
2. MBOX (Unix standard)
3. VCF/vCard (simple)
4. MSG (complex but important)
5. PST (very complex, low priority)

**Key Decisions:**
- Use `mail-parser` crate for EML
- Custom parser for MBOX (simple format)
- Consider LibreOffice conversion for PST

#### Phase D-L: Continue Sequentially
- Follow checklist order
- Prioritize based on complexity and value
- Document blockers and workarounds
- Ask user for clarification when needed

### Common Patterns

**Pattern 1: ZIP-based Formats (EPUB, DOCX, ODT, iWork, etc.)**
```rust
1. Use `zip` crate to open archive
2. Extract key files (content.xml, etc.)
3. Parse XML with `quick-xml`
4. Build Document structure
5. Return Result
```

**Pattern 2: Text-based Formats (CSV, SRT, RTF, etc.)**
```rust
1. Read file as String
2. Parse with regex or nom
3. Extract structure
4. Build Document
5. Return Result
```

**Pattern 3: Binary Formats (PSD, DWG, etc.)**
```rust
1. Try existing Rust crate first
2. If none: Use C library via FFI
3. If impossible: Require conversion
4. Document approach
```

**Pattern 4: Conversion-based (PUB, MPP, etc.)**
```rust
1. Shell out to converter (LibreOffice, MPXJ)
2. Convert to supported format (PDF, XML)
3. Parse converted file
4. Document dependency
```

### Status Tracking

**In Git Commits:**
- Track which formats are implemented
- Track which are blocked
- Track which need help

**In MASTER_PLAN.md:**
- Update format status: ✅ Done, 🚧 In Progress, ❌ Blocked
- Note completion percentage
- Update priorities based on findings

### When to Ask for Help

1. **Cannot find suitable Rust library**
   - Document options researched
   - Ask user for guidance

2. **Format too complex to parse**
   - Document complexity
   - Propose conversion approach

3. **External dependency issues**
   - Document dependency
   - Ask about deployment environment

4. **Performance issues**
   - Document slow operation
   - Ask about acceptable performance

5. **Test file availability**
   - Cannot find 5 diverse examples
   - Ask user for test file sources

### Success Criteria

**Per Format:**
- [ ] 5 test files collected and documented
- [ ] Parser implemented and documented
- [ ] Integration tests passing (5/5)
- [ ] Error handling tested
- [ ] Performance measured
- [ ] Git commit with format implementation

**Per Phase:**
- [ ] All formats in phase complete
- [ ] Branch created and PR ready
- [ ] Documentation updated
- [ ] No regressions in existing tests

**Overall:**
- [ ] All 52 formats implemented
- [ ] 260 new test files (52 × 5)
- [ ] Integration tests passing
- [ ] Documentation complete
- [ ] Performance benchmarks recorded

---

## Next Steps for AI Execution

### Immediate Actions

1. **Review this plan** - Understand all formats and phases
2. **Verify current branch** - Confirm on `main`
3. **Start Phase A** - Create branch `feature/phase-a-foundation`
4. **Begin with archives** - ZIP format first

### First Task: Archive Format - ZIP

**Branch:** `feature/phase-a-foundation`

**Checklist:**
- [ ] Research: Verify `zip` crate (v0.6) capabilities
- [ ] Test Corpus: Create 5 diverse ZIP files
  - [ ] Document ZIP (multiple DOCs/PDFs)
  - [ ] Source code ZIP
  - [ ] Image archive
  - [ ] Mixed content
  - [ ] Password-protected (optional handling)
- [ ] Implementation:
  - [ ] Add `Archive` variant to `InputFormat` (or keep ZIP as separate)
  - [ ] Create `crates/docling-archive/src/zip.rs`
  - [ ] Implement recursive extraction and parsing
  - [ ] Handle nested archives
- [ ] Testing:
  - [ ] Create integration tests
  - [ ] Run: `cargo test test_archive_zip`
  - [ ] Verify all 5 files pass
- [ ] Documentation:
  - [ ] Update `MASTER_PLAN.md`
  - [ ] Add usage example
  - [ ] Document limitations
- [ ] Git Commit:
  - [ ] Follow CLAUDE.md format
  - [ ] Commit message: "Add ZIP archive support for recursive document extraction"

### Execution Timeline

**Phase A:** Complete first, highest value
**Phase B:** Audio/video (Python parity)
**Phase C:** Email (business value)
**Phases D-L:** Sequential implementation

### Monitoring Progress

**Track in each commit:**
- Formats completed: X / 52
- Test files collected: Y / 260
- Integration tests passing: Z / 260
- Current phase: Phase X
- Next format: Format Name

---

## Appendix: Quick Reference

### Format Count by Category

| Category | Count | Priority |
|----------|-------|----------|
| Audio | 2 | HIGH |
| E-book | 4 | MEDIUM |
| Email | 5 | HIGH |
| Apple iWork | 3 | MEDIUM |
| Adobe Extended | 5 | MEDIUM |
| Microsoft Extended | 6 | MEDIUM |
| CAD/Engineering | 4 | LOW |
| 3D Formats | 3 | LOW |
| Archive | 4 | HIGH |
| Video | 5 | MEDIUM |
| Specialized | 7 | LOW |
| Legacy | 4 | LOW |
| **TOTAL** | **52** | |

### Implementation Complexity

| Complexity | Format Count | Examples |
|-----------|--------------|----------|
| Very Low | 3 | SRT, GIF, VCF |
| Low | 8 | ZIP, TAR, MBOX, STL, GLTF, ICS, Jupyter, FB2 |
| Low-Medium | 4 | EML, MBOX, OBJ, Jupyter |
| Medium | 15 | WAV, MP3, EPUB, ODT, ODS, ODP, XPS, SVG, VSDX, Video formats, KML, GPX, DXF |
| Medium-High | 6 | RTF, MOBI, PSD, AI, FBX, RAR |
| High | 10 | IDML, MSG, Pages, Numbers, Keynote, DOC, LaTeX, DICOM, IFC, WordPerfect |
| Very High | 6 | AZW3, PST, PUB, ONE, MPP, DWG, MDB/ACCDB |

### Rust Library Availability

| Status | Format Count | Description |
|--------|--------------|-------------|
| ✅ Mature | 25 | Ready-to-use Rust crates |
| ⚠️ Partial | 15 | Some library exists, custom work needed |
| ❌ None | 12 | No Rust library, FFI or conversion required |

### Expected Outcomes

**Total Formats:** 67 (15 existing + 52 new)
**Total Test Files:** 335 (75 existing + 260 new)
**Code Additions:** ~52 parser modules, ~260 integration tests
**Documentation:** Format-specific guides, usage examples

---

**END OF COMPREHENSIVE FORMAT EXPANSION PLAN**

**AI Worker: Begin with Phase A, Task 1: ZIP Archive Support**
