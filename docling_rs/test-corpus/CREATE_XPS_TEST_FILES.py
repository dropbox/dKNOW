#!/usr/bin/env python3
"""
Create test XPS (XML Paper Specification) files for docling_rs testing.

XPS is a ZIP archive containing:
- [Content_Types].xml - MIME types
- _rels/.rels - Package relationships
- FixedDocumentSequence.fdseq - Document sequence
- Documents/1/FixedDocument.fdoc - Page list
- Documents/1/Pages/*.fpage - Individual pages
- docProps/core.xml - Metadata

Each .fpage file contains XAML-like XML with Glyphs elements.
"""

import os
import zipfile
from pathlib import Path

# Create output directory
output_dir = Path("xps")
output_dir.mkdir(exist_ok=True)

def create_content_types():
    """[Content_Types].xml"""
    return '''<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Default Extension="fdseq" ContentType="application/vnd.ms-package.xps-fixeddocumentsequence+xml"/>
  <Default Extension="fdoc" ContentType="application/vnd.ms-package.xps-fixeddocument+xml"/>
  <Default Extension="fpage" ContentType="application/vnd.ms-package.xps-fixedpage+xml"/>
</Types>'''

def create_rels():
    """_rels/.rels"""
    return '''<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.microsoft.com/xps/2005/06/fixedrepresentation" Target="/FixedDocumentSequence.fdseq"/>
</Relationships>'''

def create_core_props(title="XPS Document", author="Test Author", subject="Test Subject"):
    """docProps/core.xml"""
    return f'''<?xml version="1.0" encoding="UTF-8"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties"
                   xmlns:dc="http://purl.org/dc/elements/1.1/"
                   xmlns:dcterms="http://purl.org/dc/terms/"
                   xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <dc:title>{title}</dc:title>
  <dc:creator>{author}</dc:creator>
  <dc:subject>{subject}</dc:subject>
  <dcterms:created xsi:type="dcterms:W3CDTF">2025-11-07T10:00:00Z</dcterms:created>
  <dcterms:modified xsi:type="dcterms:W3CDTF">2025-11-07T10:00:00Z</dcterms:modified>
</cp:coreProperties>'''

def create_fdseq():
    """FixedDocumentSequence.fdseq"""
    return '''<?xml version="1.0" encoding="UTF-8"?>
<FixedDocumentSequence xmlns="http://schemas.microsoft.com/xps/2005/06">
  <DocumentReference Source="/Documents/1/FixedDocument.fdoc"/>
</FixedDocumentSequence>'''

def create_fdoc(num_pages):
    """Documents/1/FixedDocument.fdoc"""
    pages = "\n  ".join([f'<PageContent Source="/Documents/1/Pages/{i}.fpage"/>' for i in range(1, num_pages + 1)])
    return f'''<?xml version="1.0" encoding="UTF-8"?>
<FixedDocument xmlns="http://schemas.microsoft.com/xps/2005/06">
  {pages}
</FixedDocument>'''

def create_fpage(page_num, text_elements):
    """Documents/1/Pages/N.fpage

    Args:
        page_num: Page number (1-indexed)
        text_elements: List of (text, x, y, font_size) tuples
    """
    glyphs = []
    for text, x, y, font_size in text_elements:
        glyphs.append(f'    <Glyphs UnicodeString="{text}" OriginX="{x}" OriginY="{y}" FontRenderingEmSize="{font_size}" FontUri="/Resources/Fonts/Arial.otf"/>')

    glyphs_xml = "\n".join(glyphs)

    return f'''<?xml version="1.0" encoding="UTF-8"?>
<FixedPage Width="816" Height="1056" xmlns="http://schemas.microsoft.com/xps/2005/06">
{glyphs_xml}
</FixedPage>'''

def create_xps_file(filename, title, author, subject, pages_data):
    """Create a complete XPS file.

    Args:
        filename: Output filename (e.g., "simple_text.xps")
        title: Document title
        author: Document author
        subject: Document subject
        pages_data: List of page data, where each page is a list of (text, x, y, font_size) tuples
    """
    filepath = output_dir / filename

    with zipfile.ZipFile(filepath, 'w', zipfile.ZIP_DEFLATED) as zf:
        # Add [Content_Types].xml (uncompressed, must be first)
        zf.writestr("[Content_Types].xml", create_content_types(), compress_type=zipfile.ZIP_STORED)

        # Add _rels/.rels
        zf.writestr("_rels/.rels", create_rels())

        # Add metadata
        zf.writestr("docProps/core.xml", create_core_props(title, author, subject))

        # Add document structure
        zf.writestr("FixedDocumentSequence.fdseq", create_fdseq())
        zf.writestr("Documents/1/FixedDocument.fdoc", create_fdoc(len(pages_data)))

        # Add pages
        for i, page_data in enumerate(pages_data, 1):
            zf.writestr(f"Documents/1/Pages/{i}.fpage", create_fpage(i, page_data))

    print(f"Created: {filepath} ({len(pages_data)} pages)")

# Test File 1: Simple single-page text
create_xps_file(
    "simple_text.xps",
    title="Simple Text Document",
    author="Test Author",
    subject="Basic XPS Test",
    pages_data=[
        [  # Page 1
            ("Hello, World!", 100, 100, 24),
            ("This is a simple XPS document.", 100, 150, 16),
        ]
    ]
)

# Test File 2: Multi-page document
create_xps_file(
    "multi_page.xps",
    title="Multi-Page Document",
    author="Test Author",
    subject="Multi-page XPS Test",
    pages_data=[
        [  # Page 1
            ("Page 1", 100, 100, 32),
            ("This is the first page.", 100, 150, 16),
            ("It contains multiple text elements.", 100, 180, 16),
        ],
        [  # Page 2
            ("Page 2", 100, 100, 32),
            ("This is the second page.", 100, 150, 16),
            ("More content here.", 100, 180, 16),
        ],
        [  # Page 3
            ("Page 3", 100, 100, 32),
            ("This is the final page.", 100, 150, 16),
            ("The end.", 100, 180, 16),
        ]
    ]
)

# Test File 3: Rich text with various font sizes
create_xps_file(
    "formatted.xps",
    title="Formatted Document",
    author="Formatting Tester",
    subject="Font size variations",
    pages_data=[
        [  # Page 1
            ("Large Heading", 100, 100, 36),
            ("Medium subheading", 100, 160, 24),
            ("Regular body text goes here.", 100, 200, 16),
            ("Small footnote text", 100, 230, 12),
        ]
    ]
)

# Test File 4: Report-style document
create_xps_file(
    "report.xps",
    title="Annual Report 2025",
    author="Corporate Communications",
    subject="Financial and operational report",
    pages_data=[
        [  # Cover page
            ("ANNUAL REPORT", 200, 300, 48),
            ("2025", 200, 370, 36),
            ("Company Name", 200, 450, 24),
        ],
        [  # Contents
            ("Contents", 100, 100, 32),
            ("1. Executive Summary ............... 3", 100, 150, 16),
            ("2. Financial Results ............... 5", 100, 180, 16),
            ("3. Operations Review ............... 8", 100, 210, 16),
        ],
        [  # Executive Summary
            ("Executive Summary", 100, 100, 28),
            ("This year has been successful.", 100, 150, 16),
            ("Revenue increased by 15%.", 100, 180, 16),
            ("Customer satisfaction improved.", 100, 210, 16),
        ]
    ]
)

# Test File 5: Technical specification
create_xps_file(
    "technical_spec.xps",
    title="Technical Specification v1.0",
    author="Engineering Team",
    subject="System design specification",
    pages_data=[
        [  # Title page
            ("Technical Specification", 100, 100, 36),
            ("Version 1.0", 100, 150, 24),
            ("November 2025", 100, 190, 16),
        ],
        [  # Introduction
            ("1. Introduction", 100, 100, 28),
            ("This document describes the system architecture.", 100, 140, 16),
            ("The system consists of three main components:", 100, 170, 16),
            ("- Frontend (React)", 120, 200, 14),
            ("- Backend (Rust)", 120, 220, 14),
            ("- Database (PostgreSQL)", 120, 240, 14),
        ],
        [  # Architecture
            ("2. System Architecture", 100, 100, 28),
            ("The architecture follows a microservices pattern.", 100, 140, 16),
            ("Each service is independently deployable.", 100, 170, 16),
        ],
        [  # Requirements
            ("3. Requirements", 100, 100, 28),
            ("Functional Requirements:", 100, 140, 20),
            ("- User authentication and authorization", 120, 170, 14),
            ("- Data persistence and retrieval", 120, 190, 14),
            ("- Real-time notifications", 120, 210, 14),
            ("Non-Functional Requirements:", 100, 250, 20),
            ("- 99.9% uptime", 120, 280, 14),
            ("- < 200ms response time", 120, 300, 14),
        ]
    ]
)

print("\nAll XPS test files created successfully!")
print(f"Location: {output_dir.absolute()}")
print("\nFiles created:")
for f in sorted(output_dir.glob("*.xps")):
    size = f.stat().st_size
    print(f"  {f.name:30s} {size:>7,} bytes")
