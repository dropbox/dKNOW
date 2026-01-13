# Tools and Software Available for Testing

**User:** "Worker is free to open any app to open documents and get images"
**User:** "What software do you need?"

---

## ✅ TOOLS YOU HAVE (Already Installed)

### Document Processing
- **LibreOffice** (`/opt/homebrew/bin/soffice`) - Open/convert Office docs
- **macOS Preview** - View PDFs, images, documents
- **macOS Quick Look** - Preview any file format
- **Finder** - Open documents in default applications

### Image Tools
- **Preview** - View and extract images from documents
- **screencapture** - Screenshot any open document
- **sips** - Scriptable image processing (convert, resize)
- **ImageMagick** (if installed) - Advanced image manipulation

### PDF Tools
- **LibreOffice** - Can convert anything to PDF
- **Preview** - View PDFs, extract pages
- **pdftoppm** (poppler-utils) - PDF to PNG/JPEG
- **pdfimages** (poppler-utils) - Extract images from PDFs

### Development Tools
- **Rust/Cargo** - Build and test
- **Git** - Version control
- **Python** - Baseline validation (optional)

---

## 🛠️ TOOLS YOU CAN USE

### Method 1: LibreOffice Headless

**What it does:** Convert any document to PDF/HTML/images
```bash
# DOCX → PDF
soffice --headless --convert-to pdf document.docx

# PPTX → PDF
soffice --headless --convert-to pdf presentation.pptx

# Any format → HTML
soffice --headless --convert-to html document.docx
```

**Use for:**
- Ground truth rendering
- Visual comparison baseline
- Image extraction (save as HTML, extract images)

---

### Method 2: macOS Native Apps

**Preview.app:**
```bash
# Open document visually
open -a Preview document.docx

# Extract images from PDF
# Open PDF in Preview, File → Export → Images
```

**Quick Look:**
```bash
# Quick preview
qlmanage -p document.docx

# Generate thumbnail
qlmanage -t -s 1000 document.docx -o /tmp/
```

**Screenshots:**
```bash
# Screenshot open document
screencapture -w screenshot.png  # Click window
```

---

### Method 3: Image Extraction Tools

**From PDF:**
```bash
# Extract images from PDF
pdfimages -all document.pdf output_prefix

# Convert PDF pages to images
pdftoppm -png document.pdf output_prefix
```

**From DOCX:**
```bash
# DOCX is a ZIP - extract directly
unzip -j document.docx "word/media/*" -d images/

# Lists all images in DOCX
unzip -l document.docx | grep "word/media"
```

**From HTML:**
```bash
# Extract image URLs
grep -o 'src="[^"]*"' document.html

# Download images
wget -r -l1 -H -nd -A jpg,png,gif document.html
```

---

### Method 4: Comparison Tools

**Visual Diff:**
```bash
# Compare two images
compare original.png our_output.png diff.png  # ImageMagick

# Side-by-side
open -a Preview original.pdf our_output.pdf
```

**Text Diff:**
```bash
# Detailed diff
diff -u original.md our_output.md

# Side-by-side
diff -y original.md our_output.md | less
```

---

## 📋 WHAT SOFTWARE YOU NEED (To Install)

### For Better Visual Testing

**1. ImageMagick** (image manipulation)
```bash
brew install imagemagick

# Use for: compare, convert, identify images
```

**2. Poppler Utils** (PDF tools)
```bash
brew install poppler

# Provides: pdftoppm, pdfimages, pdfinfo
```

**3. Headless Chrome/Chromium** (HTML rendering)
```bash
brew install chromium

# Use for: HTML → PDF with accurate rendering
chromium --headless --print-to-pdf=output.pdf input.html
```

**4. ImageMagick for Visual Diff**
```bash
# Compare two PDFs visually
convert original.pdf original.png
convert output.pdf output.png
compare -metric RMSE original.png output.png diff.png
```

---

## 🎯 HOW TO USE FOR QUALITY TESTING

### Extract Images from DOCX

**Method 1: Unzip**
```bash
unzip -j test.docx "word/media/*" -d /tmp/images/
ls /tmp/images/  # All images from document
```

**Method 2: LibreOffice**
```bash
# Convert to HTML (images embedded or extracted)
soffice --headless --convert-to html test.docx
# Images in test_html_files/
```

**Method 3: macOS Preview**
```bash
# Open DOCX, manually save images
open -a Preview test.docx
# File → Export → Select images
```

---

### Generate Visual Ground Truth

**For any format:**
```bash
# 1. Open in native application
open test.docx  # Opens in Word/LibreOffice

# 2. Print to PDF
# File → Print → Save as PDF

# 3. Screenshot
screencapture -w groundtruth.png  # Click on document window
```

**For our output:**
```bash
# 1. Parse to markdown
cargo run --bin docling test.docx > output.md

# 2. Convert markdown to HTML
# (Use pandoc or markdown library)

# 3. Render HTML
open output.html  # Or use headless Chrome

# 4. Screenshot
screencapture -w our_output.png
```

**Compare screenshots:**
```bash
open groundtruth.png our_output.png
# Visual inspection
```

---

### Extract Formatting Information

**DOCX XML:**
```bash
# DOCX is ZIP - extract XML
unzip test.docx -d /tmp/docx_contents/

# View document XML
cat /tmp/docx_contents/word/document.xml | xmllint --format -

# Find formatting tags
grep -E "<w:b/>|<w:i/>|<w:tbl>" /tmp/docx_contents/word/document.xml
```

**Check if bold/italic present:**
```bash
# If XML has <w:b/> but our markdown doesn't have **
# → Parser bug: Not extracting formatting

# If DocItems have formatting but markdown doesn't
# → Serializer bug: Not outputting formatting
```

---

## 🎯 RECOMMENDATION FOR WORKER

**You have these tools - USE THEM:**

1. **Visual Inspection**
   - Open original DOCX in LibreOffice/Preview
   - Open our markdown output (rendered to HTML)
   - Compare side-by-side
   - Document visual differences

2. **Image Extraction**
   - Unzip DOCX to see embedded images
   - Check if our parser extracts them
   - If not, fix parser to extract from word/media/

3. **Format Comparison**
   - Check DOCX XML for bold/italic tags
   - Check if our markdown has **bold** and *italic*
   - If not, add to parser/serializer

4. **Table Verification**
   - Check DOCX XML for <w:tbl>
   - Check if our markdown has table syntax
   - If not, fix table extraction

5. **Automated Visual Testing**
   - Use visual LLM tests (now working!)
   - Document quality scores
   - Fix issues LLM identifies
   - Re-test until 100%

---

## TOOLS NEEDED (Install These)

```bash
brew install imagemagick    # Visual diff
brew install poppler        # PDF tools (pdftoppm, pdfimages)
brew install chromium       # Better HTML rendering
```

**With these:** Can do comprehensive visual validation

---

**Worker: You have LibreOffice, Preview, screencapture, and many tools. USE THEM to verify quality visually, extract images, compare outputs. Don't just rely on text tests!**
