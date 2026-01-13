//! High-performance image I/O utilities with optimized JPEG handling
//!
//! This module provides optimized image loading and saving, using:
//! - **mozjpeg** (C library, SIMD-optimized) for JPEG decode/encode (3-5x faster than pure Rust)
//! - **image crate** for PNG and other formats
//!
//! # Performance
//! - JPEG decode: 3-5x faster than `image::jpeg` (pure Rust)
//! - JPEG encode: 2-4x faster with better quality at same file size
//! - PNG: Uses `image` crate (already fast)

use image::{ImageBuffer, Rgb, RgbImage};
use std::fs;
use std::path::Path;
use thiserror::Error;

/// Errors that can occur during image I/O operations
#[derive(Error, Debug)]
pub enum ImageError {
    #[error("Failed to read image file: {0}")]
    ReadError(String),

    #[error("Failed to decode image: {0}")]
    DecodeError(String),

    #[error("Failed to encode image: {0}")]
    EncodeError(String),

    #[error("Failed to write image file: {0}")]
    WriteError(String),

    #[error("Unsupported image format: {0}")]
    UnsupportedFormat(String),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Load an image from a file path, automatically detecting format
///
/// Uses optimized mozjpeg for JPEG files (3-5x faster decode).
/// Falls back to `image` crate for PNG and other formats.
///
/// # Arguments
/// * `path` - Path to the image file
///
/// # Returns
/// * `Result<RgbImage>` - Loaded RGB image or error
///
/// # Example
/// ```no_run
/// use video_extract_core::image_io::load_image;
/// let img = load_image("photo.jpg")?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn load_image<P: AsRef<Path>>(path: P) -> Result<RgbImage, ImageError> {
    let path = path.as_ref();

    // Determine format from extension
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    match extension.as_str() {
        "jpg" | "jpeg" => load_jpeg_mozjpeg(path),
        "png" => load_png(path),
        "heic" | "heif" => {
            // HEIC/HEIF formats - decode via FFmpeg (supports Tile Grid)
            load_heic_via_ffmpeg(path)
        }
        "arw" | "cr2" | "dng" | "nef" | "raf" | "rw2" | "orf" | "pef" | "dcr" | "x3f" => {
            // RAW camera formats - decode via dcraw
            load_raw_via_ffmpeg(path)
        }
        _ => {
            // Fallback to image crate for other formats
            let img = image::open(path)
                .map_err(|e| ImageError::DecodeError(format!("Failed to load image: {e}")))?;
            Ok(img.to_rgb8())
        }
    }
}

/// Load JPEG image using mozjpeg (3-5x faster than pure Rust)
fn load_jpeg_mozjpeg<P: AsRef<Path>>(path: P) -> Result<RgbImage, ImageError> {
    // Read file into memory
    let data = fs::read(path.as_ref())
        .map_err(|e| ImageError::ReadError(format!("Failed to read JPEG file: {e}")))?;

    // Decompress using mozjpeg
    let d = mozjpeg::Decompress::new_mem(&data)
        .map_err(|e| ImageError::DecodeError(format!("Failed to create decompressor: {e}")))?;

    // Get dimensions before consuming decompressor
    let (width, height) = (d.width(), d.height());

    // Convert to RGB format and read all scanlines
    let mut rgb = d
        .rgb()
        .map_err(|e| ImageError::DecodeError(format!("Failed to decode RGB: {e}")))?;

    let image_data = rgb.read_scanlines().expect("Failed to read scanlines");

    // Convert to image::RgbImage
    let img = ImageBuffer::<Rgb<u8>, Vec<u8>>::from_raw(width as u32, height as u32, image_data)
        .ok_or_else(|| {
            ImageError::DecodeError(format!(
                "Failed to create image buffer from mozjpeg output ({}x{})",
                width, height
            ))
        })?;

    Ok(img)
}

/// Load PNG image using image crate
fn load_png<P: AsRef<Path>>(path: P) -> Result<RgbImage, ImageError> {
    let img = image::open(path.as_ref())
        .map_err(|e| ImageError::DecodeError(format!("Failed to load PNG: {e}")))?;

    Ok(img.to_rgb8())
}

/// Load HEIC/HEIF formats using FFmpeg (supports Tile Grid)
fn load_heic_via_ffmpeg<P: AsRef<Path>>(path: P) -> Result<RgbImage, ImageError> {
    use std::process::Command;
    let path = path.as_ref();

    // Create temporary output path for decoded image
    let temp_dir = std::env::temp_dir();
    let temp_png = temp_dir.join(format!("heic_decode_{}.png",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));

    // Use FFmpeg to decode HEIC to PNG format
    // FFmpeg automatically handles HEIC/HEIF Tile Grid composition
    let output = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel", "error",
            "-i", path.to_str().unwrap(),
            "-pix_fmt", "rgb24",
            "-f", "image2",
            temp_png.to_str().unwrap(),
        ])
        .output()
        .map_err(|e| ImageError::DecodeError(format!("Failed to run ffmpeg for HEIC decode: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ImageError::DecodeError(format!(
            "ffmpeg failed to decode HEIC image: {stderr}"
        )));
    }

    // Load the decoded PNG using image crate
    let img = image::open(&temp_png)
        .map_err(|e| ImageError::DecodeError(format!("Failed to load ffmpeg-decoded PNG: {e}")))?
        .to_rgb8();

    // Clean up temporary file
    let _ = std::fs::remove_file(&temp_png);

    Ok(img)
}

/// Load RAW camera formats using dcraw (decodes to temporary PPM, then loads)
fn load_raw_via_ffmpeg<P: AsRef<Path>>(path: P) -> Result<RgbImage, ImageError> {
    use std::process::Command;
    let path = path.as_ref();

    // Create temporary output path for decoded image
    let temp_dir = std::env::temp_dir();
    let temp_ppm = temp_dir.join(format!("raw_decode_{}.ppm",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));

    // Use dcraw to decode RAW to PPM format
    // dcraw options:
    //   -c: Write to stdout
    //   -w: Use camera white balance
    //   -6: Output 16-bit PPM
    //   -T: Output TIFF (not used, PPM is simpler)
    let output = Command::new("dcraw")
        .args([
            "-c",                        // Write to stdout
            "-w",                        // Camera white balance
            "-q", "3",                   // High quality interpolation (AHD)
            "-o", "1",                   // sRGB color space
            path.to_str().unwrap(),
        ])
        .output()
        .map_err(|e| ImageError::DecodeError(format!("Failed to run dcraw for RAW decode: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ImageError::DecodeError(format!(
            "dcraw failed to decode RAW image: {stderr}"
        )));
    }

    // Write dcraw output (PPM format) to temp file
    fs::write(&temp_ppm, output.stdout)
        .map_err(|e| ImageError::WriteError(format!("Failed to write temp PPM: {e}")))?;

    // Load the decoded PPM using image crate
    let img = image::open(&temp_ppm)
        .map_err(|e| ImageError::DecodeError(format!("Failed to load dcraw-decoded PPM: {e}")))?
        .to_rgb8();

    // Clean up temporary file
    let _ = std::fs::remove_file(&temp_ppm);

    Ok(img)
}

/// Save an RGB image to a file, automatically determining format from extension
///
/// Uses optimized mozjpeg for JPEG files (2-4x faster encode with better quality).
/// Falls back to `image` crate for PNG and other formats.
///
/// # Arguments
/// * `image` - RGB image to save
/// * `path` - Output file path
/// * `quality` - JPEG quality (1-100, default: 85 for mozjpeg, ignored for PNG)
///
/// # Example
/// ```no_run
/// use image::RgbImage;
/// use video_extract_core::image_io::save_image;
///
/// let img = RgbImage::new(640, 480);
/// save_image(&img, "output.jpg", 90)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn save_image<P: AsRef<Path>>(
    image: &RgbImage,
    path: P,
    quality: u8,
) -> Result<(), ImageError> {
    let path = path.as_ref();

    // Determine format from extension
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    match extension.as_str() {
        "jpg" | "jpeg" => save_jpeg_mozjpeg(image, path, quality),
        "png" => save_png(image, path),
        _ => {
            // Fallback to image crate
            image
                .save(path)
                .map_err(|e| ImageError::WriteError(format!("Failed to save image: {e}")))?;
            Ok(())
        }
    }
}

/// Save JPEG image using mozjpeg (2-4x faster than pure Rust with better quality)
fn save_jpeg_mozjpeg<P: AsRef<Path>>(
    image: &RgbImage,
    path: P,
    quality: u8,
) -> Result<(), ImageError> {
    let (width, height) = image.dimensions();

    // Create output file
    let mut file = fs::File::create(path.as_ref())
        .map_err(|e| ImageError::WriteError(format!("Failed to create output file: {e}")))?;

    // Create mozjpeg compressor
    let mut comp = mozjpeg::Compress::new(mozjpeg::ColorSpace::JCS_RGB);
    comp.set_size(width as usize, height as usize);
    comp.set_quality(quality.clamp(1, 100) as f32);

    // Start compression with writer
    let mut comp_started = comp
        .start_compress(&mut file)
        .map_err(|e| ImageError::EncodeError(format!("Failed to start compression: {e}")))?;

    // Write image data scanlines
    let raw_data = image.as_raw();

    comp_started
        .write_scanlines(raw_data)
        .map_err(|e| ImageError::EncodeError(format!("Failed to write scanlines: {e}")))?;

    // Finish compression
    comp_started
        .finish()
        .map_err(|e| ImageError::EncodeError(format!("Failed to finish compression: {e}")))?;

    Ok(())
}

/// Save PNG image using image crate
fn save_png<P: AsRef<Path>>(image: &RgbImage, path: P) -> Result<(), ImageError> {
    image
        .save(path.as_ref())
        .map_err(|e| ImageError::WriteError(format!("Failed to save PNG: {e}")))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgb;

    #[test]
    fn test_save_and_load_jpeg() {
        let img = RgbImage::from_pixel(100, 100, Rgb([255, 0, 0]));
        let temp_path = "/tmp/test_mozjpeg.jpg";

        // Save JPEG
        save_image(&img, temp_path, 90).expect("Failed to save JPEG");

        // Load JPEG
        let loaded = load_image(temp_path).expect("Failed to load JPEG");

        // Verify dimensions
        assert_eq!(loaded.dimensions(), (100, 100));

        // Clean up
        let _ = std::fs::remove_file(temp_path);
    }

    #[test]
    fn test_save_and_load_png() {
        let img = RgbImage::from_pixel(50, 50, Rgb([0, 255, 0]));
        let temp_path = "/tmp/test_png.png";

        // Save PNG
        save_image(&img, temp_path, 100).expect("Failed to save PNG");

        // Load PNG
        let loaded = load_image(temp_path).expect("Failed to load PNG");

        // Verify dimensions and exact pixel values (PNG is lossless)
        assert_eq!(loaded.dimensions(), (50, 50));
        assert_eq!(loaded.get_pixel(25, 25), &Rgb([0, 255, 0]));

        // Clean up
        let _ = std::fs::remove_file(temp_path);
    }

    #[test]
    fn test_jpeg_quality_settings() {
        let img = RgbImage::from_pixel(200, 200, Rgb([128, 128, 128]));

        // Test different quality levels
        for quality in [50, 75, 90, 95, 100] {
            let temp_path = format!("/tmp/test_quality_{quality}.jpg");
            save_image(&img, &temp_path, quality).expect("Failed to save JPEG");

            // Verify file was created and has reasonable size
            let metadata = std::fs::metadata(&temp_path).expect("Failed to get metadata");
            assert!(metadata.len() > 0, "JPEG file is empty");

            // Clean up
            let _ = std::fs::remove_file(&temp_path);
        }
    }
}
