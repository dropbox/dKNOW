# Investigation Reports

Technical investigation reports for various features and optimizations.

## Documents

- **HEIF_HEIC_SUPPORT_INVESTIGATION.md**: HEIF/HEIC format support investigation (N=238)
  - Result: Partial support (individual tiles decode, full Tile Grid composition requires stream group support)

- **INT8_QUANTIZATION_INVESTIGATION.md**: INT8 quantization for pose estimation (N=?)
  - Result: Negative - no performance benefit, CoreML incompatible

- **UNSUPPORTED_FORMATS_RESEARCH.md**: Research on unsupported media formats (2025-11-01)
  - Comprehensive list of formats and their support status

- **OUTPUT_VALIDATION_FRAMEWORK.md**: Framework for validating plugin outputs (N=244)
  - Status: Implemented
