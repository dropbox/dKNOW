#!/usr/bin/env python3
"""Export COMPREHENSIVE_MATRIX.md to Google Sheets with proper tabs."""

import sys, os, re

sys.path.insert(0, os.path.expanduser("~/pc/internal_tools"))
from gdrive.gdocs_publisher import GoogleDocsPublisher

# Read the comprehensive matrix
with open("docs/COMPREHENSIVE_MATRIX.md") as f:
    content = f.read()


# Extract tables using section markers
def extract_tables(text):
    """Extract all markdown tables from text."""
    tables = []
    lines = text.split("\n")
    i = 0
    while i < len(lines):
        if (
            lines[i].strip().startswith("|")
            and i + 1 < len(lines)
            and lines[i + 1].strip().startswith("|")
        ):
            # Found table start
            table_lines = []
            while i < len(lines) and lines[i].strip().startswith("|"):
                table_lines.append(lines[i])
                i += 1

            # Parse table
            if len(table_lines) >= 2:
                headers = [c.strip() for c in table_lines[0].split("|")[1:-1]]
                data = []
                for line in table_lines[2:]:  # Skip header and separator
                    row = [c.strip() for c in line.split("|")[1:-1]]
                    if row:
                        data.append(row)
                if data:
                    tables.append({"headers": headers, "data": data})
        i += 1
    return tables


tables = extract_tables(content)
print(f"Found {len(tables)} tables in COMPREHENSIVE_MATRIX.md")

# Create multi-tab spreadsheet
publisher = GoogleDocsPublisher()

# Prepare sheet data
sheets_data = []

# Extract section headers to name tabs
section_names = re.findall(r"###\s+(\d+\.\d+)\s+(.+)", content)
print(f"Found {len(section_names)} sections")

# Map tables to sections
if len(tables) >= 4:
    sheets_data.append(
        {
            "title": "Video Matrix",
            "headers": tables[0]["headers"],
            "data": tables[0]["data"],
            "freeze_header": True,
        }
    )
    sheets_data.append(
        {
            "title": "Audio Matrix",
            "headers": tables[1]["headers"],
            "data": tables[1]["data"],
            "freeze_header": True,
        }
    )
    sheets_data.append(
        {
            "title": "Image Matrix",
            "headers": tables[2]["headers"],
            "data": tables[2]["data"],
            "freeze_header": True,
        }
    )
    sheets_data.append(
        {
            "title": "Universal Transforms",
            "headers": tables[3]["headers"],
            "data": tables[3]["data"],
            "freeze_header": True,
        }
    )
if len(tables) >= 5:
    sheets_data.append(
        {
            "title": "Format Metadata",
            "headers": tables[4]["headers"],
            "data": tables[4]["data"],
            "freeze_header": True,
        }
    )
if len(tables) >= 6:
    sheets_data.append(
        {
            "title": "Transform Implementations",
            "headers": tables[5]["headers"],
            "data": tables[5]["data"],
            "freeze_header": True,
        }
    )

print(f"Creating spreadsheet with {len(sheets_data)} tabs...")

# Create the spreadsheet (using existing multi-sheet logic from publish_reports_to_sheets.py)
# ... implementation here
