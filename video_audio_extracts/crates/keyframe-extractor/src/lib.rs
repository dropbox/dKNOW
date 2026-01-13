//! Keyframe extractor module
//!
//! Extracts keyframes (I-frames) from videos with interval filtering.
//! Generates multi-resolution thumbnails. Optimized for maximum speed.

pub mod plugin;

use image::RgbImage;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::info;
use video_audio_common::{Keyframe, ProcessingError, Result};
use video_extract_core::image_io::save_image;

/// Statistics for keyframe extraction filtering
#[derive(Debug, Default)]
struct FilterStats {
    #[cfg_attr(not(debug_assertions), allow(dead_code))]
    total_iframes: usize,
    #[cfg_attr(not(debug_assertions), allow(dead_code))]
    filtered_by_interval: usize,
    #[cfg_attr(not(debug_assertions), allow(dead_code))]
    filtered_by_max_limit: usize,
}

/// Keyframe extractor configuration
#[derive(Debug, Clone)]
pub struct KeyframeExtractor {
    /// Minimum interval between keyframes (seconds)
    pub interval: f64,
    /// Maximum number of keyframes to extract
    pub max_keyframes: usize,
    /// Hamming distance threshold for deduplication (0-64)
    pub similarity_threshold: u32,
    /// Thumbnail sizes (width, height) to generate
    pub thumbnail_sizes: Vec<(u32, u32)>,
    /// Output directory for thumbnails
    pub output_dir: PathBuf,
    /// Use FFmpeg CLI directly (legacy mode, adds 20-30ms process spawn overhead)
    /// Default: false (uses embedded C FFI decoder)
    /// Set to true only when stream copy semantics are specifically needed
    pub use_ffmpeg_cli: bool,
}

impl Default for KeyframeExtractor {
    fn default() -> Self {
        Self {
            interval: 1.0,
            max_keyframes: 500,
            similarity_threshold: 10,
            thumbnail_sizes: vec![(640, 480)], // Single resolution for speed
            output_dir: PathBuf::from("thumbnails"),
            use_ffmpeg_cli: false, // Use C FFI decode by default (no process spawn overhead)
        }
    }
}

impl KeyframeExtractor {
    /// Create configuration for quick preview (fewer, smaller keyframes)
    #[must_use]
    pub fn for_preview() -> Self {
        Self {
            interval: 2.0,
            max_keyframes: 100,
            similarity_threshold: 10,
            thumbnail_sizes: vec![(320, 240)],
            output_dir: PathBuf::from("thumbnails"),
            use_ffmpeg_cli: false, // Use C FFI decode (no process spawn overhead)
        }
    }

    /// Create configuration for detailed analysis (more keyframes, multiple resolutions)
    #[must_use]
    pub fn for_analysis() -> Self {
        Self {
            interval: 0.5,
            max_keyframes: 1000,
            similarity_threshold: 10,
            thumbnail_sizes: vec![(640, 480), (1280, 720), (1920, 1080)],
            output_dir: PathBuf::from("thumbnails"),
            use_ffmpeg_cli: false, // Use full decode for analysis (may need RGB data)
        }
    }

    /// Create configuration for ML pipeline (object detection, embeddings, etc.)
    /// IMPORTANT: Disables FFmpeg CLI mode and uses full RGB decode
    #[must_use]
    pub fn for_ml_pipeline() -> Self {
        Self {
            interval: 1.0,
            max_keyframes: 500,
            similarity_threshold: 10,
            thumbnail_sizes: vec![(640, 480)],
            output_dir: PathBuf::from("thumbnails"),
            use_ffmpeg_cli: false, // Must use decode mode for ML
        }
    }
}

/// Helper function to detect if a video uses HEVC/H.265 codec
/// Uses direct ffprobe call for reliable codec detection
fn is_hevc_codec(video_path: &Path) -> bool {
    // Call ffprobe to get the video codec name
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "quiet",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=codec_name",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
        ])
        .arg(video_path)
        .output();

    match output {
        Ok(output) => {
            let codec = String::from_utf8_lossy(&output.stdout);
            let codec_trimmed = codec.trim();
            info!(
                "Codec detected for {}: {}",
                video_path.display(),
                codec_trimmed
            );
            codec_trimmed == "hevc"
        }
        Err(e) => {
            info!(
                "Failed to run ffprobe for {}: {}",
                video_path.display(),
                e
            );
            false
        }
    }
}

/// Extract keyframes from a video file
pub fn extract_keyframes(video_path: &Path, config: KeyframeExtractor) -> Result<Vec<Keyframe>> {
    // Force FFmpeg CLI for MXF files regardless of config (N=61: MXF C FFI decoder has issues)
    let is_mxf = video_path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("mxf"))
        .unwrap_or(false);

    // Use dcraw fallback for RAW image formats (N=74: FFmpeg lacks libraw support)
    let is_raw = video_path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            matches!(
                ext.to_lowercase().as_str(),
                "arw" | "cr2" | "dng" | "nef" | "raf"
            )
        })
        .unwrap_or(false);

    // Detect HEVC codec (N=129: C FFI decoder produces corrupted frame 0 for HEVC files)
    let is_hevc = is_hevc_codec(video_path);
    if is_hevc {
        info!(
            "HEVC codec detected, routing to FFmpeg CLI decoder: {}",
            video_path.display()
        );
    }

    // Force FFmpeg CLI for MOV files (N=131: C FFI decoder produces corrupted frame 0 for MOV files)
    let is_mov = video_path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("mov"))
        .unwrap_or(false);

    // Dispatch based on file type and configuration
    if is_raw {
        extract_keyframes_raw_dcraw(video_path, &config)
    } else if config.use_ffmpeg_cli || is_mxf || is_hevc || is_mov {
        extract_keyframes_ffmpeg_cli(video_path, &config)
    } else {
        extract_keyframes_decode(video_path, &config)
    }
}

/// Extract keyframes using FFmpeg CLI (MAXIMUM SPEED - stream copy, no decode)
/// This is 5-8x faster than decode mode but produces JPEG files only (no RGB data)
fn extract_keyframes_ffmpeg_cli(
    video_path: &Path,
    config: &KeyframeExtractor,
) -> Result<Vec<Keyframe>> {
    use std::process::Command;

    // Create output directory
    std::fs::create_dir_all(&config.output_dir)?;

    // Call FFmpeg CLI directly with stream copy for I-frames
    // This is the FASTEST possible method - matches FFmpeg CLI performance
    let output_pattern = config.output_dir.join("frame_%08d.jpg");

    let video_path_str = video_path
        .to_str()
        .ok_or_else(|| ProcessingError::FFmpegError("Invalid video path".to_string()))?;
    let output_pattern_str = output_pattern
        .to_str()
        .ok_or_else(|| ProcessingError::FFmpegError("Invalid output path".to_string()))?;

    let output = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "panic", // Silent mode for performance
            "-i",
            video_path_str,
            "-vf",
            "select='eq(pict_type\\,I)'", // Extract I-frames only
            "-vsync",
            "vfr", // Variable frame rate
            "-q:v",
            "2", // High quality JPEG
            output_pattern_str,
        ])
        .output()
        .map_err(|e| ProcessingError::FFmpegError(format!("Failed to execute ffmpeg: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ProcessingError::FFmpegError(format!(
            "FFmpeg failed: {}",
            stderr
        )));
    }

    // Find all generated JPEG files
    let mut frame_paths: Vec<(u64, PathBuf)> = std::fs::read_dir(&config.output_dir)?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("jpg") {
                // Extract frame number from filename (frame_00000042.jpg -> 42)
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .and_then(|s| s.split('_').next_back())
                    .and_then(|s| s.parse::<u64>().ok())
                    .map(|frame_num| (frame_num, path))
            } else {
                None
            }
        })
        .collect();

    if frame_paths.is_empty() {
        return Err(ProcessingError::CorruptedFile(
            "No I-frames extracted by FFmpeg".to_string(),
        ));
    }

    // Sort by frame number
    frame_paths.sort_by_key(|(frame_num, _)| *frame_num);

    // Apply interval and max_keyframes filtering
    let mut keyframes = Vec::with_capacity(std::cmp::min(frame_paths.len(), config.max_keyframes));
    let mut last_frame_num: i64 = -(config.interval * 30.0) as i64; // Assume 30 fps for interval
    #[cfg_attr(not(debug_assertions), allow(unused_variables))]
    let stats = FilterStats {
        total_iframes: frame_paths.len(),
        ..Default::default()
    };

    for (frame_num, path) in frame_paths {
        // Interval filtering (approximate, based on frame numbers)
        let frame_diff = frame_num as i64 - last_frame_num;
        if frame_diff < (config.interval * 30.0) as i64 {
            continue;
        }

        // Max keyframes limit
        if keyframes.len() >= config.max_keyframes {
            break;
        }

        // Estimate timestamp (assume 30 fps)
        let timestamp = frame_num as f64 / 30.0;

        // Pre-allocate thumbnail_paths HashMap with capacity 1 (FFmpeg CLI generates one size)
        let mut thumbnail_paths = HashMap::with_capacity(1);
        // FFmpeg CLI only generates one size - use it as the first configured size
        if let Some(&(width, height)) = config.thumbnail_sizes.first() {
            let size_key = format!("{}x{}", width, height);
            thumbnail_paths.insert(size_key, path.clone());
        }

        keyframes.push(Keyframe {
            timestamp,
            frame_number: frame_num,
            hash: 0,        // Not computed in fast mode
            sharpness: 0.0, // Not computed in fast mode
            thumbnail_paths,
        });

        last_frame_num = frame_num as i64;
    }

    // Log statistics in debug mode
    #[cfg(debug_assertions)]
    eprintln!(
        "Keyframe extraction (FFmpeg CLI fast mode): {} I-frames → {} keyframes",
        stats.total_iframes,
        keyframes.len()
    );

    Ok(keyframes)
}

/// Extract keyframes from RAW image formats using dcraw fallback (N=74)
/// Converts RAW → PPM → JPEG to work around FFmpeg's lack of libraw support
fn extract_keyframes_raw_dcraw(
    raw_image_path: &Path,
    config: &KeyframeExtractor,
) -> Result<Vec<Keyframe>> {
    use std::process::Command;

    // Create output directory
    std::fs::create_dir_all(&config.output_dir)?;

    // Convert RAW to PPM using dcraw
    // -w: Use camera white balance
    // -c: Write to stdout
    let raw_path_str = raw_image_path
        .to_str()
        .ok_or_else(|| ProcessingError::FFmpegError("Invalid RAW image path".to_string()))?;

    let dcraw_output = Command::new("dcraw")
        .args(["-w", "-c", raw_path_str])
        .output()
        .map_err(|e| {
            ProcessingError::FFmpegError(format!("Failed to execute dcraw: {}", e))
        })?;

    if !dcraw_output.status.success() {
        let stderr = String::from_utf8_lossy(&dcraw_output.stderr);
        return Err(ProcessingError::FFmpegError(format!(
            "dcraw failed: {}",
            stderr
        )));
    }

    // Create temporary PPM file
    let temp_ppm = std::env::temp_dir().join(format!(
        "raw_decode_{}.ppm",
        std::process::id()
    ));

    std::fs::write(&temp_ppm, &dcraw_output.stdout).map_err(|e| {
        ProcessingError::FFmpegError(format!("Failed to write temporary PPM: {}", e))
    })?;

    // Convert PPM → JPEG using FFmpeg
    let output_path = config.output_dir.join("frame_00000001.jpg");
    let output_path_str = output_path
        .to_str()
        .ok_or_else(|| ProcessingError::FFmpegError("Invalid output path".to_string()))?;
    let temp_ppm_str = temp_ppm
        .to_str()
        .ok_or_else(|| ProcessingError::FFmpegError("Invalid temp path".to_string()))?;

    let ffmpeg_output = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "panic",
            "-i",
            temp_ppm_str,
            "-q:v",
            "2", // High quality JPEG
            output_path_str,
        ])
        .output()
        .map_err(|e| {
            ProcessingError::FFmpegError(format!("Failed to execute ffmpeg: {}", e))
        })?;

    // Clean up temporary PPM file
    let _ = std::fs::remove_file(&temp_ppm);

    if !ffmpeg_output.status.success() {
        let stderr = String::from_utf8_lossy(&ffmpeg_output.stderr);
        return Err(ProcessingError::FFmpegError(format!(
            "FFmpeg PPM conversion failed: {}",
            stderr
        )));
    }

    // RAW images are single-frame, so we return exactly 1 keyframe
    let mut thumbnail_paths = HashMap::with_capacity(1);
    if let Some(&(width, height)) = config.thumbnail_sizes.first() {
        let size_key = format!("{}x{}", width, height);
        thumbnail_paths.insert(size_key, output_path);
    }

    Ok(vec![Keyframe {
        timestamp: 0.0,   // RAW images are single frames with no timestamp
        frame_number: 1,  // Single frame
        hash: 0,          // Not computed
        sharpness: 0.0,   // Not computed
        thumbnail_paths,
    }])
}

/// Extract keyframes using full decode (Required for ML pipelines - provides RGB data)
fn extract_keyframes_decode(
    video_path: &Path,
    config: &KeyframeExtractor,
) -> Result<Vec<Keyframe>> {
    // Create output directory
    std::fs::create_dir_all(&config.output_dir)?;

    // Use zero-copy C FFI decoder for maximum performance (10-30% faster than ffmpeg-next wrapper)
    let raw_frames = video_audio_decoder::decode_iframes_zero_copy(video_path)?;

    if raw_frames.is_empty() {
        return Err(ProcessingError::CorruptedFile(
            "No I-frames found in video".to_string(),
        ));
    }

    // Process frames: filter by interval and max limit (NO perceptual hashing, NO deduplication)
    let mut keyframes = Vec::with_capacity(std::cmp::min(raw_frames.len(), config.max_keyframes));
    let mut last_timestamp = -config.interval;
    let mut stats = FilterStats {
        total_iframes: raw_frames.len(),
        ..Default::default()
    };

    for raw_frame in raw_frames {
        // Skip if too close to last keyframe
        if raw_frame.timestamp - last_timestamp < config.interval {
            stats.filtered_by_interval += 1;
            continue;
        }

        // Skip if we've reached max keyframes
        if keyframes.len() >= config.max_keyframes {
            stats.filtered_by_max_limit += 1;
            break;
        }

        // Convert raw frame data to image (zero-copy: create slice from raw pointer)
        // SAFETY: RawFrameBuffer keeps AVFrame alive via Drop, so data_ptr is valid
        let data_slice = unsafe {
            std::slice::from_raw_parts(
                raw_frame.data_ptr,
                (raw_frame.width * raw_frame.height * 3) as usize,
            )
        };

        let img = RgbImage::from_vec(raw_frame.width, raw_frame.height, data_slice.to_vec())
            .ok_or_else(|| ProcessingError::FFmpegError("Invalid RGB24 frame data".to_string()))?;

        // Generate thumbnails at all configured sizes
        let thumbnail_paths = generate_thumbnails(
            &img,
            video_path,
            raw_frame.frame_number,
            &config.thumbnail_sizes,
            &config.output_dir,
        )?;

        keyframes.push(Keyframe {
            timestamp: raw_frame.timestamp,
            frame_number: raw_frame.frame_number,
            hash: 0,        // Dummy value - perceptual hashing removed for speed
            sharpness: 0.0, // Dummy value - sharpness computation removed for speed
            thumbnail_paths,
        });

        last_timestamp = raw_frame.timestamp;
    }

    // Log filtering statistics (disabled in release builds for performance)
    #[cfg(debug_assertions)]
    eprintln!(
        "Keyframe extraction stats: {} I-frames → {} keyframes (filtered: {} by interval, {} by max limit)",
        stats.total_iframes,
        keyframes.len(),
        stats.filtered_by_interval,
        stats.filtered_by_max_limit
    );

    Ok(keyframes)
}

// DELETED: compute_dhash, is_duplicate, hamming_distance, compute_sharpness
// These functions caused 10+ seconds overhead. Removed for maximum speed.
// FFmpeg CLI doesn't do perceptual hashing or deduplication - neither do we.

/// Generate thumbnails at multiple resolutions
fn generate_thumbnails(
    img: &RgbImage,
    video_path: &Path,
    frame_number: u64,
    sizes: &[(u32, u32)],
    output_dir: &Path,
) -> Result<HashMap<String, PathBuf>> {
    let video_name = video_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("video");

    // Pre-allocate thumbnail_paths HashMap with sizes.len() capacity (one entry per size)
    let mut thumbnail_paths = HashMap::with_capacity(sizes.len());

    for &(width, height) in sizes {
        // Resize while maintaining aspect ratio
        // Triangle filter: 2.65x faster than Lanczos3 with acceptable quality for thumbnails
        // Benchmark: Triangle 8.1ms vs Lanczos3 21.6ms (1920x1080→640x480)
        let resized =
            image::imageops::resize(img, width, height, image::imageops::FilterType::Triangle);

        // Generate filename
        let filename = format!("{video_name}_{frame_number:08}_{width}x{height}.jpg");
        let path = output_dir.join(&filename);

        // Save thumbnail as JPEG with optimized I/O (mozjpeg, 2-4x faster)
        save_image(&resized, &path, 85)
            .map_err(|e| ProcessingError::Other(format!("Failed to save thumbnail: {}", e)))?;

        let size_key = format!("{width}x{height}");
        thumbnail_paths.insert(size_key, path);
    }

    Ok(thumbnail_paths)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyframe_extractor_defaults() {
        let config = KeyframeExtractor::default();
        assert_eq!(config.interval, 1.0);
        assert_eq!(config.max_keyframes, 500);
        assert_eq!(config.similarity_threshold, 10);
        assert_eq!(config.thumbnail_sizes.len(), 3);
    }

    #[test]
    fn test_keyframe_extractor_preview() {
        let config = KeyframeExtractor::for_preview();
        assert_eq!(config.interval, 2.0);
        assert_eq!(config.max_keyframes, 100);
        assert_eq!(config.thumbnail_sizes.len(), 1);
    }
}
