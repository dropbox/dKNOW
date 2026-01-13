# Troubleshooting Guide

This guide covers common issues, their root causes, and solutions when using docling-rs.

---

## Table of Contents

1. [Installation Issues](#installation-issues)
2. [OCR Problems](#ocr-problems)
3. [Test Failures](#test-failures)
4. [Performance Issues](#performance-issues)
5. [Format-Specific Issues](#format-specific-issues)
6. [Debugging Techniques](#debugging-techniques)

---

## Installation Issues

### pdfium Library Not Found

**Error:**
```
Failed to load pdfium library: Library not found
```

**Cause:** pdfium binary not installed or not in library path

**Solution:**

**macOS:**
```bash
# Install via Homebrew
brew install pdfium

# Set library path
export DYLD_LIBRARY_PATH=/opt/homebrew/lib:$DYLD_LIBRARY_PATH
```

**Linux:**
```bash
# Install system library
sudo apt-get install libpdfium-dev  # Debian/Ubuntu
sudo dnf install pdfium-devel        # Fedora

# Set library path
export LD_LIBRARY_PATH=/usr/local/lib:$LD_LIBRARY_PATH
```

**Windows:**
```powershell
# Download pdfium binary from chromium project and add to PATH
```

---

### PyTorch/libtorch Not Found

**Error:**
```
Error:
Cannot find a libtorch install, you can either:
- Install libtorch manually and set the LIBTORCH environment variable to appropriate path.
- Use a system wide install in /usr/lib/libtorch.so.
- Use a Python environment with PyTorch installed by setting LIBTORCH_USE_PYTORCH=1
```

**Cause:** Building with the `pytorch` feature requires a PyTorch/libtorch installation

**Solution:**

**Option 1: Use the setup script (recommended):**
```bash
# Source the environment setup script
source setup_env.sh

# Build with PDF ML support
cargo build --release -p docling-cli
```

**Option 2: Use existing Python PyTorch:**
```bash
# If you have PyTorch installed in your Python environment
export LIBTORCH_USE_PYTORCH=1
cargo build --release -p docling-cli
```

**Option 3: Use ONNX backend (no PyTorch required):**
```bash
# Build with ONNX-only backend - no PyTorch dependency
cargo build --release -p docling-cli --no-default-features --features pdf-ml-onnx
```

---

### ML Models Not Found

**Error:**
```
Failed to load model: File not found
```

**Cause:** PyTorch/ONNX model weights not downloaded

**Solution:**
```bash
# Set up environment for PyTorch C++ backend
source setup_env.sh

# Build with PDF ML support
cargo build --features pdf-ml

# Models are automatically downloaded on first use
# Or manually set cache directory:
export PDF_ML_CACHE_DIR=~/.cache/docling-models
```

---

### LibreOffice Not Found (DOC Conversion)

**Error:**
```
Failed to convert DOC to DOCX: soffice not found
```

**Cause:** LibreOffice not installed (required for legacy DOC format)

**Solution:**

**macOS:**
```bash
brew install --cask libreoffice
```

**Linux:**
```bash
sudo apt-get install libreoffice  # Debian/Ubuntu
sudo dnf install libreoffice      # Fedora
```

**Windows:**
```
Download from: https://www.libreoffice.org/download/download/
```

---

## OCR Problems

### OCR Stability Improvements

**Historical Issue:** OCR tests previously showed non-deterministic behavior

**Status (as of N=300):**
- All OCR canonical tests now pass (13/13, 100%)
- OCR stability improvements implemented
- Output consistency achieved across runs

**If You Experience OCR Variations:**

**Solution 1: Improve Input Quality**
```bash
# Increase image resolution (300+ DPI recommended)
# Enhance contrast and deskew images
# Remove noise and artifacts
```

**Solution 2: Use ONNX Backend**
```bash
# ONNX backend uses different models and may produce different results
export ONNX_BACKEND=1
cargo build --features pdf-ml
```

**Solution 3: Disable OCR for Text PDFs**
```rust
use docling_backend::DocumentConverter;

// Create converter without OCR (default)
let converter = DocumentConverter::new()?;
```

**See Also:**
- [Format Support Matrix - OCR Support](FORMATS.md#ocr-support)

---

### OCR Producing Garbled Text

**Symptom:** OCR output contains nonsense characters

**Example:**
```
blue preactions by Iavierormer
2
O Ohsrier 1
amalignal benign malignant
```

**Cause:** Low-quality image, rotated text, or complex layout

**Solution:**

1. **Pre-process images:**
```bash
# Increase resolution
convert input.png -density 300 output.png

# Rotate if needed
convert input.png -rotate 90 output.png
```

2. **Use ONNX OCR models:**
```bash
# ONNX backend includes PaddleOCR models for accurate recognition
export ONNX_BACKEND=1
cargo build --features pdf-ml
```

3. **Check model weights are properly loaded**

---

### OCR Not Running (Images Return Empty)

**Symptom:** Image files convert to empty markdown

**Cause:** OCR disabled or not configured

**Solution:**
```rust
// Note: DocumentConverter is in docling-backend crate
use docling_backend::DocumentConverter;

// Enable OCR explicitly
let converter = DocumentConverter::with_ocr(true)?;
let result = converter.convert("image.png")?;
```

**Verify OCR is working:**
```bash
# Test with known image file
cargo test test_canon_png -- --nocapture
# Should show text extracted from PNG
```

---

## Test Failures

### Canonical Tests Failing

**Symptom:**
```
test test_canon_pdf_2305_03393v1_text ... FAILED
Expected: 45256 chars
Actual:   45320 chars
```

**Expected Results:**
- **Non-OCR Tests:** 100% pass rate (all formats)
- **OCR Tests:** 100% pass rate (all OCR issues resolved as of N=300)
- **JATS Tests:** 100% pass rate (table width issues resolved)
- **Overall:** 100% pass rate (97/97 tests)

**If Different Pass Rate:**

1. **Verify test corpus exists:**
```bash
ls test-corpus/pdf/*.pdf
ls test-corpus/groundtruth/docling_v2/*.md
```

2. **Run single failing test with debug output:**
```bash
cargo test test_canon_pdf_2305_03393v1_text -- --exact --nocapture
```

---

### Table Width Differences (Historical Issue - Resolved)

**Historical Issue:** JATS tests previously failed due to table column padding differences

**Status (as of N=300):**
- All JATS canonical tests now pass (15/15, 100%)
- Table width padding issues resolved
- Markdown table formatting consistent with expected outputs

**If You Experience Table Formatting Issues:**

**Solution:** Report the specific test failure with the actual vs expected output

---

### Test Corpus Missing

**Symptom:**
```
Test file not found: test-corpus/pdf/2305.03393v1.pdf
```

**Cause:** Test corpus files not present (git-ignored, need to be set up)

**Solution:**
```bash
# Test corpus files should be in test-corpus/ directory
# Copy test files to appropriate directories:
mkdir -p test-corpus/{pdf,docx,html,pptx,xlsx}

# Add your own test documents
cp /path/to/your/documents/*.pdf test-corpus/pdf/

# Verify setup
cargo test test_canon_pdf -- --exact
```

**See:** [CLAUDE.md - Test Corpus Setup](../CLAUDE.md#test-corpus-setup-required-for-benchmarking)

---

### Thread Safety Crashes (SIGSEGV)

**Symptom:**
```
signal: 11, SIGSEGV: invalid memory reference
```

**Cause:** pdfium C library is not thread-safe

**Solution:** Run tests sequentially
```bash
cargo test -- --test-threads=1
```

**Why:** Parallel test execution causes race conditions in pdfium's internal state.

---

## Performance Issues

### Slow PDF Conversion

**Symptom:** PDF conversion takes 10+ seconds per page

**Likely Causes:**

1. **OCR is enabled:**
```rust
// Disable OCR for faster text extraction
let converter = DocumentConverter::new()?; // OCR disabled by default
```

2. **Large PDFs with images:**
```bash
# Check PDF size and complexity
pdfinfo file.pdf
# Look for: Pages, File size, Images
```

3. **Debug build:**
```bash
# Use release build (5-10x faster)
cargo build --release
./target/release/your_binary
```

**Expected Performance (Release Build):**
- Simple PDF (text only): 0.3-1.0s per document
- Complex PDF (tables, images): 1-3s per document
- With OCR: +5-15s per page

---

### High Memory Usage

**Symptom:** Process uses >1GB RAM for single document

**Causes:**

1. **Large PDF with many images**
2. **OCR processing (ML models in memory)**
3. **Multiple converters instantiated**

**Solutions:**

1. **Reuse converter:**
```rust
// GOOD: Reuse converter
let converter = DocumentConverter::new()?;
for file in files {
    converter.convert(&file)?;
}

// BAD: Create new converter per file
for file in files {
    let converter = DocumentConverter::new()?; // Initializes ML models each time!
    converter.convert(&file)?;
}
```

2. **Process in batches:**
```rust
// Process files in chunks of 100
for chunk in files.chunks(100) {
    let converter = DocumentConverter::new()?;
    for file in chunk {
        converter.convert(file)?;
    }
    drop(converter); // Free memory between chunks
}
```

3. **Disable OCR if not needed:**
```rust
let converter = DocumentConverter::new()?; // OCR disabled
```

---

## Format-Specific Issues

### DOCX: Missing Images

**Symptom:** Images in DOCX don't appear in markdown output

**Cause:** Image extraction produces base64 data but markdown export omits embedded images

**Workaround:** Extract images separately or use HTML export:
```bash
# Convert markdown to HTML with pandoc
docling convert input.docx | pandoc -f markdown -t html > output.html
```

---

### PDF: Missing Tables

**Symptom:** Tables render as plain text instead of markdown tables

**Cause:** Table detection disabled

**Solution:**
```rust
// Table structure detection is enabled by default
let converter = DocumentConverter::new()?;
// Tables should be detected automatically
```

**Verify:**
```bash
cargo test test_canon_pdf_multi_page_tables -- --exact --nocapture
# Should show markdown tables in output
```

---

### Archives: Not Extracting Contents

**Symptom:** ZIP file converts to empty output

**Cause:** Rust backend not enabled

**Solution:**
```bash
USE_RUST_BACKEND=1 cargo run -- input.zip
```

**Or in code:**
```rust
std::env::set_var("USE_RUST_BACKEND", "1");
let converter = DocumentConverter::new()?;
let result = converter.convert("archive.zip")?;
```

---

### HTML: Lost Formatting

**Symptom:** HTML converts but loses structure

**Cause:** Complex CSS layouts or JavaScript-rendered content

**Solution:**
1. **Pre-render JavaScript** (if dynamic content)
2. **Simplify HTML** (remove unnecessary CSS)
3. **Use readability mode** (future feature)

---

## Debugging Techniques

### Inspecting Test Outputs

When tests fail, outputs are saved to `test-results/`:

```bash
# Find latest test run
ls -lt test-results/outputs/

# Compare outputs
diff test-corpus/groundtruth/docling_v2/file.md test-results/outputs/pdf/file.txt

# View full output
cat test-results/outputs/pdf/file.txt | less
```

---

### Enabling Debug Logging

```rust
env_logger::init(); // Initialize logging

// In your code
log::debug!("Converting file: {:?}", path);
```

```bash
RUST_LOG=debug cargo test -- --nocapture
```

---

### Comparing Outputs

```bash
# Convert a document
cargo run -- test.pdf > actual_output.md

# Compare with expected output
diff expected_output.md actual_output.md

# For detailed character-level differences
diff --color=always expected_output.md actual_output.md | head -50
```

---

### Bisecting Test Failures

```bash
# Run single test
cargo test test_canon_pdf_specific_file -- --exact --nocapture

# Check first 200 characters (usually matches)
head -c 200 test-results/outputs/pdf/file.txt

# Find where difference starts
diff <(head -c 1000 expected.md) <(head -c 1000 actual.md)
```

---

### Profiling Performance

```bash
# Install flamegraph
cargo install flamegraph

# Profile specific test
cargo flamegraph --test integration_tests -- test_canon_pdf_2305 --exact

# Open flamegraph.svg in browser
open flamegraph.svg
```

---

## Getting Help

### Reporting Bugs

When reporting issues, include:

1. **Docling version:**
```bash
cargo --version
rustc --version
```

2. **Operating system:**
```bash
uname -a  # macOS/Linux
ver       # Windows
```

3. **Sample file** (if possible)

4. **Full error message:**
```bash
cargo test -- --nocapture 2>&1 | tee error.log
```

5. **Steps to reproduce**

### Known Issues

See [N=101 Test Failure Analysis](../reports/feature/phase-e-open-standards/n101_test_failure_analysis_2025-11-08.md) for comprehensive list of known issues.

---

## References

- **N=101 Report:** [Test Failure Analysis](../reports/feature/phase-e-open-standards/n101_test_failure_analysis_2025-11-08.md)
- **Format Support:** [FORMATS.md](FORMATS.md)
- **User Guide:** [USER_GUIDE.md](USER_GUIDE.md)
- **docling-rs issues:** https://github.com/ayates_dbx/docling_rs/issues

---

**Last Updated:** 2026-01-07 (N=4423)
