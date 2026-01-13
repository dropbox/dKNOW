# docling-ocr

Optical Character Recognition (OCR) support for docling-rs.

## Overview

`docling-ocr` provides OCR (Optical Character Recognition) integration for extracting text from images and scanned documents. This crate serves as the foundation for OCR functionality in docling-rs, enabling text extraction from:

- Scanned PDFs
- Images (PNG, JPEG, TIFF, etc.)
- Document images embedded in other formats
- Screenshots and photos of documents

## Status

**Current Status:** Placeholder

This crate is reserved for future implementation of OCR integration. Currently, OCR functionality in docling-rs is handled through the Python bridge to the original docling library, which uses:

- **Tesseract OCR:** Industry-standard open-source OCR engine
- **EasyOCR:** Deep learning-based OCR for multiple languages
- **PaddleOCR:** High-performance OCR with multilingual support

## Current Usage (Hybrid Mode)

To use OCR features today, use docling-backend with the hybrid Python bridge:

```rust
use docling_backend::{BackendOptions, DocumentConverter};  // Note: DocumentConverter is in docling-backend crate

// Enable OCR for scanned PDFs
let options = BackendOptions {
    enable_ocr: true,
    ..Default::default()
};

let converter = DocumentConverter::new(options)?;
let result = converter.convert("scanned_document.pdf")?;
```

## Future Roadmap

When native Rust OCR backends are implemented (Phase I), this crate will provide:

### Planned Features

- **OCR Engine Integration:**
  - Tesseract OCR bindings
  - Deep learning OCR models (ONNX)
  - Multiple OCR engine support
- **Text Detection:**
  - Text region detection
  - Line and word segmentation
  - Character bounding boxes
- **Language Support:**
  - Multi-language OCR
  - Language detection
  - Custom trained models
- **Image Preprocessing:**
  - Deskewing and dewarping
  - Noise reduction
  - Contrast enhancement
  - Binarization
- **Performance:**
  - Parallel page processing
  - GPU acceleration (when available)
  - Caching and optimization

### Example (Future API)

```rust
use docling_ocr::{OcrEngine, OcrConfig, Language};

// Create OCR engine
let config = OcrConfig {
    languages: vec![Language::English, Language::German],
    enable_preprocessing: true,
    ..Default::default()
};
let engine = OcrEngine::new(config)?;

// Extract text from image
let text = engine.extract_text("scanned_page.png")?;
println!("Extracted text: {}", text);

// Get detailed results with bounding boxes
let results = engine.detect_text("scanned_page.png")?;
for word in results.words {
    println!("Word: {} at {:?}", word.text, word.bbox);
}
```

## OCR Engines

### Tesseract

Tesseract is the industry-standard open-source OCR engine:

- **Languages:** 100+ languages supported
- **Accuracy:** High accuracy for printed text
- **Performance:** Fast on modern CPUs
- **License:** Apache 2.0

### Deep Learning Models

Modern deep learning OCR models offer:

- **Higher Accuracy:** Better for complex layouts
- **Handwriting Support:** Can recognize handwritten text
- **Multi-script:** Handle mixed scripts in one document
- **GPU Acceleration:** Faster with GPU support

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
docling-ocr = "2.58.0"
```

## Related Crates

- **docling-core:** Main library with document processing API
- **docling-backend:** Format-specific backend implementations (includes OCR)
- **docling-py:** Python bridge for OCR access (current implementation)
- **docling-models:** ML model types for OCR models

## Performance Considerations

### OCR is Compute-Intensive

OCR is significantly slower than text extraction from native PDFs:

- **Native PDF text:** ~1s per document
- **OCR (Tesseract):** ~5-20s per page
- **OCR (Deep Learning):** ~10-30s per page (CPU), ~1-3s per page (GPU)

### Optimization Strategies

When using OCR:

1. **Detect if OCR is needed:** Check if PDF already contains text
2. **Parallel processing:** Process pages in parallel
3. **Selective OCR:** Only OCR pages that need it
4. **Image preprocessing:** Improve quality before OCR
5. **GPU acceleration:** Use GPU if available for deep learning models

## License

Licensed under the MIT License. See LICENSE file for details.

## Contributing

This crate is part of the docling-rs project. For contribution guidelines, see the main repository.

## References

- **Tesseract OCR:** https://github.com/tesseract-ocr/tesseract
- **Python docling:** https://github.com/docling-project/docling
- **docling-rs repository:** https://github.com/dropbox/dKNOW/docling_rs

## Note on OCR Integration

The docling-rs project uses a **hybrid approach** for OCR functionality:

**Current (Phase H):**
- OCR handled by Python docling library (Tesseract/EasyOCR)
- Rust handles document parsing and serialization
- Enable with `enable_ocr: true` in backend options

**Future (Phase I):**
- Native Rust OCR integration (tesseract-rs, ONNX models)
- Eliminate Python dependency for OCR
- Full control over OCR pipeline

This crate will be populated during Phase I when native Rust OCR backends are implemented.

## Testing

To test OCR functionality today, use the integration tests with OCR-enabled PDFs:

```bash
# Run OCR tests (requires Python docling)
USE_HYBRID_SERIALIZER=1 cargo test test_canon_pdf_ocr -- --nocapture
```

Example OCR test files in the test corpus:
- `amt_handbook_sample.pdf` - Scanned aviation handbook
- Other scanned PDFs in `test-corpus/pdf/`
