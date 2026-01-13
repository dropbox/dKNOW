/// Media ingestion module using `FFmpeg`
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, info};
use video_audio_common::{MediaInfo, ProcessingError, Result, StreamInfo, StreamType};

/// Initialize `FFmpeg` library (must be called once at startup)
pub fn init() -> Result<()> {
    ffmpeg_next::init()
        .map_err(|e| ProcessingError::FFmpegError(format!("Failed to initialize FFmpeg: {e}")))
}

/// Extract media information from a file using `FFmpeg`
pub fn ingest_media(path: &Path) -> Result<MediaInfo> {
    debug!("Ingesting media file: {:?}", path);

    // Open the input file
    let input = ffmpeg_next::format::input(path)
        .map_err(|e| ProcessingError::FFmpegError(format!("Failed to open file {path:?}: {e}")))?;

    // Extract format information
    let format_name = input
        .format()
        .name()
        .split(',')
        .next()
        .unwrap_or("unknown")
        .to_string();

    let duration = input.duration() as f64 / f64::from(ffmpeg_next::ffi::AV_TIME_BASE);

    // Extract metadata
    let mut metadata = HashMap::with_capacity(15); // Typical media has 10-15 metadata entries
    for (key, value) in input.metadata().iter() {
        metadata.insert(key.to_string(), value.to_string());
    }

    // Extract stream information
    let mut streams = Vec::with_capacity(input.streams().count());
    for stream in input.streams() {
        let codec = stream.parameters();
        let codec_name = codec.id().name().to_string();

        let stream_type = match codec.medium() {
            ffmpeg_next::media::Type::Video => StreamType::Video,
            ffmpeg_next::media::Type::Audio => StreamType::Audio,
            ffmpeg_next::media::Type::Subtitle => StreamType::Subtitle,
            _ => continue, // Skip other stream types
        };

        let bitrate = unsafe { (*codec.as_ptr()).bit_rate as u64 };

        // For ffmpeg-next 8.0, Parameters provides direct access to fields
        // Video-specific information
        let (width, height, fps) = if stream_type == StreamType::Video {
            let width = unsafe { (*codec.as_ptr()).width as u32 };
            let height = unsafe { (*codec.as_ptr()).height as u32 };
            let rate = stream.avg_frame_rate();
            let fps_val = if rate.1 > 0 {
                f64::from(rate.0) / f64::from(rate.1)
            } else {
                0.0
            };
            (Some(width), Some(height), Some(fps_val))
        } else {
            (None, None, None)
        };

        // Audio-specific information
        let (sample_rate, channels) = if stream_type == StreamType::Audio {
            let sample_rate = unsafe { (*codec.as_ptr()).sample_rate as u32 };
            let channels = unsafe { (*codec.as_ptr()).ch_layout.nb_channels as u8 };
            (Some(sample_rate), Some(channels))
        } else {
            (None, None)
        };

        streams.push(StreamInfo {
            stream_type,
            codec: codec_name,
            bitrate,
            width,
            height,
            fps,
            sample_rate,
            channels,
        });
    }

    let media_info = MediaInfo {
        format: format_name,
        duration,
        streams,
        metadata,
    };

    info!(
        "Ingested media: format={}, duration={:.2}s, streams={}",
        media_info.format,
        media_info.duration,
        media_info.streams.len()
    );

    Ok(media_info)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init() {
        // FFmpeg should initialize successfully
        assert!(init().is_ok());
    }
}
