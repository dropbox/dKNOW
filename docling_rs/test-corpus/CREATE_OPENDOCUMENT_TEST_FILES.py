#!/usr/bin/env python3
"""
Script to create OpenDocument test files (ODT, ODS, ODP)

Creates minimal but valid OpenDocument files for testing the docling_rs
OpenDocument parsers.
"""

import os
import zipfile
from pathlib import Path

# Base directory
BASE_DIR = Path(__file__).parent
ODF_DIR = BASE_DIR / "opendocument"


def create_odt_file(filepath, title, author, content_paragraphs):
    """Create a minimal but valid ODT file"""

    # Mimetype (must be first file, uncompressed)
    mimetype = "application/vnd.oasis.opendocument.text"

    # META-INF/manifest.xml
    manifest = """<?xml version="1.0" encoding="UTF-8"?>
<manifest:manifest xmlns:manifest="urn:oasis:names:tc:opendocument:xmlns:manifest:1.0" manifest:version="1.2">
 <manifest:file-entry manifest:full-path="/" manifest:media-type="application/vnd.oasis.opendocument.text"/>
 <manifest:file-entry manifest:full-path="meta.xml" manifest:media-type="text/xml"/>
 <manifest:file-entry manifest:full-path="content.xml" manifest:media-type="text/xml"/>
</manifest:manifest>"""

    # meta.xml
    meta = f"""<?xml version="1.0" encoding="UTF-8"?>
<office:document-meta xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
                      xmlns:dc="http://purl.org/dc/elements/1.1/"
                      xmlns:meta="urn:oasis:names:tc:opendocument:xmlns:meta:1.0">
 <office:meta>
  <dc:title>{title}</dc:title>
  <dc:creator>{author}</dc:creator>
  <dc:subject>Test Document</dc:subject>
 </office:meta>
</office:document-meta>"""

    # content.xml
    paragraphs_xml = "\n  ".join([f'<text:p text:style-name="P1">{p}</text:p>' for p in content_paragraphs])

    content = f"""<?xml version="1.0" encoding="UTF-8"?>
<office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
                         xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
                         xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0">
 <office:body>
  <office:text>
   <text:h text:style-name="Heading_20_1" text:outline-level="1">{title}</text:h>
   {paragraphs_xml}
  </office:text>
 </office:body>
</office:document-content>"""

    # Create ZIP file
    with zipfile.ZipFile(filepath, 'w', zipfile.ZIP_DEFLATED) as zf:
        # mimetype must be first and uncompressed
        zf.writestr('mimetype', mimetype, compress_type=zipfile.ZIP_STORED)
        zf.writestr('META-INF/manifest.xml', manifest)
        zf.writestr('meta.xml', meta)
        zf.writestr('content.xml', content)

    print(f"Created: {filepath.name}")


def create_ods_file(filepath, title, sheets_data):
    """Create a minimal but valid ODS file

    sheets_data: list of (sheet_name, rows) where rows is list of row lists
    Example: [("Sheet1", [["A1", "B1"], ["A2", "B2"]])]
    """

    # Mimetype
    mimetype = "application/vnd.oasis.opendocument.spreadsheet"

    # META-INF/manifest.xml
    manifest = """<?xml version="1.0" encoding="UTF-8"?>
<manifest:manifest xmlns:manifest="urn:oasis:names:tc:opendocument:xmlns:manifest:1.0" manifest:version="1.2">
 <manifest:file-entry manifest:full-path="/" manifest:media-type="application/vnd.oasis.opendocument.spreadsheet"/>
 <manifest:file-entry manifest:full-path="content.xml" manifest:media-type="text/xml"/>
</manifest:manifest>"""

    # Build tables XML
    tables_xml = []
    for sheet_name, rows in sheets_data:
        rows_xml = []
        for row in rows:
            cells_xml = []
            for cell_value in row:
                # Determine cell type and value
                if isinstance(cell_value, (int, float)):
                    cells_xml.append(f'<table:table-cell office:value-type="float" office:value="{cell_value}"><text:p>{cell_value}</text:p></table:table-cell>')
                else:
                    cells_xml.append(f'<table:table-cell office:value-type="string"><text:p>{cell_value}</text:p></table:table-cell>')
            rows_xml.append(f'<table:table-row>{"".join(cells_xml)}</table:table-row>')

        table_xml = f"""<table:table table:name="{sheet_name}">
  {"".join(rows_xml)}
 </table:table>"""
        tables_xml.append(table_xml)

    # content.xml
    content = f"""<?xml version="1.0" encoding="UTF-8"?>
<office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
                         xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
                         xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0">
 <office:body>
  <office:spreadsheet>
   {" ".join(tables_xml)}
  </office:spreadsheet>
 </office:body>
</office:document-content>"""

    # Create ZIP file
    with zipfile.ZipFile(filepath, 'w', zipfile.ZIP_DEFLATED) as zf:
        zf.writestr('mimetype', mimetype, compress_type=zipfile.ZIP_STORED)
        zf.writestr('META-INF/manifest.xml', manifest)
        zf.writestr('content.xml', content)

    print(f"Created: {filepath.name}")


def create_odp_file(filepath, title, author, slides_content):
    """Create a minimal but valid ODP file

    slides_content: list of slide text content
    """

    # Mimetype
    mimetype = "application/vnd.oasis.opendocument.presentation"

    # META-INF/manifest.xml
    manifest = """<?xml version="1.0" encoding="UTF-8"?>
<manifest:manifest xmlns:manifest="urn:oasis:names:tc:opendocument:xmlns:manifest:1.0" manifest:version="1.2">
 <manifest:file-entry manifest:full-path="/" manifest:media-type="application/vnd.oasis.opendocument.presentation"/>
 <manifest:file-entry manifest:full-path="meta.xml" manifest:media-type="text/xml"/>
 <manifest:file-entry manifest:full-path="content.xml" manifest:media-type="text/xml"/>
</manifest:manifest>"""

    # meta.xml
    meta = f"""<?xml version="1.0" encoding="UTF-8"?>
<office:document-meta xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
                      xmlns:dc="http://purl.org/dc/elements/1.1/">
 <office:meta>
  <dc:title>{title}</dc:title>
  <dc:creator>{author}</dc:creator>
 </office:meta>
</office:document-meta>"""

    # Build slides XML
    slides_xml = []
    for i, slide_text in enumerate(slides_content, 1):
        slide_xml = f"""<draw:page draw:name="Slide {i}">
  <draw:frame>
   <draw:text-box>
    <text:p>{slide_text}</text:p>
   </draw:text-box>
  </draw:frame>
 </draw:page>"""
        slides_xml.append(slide_xml)

    # content.xml
    content = f"""<?xml version="1.0" encoding="UTF-8"?>
<office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
                         xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0"
                         xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0">
 <office:body>
  <office:presentation>
   {" ".join(slides_xml)}
  </office:presentation>
 </office:body>
</office:document-content>"""

    # Create ZIP file
    with zipfile.ZipFile(filepath, 'w', zipfile.ZIP_DEFLATED) as zf:
        zf.writestr('mimetype', mimetype, compress_type=zipfile.ZIP_STORED)
        zf.writestr('META-INF/manifest.xml', manifest)
        zf.writestr('meta.xml', meta)
        zf.writestr('content.xml', content)

    print(f"Created: {filepath.name}")


def main():
    print("Creating OpenDocument test files...")
    print()

    # Create ODT files
    print("Creating ODT (text) files...")
    odt_dir = ODF_DIR / "odt"
    odt_dir.mkdir(parents=True, exist_ok=True)

    create_odt_file(
        odt_dir / "simple_text.odt",
        "Simple Document",
        "Test Author",
        ["This is a simple ODT document.", "It has two paragraphs."]
    )

    create_odt_file(
        odt_dir / "multi_paragraph.odt",
        "Multi-Paragraph Document",
        "John Doe",
        [
            "First paragraph with some text.",
            "Second paragraph with more text.",
            "Third paragraph continues the document.",
            "Fourth paragraph adds more content.",
            "Fifth paragraph concludes the document."
        ]
    )

    create_odt_file(
        odt_dir / "report.odt",
        "Test Report 2024",
        "Jane Smith",
        [
            "Executive Summary: This report provides an overview of our testing infrastructure.",
            "Introduction: The OpenDocument format is widely used for office documents.",
            "Methodology: We tested various aspects of ODT parsing including metadata extraction.",
            "Results: The parser successfully extracted all text content and metadata fields.",
            "Conclusion: ODT support is now fully integrated into the docling_rs project."
        ]
    )

    create_odt_file(
        odt_dir / "meeting_notes.odt",
        "Meeting Notes - Nov 7 2024",
        "Team Lead",
        [
            "Attendees: Alice, Bob, Charlie",
            "Agenda: Discuss OpenDocument format implementation",
            "Discussion: We reviewed the ODT, ODS, and ODP parsers",
            "Action Items: Complete integration tests and documentation",
            "Next Meeting: November 14, 2024"
        ]
    )

    create_odt_file(
        odt_dir / "technical_spec.odt",
        "OpenDocument Technical Specification",
        "Tech Writer",
        [
            "Overview: OpenDocument Format (ODF) is an ISO standard for office documents.",
            "File Structure: ODF files are ZIP archives containing XML files.",
            "Content Storage: Main content is stored in content.xml",
            "Metadata: Document metadata is stored in meta.xml",
            "Compatibility: ODF is supported by LibreOffice, OpenOffice, and many other applications."
        ]
    )

    print()

    # Create ODS files
    print("Creating ODS (spreadsheet) files...")
    ods_dir = ODF_DIR / "ods"
    ods_dir.mkdir(parents=True, exist_ok=True)

    create_ods_file(
        ods_dir / "simple_spreadsheet.ods",
        "Simple Spreadsheet",
        [("Sheet1", [["Name", "Age"], ["Alice", 30], ["Bob", 25]])]
    )

    create_ods_file(
        ods_dir / "budget.ods",
        "Budget 2024",
        [(
            "Budget",
            [
                ["Category", "Amount", "Notes"],
                ["Salaries", 100000, "Team salaries"],
                ["Equipment", 25000, "Hardware purchases"],
                ["Software", 15000, "License fees"],
                ["Total", 140000, "Sum of all costs"]
            ]
        )]
    )

    create_ods_file(
        ods_dir / "inventory.ods",
        "Inventory",
        [(
            "Stock",
            [
                ["Product", "Quantity", "Price"],
                ["Widget A", 150, 12.99],
                ["Widget B", 200, 24.50],
                ["Widget C", 75, 45.00]
            ]
        )]
    )

    create_ods_file(
        ods_dir / "multi_sheet.ods",
        "Multi-Sheet",
        [
            ("Sales", [["Q1", 1000], ["Q2", 1500], ["Q3", 1200], ["Q4", 1800]]),
            ("Expenses", [["Q1", 800], ["Q2", 900], ["Q3", 850], ["Q4", 950]])
        ]
    )

    create_ods_file(
        ods_dir / "test_data.ods",
        "Test Data",
        [(
            "Data",
            [
                ["ID", "Value", "Status"],
                [1, 100, "Active"],
                [2, 200, "Pending"],
                [3, 300, "Complete"],
                [4, 400, "Active"],
                [5, 500, "Complete"]
            ]
        )]
    )

    print()

    # Create ODP files
    print("Creating ODP (presentation) files...")
    odp_dir = ODF_DIR / "odp"
    odp_dir.mkdir(parents=True, exist_ok=True)

    create_odp_file(
        odp_dir / "simple_presentation.odp",
        "Simple Presentation",
        "Presenter",
        [
            "Title Slide: Welcome to OpenDocument",
            "Slide 2: Key Features",
            "Slide 3: Thank You"
        ]
    )

    create_odp_file(
        odp_dir / "project_overview.odp",
        "Project Overview",
        "Project Manager",
        [
            "Project Status Update",
            "Milestone 1: Architecture Design - Complete",
            "Milestone 2: Implementation - In Progress",
            "Milestone 3: Testing - Not Started",
            "Next Steps and Timeline"
        ]
    )

    create_odp_file(
        odp_dir / "training.odp",
        "OpenDocument Training",
        "Trainer",
        [
            "Introduction to OpenDocument Format",
            "ODT: Text Documents",
            "ODS: Spreadsheets",
            "ODP: Presentations",
            "Practical Examples",
            "Questions and Answers"
        ]
    )

    create_odp_file(
        odp_dir / "sales_pitch.odp",
        "Product Demo 2024",
        "Sales Team",
        [
            "Our Product: Revolutionary Document Processing",
            "Problem: Complex document formats",
            "Solution: Unified parsing library",
            "Benefits: Speed, accuracy, flexibility",
            "Pricing: Competitive and transparent",
            "Contact Us: sales@example.com"
        ]
    )

    create_odp_file(
        odp_dir / "technical_talk.odp",
        "Rust for Document Processing",
        "Tech Lead",
        [
            "Why Rust? Safety and Performance",
            "OpenDocument Format Structure",
            "Implementation Approach",
            "Challenges and Solutions",
            "Performance Benchmarks",
            "Future Work: More Formats"
        ]
    )

    print()
    print("✓ All OpenDocument test files created successfully!")
    print()
    print(f"ODT files: {len(list((ODF_DIR / 'odt').glob('*.odt')))}")
    print(f"ODS files: {len(list((ODF_DIR / 'ods').glob('*.ods')))}")
    print(f"ODP files: {len(list((ODF_DIR / 'odp').glob('*.odp')))}")


if __name__ == "__main__":
    main()
