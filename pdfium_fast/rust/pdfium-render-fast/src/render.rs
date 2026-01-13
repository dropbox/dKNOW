//! Bitmap rendering and image output

use crate::error::{PdfError, Result};
use pdfium_sys::*;
use std::path::Path;

/// Pixel format for rendered bitmaps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PixelFormat {
    /// BGRA format (4 bytes per pixel) - default, fastest rendering
    #[default]
    Bgra,
    /// BGR format (3 bytes per pixel) - 25% memory savings
    Bgr,
    /// Grayscale format (1 byte per pixel) - 75% memory savings
    Gray,
}

impl PixelFormat {
    /// Get the number of bytes per pixel for this format.
    pub fn bytes_per_pixel(self) -> usize {
        match self {
            PixelFormat::Bgra => 4,
            PixelFormat::Bgr => 3,
            PixelFormat::Gray => 1,
        }
    }

    /// Convert to PDFium bitmap format constant.
    pub(crate) fn to_fpdf_format(self) -> i32 {
        match self {
            PixelFormat::Bgra => FPDFBitmap_BGRA as i32,
            PixelFormat::Bgr => FPDFBitmap_BGR as i32,
            PixelFormat::Gray => FPDFBitmap_Gray as i32,
        }
    }
}

/// Configuration for page rendering.
#[derive(Debug, Clone)]
pub struct PdfRenderConfig {
    dpi: f64,
    pixel_format: PixelFormat,
    /// Optional target width (overrides DPI)
    target_width: Option<u32>,
    /// Optional target height (overrides DPI)
    target_height: Option<u32>,
}

impl Default for PdfRenderConfig {
    fn default() -> Self {
        Self {
            dpi: 300.0,
            pixel_format: PixelFormat::Bgra,
            target_width: None,
            target_height: None,
        }
    }
}

impl PdfRenderConfig {
    /// Create a new render configuration with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the target DPI for rendering.
    ///
    /// Default: 300 DPI
    pub fn set_target_dpi(mut self, dpi: f64) -> Self {
        self.dpi = dpi;
        self.target_width = None;
        self.target_height = None;
        self
    }

    /// Set the pixel format for the output bitmap.
    ///
    /// Default: BGRA (4 bytes per pixel)
    pub fn set_pixel_format(mut self, format: PixelFormat) -> Self {
        self.pixel_format = format;
        self
    }

    /// Set a target width in pixels (height calculated from aspect ratio).
    pub fn set_target_width(mut self, width: u32) -> Self {
        self.target_width = Some(width);
        self.target_height = None;
        self
    }

    /// Set a target height in pixels (width calculated from aspect ratio).
    pub fn set_target_height(mut self, height: u32) -> Self {
        self.target_height = Some(height);
        self.target_width = None;
        self
    }

    /// Get the DPI setting.
    pub fn dpi(&self) -> f64 {
        self.dpi
    }

    /// Get the pixel format.
    pub fn pixel_format(&self) -> PixelFormat {
        self.pixel_format
    }

    /// Calculate output dimensions for a page.
    pub fn calculate_size(&self, page_width: f64, page_height: f64) -> (u32, u32) {
        if let Some(target_width) = self.target_width {
            let aspect = page_height / page_width;
            let height = (target_width as f64 * aspect).round() as u32;
            (target_width, height)
        } else if let Some(target_height) = self.target_height {
            let aspect = page_width / page_height;
            let width = (target_height as f64 * aspect).round() as u32;
            (width, target_height)
        } else {
            let scale = self.dpi / 72.0;
            let width = (page_width * scale).round() as u32;
            let height = (page_height * scale).round() as u32;
            (width, height)
        }
    }
}

/// A rendered bitmap from a PDF page.
pub struct PdfBitmap {
    handle: FPDF_BITMAP,
    width: u32,
    height: u32,
    format: PixelFormat,
}

// SAFETY: Bitmap handles are safe to send between threads
unsafe impl Send for PdfBitmap {}

impl PdfBitmap {
    pub(crate) fn new(handle: FPDF_BITMAP, width: u32, height: u32, format: PixelFormat) -> Self {
        Self {
            handle,
            width,
            height,
            format,
        }
    }

    /// Create a PdfBitmap from a raw handle (for image extraction).
    ///
    /// The format is determined from the PDFium bitmap format.
    /// This method takes ownership of the handle - it will be destroyed on drop.
    pub(crate) fn from_handle(handle: FPDF_BITMAP) -> Option<Self> {
        if handle.is_null() {
            return None;
        }

        let width = unsafe { FPDFBitmap_GetWidth(handle) } as u32;
        let height = unsafe { FPDFBitmap_GetHeight(handle) } as u32;
        let fpdf_format = unsafe { FPDFBitmap_GetFormat(handle) };

        let format = {
            let fmt = fpdf_format as u32;
            if fmt == FPDFBitmap_Gray {
                PixelFormat::Gray
            } else if fmt == FPDFBitmap_BGR {
                PixelFormat::Bgr
            } else {
                PixelFormat::Bgra // BGRA, BGRx, or unknown
            }
        };

        Some(Self {
            handle,
            width,
            height,
            format,
        })
    }

    /// Get the width of the bitmap in pixels.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Get the height of the bitmap in pixels.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Get the pixel format of the bitmap.
    pub fn format(&self) -> PixelFormat {
        self.format
    }

    /// Get the stride (bytes per row) of the bitmap.
    pub fn stride(&self) -> usize {
        unsafe { FPDFBitmap_GetStride(self.handle) as usize }
    }

    /// Get the raw pixel data.
    ///
    /// Returns a slice of the bitmap data. The format depends on the pixel format:
    /// - BGRA: 4 bytes per pixel (Blue, Green, Red, Alpha)
    /// - BGR: 3 bytes per pixel (Blue, Green, Red)
    /// - Gray: 1 byte per pixel
    pub fn data(&self) -> &[u8] {
        let stride = self.stride();
        let size = stride * self.height as usize;
        unsafe {
            let ptr = FPDFBitmap_GetBuffer(self.handle) as *const u8;
            std::slice::from_raw_parts(ptr, size)
        }
    }

    /// Get the pixel data as a Vec (copies the data).
    pub fn to_vec(&self) -> Vec<u8> {
        self.data().to_vec()
    }

    /// Convert BGRA to RGB data (copies and converts).
    pub fn to_rgb(&self) -> Vec<u8> {
        match self.format {
            PixelFormat::Bgra => {
                let stride = self.stride();
                let mut rgb = Vec::with_capacity((self.width * self.height * 3) as usize);
                let data = self.data();

                for y in 0..self.height as usize {
                    for x in 0..self.width as usize {
                        let offset = y * stride + x * 4;
                        // BGRA -> RGB
                        rgb.push(data[offset + 2]); // R
                        rgb.push(data[offset + 1]); // G
                        rgb.push(data[offset]); // B
                    }
                }
                rgb
            }
            PixelFormat::Bgr => {
                let stride = self.stride();
                let mut rgb = Vec::with_capacity((self.width * self.height * 3) as usize);
                let data = self.data();

                for y in 0..self.height as usize {
                    for x in 0..self.width as usize {
                        let offset = y * stride + x * 3;
                        // BGR -> RGB
                        rgb.push(data[offset + 2]); // R
                        rgb.push(data[offset + 1]); // G
                        rgb.push(data[offset]); // B
                    }
                }
                rgb
            }
            PixelFormat::Gray => {
                let stride = self.stride();
                let mut rgb = Vec::with_capacity((self.width * self.height * 3) as usize);
                let data = self.data();

                for y in 0..self.height as usize {
                    for x in 0..self.width as usize {
                        let offset = y * stride + x;
                        let gray = data[offset];
                        rgb.push(gray);
                        rgb.push(gray);
                        rgb.push(gray);
                    }
                }
                rgb
            }
        }
    }

    /// Save the bitmap as a PNG file.
    ///
    /// # Arguments
    ///
    /// * `path` - Output file path
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    /// let bitmap = page.render()?;
    ///
    /// bitmap.save_as_png("page.png")?;
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn save_as_png<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        use png::{BitDepth, ColorType, Encoder};
        use std::fs::File;
        use std::io::BufWriter;

        let file = File::create(path)?;
        let writer = BufWriter::new(file);

        let (color_type, data) = match self.format {
            PixelFormat::Bgra => {
                // Convert BGRA to RGBA
                let stride = self.stride();
                let src = self.data();
                let mut rgba = Vec::with_capacity((self.width * self.height * 4) as usize);

                for y in 0..self.height as usize {
                    for x in 0..self.width as usize {
                        let offset = y * stride + x * 4;
                        rgba.push(src[offset + 2]); // R
                        rgba.push(src[offset + 1]); // G
                        rgba.push(src[offset]); // B
                        rgba.push(src[offset + 3]); // A
                    }
                }
                (ColorType::Rgba, rgba)
            }
            PixelFormat::Bgr => {
                // Convert BGR to RGB
                (ColorType::Rgb, self.to_rgb())
            }
            PixelFormat::Gray => {
                let stride = self.stride();
                let src = self.data();
                let mut gray = Vec::with_capacity((self.width * self.height) as usize);

                for y in 0..self.height as usize {
                    for x in 0..self.width as usize {
                        gray.push(src[y * stride + x]);
                    }
                }
                (ColorType::Grayscale, gray)
            }
        };

        let mut encoder = Encoder::new(writer, self.width, self.height);
        encoder.set_color(color_type);
        encoder.set_depth(BitDepth::Eight);

        let mut png_writer = encoder
            .write_header()
            .map_err(|e| PdfError::PngEncoding(e.to_string()))?;

        png_writer
            .write_image_data(&data)
            .map_err(|e| PdfError::PngEncoding(e.to_string()))?;

        Ok(())
    }

    /// Save the bitmap as a JPEG file.
    ///
    /// # Arguments
    ///
    /// * `path` - Output file path
    /// * `quality` - JPEG quality (1-100, default 85)
    pub fn save_as_jpeg<P: AsRef<Path>>(&self, path: P, quality: u8) -> Result<()> {
        use jpeg_encoder::{ColorType as JpegColorType, Encoder};
        use std::fs::File;
        use std::io::BufWriter;

        let rgb = self.to_rgb();
        let file = File::create(path)?;
        let writer = BufWriter::new(file);

        let encoder = Encoder::new(writer, quality);
        encoder
            .encode(
                &rgb,
                self.width as u16,
                self.height as u16,
                JpegColorType::Rgb,
            )
            .map_err(|e| PdfError::JpegEncoding(e.to_string()))?;

        Ok(())
    }

    /// Save the bitmap as PPM (Portable Pixmap) format.
    ///
    /// PPM is used for baseline comparisons with upstream pdfium_test.
    pub fn save_as_ppm<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        use std::fs::File;
        use std::io::{BufWriter, Write};

        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        // P6 header (binary PPM)
        writeln!(writer, "P6")?;
        writeln!(writer, "# PDF test render")?;
        writeln!(writer, "{} {}", self.width, self.height)?;
        writeln!(writer, "255")?;

        // RGB data
        let rgb = self.to_rgb();
        writer.write_all(&rgb)?;

        Ok(())
    }
}

impl Drop for PdfBitmap {
    fn drop(&mut self) {
        unsafe {
            FPDFBitmap_Destroy(self.handle);
        }
    }
}

/// A rendered page from parallel rendering.
///
/// Contains the pixel data and metadata for a rendered page.
#[derive(Debug, Clone)]
pub struct RenderedPage {
    /// Page index (0-based).
    pub page_index: usize,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Row stride in bytes.
    pub stride: u32,
    /// Raw pixel data.
    pub data: Vec<u8>,
    /// Pixel format.
    pub format: PixelFormat,
}

impl RenderedPage {
    /// Create a new RenderedPage.
    pub fn new(
        page_index: usize,
        width: u32,
        height: u32,
        stride: u32,
        data: Vec<u8>,
        format: PixelFormat,
    ) -> Self {
        Self {
            page_index,
            width,
            height,
            stride,
            data,
            format,
        }
    }

    /// Get the number of bytes per pixel for this format.
    pub fn bytes_per_pixel(&self) -> usize {
        self.format.bytes_per_pixel()
    }

    /// Get the total data size in bytes.
    pub fn data_size(&self) -> usize {
        self.data.len()
    }

    /// Convert to RGB data (copies and converts).
    pub fn to_rgb(&self) -> Vec<u8> {
        match self.format {
            PixelFormat::Bgra => {
                let mut rgb = Vec::with_capacity((self.width * self.height * 3) as usize);
                for y in 0..self.height as usize {
                    for x in 0..self.width as usize {
                        let offset = y * self.stride as usize + x * 4;
                        // BGRA -> RGB
                        rgb.push(self.data[offset + 2]); // R
                        rgb.push(self.data[offset + 1]); // G
                        rgb.push(self.data[offset]); // B
                    }
                }
                rgb
            }
            PixelFormat::Bgr => {
                let mut rgb = Vec::with_capacity((self.width * self.height * 3) as usize);
                for y in 0..self.height as usize {
                    for x in 0..self.width as usize {
                        let offset = y * self.stride as usize + x * 3;
                        // BGR -> RGB
                        rgb.push(self.data[offset + 2]); // R
                        rgb.push(self.data[offset + 1]); // G
                        rgb.push(self.data[offset]); // B
                    }
                }
                rgb
            }
            PixelFormat::Gray => {
                let mut rgb = Vec::with_capacity((self.width * self.height * 3) as usize);
                for y in 0..self.height as usize {
                    for x in 0..self.width as usize {
                        let offset = y * self.stride as usize + x;
                        let gray = self.data[offset];
                        rgb.push(gray);
                        rgb.push(gray);
                        rgb.push(gray);
                    }
                }
                rgb
            }
        }
    }

    /// Save the rendered page as a PNG file.
    pub fn save_as_png<P: AsRef<std::path::Path>>(&self, path: P) -> Result<()> {
        use png::{BitDepth, ColorType, Encoder};
        use std::fs::File;
        use std::io::BufWriter;

        let file = File::create(path)?;
        let writer = BufWriter::new(file);

        let (color_type, data) = match self.format {
            PixelFormat::Bgra => {
                // Convert BGRA to RGBA
                let mut rgba = Vec::with_capacity((self.width * self.height * 4) as usize);
                for y in 0..self.height as usize {
                    for x in 0..self.width as usize {
                        let offset = y * self.stride as usize + x * 4;
                        rgba.push(self.data[offset + 2]); // R
                        rgba.push(self.data[offset + 1]); // G
                        rgba.push(self.data[offset]); // B
                        rgba.push(self.data[offset + 3]); // A
                    }
                }
                (ColorType::Rgba, rgba)
            }
            PixelFormat::Bgr => (ColorType::Rgb, self.to_rgb()),
            PixelFormat::Gray => {
                let mut gray = Vec::with_capacity((self.width * self.height) as usize);
                for y in 0..self.height as usize {
                    for x in 0..self.width as usize {
                        gray.push(self.data[y * self.stride as usize + x]);
                    }
                }
                (ColorType::Grayscale, gray)
            }
        };

        let mut encoder = Encoder::new(writer, self.width, self.height);
        encoder.set_color(color_type);
        encoder.set_depth(BitDepth::Eight);

        let mut png_writer = encoder
            .write_header()
            .map_err(|e| PdfError::PngEncoding(e.to_string()))?;

        png_writer
            .write_image_data(&data)
            .map_err(|e| PdfError::PngEncoding(e.to_string()))?;

        Ok(())
    }

    /// Save the rendered page as a JPEG file.
    pub fn save_as_jpeg<P: AsRef<std::path::Path>>(&self, path: P, quality: u8) -> Result<()> {
        use jpeg_encoder::{ColorType as JpegColorType, Encoder};
        use std::fs::File;
        use std::io::BufWriter;

        let rgb = self.to_rgb();
        let file = File::create(path)?;
        let writer = BufWriter::new(file);

        let encoder = Encoder::new(writer, quality);
        encoder
            .encode(
                &rgb,
                self.width as u16,
                self.height as u16,
                JpegColorType::Rgb,
            )
            .map_err(|e| PdfError::JpegEncoding(e.to_string()))?;

        Ok(())
    }

    /// Save the rendered page as a PPM file.
    pub fn save_as_ppm<P: AsRef<std::path::Path>>(&self, path: P) -> Result<()> {
        use std::fs::File;
        use std::io::{BufWriter, Write};

        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        // P6 header (binary PPM)
        writeln!(writer, "P6")?;
        writeln!(writer, "# PDF test render")?;
        writeln!(writer, "{} {}", self.width, self.height)?;
        writeln!(writer, "255")?;

        // RGB data
        let rgb = self.to_rgb();
        writer.write_all(&rgb)?;

        Ok(())
    }
}

/// Status of progressive rendering operation.
///
/// Progressive rendering allows rendering to be paused and resumed,
/// which is useful for responsive UIs that need to remain interactive
/// during lengthy rendering operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderStatus {
    /// Rendering is ready to start.
    Ready,
    /// Rendering was paused and needs to continue.
    ToBeContinued,
    /// Rendering completed successfully.
    Done,
    /// Rendering failed.
    Failed,
}

impl RenderStatus {
    /// Convert from raw PDFium status value.
    fn from_raw(value: i32) -> Self {
        match value as u32 {
            FPDF_RENDER_READY => RenderStatus::Ready,
            FPDF_RENDER_TOBECONTINUED => RenderStatus::ToBeContinued,
            FPDF_RENDER_DONE => RenderStatus::Done,
            _ => RenderStatus::Failed,
        }
    }

    /// Check if rendering is complete (either done or failed).
    pub fn is_finished(&self) -> bool {
        matches!(self, RenderStatus::Done | RenderStatus::Failed)
    }

    /// Check if rendering completed successfully.
    pub fn is_success(&self) -> bool {
        matches!(self, RenderStatus::Done)
    }

    /// Check if rendering needs to continue.
    pub fn needs_continue(&self) -> bool {
        matches!(self, RenderStatus::ToBeContinued)
    }
}

/// Progressive renderer for a PDF page.
///
/// Progressive rendering allows rendering to be paused and resumed via a callback,
/// which is useful for maintaining responsive UIs during rendering of complex pages.
///
/// # Example
///
/// ```no_run
/// use pdfium_render_fast::{Pdfium, PdfRenderConfig, ProgressiveRender, RenderStatus};
/// use std::sync::atomic::{AtomicBool, Ordering};
/// use std::sync::Arc;
///
/// let pdfium = Pdfium::new()?;
/// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
/// let page = doc.page(0)?;
/// let config = PdfRenderConfig::new().set_target_dpi(150.0);
///
/// // Create a flag to control pausing (could be set by UI interaction)
/// let should_pause = Arc::new(AtomicBool::new(false));
/// let pause_flag = should_pause.clone();
///
/// // Start progressive rendering
/// let mut renderer = ProgressiveRender::start(
///     &page,
///     &config,
///     move || pause_flag.load(Ordering::Relaxed)
/// )?;
///
/// // Continue rendering until complete
/// loop {
///     let status = renderer.continue_render()?;
///     match status {
///         RenderStatus::Done => {
///             let bitmap = renderer.finish()?;
///             bitmap.save_as_png("output.png")?;
///             break;
///         }
///         RenderStatus::ToBeContinued => {
///             // Could update UI, process events, etc.
///             continue;
///         }
///         RenderStatus::Failed => {
///             eprintln!("Rendering failed");
///             break;
///         }
///         RenderStatus::Ready => continue,
///     }
/// }
/// # Ok::<(), pdfium_render_fast::PdfError>(())
/// ```
pub struct ProgressiveRender<F>
where
    F: FnMut() -> bool + 'static,
{
    page_handle: FPDF_PAGE,
    bitmap_handle: FPDF_BITMAP,
    width: u32,
    height: u32,
    format: PixelFormat,
    status: RenderStatus,
    // Store callback in a Box to keep a stable address for the IFSDK_PAUSE struct
    callback: Box<F>,
    // The pause structure must live as long as rendering
    pause_struct: Box<_IFSDK_PAUSE>,
}

// Manual implementation of Send - the callback must be Send
unsafe impl<F> Send for ProgressiveRender<F> where F: FnMut() -> bool + Send + 'static {}

impl<F> ProgressiveRender<F>
where
    F: FnMut() -> bool + 'static,
{
    /// Start progressive rendering of a page.
    ///
    /// The callback function is called periodically during rendering.
    /// Return `true` from the callback to pause rendering, `false` to continue.
    ///
    /// # Arguments
    ///
    /// * `page` - The page to render
    /// * `config` - Render configuration (DPI, format, etc.)
    /// * `should_pause` - Callback that returns `true` to pause rendering
    pub fn start(
        page: &crate::page::PdfPage,
        config: &PdfRenderConfig,
        should_pause: F,
    ) -> Result<Self> {
        let page_handle = page.handle();

        // Calculate dimensions
        let page_width = unsafe { FPDF_GetPageWidthF(page_handle) } as f64;
        let page_height = unsafe { FPDF_GetPageHeightF(page_handle) } as f64;
        let (width, height) = config.calculate_size(page_width, page_height);

        // Create bitmap
        let bitmap_handle = unsafe {
            FPDFBitmap_Create(
                width as i32,
                height as i32,
                config.pixel_format().to_fpdf_format(),
            )
        };

        if bitmap_handle.is_null() {
            return Err(PdfError::BitmapCreationFailed {
                reason: "Failed to create bitmap for progressive rendering".to_string(),
            });
        }

        // Fill with white background
        unsafe {
            FPDFBitmap_FillRect(bitmap_handle, 0, 0, width as i32, height as i32, 0xFFFFFFFF);
        }

        // Box the callback to get a stable address
        let mut callback = Box::new(should_pause);

        // Create the pause structure with the callback pointer
        // We use a trampoline function that casts user data back to our callback
        let pause_struct = Box::new(_IFSDK_PAUSE {
            version: 1,
            NeedToPauseNow: Some(pause_callback::<F>),
            user: callback.as_mut() as *mut F as *mut std::os::raw::c_void,
        });

        // Start rendering
        let status = unsafe {
            FPDF_RenderPageBitmap_Start(
                bitmap_handle,
                page_handle,
                0,
                0,
                width as i32,
                height as i32,
                0, // no rotation
                FPDF_ANNOT as i32,
                pause_struct.as_ref() as *const _ as *mut _,
            )
        };

        Ok(Self {
            page_handle,
            bitmap_handle,
            width,
            height,
            format: config.pixel_format(),
            status: RenderStatus::from_raw(status),
            callback,
            pause_struct,
        })
    }

    /// Continue progressive rendering.
    ///
    /// Call this method after rendering has been paused to continue.
    /// Returns the new rendering status.
    pub fn continue_render(&mut self) -> Result<RenderStatus> {
        if self.status.is_finished() {
            return Ok(self.status);
        }

        // Update user pointer in case callback was moved (shouldn't happen with Box, but be safe)
        self.pause_struct.user = self.callback.as_mut() as *mut F as *mut std::os::raw::c_void;

        let status = unsafe {
            FPDF_RenderPage_Continue(
                self.page_handle,
                self.pause_struct.as_ref() as *const _ as *mut _,
            )
        };

        self.status = RenderStatus::from_raw(status);
        Ok(self.status)
    }

    /// Get the current rendering status.
    pub fn status(&self) -> RenderStatus {
        self.status
    }

    /// Finish rendering and get the bitmap.
    ///
    /// This closes the rendering resources. Can be called even if rendering
    /// is not complete to get a partial result.
    pub fn finish(mut self) -> Result<PdfBitmap> {
        // Close rendering resources
        unsafe {
            FPDF_RenderPage_Close(self.page_handle);
        }

        // Take ownership of the bitmap handle
        let handle = self.bitmap_handle;
        self.bitmap_handle = std::ptr::null_mut(); // Prevent double-free in Drop

        Ok(PdfBitmap::new(handle, self.width, self.height, self.format))
    }

    /// Cancel rendering without getting the bitmap.
    ///
    /// This releases all resources without returning the partial render.
    pub fn cancel(mut self) {
        unsafe {
            FPDF_RenderPage_Close(self.page_handle);
            if !self.bitmap_handle.is_null() {
                FPDFBitmap_Destroy(self.bitmap_handle);
            }
        }
        self.bitmap_handle = std::ptr::null_mut();
    }
}

impl<F> Drop for ProgressiveRender<F>
where
    F: FnMut() -> bool + 'static,
{
    fn drop(&mut self) {
        // Only clean up if not already finished via finish() or cancel()
        if !self.bitmap_handle.is_null() {
            unsafe {
                FPDF_RenderPage_Close(self.page_handle);
                FPDFBitmap_Destroy(self.bitmap_handle);
            }
        }
    }
}

/// Trampoline function for the pause callback.
///
/// This is called by PDFium and converts the user pointer back to our callback.
unsafe extern "C" fn pause_callback<F>(pause: *mut _IFSDK_PAUSE) -> FPDF_BOOL
where
    F: FnMut() -> bool,
{
    if pause.is_null() {
        return 0;
    }

    let user = (*pause).user;
    if user.is_null() {
        return 0;
    }

    let callback = &mut *(user as *mut F);
    if callback() {
        1 // Pause
    } else {
        0 // Continue
    }
}
