# Apple iWork Test Files

## Overview

Created: 2025-11-08
Total Files: 12 (4 Pages + 4 Numbers + 4 Keynote)

These are synthetic test files created to support Apple iWork format parsing development. Since real-world iWork files are difficult to find online (proprietary format, not commonly shared on GitHub), these files were manually constructed with valid ZIP archive structure and XML content.

## File Structure

Apple iWork files (.pages, .numbers, .key) are ZIP archives containing:
- `index.xml` - Main document content in XML format
- `QuickLook/` - Directory containing preview thumbnails
- `Data/` - Directory for embedded media and resources

## Test Files

### Apple Pages (.pages) - 4 files

1. **minimal-test.pages** (956 bytes)
   - Simple test document with basic text
   - Purpose: Minimal valid structure test

2. **proposal.pages** (1.2 KB)
   - Project proposal with headings, lists, and table
   - Purpose: Test structured document parsing
   - Content: Executive summary, objectives, budget table

3. **resume.pages** (1.2 KB)
   - Professional resume format
   - Purpose: Test heading hierarchy, lists, sections
   - Content: Experience, education, skills sections

4. **cover-letter.pages** (1.3 KB)
   - Formal business letter
   - Purpose: Test paragraph formatting and lists
   - Content: Multi-paragraph letter with bullet points

### Apple Numbers (.numbers) - 4 files

1. **minimal-test.numbers** (952 bytes)
   - Simple 2x2 table
   - Purpose: Minimal valid spreadsheet test

2. **sales-report.numbers** (969 bytes)
   - Quarterly sales data
   - Purpose: Test numeric data and calculations
   - Content: Monthly revenue, expenses, profit

3. **budget.numbers** (986 bytes)
   - Personal budget tracker
   - Purpose: Test multi-column numeric tables
   - Content: Budget vs actual spending by category

4. **inventory.numbers** (1.0 KB)
   - Product inventory spreadsheet
   - Purpose: Test mixed data types (text, numbers, calculations)
   - Content: SKU, product names, quantities, prices

### Apple Keynote (.key) - 4 files

1. **minimal-test.key** (996 bytes)
   - Simple 2-slide presentation
   - Purpose: Minimal valid presentation test

2. **business-review.key** (1.1 KB)
   - Company overview presentation
   - Purpose: Test multi-slide structure
   - Content: 3 slides with titles, body text, lists

3. **training.key** (1.0 KB)
   - Employee onboarding presentation
   - Purpose: Test slide transitions and lists
   - Content: 4 slides with mixed content types

4. **product-launch.key** (1.2 KB)
   - Product announcement presentation
   - Purpose: Test comprehensive slide structure
   - Content: 5 slides with problem/solution format

## XML Structure Examples

### Pages Document Structure
```xml
<?xml version="1.0" encoding="UTF-8"?>
<sl:document xmlns:sl="http://developer.apple.com/namespaces/sl"
             xmlns:sf="http://developer.apple.com/namespaces/sf">
  <sl:publication-info>
    <sl:author>Author Name</sl:author>
    <sl:title>Document Title</sl:title>
  </sl:publication-info>
  <sl:body>
    <sf:text-storage>
      <sf:p sf:style="heading1">Heading</sf:p>
      <sf:p>Paragraph text</sf:p>
      <sf:list>
        <sf:list-item>Item 1</sf:list-item>
      </sf:list>
    </sf:text-storage>
  </sl:body>
</sl:document>
```

### Numbers Spreadsheet Structure
```xml
<?xml version="1.0" encoding="UTF-8"?>
<ls:document xmlns:ls="http://developer.apple.com/namespaces/ls"
             xmlns:sf="http://developer.apple.com/namespaces/sf">
  <ls:workspace>
    <ls:sheet ls:name="Sheet 1">
      <ls:table ls:name="Table 1">
        <ls:table-model>
          <ls:row>
            <ls:cell><sf:string>Text</sf:string></ls:cell>
            <ls:cell><sf:number>123</sf:number></ls:cell>
          </ls:row>
        </ls:table-model>
      </ls:table>
    </ls:sheet>
  </ls:workspace>
</ls:document>
```

### Keynote Presentation Structure
```xml
<?xml version="1.0" encoding="UTF-8"?>
<key:presentation xmlns:key="http://developer.apple.com/namespaces/keynote2"
                  xmlns:sf="http://developer.apple.com/namespaces/sf">
  <key:slide-list>
    <key:slide key:number="1">
      <key:title><sf:text>Slide Title</sf:text></key:title>
      <key:body>
        <sf:p>Slide content</sf:p>
      </key:body>
    </key:slide>
  </key:slide-list>
</key:presentation>
```

## Verification

All files are valid ZIP archives containing properly structured XML:

```bash
# Verify file types
file test-corpus/apple-pages/*.pages
# Output: Zip archive data, at least v2.0 to extract

# List contents
unzip -l test-corpus/apple-pages/proposal.pages
# Output: index.xml, QuickLook/, Data/

# Extract and view XML
unzip -p test-corpus/apple-pages/proposal.pages index.xml
```

## Notes

1. **Simplified XML**: These files use simplified XML structures that capture the essential elements of iWork formats without all the complexity of real-world files (themes, styles, fonts, media embeddings).

2. **No Real Media**: Thumbnail files are placeholders, not actual images.

3. **Basic Namespaces**: Uses standard Apple namespaces but may not include all attributes present in files created by actual iWork applications.

4. **Testing Focus**: Designed to test basic parsing capabilities:
   - ZIP archive extraction
   - XML parsing
   - Text content extraction
   - Structure identification (headings, lists, tables, slides)

5. **Future Enhancements**: If real-world iWork files become available, they should be added to supplement these synthetic test files.

## Limitations

These synthetic files may not expose edge cases found in real-world documents:
- Complex styling and formatting
- Embedded images and media
- Custom templates and themes
- Advanced formulas (Numbers)
- Animations and transitions (Keynote)
- Track changes and comments

For comprehensive testing, real-world iWork files from various sources and versions would be ideal.

## Source Creation Method

Files were created using:
1. Manual XML authoring based on Apple's iWork format specifications
2. ZIP compression to create valid .pages/.numbers/.key archives
3. Verification with `file` and `unzip` commands

Created by: Claude Code AI Agent
Date: 2025-11-08
