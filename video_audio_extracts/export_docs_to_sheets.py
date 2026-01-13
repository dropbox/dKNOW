#!/usr/bin/env python3
"""
Export 6 documentation reports to Google Sheets.
Creates a single spreadsheet with 6 tabs, one for each report.
"""

import sys
import os

# Add gdrive tools to path
sys.path.insert(0, os.path.expanduser("~/pc/internal_tools/gdrive"))

from gdocs_publisher import GoogleDocsPublisher
from googleapiclient.discovery import build
from google.oauth2.credentials import Credentials


def read_markdown_file(filepath):
    """Read markdown file content."""
    with open(filepath, "r", encoding="utf-8") as f:
        return f.read()


def create_multi_tab_spreadsheet():
    """Create a Google Sheets spreadsheet with 6 tabs for documentation reports."""

    # Change to gdrive directory for credentials
    original_dir = os.getcwd()
    gdrive_dir = os.path.expanduser("~/pc/internal_tools/gdrive")
    os.chdir(gdrive_dir)

    # Initialize publisher
    publisher = GoogleDocsPublisher()

    # Change back to original directory
    os.chdir(original_dir)

    # Define the 6 reports
    reports = [
        {
            "title": "1. Format Support",
            "file": "docs/FORMAT_SUPPORT.md",
        },
        {
            "title": "2. Transformations",
            "file": "docs/TRANSFORMATIONS.md",
        },
        {
            "title": "3. Routing & Optimizations",
            "file": "docs/ROUTING_AND_OPTIMIZATIONS.md",
        },
        {
            "title": "4. Test Coverage Grid",
            "file": "docs/TEST_COVERAGE_GRID.md",
        },
        {
            "title": "5. Functionality Grid",
            "file": "docs/FUNCTIONALITY_GRID.md",
        },
        {
            "title": "6. Format Conversion Grid",
            "file": "docs/FORMAT_CONVERSION_GRID.md",
        },
    ]

    print("Creating spreadsheet: Video & Audio Extract System Documentation")
    print("=" * 80)

    # Create the first tab from the first report
    first_report = reports[0]
    markdown_content = read_markdown_file(first_report["file"])

    # Try to extract tables from markdown
    tables = publisher.extract_markdown_tables(markdown_content)

    if tables:
        print(f"\n1. Creating spreadsheet with first tab: {first_report['title']}")
        print(f"   File: {first_report['file']}")
        print(f"   Tables found: {len(tables)}")

        # Create sheet from first table
        sheet_id = publisher.create_sheet_from_markdown(
            markdown_content=markdown_content,
            title="Video & Audio Extract System Documentation",
            table_index=0,
            freeze_header=True,
            auto_resize=True,
        )

        print(f"   ✓ Spreadsheet created: {sheet_id}")

        # Now add remaining tabs
        service = build("sheets", "v4", credentials=publisher.creds)

        # Rename first sheet
        requests = [
            {
                "updateSheetProperties": {
                    "properties": {"sheetId": 0, "title": first_report["title"]},
                    "fields": "title",
                }
            }
        ]

        service.spreadsheets().batchUpdate(
            spreadsheetId=sheet_id, body={"requests": requests}
        ).execute()

        print(f"   ✓ First tab renamed to: {first_report['title']}")

        # Add remaining tabs
        for i, report in enumerate(reports[1:], start=2):
            print(f"\n{i}. Adding tab: {report['title']}")
            print(f"   File: {report['file']}")

            markdown_content = read_markdown_file(report["file"])
            tables = publisher.extract_markdown_tables(markdown_content)

            if tables:
                print(f"   Tables found: {len(tables)}")

                # Add new sheet
                add_sheet_request = {
                    "addSheet": {"properties": {"title": report["title"]}}
                }

                response = (
                    service.spreadsheets()
                    .batchUpdate(
                        spreadsheetId=sheet_id, body={"requests": [add_sheet_request]}
                    )
                    .execute()
                )

                new_sheet_id = response["replies"][0]["addSheet"]["properties"][
                    "sheetId"
                ]

                # Populate the new sheet with first table
                table = tables[0]
                headers = table["headers"]
                rows = table["data"]

                # Write data
                range_name = f"'{report['title']}'!A1"
                values = [headers] + rows

                body = {"values": values}

                service.spreadsheets().values().update(
                    spreadsheetId=sheet_id,
                    range=range_name,
                    valueInputOption="RAW",
                    body=body,
                ).execute()

                # Format header row
                format_requests = [
                    {
                        "repeatCell": {
                            "range": {
                                "sheetId": new_sheet_id,
                                "startRowIndex": 0,
                                "endRowIndex": 1,
                            },
                            "cell": {
                                "userEnteredFormat": {
                                    "backgroundColor": {
                                        "red": 0.9,
                                        "green": 0.9,
                                        "blue": 0.9,
                                    },
                                    "textFormat": {"bold": True},
                                }
                            },
                            "fields": "userEnteredFormat(backgroundColor,textFormat)",
                        }
                    },
                    {
                        "updateSheetProperties": {
                            "properties": {
                                "sheetId": new_sheet_id,
                                "gridProperties": {"frozenRowCount": 1},
                            },
                            "fields": "gridProperties.frozenRowCount",
                        }
                    },
                    {
                        "autoResizeDimensions": {
                            "dimensions": {
                                "sheetId": new_sheet_id,
                                "dimension": "COLUMNS",
                                "startIndex": 0,
                                "endIndex": len(headers),
                            }
                        }
                    },
                ]

                service.spreadsheets().batchUpdate(
                    spreadsheetId=sheet_id, body={"requests": format_requests}
                ).execute()

                print(f"   ✓ Tab added and populated with {len(rows)} rows")
            else:
                print(f"   ⚠ No tables found in {report['file']}, skipping")

        # Get final spreadsheet URL
        spreadsheet = service.spreadsheets().get(spreadsheetId=sheet_id).execute()
        url = spreadsheet.get("spreadsheetUrl")

        print("\n" + "=" * 80)
        print(f"✓ Spreadsheet created successfully!")
        print(f"  Title: Video & Audio Extract System Documentation")
        print(f"  ID: {sheet_id}")
        print(f"  URL: {url}")
        print(f"  Tabs: {len(reports)}")
        print("=" * 80)

        return sheet_id, url
    else:
        print(f"ERROR: No tables found in {first_report['file']}")
        print("Cannot create spreadsheet without table data")
        return None, None


if __name__ == "__main__":
    try:
        sheet_id, url = create_multi_tab_spreadsheet()
        if sheet_id:
            print("\n✓ Export complete!")
            print(f"  Open: {url}")
            sys.exit(0)
        else:
            print("\n✗ Export failed - no tables found")
            sys.exit(1)
    except Exception as e:
        print(f"\n✗ ERROR: {e}")
        import traceback

        traceback.print_exc()
        sys.exit(1)
