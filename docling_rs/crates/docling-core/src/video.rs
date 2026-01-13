//! Video format processing module
//!
//! This module provides video subtitle extraction and transcription functionality
//! for MP4, MKV, MOV, and AVI video files, as well as standalone subtitle file processing.

use crate::error::{DoclingError, Result};
use std::path::Path;

/// Process MP4 video file
///
/// Extracts subtitles from MP4 video container and optionally transcribes audio.
///
/// # Arguments
///
/// * `path` - Path to the MP4 file
///
/// # Returns
///
/// Markdown-formatted string with extracted subtitles and/or transcription
///
/// # Errors
/// Returns an error if the video feature is disabled or processing fails.
#[must_use = "this function returns the extracted markdown content"]
pub fn process_mp4<P: AsRef<Path>>(path: P) -> Result<String> {
    let path = path.as_ref();

    // Check if docling-video crate is available
    #[cfg(feature = "video")]
    {
        docling_video::process_mp4(path)
            .map_err(|e| DoclingError::ConversionError(format!("MP4 processing failed: {e}")))
    }

    #[cfg(not(feature = "video"))]
    {
        Err(DoclingError::ConversionError(format!(
            "MP4 support not enabled. Enable the 'video' feature to process video files: {}",
            path.display()
        )))
    }
}

/// Process MKV video file
///
/// Extracts subtitles from Matroska video container and optionally transcribes audio.
///
/// # Arguments
///
/// * `path` - Path to the MKV file
///
/// # Returns
///
/// Markdown-formatted string with extracted subtitles and/or transcription
///
/// # Errors
/// Returns an error if the video feature is disabled or processing fails.
#[must_use = "this function returns the extracted markdown content"]
pub fn process_mkv<P: AsRef<Path>>(path: P) -> Result<String> {
    let path = path.as_ref();

    #[cfg(feature = "video")]
    {
        docling_video::process_mkv(path)
            .map_err(|e| DoclingError::ConversionError(format!("MKV processing failed: {e}")))
    }

    #[cfg(not(feature = "video"))]
    {
        Err(DoclingError::ConversionError(format!(
            "MKV support not enabled. Enable the 'video' feature to process video files: {}",
            path.display()
        )))
    }
}

/// Process MOV video file
///
/// Extracts subtitles from `QuickTime` video container and optionally transcribes audio.
///
/// # Arguments
///
/// * `path` - Path to the MOV file
///
/// # Returns
///
/// Markdown-formatted string with extracted subtitles and/or transcription
///
/// # Errors
/// Returns an error if the video feature is disabled or processing fails.
#[must_use = "this function returns the extracted markdown content"]
pub fn process_mov<P: AsRef<Path>>(path: P) -> Result<String> {
    let path = path.as_ref();

    #[cfg(feature = "video")]
    {
        docling_video::process_mov(path)
            .map_err(|e| DoclingError::ConversionError(format!("MOV processing failed: {e}")))
    }

    #[cfg(not(feature = "video"))]
    {
        Err(DoclingError::ConversionError(format!(
            "MOV support not enabled. Enable the 'video' feature to process video files: {}",
            path.display()
        )))
    }
}

/// Process AVI video file
///
/// Extracts subtitles from AVI video container and optionally transcribes audio.
///
/// # Arguments
///
/// * `path` - Path to the AVI file
///
/// # Returns
///
/// Markdown-formatted string with extracted subtitles and/or transcription
///
/// # Errors
/// Returns an error if the video feature is disabled or processing fails.
#[must_use = "this function returns the extracted markdown content"]
pub fn process_avi<P: AsRef<Path>>(path: P) -> Result<String> {
    let path = path.as_ref();

    #[cfg(feature = "video")]
    {
        docling_video::process_avi(path)
            .map_err(|e| DoclingError::ConversionError(format!("AVI processing failed: {e}")))
    }

    #[cfg(not(feature = "video"))]
    {
        Err(DoclingError::ConversionError(format!(
            "AVI support not enabled. Enable the 'video' feature to process video files: {}",
            path.display()
        )))
    }
}

/// Process SRT subtitle file
///
/// Parses SRT (`SubRip`) subtitle file and converts to markdown format.
///
/// # Arguments
///
/// * `path` - Path to the SRT file
///
/// # Returns
///
/// Markdown-formatted string with subtitles
///
/// # Errors
/// Returns an error if the video feature is disabled or parsing fails.
#[must_use = "this function returns the extracted markdown content"]
pub fn process_srt<P: AsRef<Path>>(path: P) -> Result<String> {
    let path = path.as_ref();

    #[cfg(feature = "video")]
    {
        docling_video::process_srt(path)
            .map_err(|e| DoclingError::ConversionError(format!("SRT processing failed: {e}")))
    }

    #[cfg(not(feature = "video"))]
    {
        Err(DoclingError::ConversionError(format!(
            "SRT support not enabled. Enable the 'video' feature to process subtitle files: {}",
            path.display()
        )))
    }
}

/// Process `WebVTT` subtitle file
///
/// Parses a `WebVTT` subtitle file and converts it to markdown format.
///
/// # Arguments
///
/// * `path` - Path to the `WebVTT` file
///
/// # Returns
///
/// Markdown-formatted string with subtitles
///
/// # Errors
/// Returns an error if the video feature is disabled or parsing fails.
#[must_use = "this function returns the extracted markdown content"]
pub fn process_webvtt<P: AsRef<Path>>(path: P) -> Result<String> {
    let path = path.as_ref();

    #[cfg(feature = "video")]
    {
        docling_video::process_webvtt(path)
            .map_err(|e| DoclingError::ConversionError(format!("WebVTT processing failed: {e}")))
    }

    #[cfg(not(feature = "video"))]
    {
        Err(DoclingError::ConversionError(format!(
            "WebVTT support not enabled. Enable the 'video' feature to process subtitle files: {}",
            path.display()
        )))
    }
}
