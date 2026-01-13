//! FFI bindings to PDFium
//!
//! This crate provides low-level bindings to the PDFium library.
//! PDFium is a PDF rendering library developed by Google as part of the Chromium project.
//!
//! # High-Level API (N=50)
//!
//! For ergonomic rendering with pixel format options, use the high-level wrapper:
//!
//! ```no_run
//! use pdfium_sys::{PixelFormat, RenderOptions};
//!
//! let options = RenderOptions {
//!     dpi: 150.0,
//!     thread_count: 8,
//!     pixel_format: PixelFormat::Gray,  // 75% less memory for ML
//! };
//! ```

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

// Include generated bindings
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

// ========================================
// High-Level API (N=50)
// ========================================

/// Output pixel format for rendered bitmaps.
///
/// # Memory Savings
/// - `Bgrx` (default): 4 bytes/pixel - backward compatible
/// - `Bgr`: 3 bytes/pixel - 25% less memory
/// - `Gray`: 1 byte/pixel - 75% less memory (ideal for ML pipelines)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum PixelFormat {
    /// BGRx format: 4 bytes per pixel (Blue, Green, Red, unused).
    /// Default format, backward compatible with all rendering code.
    #[default]
    Bgrx = 0,

    /// BGR format: 3 bytes per pixel (Blue, Green, Red).
    /// 25% less memory bandwidth than BGRx.
    Bgr = 1,

    /// Grayscale format: 1 byte per pixel.
    /// 75% less memory, ideal for ML/OCR pipelines that don't need color.
    Gray = 2,
}

impl PixelFormat {
    /// Returns bytes per pixel for this format.
    pub fn bytes_per_pixel(&self) -> usize {
        match self {
            PixelFormat::Bgrx => 4,
            PixelFormat::Bgr => 3,
            PixelFormat::Gray => 1,
        }
    }

    /// Converts to the FFI constant value.
    pub fn to_ffi(&self) -> i32 {
        *self as i32
    }
}

/// Rendering options for parallel page rendering.
///
/// # Example
/// ```no_run
/// use pdfium_sys::{PixelFormat, RenderOptions};
///
/// // Default options (300 DPI, 4 threads, BGRx)
/// let default_opts = RenderOptions::default();
///
/// // Custom options for ML (lower DPI, grayscale)
/// let ml_opts = RenderOptions {
///     dpi: 150.0,
///     thread_count: 8,
///     pixel_format: PixelFormat::Gray,
/// };
/// ```
#[derive(Debug, Clone)]
pub struct RenderOptions {
    /// Render DPI (dots per inch). Default: 300.0
    pub dpi: f64,

    /// Number of render threads. Default: 4
    /// Use 0 for auto-detection based on CPU cores.
    pub thread_count: i32,

    /// Output pixel format. Default: Bgrx
    pub pixel_format: PixelFormat,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            dpi: 300.0,
            thread_count: 4,
            pixel_format: PixelFormat::Bgrx,
        }
    }
}

impl RenderOptions {
    /// Create new render options with specified DPI.
    pub fn with_dpi(dpi: f64) -> Self {
        Self {
            dpi,
            ..Default::default()
        }
    }

    /// Create options optimized for web display (150 DPI, smaller output).
    pub fn web_preset() -> Self {
        Self {
            dpi: 150.0,
            thread_count: 4,
            pixel_format: PixelFormat::Bgrx,
        }
    }

    /// Create options optimized for ML pipelines (grayscale, lower DPI).
    pub fn ml_preset() -> Self {
        Self {
            dpi: 150.0,
            thread_count: 8,
            pixel_format: PixelFormat::Gray,
        }
    }

    /// Create options optimized for thumbnails (72 DPI, small output).
    pub fn thumbnail_preset() -> Self {
        Self {
            dpi: 72.0,
            thread_count: 4,
            pixel_format: PixelFormat::Bgrx,
        }
    }

    /// Convert to FFI options struct for use with FPDF_RenderPagesParallelV2.
    pub fn to_ffi(&self) -> FPDF_PARALLEL_OPTIONS {
        FPDF_PARALLEL_OPTIONS {
            worker_count: self.thread_count,
            max_queue_size: 0,
            form_handle: std::ptr::null_mut(),
            dpi: self.dpi,
            output_format: self.pixel_format.to_ffi(),
            reserved: [std::ptr::null_mut(); 1],
        }
    }
}

/// Information about a rendered page.
#[derive(Debug, Clone)]
pub struct RenderedPage {
    /// Page index (0-based).
    pub page_index: i32,

    /// Width in pixels.
    pub width: i32,

    /// Height in pixels.
    pub height: i32,

    /// Row stride in bytes (may be larger than width * bytes_per_pixel for alignment).
    pub stride: i32,

    /// Raw pixel data.
    pub data: Vec<u8>,

    /// Pixel format of the data.
    pub format: PixelFormat,
}

impl RenderedPage {
    /// Returns bytes per pixel for this page's format.
    pub fn bytes_per_pixel(&self) -> usize {
        self.format.bytes_per_pixel()
    }

    /// Returns total data size in bytes.
    pub fn data_size(&self) -> usize {
        self.data.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fpdf_init() {
        unsafe {
            FPDF_InitLibrary();
            FPDF_DestroyLibrary();
        }
    }

    #[test]
    fn test_pixel_format() {
        assert_eq!(PixelFormat::Bgrx.bytes_per_pixel(), 4);
        assert_eq!(PixelFormat::Bgr.bytes_per_pixel(), 3);
        assert_eq!(PixelFormat::Gray.bytes_per_pixel(), 1);

        assert_eq!(PixelFormat::Bgrx.to_ffi(), FPDF_PARALLEL_FORMAT_BGRx as i32);
        assert_eq!(PixelFormat::Bgr.to_ffi(), FPDF_PARALLEL_FORMAT_BGR as i32);
        assert_eq!(PixelFormat::Gray.to_ffi(), FPDF_PARALLEL_FORMAT_GRAY as i32);
    }

    #[test]
    fn test_render_options() {
        let opts = RenderOptions::default();
        assert_eq!(opts.dpi, 300.0);
        assert_eq!(opts.thread_count, 4);
        assert_eq!(opts.pixel_format, PixelFormat::Bgrx);

        let ffi = opts.to_ffi();
        assert_eq!(ffi.dpi, 300.0);
        assert_eq!(ffi.worker_count, 4);
        assert_eq!(ffi.output_format, 0);
    }

    #[test]
    fn test_presets() {
        let ml = RenderOptions::ml_preset();
        assert_eq!(ml.pixel_format, PixelFormat::Gray);

        let web = RenderOptions::web_preset();
        assert_eq!(web.dpi, 150.0);

        let thumb = RenderOptions::thumbnail_preset();
        assert_eq!(thumb.dpi, 72.0);
    }
}
