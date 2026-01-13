# docling-models

Machine learning model types for docling-rs document processing.

## Overview

`docling-models` provides type definitions and interfaces for machine learning models used in docling document processing. This crate serves as a foundation for ML-based features such as:

- Document layout analysis
- Table detection and extraction
- Optical Character Recognition (OCR)
- Text segmentation
- Content classification

## Status

**Current Status:** Placeholder

This crate is reserved for future implementation of ML model integration. Currently, ML functionality in docling-rs is handled through the Python bridge to the original docling library.

## Future Roadmap

When native Rust ML backends are implemented (Phase I), this crate will provide:

### Planned Features

- **Model Loading:** Load pre-trained models for document analysis
- **Inference Engine:** Run inference on document content
- **Model Types:**
  - Layout analysis models
  - Table detection models
  - OCR models
  - Text classification models
- **Type Definitions:**
  - Model configuration structures
  - Inference input/output types
  - Model metadata

### Example (Future API)

```rust
use docling_models::{LayoutModel, ModelConfig};

// Load a pre-trained layout analysis model
let config = ModelConfig::default();
let model = LayoutModel::load("path/to/model.onnx", config)?;

// Run inference on a page
let predictions = model.predict(&page_image)?;
```

## Current Usage

For ML-based document processing features, docling-rs currently uses the hybrid approach with the Python docling library. See:

- **docling-py:** Python bridge for hybrid Rust/Python processing
- **docling-backend:** Backend implementations using Python ML models
- **docling-core:** High-level API that integrates all processing backends

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
docling-models = "2.58.0"
```

## Related Crates

- **docling-core:** Main library with document processing API
- **docling-backend:** Format-specific backend implementations
- **docling-py:** Python bridge for ML model access
- **docling-ocr:** OCR integration (currently via Python)
- **docling-pipeline:** Document processing pipeline orchestration

## License

Licensed under the MIT License. See LICENSE file for details.

## Contributing

This crate is part of the docling-rs project. For contribution guidelines, see the main repository.

## References

- **Python docling:** https://github.com/docling-project/docling
- **docling-rs repository:** https://github.com/ayates_dbx/docling_rs

## Note on ML Integration

The docling-rs project uses a **hybrid approach** for ML functionality:

**Current (Phase H):**
- ML inference handled by Python docling library
- Rust handles parsing, serialization, and I/O
- Best of both worlds: battle-tested ML + Rust performance

**Future (Phase I):**
- Native Rust ML inference (ONNX runtime, tract, etc.)
- Eliminate Python dependency
- Full Rust stack for document processing

This crate will be populated during Phase I when native Rust ML backends are implemented.
