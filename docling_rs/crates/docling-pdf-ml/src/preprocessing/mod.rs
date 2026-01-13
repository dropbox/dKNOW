//! # Preprocessing - Image Preprocessing for ML Models
//!
//! This module contains image preprocessing utilities for preparing inputs
//! to various ML models in the pipeline. Each preprocessing module is tailored
//! to the specific requirements of its corresponding model.
//!
//! ## Preprocessing Modules
//!
//! ### Layout Preprocessing
//! - **Module:** [`layout`]
//! - **Purpose:** Prepares page images for layout detection model
//! - **Operations:**
//!   - Resize to 640×640 (maintaining aspect ratio)
//!   - Pad to square (if needed)
//!   - Normalize to [0.0, 1.0] range
//!   - Convert HWC → CHW (channel-first format)
//! - **Output:** 1×3×640×640 tensor (f32)
//!
//! ### RapidOCR Preprocessing
//! - **Module:** [`rapidocr`]
//! - **Purpose:** Prepares text region images for OCR models
//! - **Models:** DbNet (detection), AngleNet (classification), CrnnNet (recognition)
//! - **Operations:**
//!   - Resize with aspect ratio preservation
//!   - Normalize to model-specific mean/std
//!   - Convert to channel-first format
//! - **Note:** Each OCR model has different preprocessing requirements
//!
//! ### TableFormer Preprocessing
//! - **Module:** [`tableformer`]
//! - **Purpose:** Prepares table region images for table structure analysis
//! - **Operations:**
//!   - Scale table region to fixed height (maintains aspect ratio)
//!   - Normalize to ImageNet statistics
//!   - Convert to channel-first format
//! - **Output:** 1×3×H×W tensor (variable width)
//!
//! ### PIL-Compatible Resize
//! - **Modules:** [`pil_resize`], [`pil_resize_fixed_point`]
//! - **Purpose:** Python PIL-compatible image resizing for exact baseline matching
//! - **Implementations:**
//!   - `pil_resize`: Floating-point implementation (exact match with Python)
//!   - `pil_resize_fixed_point`: Integer-only implementation (for embedded systems)
//! - **Filters:** Bilinear, Bicubic, Lanczos3 (all PIL-compatible)
//!
//! ## Usage Example
//!
//! ```ignore
//! // NOTE: Preprocessing is internal API, use Pipeline instead
//! use docling_pdf_ml::preprocessing::layout::layout_preprocess;
//! use ndarray::Array3;
//!
//! // Layout preprocessing (internal API)
//! let page_image: Array3<u8> = Array3::zeros((792, 612, 3)); // HWC format
//! let preprocessed = layout_preprocess(&page_image);
//! // preprocessed: 1×3×640×640 tensor ready for layout model
//! ```
//!
//! ## Precision Requirements
//!
//! All preprocessing modules are designed to match Python baselines **exactly**:
//! - Floating-point operations match NumPy/PIL behavior
//! - Integer operations use same rounding modes as Python
//! - Resize filters are bit-exact with PIL (Pillow)
//!
//! This precision is critical for:
//! - Validating ML model outputs against Python baselines
//! - Ensuring consistent results across platforms
//! - Debugging preprocessing-related issues

pub mod layout;
pub mod pil_resize;
pub mod pil_resize_fixed_point;
pub mod rapidocr;
pub mod tableformer;
