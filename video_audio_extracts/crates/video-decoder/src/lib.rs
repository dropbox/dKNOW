/// Hardware-accelerated video decoder module
///
/// Implements frame extraction with configurable filtering and pixel format conversion.
/// Supports hardware acceleration (`VideoToolbox` on macOS, NVDEC on NVIDIA, VAAPI on Linux).
///
/// This module provides two decoding paths:
/// 1. **ffmpeg-next wrapper** (legacy, safe): decode_video()
/// 2. **Direct C FFI** (zero-copy, maximum performance): c_ffi::decode_iframes_zero_copy()
pub mod c_ffi;

use ffmpeg_next as ffmpeg;
use std::path::Path;
use video_audio_common::{ProcessingError, Result};

// Re-export zero-copy types for convenience
pub use c_ffi::{
    decode_iframes_streaming, decode_iframes_yuv, decode_iframes_zero_copy,
    encode_yuv_frame_to_jpeg, encode_yuv_frame_to_jpeg_bytes, extract_audio_to_wav, RawFrameBuffer,
    StreamFrame, YuvFrame,
};

/// Pixel format for decoded frames
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    /// YUV 4:2:0 planar format (most common)
    YUV420P,
    /// RGB 24-bit format (for image processing)
    RGB24,
}

impl PixelFormat {
    /// Convert to `FFmpeg` pixel format
    fn to_ffmpeg_format(self) -> ffmpeg::format::Pixel {
        match self {
            PixelFormat::YUV420P => ffmpeg::format::Pixel::YUV420P,
            PixelFormat::RGB24 => ffmpeg::format::Pixel::RGB24,
        }
    }
}

/// Frame filtering options
#[derive(Debug, Clone)]
pub enum FrameFilter {
    /// Extract every Nth frame
    EveryNth(u32),
    /// Extract only I-frames (keyframes)
    IFramesOnly,
    /// Extract frames at specific timestamps (seconds)
    Timestamps(Vec<f64>),
}

/// Decoded video frame
#[derive(Debug, Clone)]
pub struct Frame {
    /// Frame timestamp in seconds
    pub timestamp: f64,
    /// Frame number (0-indexed)
    pub frame_number: u64,
    /// Frame width in pixels
    pub width: u32,
    /// Frame height in pixels
    pub height: u32,
    /// Pixel format
    pub format: PixelFormat,
    /// Raw frame data (row-major order)
    pub data: Vec<u8>,
    /// Whether this is a keyframe (I-frame)
    pub is_keyframe: bool,
}

/// Video decoder configuration
///
/// Note: Uses multi-threaded software decoding (libavcodec) which is faster than
/// hardware acceleration on modern CPUs. Hardware decode (VideoToolbox) was tested
/// and found to be 5-10x slower due to initialization overhead and GPU transfer costs.
#[derive(Debug, Clone)]
pub struct DecoderConfig {
    /// Output pixel format
    pub output_format: PixelFormat,
    /// Frame filter (None = all frames)
    pub frame_filter: Option<FrameFilter>,
}

impl Default for DecoderConfig {
    fn default() -> Self {
        Self {
            output_format: PixelFormat::YUV420P,
            frame_filter: None,
        }
    }
}

/// Initialize `FFmpeg` library
fn init_ffmpeg() {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| {
        ffmpeg::init().expect("Failed to initialize FFmpeg");
    });
}

/// Decode video frames with filtering and format conversion
///
/// # Errors
///
/// Returns an error if:
/// - The input file cannot be opened
/// - No video stream is found
/// - The decoder cannot be created
/// - Frame decoding or conversion fails
#[allow(clippy::too_many_lines, clippy::cast_precision_loss)]
pub fn decode_video(input_path: &Path, config: &DecoderConfig) -> Result<Vec<Frame>> {
    init_ffmpeg();

    // Open input file
    let mut ictx = ffmpeg::format::input(&input_path)
        .map_err(|e| ProcessingError::FFmpegError(format!("Failed to open input file: {e}")))?;

    // Find video stream
    let video_stream = ictx
        .streams()
        .best(ffmpeg::media::Type::Video)
        .ok_or(ProcessingError::NoVideoStream)?;

    let stream_index = video_stream.index();
    let time_base = video_stream.time_base();

    // Get codec parameters and create decoder
    let codec_params = video_stream.parameters();

    // Create multi-threaded software decoder (faster than hardware decode on modern CPUs)
    let mut decoder = ffmpeg::codec::context::Context::from_parameters(codec_params)
        .map_err(|e| ProcessingError::FFmpegError(format!("Failed to create context: {e}")))?
        .decoder()
        .video()
        .map_err(|e| ProcessingError::FFmpegError(format!("Failed to create decoder: {e}")))?;

    let width = decoder.width();
    let height = decoder.height();
    let src_format = decoder.format();

    // Setup software scaler for pixel format conversion
    let mut scaler = ffmpeg::software::scaling::Context::get(
        src_format,
        width,
        height,
        config.output_format.to_ffmpeg_format(),
        width,
        height,
        ffmpeg::software::scaling::Flags::BILINEAR,
    )
    .map_err(|e| ProcessingError::FFmpegError(format!("Failed to create scaler: {e}")))?;

    // Pre-allocate with conservative estimate (100 frames for typical short clips)
    let mut frames = Vec::with_capacity(100);
    let mut frame_number = 0u64;
    let mut decoded_frame = ffmpeg::util::frame::video::Video::empty();
    let mut converted_frame = ffmpeg::util::frame::video::Video::empty();

    // Process packets
    for (stream, packet) in ictx.packets() {
        if stream.index() != stream_index {
            continue;
        }

        // Decode packet
        if decoder.send_packet(&packet).is_ok() {
            while decoder.receive_frame(&mut decoded_frame).is_ok() {
                let timestamp = decoded_frame.timestamp().unwrap_or(0) as f64
                    * f64::from(time_base.0) as f64
                    / f64::from(time_base.1) as f64;

                let is_keyframe = decoded_frame.is_key();

                // Apply frame filter
                let should_include = match &config.frame_filter {
                    None => true,
                    Some(FrameFilter::EveryNth(n)) => frame_number.is_multiple_of(u64::from(*n)),
                    Some(FrameFilter::IFramesOnly) => is_keyframe,
                    Some(FrameFilter::Timestamps(timestamps)) => {
                        timestamps.iter().any(|&ts| (ts - timestamp).abs() < 0.04)
                        // ~1 frame at 24fps
                    }
                };

                if should_include {
                    // Convert pixel format
                    scaler
                        .run(&decoded_frame, &mut converted_frame)
                        .map_err(|e| {
                            ProcessingError::FFmpegError(format!("Failed to convert frame: {e}"))
                        })?;

                    // Copy frame data
                    let data = copy_frame_data(&converted_frame, config.output_format);

                    frames.push(Frame {
                        timestamp,
                        frame_number,
                        width,
                        height,
                        format: config.output_format,
                        data,
                        is_keyframe,
                    });
                }

                frame_number += 1;
            }
        }
    }

    // Flush decoder
    decoder.send_eof().ok();
    while decoder.receive_frame(&mut decoded_frame).is_ok() {
        let timestamp = decoded_frame.timestamp().unwrap_or(0) as f64
            * f64::from(time_base.0) as f64
            / f64::from(time_base.1) as f64;

        let is_keyframe = decoded_frame.is_key();

        let should_include = match &config.frame_filter {
            None => true,
            Some(FrameFilter::EveryNth(n)) => frame_number.is_multiple_of(u64::from(*n)),
            Some(FrameFilter::IFramesOnly) => is_keyframe,
            Some(FrameFilter::Timestamps(timestamps)) => {
                timestamps.iter().any(|&ts| (ts - timestamp).abs() < 0.04) // ~1 frame at 24fps
            }
        };

        if should_include {
            scaler
                .run(&decoded_frame, &mut converted_frame)
                .map_err(|e| {
                    ProcessingError::FFmpegError(format!("Failed to convert frame: {e}"))
                })?;

            let data = copy_frame_data(&converted_frame, config.output_format);

            frames.push(Frame {
                timestamp,
                frame_number,
                width,
                height,
                format: config.output_format,
                data,
                is_keyframe,
            });
        }

        frame_number += 1;
    }

    Ok(frames)
}

/// Copy frame data from `FFmpeg` frame to a contiguous buffer
fn copy_frame_data(frame: &ffmpeg::util::frame::video::Video, format: PixelFormat) -> Vec<u8> {
    match format {
        PixelFormat::RGB24 => {
            // RGB24: single plane, 3 bytes per pixel
            let width = frame.width() as usize;
            let height = frame.height() as usize;
            let stride = frame.stride(0);
            let plane_data = frame.data(0);

            let mut data = Vec::with_capacity(width * height * 3);
            for y in 0..height {
                let row_start = y * stride;
                let row_end = row_start + (width * 3);
                data.extend_from_slice(&plane_data[row_start..row_end]);
            }
            data
        }
        PixelFormat::YUV420P => {
            // YUV420P: 3 planes (Y, U, V)
            let width = frame.width() as usize;
            let height = frame.height() as usize;

            // Y plane (full resolution)
            let y_stride = frame.stride(0);
            let y_data = frame.data(0);
            let y_size = width * height;

            // U and V planes (half resolution)
            let uv_width = width / 2;
            let uv_height = height / 2;
            let u_stride = frame.stride(1);
            let v_stride = frame.stride(2);
            let u_data = frame.data(1);
            let v_data = frame.data(2);
            let uv_size = uv_width * uv_height;

            let mut data = Vec::with_capacity(y_size + uv_size * 2);

            // Copy Y plane
            for y in 0..height {
                let row_start = y * y_stride;
                let row_end = row_start + width;
                data.extend_from_slice(&y_data[row_start..row_end]);
            }

            // Copy U plane
            for y in 0..uv_height {
                let row_start = y * u_stride;
                let row_end = row_start + uv_width;
                data.extend_from_slice(&u_data[row_start..row_end]);
            }

            // Copy V plane
            for y in 0..uv_height {
                let row_start = y * v_stride;
                let row_end = row_start + uv_width;
                data.extend_from_slice(&v_data[row_start..row_end]);
            }

            data
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pixel_format_conversion() {
        assert_eq!(
            PixelFormat::YUV420P.to_ffmpeg_format(),
            ffmpeg::format::Pixel::YUV420P
        );
        assert_eq!(
            PixelFormat::RGB24.to_ffmpeg_format(),
            ffmpeg::format::Pixel::RGB24
        );
    }

    #[test]
    fn test_decoder_config_default() {
        let config = DecoderConfig::default();
        assert_eq!(config.output_format, PixelFormat::YUV420P);
        assert!(config.frame_filter.is_none());
    }
}
