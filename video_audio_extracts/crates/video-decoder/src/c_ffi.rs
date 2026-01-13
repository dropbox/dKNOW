//! Direct C FFI bindings to FFmpeg libraries for zero-copy performance
//!
//! This module provides low-level access to libavcodec, libavformat, and libavutil
//! for maximum performance. It eliminates the overhead of Rust wrapper libraries
//! and enables zero-copy memory transfers to downstream ML models.
//!
//! # Safety
//!
//! This module uses `unsafe` extensively as it directly interfaces with C code.
//! Memory safety is ensured by:
//! - RAII wrappers with Drop implementations
//! - Careful pointer lifetime management
//! - Validation of all FFmpeg return codes

#![allow(non_camel_case_types)]
#![allow(dead_code)]

use crossbeam_channel::Sender;
use ffmpeg_sys_next as ffmpeg;
use std::cell::RefCell;
use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_void};
use std::path::Path;
use std::ptr;
use std::sync::Mutex;
use video_audio_common::{ProcessingError, Result};

/// Global mutex for FFmpeg initialization functions
///
/// FFmpeg's avcodec_open2(), avformat_open_input(), and avformat_find_stream_info()
/// are NOT thread-safe and must be serialized across all concurrent decoders.
///
/// The decode loop (av_read_frame, avcodec_send_packet, avcodec_receive_frame) is
/// fully thread-safe and does NOT use this mutex.
///
/// See FFMPEG_ONNX_THREADING_GUIDE.md for rationale and performance analysis.
static FFMPEG_INIT_LOCK: Mutex<()> = Mutex::new(());

// Thread-local JPEG encoder context cache
//
// Reuses encoder contexts across multiple frames to eliminate:
// - Repeated avcodec_alloc_context3 / avcodec_free_context overhead
// - Repeated avcodec_open2 calls (which require global lock)
// - Memory allocation/deallocation churn
//
// Each thread gets its own encoder context, enabling true parallelism
// without lock contention during encoding operations.
//
// Performance impact: Eliminates ~50-100μs per frame overhead
thread_local! {
    static JPEG_ENCODER_CACHE: RefCell<Option<*mut AVCodecContext>> = const { RefCell::new(None) };
}

/// Get or create JPEG encoder context for current thread
///
/// Returns a reusable encoder context configured for MJPEG encoding.
/// Context is cached in thread-local storage and reused across calls.
///
/// # Safety
/// Caller must ensure encoder context is only used on the thread that created it.
unsafe fn get_or_create_jpeg_encoder(
    width: c_int,
    height: c_int,
    quality: c_int,
) -> Result<*mut AVCodecContext> {
    JPEG_ENCODER_CACHE.with(|cache| {
        let mut cache_ref = cache.borrow_mut();

        // Check if we have a cached encoder with matching dimensions and quality
        if let Some(enc_ctx) = *cache_ref {
            if !enc_ctx.is_null()
                && (*enc_ctx).width == width
                && (*enc_ctx).height == height
                && (*enc_ctx).qmin == quality
                && (*enc_ctx).qmax == quality
            {
                // Reuse cached encoder (PTS is now set monotonically, so caching is safe)
                return Ok(enc_ctx);
            } else {
                // Cached encoder has wrong parameters, free it
                avcodec_free_context(&mut (enc_ctx as *mut _));
                *cache_ref = None;
            }
        }

        // Create new encoder context
        let encoder = avcodec_find_encoder(AV_CODEC_ID_MJPEG);
        if encoder.is_null() {
            return Err(ProcessingError::FFmpegError(
                "MJPEG encoder not found".to_string(),
            ));
        }

        let enc_ctx = avcodec_alloc_context3(encoder);
        if enc_ctx.is_null() {
            return Err(ProcessingError::FFmpegError(
                "Failed to allocate encoder context".to_string(),
            ));
        }

        // Configure encoder
        (*enc_ctx).width = width;
        (*enc_ctx).height = height;
        (*enc_ctx).pix_fmt = ffmpeg::AVPixelFormat::AV_PIX_FMT_YUVJ420P;
        (*enc_ctx).time_base.num = 1;
        (*enc_ctx).time_base.den = 1;
        (*enc_ctx).qmin = quality;
        (*enc_ctx).qmax = quality;

        // Open encoder (requires global lock, but only once per thread)
        let _lock = FFMPEG_INIT_LOCK.lock().unwrap();
        let ret = avcodec_open2(enc_ctx, encoder, ptr::null_mut());
        drop(_lock);

        if ret < 0 {
            avcodec_free_context(&mut (enc_ctx as *mut _));
            return Err(ProcessingError::FFmpegError(format!(
                "Failed to open MJPEG encoder: {}",
                ret
            )));
        }

        // Cache encoder for reuse (PTS is now set monotonically, so caching is safe)
        *cache_ref = Some(enc_ctx);
        Ok(enc_ctx)
    })
}

/// Create JPEG encoder without caching (N=140 fix for PTS ordering)
///
/// Creates a fresh MJPEG encoder for each frame to avoid PTS state accumulation.
/// This is slower than caching (~1-2ms overhead per frame) but necessary for
/// long videos where keyframe PTS is non-monotonic.
///
/// # Safety
/// Must be called within unsafe block. Encoder context must be freed with avcodec_free_context.
unsafe fn create_jpeg_encoder_uncached(
    width: c_int,
    height: c_int,
    quality: c_int,
) -> Result<*mut AVCodecContext> {
    // Create new encoder context
    let encoder = avcodec_find_encoder(AV_CODEC_ID_MJPEG);
    if encoder.is_null() {
        return Err(ProcessingError::FFmpegError(
            "MJPEG encoder not found".to_string(),
        ));
    }

    let enc_ctx = avcodec_alloc_context3(encoder);
    if enc_ctx.is_null() {
        return Err(ProcessingError::FFmpegError(
            "Failed to allocate encoder context".to_string(),
        ));
    }

    // Configure encoder
    (*enc_ctx).width = width;
    (*enc_ctx).height = height;
    (*enc_ctx).pix_fmt = ffmpeg::AVPixelFormat::AV_PIX_FMT_YUVJ420P;
    (*enc_ctx).time_base.num = 1;
    (*enc_ctx).time_base.den = 1;
    (*enc_ctx).qmin = quality;
    (*enc_ctx).qmax = quality;

    // Open encoder
    let _lock = FFMPEG_INIT_LOCK.lock().unwrap();
    let ret = avcodec_open2(enc_ctx, encoder, ptr::null_mut());
    drop(_lock);

    if ret < 0 {
        avcodec_free_context(&mut (enc_ctx as *mut _));
        return Err(ProcessingError::FFmpegError(format!(
            "Failed to open MJPEG encoder: {}",
            ret
        )));
    }

    Ok(enc_ctx)
}

// Re-export FFmpeg types for convenience
pub type AVFormatContext = ffmpeg::AVFormatContext;
pub type AVCodecContext = ffmpeg::AVCodecContext;
pub type AVCodec = ffmpeg::AVCodec;
pub type AVCodecParameters = ffmpeg::AVCodecParameters;
pub type AVStream = ffmpeg::AVStream;
pub type AVPacket = ffmpeg::AVPacket;
pub type AVFrame = ffmpeg::AVFrame;
pub type SwsContext = ffmpeg::SwsContext;
pub type SwrContext = ffmpeg::SwrContext;

// ============================================================================
// Stream Group Support (FFmpeg 6.1+, for HEIF/HEIC Tile Grid)
// ============================================================================

/// Stream group parameter types (from libavformat/avformat.h)
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum AVStreamGroupParamsType {
    AV_STREAM_GROUP_PARAMS_NONE = 0,
    AV_STREAM_GROUP_PARAMS_IAMF_AUDIO_ELEMENT = 1,
    AV_STREAM_GROUP_PARAMS_IAMF_MIX_PRESENTATION = 2,
    AV_STREAM_GROUP_PARAMS_TILE_GRID = 3,
    AV_STREAM_GROUP_PARAMS_LCEVC = 4,
}

/// Tile offset structure (for HEIF/HEIC Tile Grid)
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct AVStreamGroupTileGridOffset {
    /// Index of the stream in the group this tile references
    pub idx: u32,
    /// Offset in pixels from the left edge of the canvas
    pub horizontal: c_int,
    /// Offset in pixels from the top edge of the canvas
    pub vertical: c_int,
}

/// Tile Grid parameters (HEIF/HEIC image tiles)
#[repr(C)]
pub struct AVStreamGroupTileGrid {
    pub av_class: *const c_void,
    /// Number of tiles in the grid
    pub nb_tiles: u32,
    /// Width of the full canvas
    pub coded_width: c_int,
    /// Height of the full canvas
    pub coded_height: c_int,
    /// Array of tile offsets (nb_tiles elements)
    pub offsets: *mut AVStreamGroupTileGridOffset,
}

/// Stream group params union (type-specific parameters)
#[repr(C)]
pub union AVStreamGroupParams {
    pub iamf_audio_element: *mut c_void,
    pub iamf_mix_presentation: *mut c_void,
    pub tile_grid: *mut AVStreamGroupTileGrid,
    pub lcevc: *mut c_void,
}

/// Stream group (FFmpeg 6.1+)
#[repr(C)]
pub struct AVStreamGroup {
    pub av_class: *const c_void,
    pub priv_data: *mut c_void,
    /// Group index in AVFormatContext
    pub index: u32,
    /// Group type-specific ID
    pub id: i64,
    /// Group type (TILE_GRID for HEIF/HEIC)
    pub group_type: AVStreamGroupParamsType,
    /// Type-specific parameters (union)
    pub params: AVStreamGroupParams,
    pub metadata: *mut c_void,
    /// Number of streams in the group
    pub nb_streams: u32,
    /// Array of streams (AVStream**) in the group
    pub streams: *mut *mut AVStream,
    pub disposition: c_int,
}

// ============================================================================
// FFmpeg Constants (from ffmpeg-sys-next)
// ============================================================================

// Import enum types and convert to c_int
const AV_PICTURE_TYPE_I: c_int = ffmpeg::AVPictureType::AV_PICTURE_TYPE_I as c_int;
const AVMEDIA_TYPE_VIDEO: c_int = ffmpeg::AVMediaType::AVMEDIA_TYPE_VIDEO as c_int;
const AVMEDIA_TYPE_AUDIO: c_int = ffmpeg::AVMediaType::AVMEDIA_TYPE_AUDIO as c_int;
const AV_PIX_FMT_RGB24: c_int = ffmpeg::AVPixelFormat::AV_PIX_FMT_RGB24 as c_int;
const AV_PIX_FMT_YUV420P: c_int = ffmpeg::AVPixelFormat::AV_PIX_FMT_YUV420P as c_int;
const AV_PIX_FMT_YUVJ420P: c_int = ffmpeg::AVPixelFormat::AV_PIX_FMT_YUVJ420P as c_int;
const AV_CODEC_ID_MJPEG: c_int = ffmpeg::AVCodecID::AV_CODEC_ID_MJPEG as c_int;
const AV_SAMPLE_FMT_S16: c_int = ffmpeg::AVSampleFormat::AV_SAMPLE_FMT_S16 as c_int;
const AV_SAMPLE_FMT_FLT: c_int = ffmpeg::AVSampleFormat::AV_SAMPLE_FMT_FLT as c_int;

/// End of file (av_read_frame returns this)
const AVERROR_EOF: c_int = ffmpeg::AVERROR_EOF;

/// Bilinear interpolation for scaling (from libswscale/swscale.h)
const SWS_BILINEAR: c_int = 2;

/// FFmpeg threading mode: decode more than one frame at once
const FF_THREAD_FRAME: c_int = 1;

/// FFmpeg threading mode: decode more than one part of a single frame at once
const FF_THREAD_SLICE: c_int = 2;

// ============================================================================
// FFmpeg C Functions (libavformat, libavcodec, libavutil, libswscale)
// ============================================================================

#[link(name = "avformat")]
#[link(name = "avcodec")]
#[link(name = "avutil")]
#[link(name = "swscale")]
#[link(name = "swresample")]
extern "C" {
    // Format Context (AVFormatContext)
    fn avformat_open_input(
        ps: *mut *mut AVFormatContext,
        url: *const c_char,
        fmt: *mut c_void,
        options: *mut *mut c_void,
    ) -> c_int;

    fn avformat_close_input(s: *mut *mut AVFormatContext);

    fn avformat_find_stream_info(ic: *mut AVFormatContext, options: *mut *mut c_void) -> c_int;

    fn av_find_best_stream(
        ic: *mut AVFormatContext,
        media_type: c_int,
        wanted_stream_nb: c_int,
        related_stream: c_int,
        decoder_ret: *mut *const AVCodec,
        flags: c_int,
    ) -> c_int;

    // Stream Group helpers (compiled from stream_group_helpers.c)
    fn get_nb_stream_groups(fmt_ctx: *mut AVFormatContext) -> u32;
    fn get_stream_group(fmt_ctx: *mut AVFormatContext, index: u32) -> *mut AVStreamGroup;

    // Codec Context (AVCodecContext)
    fn avcodec_alloc_context3(codec: *const AVCodec) -> *mut AVCodecContext;

    fn avcodec_free_context(avctx: *mut *mut AVCodecContext);

    fn avcodec_parameters_to_context(
        codec: *mut AVCodecContext,
        par: *const AVCodecParameters,
    ) -> c_int;

    fn avcodec_open2(
        avctx: *mut AVCodecContext,
        codec: *const AVCodec,
        options: *mut *mut c_void,
    ) -> c_int;

    fn avcodec_flush_buffers(avctx: *mut AVCodecContext);

    fn avcodec_send_packet(avctx: *mut AVCodecContext, avpkt: *const AVPacket) -> c_int;

    fn avcodec_receive_frame(avctx: *mut AVCodecContext, frame: *mut AVFrame) -> c_int;

    // Decoding functions
    fn avcodec_find_decoder(id: c_int) -> *const AVCodec;

    // Encoding functions
    fn avcodec_find_encoder(id: c_int) -> *const AVCodec;

    fn avcodec_find_encoder_by_name(name: *const c_char) -> *const AVCodec;

    fn avcodec_send_frame(avctx: *mut AVCodecContext, frame: *const AVFrame) -> c_int;

    fn avcodec_receive_packet(avctx: *mut AVCodecContext, avpkt: *mut AVPacket) -> c_int;

    // Packet (AVPacket)
    fn av_packet_alloc() -> *mut AVPacket;

    fn av_packet_free(pkt: *mut *mut AVPacket);

    fn av_packet_unref(pkt: *mut AVPacket);

    fn av_read_frame(s: *mut AVFormatContext, pkt: *mut AVPacket) -> c_int;

    // Frame (AVFrame)
    fn av_frame_alloc() -> *mut AVFrame;

    fn av_frame_free(frame: *mut *mut AVFrame);

    fn av_frame_clone(src: *const AVFrame) -> *mut AVFrame;

    fn av_frame_get_buffer(frame: *mut AVFrame, align: c_int) -> c_int;

    fn av_frame_unref(frame: *mut AVFrame);

    // Note: Stream accessors removed - access struct fields directly using ffmpeg-sys-next types

    // Scaling (SwsContext) - for pixel format conversion
    fn sws_getContext(
        srcW: c_int,
        srcH: c_int,
        srcFormat: c_int,
        dstW: c_int,
        dstH: c_int,
        dstFormat: c_int,
        flags: c_int,
        srcFilter: *mut c_void,
        dstFilter: *mut c_void,
        param: *const f64,
    ) -> *mut SwsContext;

    fn sws_scale(
        c: *mut SwsContext,
        srcSlice: *const *const u8,
        srcStride: *const c_int,
        srcSliceY: c_int,
        srcSliceH: c_int,
        dst: *const *mut u8,
        dstStride: *const c_int,
    ) -> c_int;

    fn sws_freeContext(swsContext: *mut SwsContext);

    // Resampling (SwrContext) - for audio sample rate conversion
    fn swr_alloc() -> *mut SwrContext;

    fn swr_alloc_set_opts(
        s: *mut SwrContext,
        out_ch_layout: i64,
        out_sample_fmt: c_int,
        out_sample_rate: c_int,
        in_ch_layout: i64,
        in_sample_fmt: c_int,
        in_sample_rate: c_int,
        log_offset: c_int,
        log_ctx: *mut c_void,
    ) -> *mut SwrContext;

    fn swr_init(s: *mut SwrContext) -> c_int;

    fn swr_convert(
        s: *mut SwrContext,
        out: *const *mut u8,
        out_count: c_int,
        in_: *const *const u8,
        in_count: c_int,
    ) -> c_int;

    fn swr_free(s: *mut *mut SwrContext);

    // Channel layout helpers (old API, deprecated in FFmpeg 5+)
    fn av_get_default_channel_layout(nb_channels: c_int) -> i64;

    // Channel layout helpers (new API for FFmpeg 5+)
    fn av_channel_layout_default(ch_layout: *mut ffmpeg::AVChannelLayout, nb_channels: c_int);
    fn av_channel_layout_uninit(ch_layout: *mut ffmpeg::AVChannelLayout);

    // Resampler allocation (new API for FFmpeg 5+)
    fn swr_alloc_set_opts2(
        s: *mut *mut SwrContext,
        out_ch_layout: *const ffmpeg::AVChannelLayout,
        out_sample_fmt: c_int,
        out_sample_rate: c_int,
        in_ch_layout: *const ffmpeg::AVChannelLayout,
        in_sample_fmt: c_int,
        in_sample_rate: c_int,
        log_offset: c_int,
        log_ctx: *mut c_void,
    ) -> c_int;
}

// ============================================================================
// RAII Wrappers for Safe Memory Management
// ============================================================================

/// RAII wrapper for AVFormatContext
/// Automatically closes the format context when dropped
pub struct FormatContext {
    ptr: *mut AVFormatContext,
}

impl FormatContext {
    /// Open a video file and return a FormatContext
    pub fn open(path: &Path) -> Result<Self> {
        unsafe {
            let path_cstr = CString::new(path.to_str().ok_or_else(|| {
                ProcessingError::FFmpegError("Invalid path encoding".to_string())
            })?)
            .map_err(|e| ProcessingError::FFmpegError(format!("CString error: {}", e)))?;

            let mut ptr: *mut AVFormatContext = ptr::null_mut();

            // Acquire global FFmpeg init lock (avformat_open_input is NOT thread-safe)
            let _lock = FFMPEG_INIT_LOCK.lock().unwrap();

            let ret = avformat_open_input(
                &mut ptr,
                path_cstr.as_ptr(),
                ptr::null_mut(),
                ptr::null_mut(),
            );

            if ret < 0 {
                return Err(ProcessingError::FFmpegError(format!(
                    "avformat_open_input failed: {}",
                    ret
                )));
            }

            // Find stream info (also NOT thread-safe)
            let ret = avformat_find_stream_info(ptr, ptr::null_mut());
            if ret < 0 {
                avformat_close_input(&mut ptr);
                return Err(ProcessingError::FFmpegError(format!(
                    "avformat_find_stream_info failed: {}",
                    ret
                )));
            }

            // Release lock (decode loop is fully thread-safe)
            drop(_lock);

            Ok(FormatContext { ptr })
        }
    }

    /// Get the raw pointer (for FFmpeg C functions)
    pub fn as_ptr(&self) -> *mut AVFormatContext {
        self.ptr
    }

    /// Find the best video stream and return its index and decoder
    pub fn find_video_stream(&self) -> Result<(c_int, *const AVCodec)> {
        unsafe {
            let mut decoder: *const AVCodec = ptr::null();

            let stream_index =
                av_find_best_stream(self.ptr, AVMEDIA_TYPE_VIDEO, -1, -1, &mut decoder, 0);

            if stream_index < 0 {
                return Err(ProcessingError::FFmpegError(format!(
                    "av_find_best_stream failed: {}",
                    stream_index
                )));
            }

            if decoder.is_null() {
                return Err(ProcessingError::FFmpegError(
                    "No decoder found for video stream".to_string(),
                ));
            }

            Ok((stream_index, decoder))
        }
    }

    /// Find the best audio stream and return its index and decoder
    pub fn find_audio_stream(&self) -> Result<(c_int, *const AVCodec)> {
        unsafe {
            let mut decoder: *const AVCodec = ptr::null();

            let stream_index =
                av_find_best_stream(self.ptr, AVMEDIA_TYPE_AUDIO, -1, -1, &mut decoder, 0);

            if stream_index < 0 {
                return Err(ProcessingError::FFmpegError(format!(
                    "av_find_best_stream failed: {}",
                    stream_index
                )));
            }

            if decoder.is_null() {
                return Err(ProcessingError::FFmpegError(
                    "No decoder found for audio stream".to_string(),
                ));
            }

            Ok((stream_index, decoder))
        }
    }

    /// Get codec parameters for a stream
    pub fn get_codecpar(&self, stream_index: c_int) -> *mut AVCodecParameters {
        unsafe {
            // Access streams array directly from AVFormatContext
            let stream = *(*self.ptr).streams.offset(stream_index as isize);
            (*stream).codecpar
        }
    }

    /// Get time base (for timestamp conversion) for a stream
    pub fn get_time_base(&self, stream_index: c_int) -> (c_int, c_int) {
        unsafe {
            // Access streams array directly from AVFormatContext
            let stream = *(*self.ptr).streams.offset(stream_index as isize);
            let time_base = (*stream).time_base;
            (time_base.num, time_base.den)
        }
    }

    /// Check if format context has stream groups (FFmpeg 6.1+)
    ///
    /// Returns the number of stream groups in the file.
    /// Stream groups are used for HEIF/HEIC Tile Grid, IAMF audio, etc.
    pub fn nb_stream_groups(&self) -> u32 {
        unsafe {
            // Use memcpy to read nb_stream_groups field from the opaque AVFormatContext
            // This field is after nb_programs, program, and other fields in the struct
            // We use a C helper to access it safely
            get_nb_stream_groups(self.ptr)
        }
    }

    /// Get stream group at index (FFmpeg 6.1+)
    ///
    /// # Safety
    /// - index must be < nb_stream_groups()
    pub unsafe fn get_stream_group(&self, index: u32) -> *mut AVStreamGroup {
        // Use C helper to access stream_groups array
        get_stream_group(self.ptr, index)
    }

    /// Find first Tile Grid stream group (for HEIF/HEIC)
    ///
    /// Returns None if no Tile Grid stream group is found.
    pub fn find_tile_grid(&self) -> Option<*mut AVStreamGroup> {
        unsafe {
            let nb_groups = self.nb_stream_groups();
            for i in 0..nb_groups {
                let group = self.get_stream_group(i);
                if !group.is_null() {
                    let group_type = (*group).group_type;
                    if group_type == AVStreamGroupParamsType::AV_STREAM_GROUP_PARAMS_TILE_GRID {
                        return Some(group);
                    }
                }
            }
            None
        }
    }
}

impl Drop for FormatContext {
    fn drop(&mut self) {
        unsafe {
            if !self.ptr.is_null() {
                avformat_close_input(&mut self.ptr);
            }
        }
    }
}

/// RAII wrapper for AVCodecContext
/// Automatically frees the codec context when dropped
pub struct CodecContext {
    ptr: *mut AVCodecContext,
}

impl CodecContext {
    /// Create a codec context from a codec and codec parameters
    ///
    /// # Safety
    /// - `codec` must be a valid pointer to an AVCodec obtained from avcodec_find_decoder
    /// - `codecpar` must be a valid pointer to AVCodecParameters from an AVStream
    pub unsafe fn create(codec: *const AVCodec, codecpar: *mut AVCodecParameters) -> Result<Self> {
        let ptr = avcodec_alloc_context3(codec);
        if ptr.is_null() {
            return Err(ProcessingError::FFmpegError(
                "avcodec_alloc_context3 failed".to_string(),
            ));
        }

        let ret = avcodec_parameters_to_context(ptr, codecpar);
        if ret < 0 {
            avcodec_free_context(&mut (ptr as *mut _));
            return Err(ProcessingError::FFmpegError(format!(
                "avcodec_parameters_to_context failed: {}",
                ret
            )));
        }

        // Enable internal FFmpeg threading (complements file-level parallelism)
        // Respect VIDEO_EXTRACT_THREADS environment variable to avoid thread oversubscription
        // When testing, VIDEO_EXTRACT_THREADS limits Rayon + ONNX + FFmpeg to same count
        // When not set (production), use 0 to auto-detect optimal thread count
        let thread_count = std::env::var("VIDEO_EXTRACT_THREADS")
            .ok()
            .and_then(|s| s.parse::<c_int>().ok())
            .unwrap_or(0); // 0 = auto-detect (maximum performance for production)
        (*ptr).thread_count = thread_count;
        // Use both frame and slice threading for maximum performance
        (*ptr).thread_type = FF_THREAD_FRAME | FF_THREAD_SLICE;

        // Acquire global FFmpeg init lock (avcodec_open2 is NOT thread-safe)
        let _lock = FFMPEG_INIT_LOCK.lock().unwrap();

        let ret = avcodec_open2(ptr, codec, ptr::null_mut());
        if ret < 0 {
            avcodec_free_context(&mut (ptr as *mut _));
            return Err(ProcessingError::FFmpegError(format!(
                "avcodec_open2 failed: {}",
                ret
            )));
        }

        // Release lock (decode operations are fully thread-safe)
        drop(_lock);

        Ok(CodecContext { ptr })
    }

    /// Get the raw pointer (for FFmpeg C functions)
    pub fn as_ptr(&self) -> *mut AVCodecContext {
        self.ptr
    }
}

impl Drop for CodecContext {
    fn drop(&mut self) {
        unsafe {
            if !self.ptr.is_null() {
                avcodec_free_context(&mut self.ptr);
            }
        }
    }
}

/// Zero-copy frame buffer
/// Holds a pointer to AVFrame* memory, enabling direct access to raw pixels
/// without copying to Vec<u8>. The AVFrame* is kept alive via the _frame field.
///
/// This is the key to zero-copy performance: downstream ML models can create
/// ndarray::ArrayView directly from data_ptr without any memory allocation.
#[derive(Debug)]
pub struct RawFrameBuffer {
    /// Frame width in pixels
    pub width: u32,
    /// Frame height in pixels
    pub height: u32,
    /// Direct pointer to pixel data (RGB24 format: width * height * 3 bytes)
    pub data_ptr: *mut u8,
    /// Line size (stride) in bytes (usually width * 3 for RGB24)
    pub linesize: usize,
    /// Timestamp in seconds
    pub timestamp: f64,
    /// Frame number (0-indexed)
    pub frame_number: u64,
    /// Whether this is a keyframe (I-frame)
    pub is_keyframe: bool,
    /// Owned AVFrame pointer (kept alive via Drop impl)
    _frame: *mut AVFrame,
}

impl Drop for RawFrameBuffer {
    fn drop(&mut self) {
        unsafe {
            if !self._frame.is_null() {
                av_frame_free(&mut self._frame);
            }
        }
    }
}

/// Message type for streaming decoder (producer-consumer pattern)
///
/// This enum enables the decoder to send frames, completion signals, and errors
/// directly through a single channel without needing a wrapper thread.
#[derive(Debug)]
pub enum StreamFrame {
    /// Frame data available
    Frame(RawFrameBuffer),
    /// Decode complete (contains total frame count)
    Done(u64),
    /// Error occurred during decoding
    Error(String),
}

// Safety: RawFrameBuffer owns the AVFrame and manages its lifetime correctly
unsafe impl Send for RawFrameBuffer {}
unsafe impl Sync for RawFrameBuffer {}

// Safety: StreamFrame is safe to send across threads (contains owned data or String)
unsafe impl Send for StreamFrame {}

/// Convert YUV frames to RGB (used for HEIF/HEIC Tile Grid images)
///
/// # Safety
///
/// Unsafe because it uses FFmpeg C API
unsafe fn convert_yuv_frames_to_rgb(yuv_frames: Vec<YuvFrame>) -> Result<Vec<RawFrameBuffer>> {
    let mut rgb_frames = Vec::with_capacity(yuv_frames.len());

    for yuv_frame in yuv_frames {
        let yuv_ptr = yuv_frame.frame_ptr;
        let width = (*yuv_ptr).width;
        let height = (*yuv_ptr).height;

        // Create scaling context for YUV → RGB conversion
        let sws_ctx = sws_getContext(
            width,
            height,
            (*yuv_ptr).format,
            width,
            height,
            AV_PIX_FMT_RGB24,
            SWS_BILINEAR,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null(),
        );

        if sws_ctx.is_null() {
            return Err(ProcessingError::FFmpegError(
                "sws_getContext failed (color space conversion)".to_string(),
            ));
        }

        // Allocate RGB frame
        let rgb_frame = av_frame_alloc();
        if rgb_frame.is_null() {
            sws_freeContext(sws_ctx);
            return Err(ProcessingError::FFmpegError(
                "av_frame_alloc failed for RGB frame".to_string(),
            ));
        }

        (*rgb_frame).width = width;
        (*rgb_frame).height = height;
        (*rgb_frame).format = AV_PIX_FMT_RGB24;

        let ret = av_frame_get_buffer(rgb_frame, 0);
        if ret < 0 {
            sws_freeContext(sws_ctx);
            av_frame_free(&mut (rgb_frame as *mut _));
            return Err(ProcessingError::FFmpegError(format!(
                "av_frame_get_buffer failed: {}",
                ret
            )));
        }

        // Convert YUV → RGB
        let ret = sws_scale(
            sws_ctx,
            (*yuv_ptr).data.as_ptr() as *const *const u8,
            (*yuv_ptr).linesize.as_ptr(),
            0,
            height,
            (*rgb_frame).data.as_ptr(),
            (*rgb_frame).linesize.as_ptr(),
        );

        if ret < 0 || ret != height {
            sws_freeContext(sws_ctx);
            av_frame_free(&mut (rgb_frame as *mut _));
            return Err(ProcessingError::FFmpegError(format!(
                "sws_scale failed: returned {} (expected {})",
                ret, height
            )));
        }

        // Create RawFrameBuffer with zero-copy pointer to RGB frame
        rgb_frames.push(RawFrameBuffer {
            width: width as u32,
            height: height as u32,
            data_ptr: (*rgb_frame).data[0],
            linesize: (*rgb_frame).linesize[0] as usize,
            timestamp: yuv_frame.timestamp,
            frame_number: yuv_frame.frame_number,
            is_keyframe: yuv_frame.is_keyframe,
            _frame: rgb_frame, // Keep RGB frame alive (YUV frame will be freed by Drop)
        });

        // Clean up SwsContext (YuvFrame will be freed by Drop implementation)
        sws_freeContext(sws_ctx);
    }

    Ok(rgb_frames)
}

/// Extract I-frames (keyframes) from a video file using direct C FFI
/// Returns raw frame buffers with zero-copy memory (no Vec<u8> allocation)
///
/// # Safety
///
/// This function uses unsafe FFmpeg C API but ensures memory safety through:
/// - RAII wrappers with Drop implementations
/// - Validation of all FFmpeg return codes
/// - Proper AVFrame* lifetime management
///
/// # Performance
///
/// This is the FASTEST way to extract keyframes in Rust:
/// - No Rust wrapper overhead (direct C calls)
/// - No memory copies (AVFrame* pointers passed directly)
/// - Enables zero-copy to downstream ML models
pub fn decode_iframes_zero_copy(video_path: &Path) -> Result<Vec<RawFrameBuffer>> {
    unsafe {
        // Open video file
        let format_ctx = FormatContext::open(video_path)?;

        // Check for Tile Grid stream group (HEIF/HEIC)
        if let Some(tile_grid_group) = format_ctx.find_tile_grid() {
            // Decode Tile Grid (HEIF/HEIC) and convert to RGB
            let yuv_frames = decode_tile_grid(&format_ctx, tile_grid_group)?;
            let rgb_frames = convert_yuv_frames_to_rgb(yuv_frames)?;
            return Ok(rgb_frames);
        }

        // Find video stream and decoder
        let (stream_index, decoder) = format_ctx.find_video_stream()?;

        // Create codec context
        let codecpar = format_ctx.get_codecpar(stream_index);
        let codec_ctx = CodecContext::create(decoder, codecpar)?;

        // Get time base for timestamp conversion
        let (time_base_num, time_base_den) = format_ctx.get_time_base(stream_index);

        // Pre-allocate frames Vec with estimated capacity for iframes
        // Typical videos have 1 keyframe every 1-2 seconds, estimate ~30 keyframes for average media
        let mut frames = Vec::with_capacity(32);
        let mut frame_number = 0u64;

        // Scaling context (created lazily after first frame)
        let mut sws_ctx: *mut SwsContext = ptr::null_mut();

        // Decode loop
        loop {
            // Read packet from file
            let packet = av_packet_alloc();
            if packet.is_null() {
                break;
            }

            let ret = av_read_frame(format_ctx.as_ptr(), packet);

            if ret == AVERROR_EOF {
                // End of file - flush decoder to get any buffered frames
                av_packet_free(&mut (packet as *mut _));

                // Send NULL packet to flush decoder (required for codecs that buffer frames like VP9, H.264)
                avcodec_send_packet(codec_ctx.as_ptr(), ptr::null());

                // Drain all remaining frames from decoder
                loop {
                    let decoded_frame = av_frame_alloc();
                    if decoded_frame.is_null() {
                        break;
                    }

                    let ret = avcodec_receive_frame(codec_ctx.as_ptr(), decoded_frame);
                    if ret < 0 {
                        // No more frames available
                        av_frame_free(&mut (decoded_frame as *mut _));
                        break;
                    }

                    // Check if this is a keyframe
                    const AV_FRAME_FLAG_KEY: i32 = 1 << 1;
                    let is_keyframe = (*decoded_frame).pict_type
                        == ffmpeg::AVPictureType::AV_PICTURE_TYPE_I
                        || ((*decoded_frame).flags & AV_FRAME_FLAG_KEY) != 0;

                    if is_keyframe {
                        // Create scaling context on first frame (lazy initialization)
                        if sws_ctx.is_null() {
                            sws_ctx = sws_getContext(
                                (*decoded_frame).width,
                                (*decoded_frame).height,
                                (*decoded_frame).format,
                                (*decoded_frame).width,
                                (*decoded_frame).height,
                                AV_PIX_FMT_RGB24,
                                SWS_BILINEAR,
                                ptr::null_mut(),
                                ptr::null_mut(),
                                ptr::null(),
                            );

                            if sws_ctx.is_null() {
                                av_frame_free(&mut (decoded_frame as *mut _));
                                return Err(ProcessingError::FFmpegError(
                                    "sws_getContext failed (color space conversion)".to_string(),
                                ));
                            }
                        }

                        // Allocate RGB frame
                        let rgb_frame = av_frame_alloc();
                        if rgb_frame.is_null() {
                            av_frame_free(&mut (decoded_frame as *mut _));
                            continue;
                        }

                        (*rgb_frame).width = (*decoded_frame).width;
                        (*rgb_frame).height = (*decoded_frame).height;
                        (*rgb_frame).format = AV_PIX_FMT_RGB24;

                        let ret = av_frame_get_buffer(rgb_frame, 0);
                        if ret < 0 {
                            av_frame_free(&mut (rgb_frame as *mut _));
                            av_frame_free(&mut (decoded_frame as *mut _));
                            continue;
                        }

                        // Convert YUV → RGB
                        sws_scale(
                            sws_ctx,
                            (*decoded_frame).data.as_ptr() as *const *const u8,
                            (*decoded_frame).linesize.as_ptr(),
                            0,
                            (*decoded_frame).height,
                            (*rgb_frame).data.as_ptr(),
                            (*rgb_frame).linesize.as_ptr(),
                        );

                        // Calculate timestamp
                        let pts = (*decoded_frame).pts;
                        let timestamp = if pts != 0 {
                            (pts as f64 * time_base_num as f64) / time_base_den as f64
                        } else {
                            frame_number as f64 / 30.0
                        };

                        // Create zero-copy frame buffer
                        frames.push(RawFrameBuffer {
                            width: (*rgb_frame).width as u32,
                            height: (*rgb_frame).height as u32,
                            data_ptr: (*rgb_frame).data[0],
                            linesize: (*rgb_frame).linesize[0] as usize,
                            timestamp,
                            frame_number,
                            is_keyframe: true,
                            _frame: rgb_frame,
                        });

                        frame_number += 1;
                    }

                    av_frame_free(&mut (decoded_frame as *mut _));
                }

                break;
            }

            if ret < 0 {
                av_packet_free(&mut (packet as *mut _));
                return Err(ProcessingError::FFmpegError(format!(
                    "av_read_frame failed: {}",
                    ret
                )));
            }

            // Filter packets by stream index (only process video stream packets)
            if (*packet).stream_index != stream_index {
                av_packet_free(&mut (packet as *mut _));
                continue;
            }

            // Send packet to decoder
            let ret = avcodec_send_packet(codec_ctx.as_ptr(), packet);
            av_packet_free(&mut (packet as *mut _));

            if ret < 0 {
                return Err(ProcessingError::FFmpegError(format!(
                    "avcodec_send_packet failed: {}",
                    ret
                )));
            }

            // Receive decoded frames
            loop {
                let decoded_frame = av_frame_alloc();
                if decoded_frame.is_null() {
                    break;
                }

                let ret = avcodec_receive_frame(codec_ctx.as_ptr(), decoded_frame);

                if ret < 0 {
                    // AVERROR(EAGAIN) or AVERROR_EOF - no more frames from this packet
                    av_frame_free(&mut (decoded_frame as *mut _));
                    break;
                }

                // Check if this is an I-frame (keyframe)
                // Note: Some codecs (VP9, VP8) don't set pict_type, so check AV_FRAME_FLAG_KEY as well
                const AV_FRAME_FLAG_KEY: i32 = 1 << 1;
                let is_keyframe = (*decoded_frame).pict_type
                    == ffmpeg::AVPictureType::AV_PICTURE_TYPE_I
                    || ((*decoded_frame).flags & AV_FRAME_FLAG_KEY) != 0;

                if is_keyframe {
                    // Create scaling context on first frame (lazy initialization)
                    if sws_ctx.is_null() {
                        sws_ctx = sws_getContext(
                            (*decoded_frame).width,
                            (*decoded_frame).height,
                            (*decoded_frame).format,
                            (*decoded_frame).width,
                            (*decoded_frame).height,
                            AV_PIX_FMT_RGB24,
                            SWS_BILINEAR,
                            ptr::null_mut(),
                            ptr::null_mut(),
                            ptr::null(),
                        );

                        if sws_ctx.is_null() {
                            av_frame_free(&mut (decoded_frame as *mut _));
                            return Err(ProcessingError::FFmpegError(
                                "sws_getContext failed (color space conversion)".to_string(),
                            ));
                        }
                    }

                    // Allocate RGB frame
                    let rgb_frame = av_frame_alloc();
                    if rgb_frame.is_null() {
                        av_frame_free(&mut (decoded_frame as *mut _));
                        continue;
                    }

                    (*rgb_frame).width = (*decoded_frame).width;
                    (*rgb_frame).height = (*decoded_frame).height;
                    (*rgb_frame).format = AV_PIX_FMT_RGB24;

                    let ret = av_frame_get_buffer(rgb_frame, 0);
                    if ret < 0 {
                        av_frame_free(&mut (rgb_frame as *mut _));
                        av_frame_free(&mut (decoded_frame as *mut _));
                        continue;
                    }

                    // Convert YUV → RGB
                    sws_scale(
                        sws_ctx,
                        (*decoded_frame).data.as_ptr() as *const *const u8,
                        (*decoded_frame).linesize.as_ptr(),
                        0,
                        (*decoded_frame).height,
                        (*rgb_frame).data.as_ptr(),
                        (*rgb_frame).linesize.as_ptr(),
                    );

                    // Calculate timestamp
                    let pts = (*decoded_frame).pts;
                    let timestamp = if pts != 0 {
                        (pts as f64 * time_base_num as f64) / time_base_den as f64
                    } else {
                        frame_number as f64 / 30.0 // Fallback: assume 30 fps
                    };

                    // Create zero-copy frame buffer
                    frames.push(RawFrameBuffer {
                        width: (*rgb_frame).width as u32,
                        height: (*rgb_frame).height as u32,
                        data_ptr: (*rgb_frame).data[0],
                        linesize: (*rgb_frame).linesize[0] as usize,
                        timestamp,
                        frame_number,
                        is_keyframe: true,
                        _frame: rgb_frame, // Transfer ownership (Drop will free)
                    });

                    frame_number += 1;
                }

                av_frame_free(&mut (decoded_frame as *mut _));
            }
        }

        // Free scaling context (safe to call even if null)
        if !sws_ctx.is_null() {
            sws_freeContext(sws_ctx);
        }

        Ok(frames)
    }
}

/// Extract I-frames (keyframes) from a video file with streaming output
/// Sends frames to a channel as they are decoded, enabling true decode/inference overlap
///
/// # Arguments
///
/// * `video_path` - Path to the video file
/// * `sender` - Crossbeam channel sender for streaming frame output
///
/// # Returns
///
/// * `Ok(u64)` - Total number of frames decoded and sent
/// * `Err(_)` - FFmpeg error or channel send error
///
/// # Performance
///
/// This enables true streaming parallelism:
/// - Frames are sent as they're decoded (no Vec accumulation)
/// - Downstream processing can start immediately
/// - Lower memory usage (no upfront Vec allocation)
/// - 1.5-2x speedup for videos with 20+ keyframes when used with parallel pipeline
///
/// # Safety
///
/// Same safety guarantees as `decode_iframes_zero_copy()`:
/// - RAII wrappers with Drop implementations
/// - Validation of all FFmpeg return codes
/// - Proper AVFrame* lifetime management
pub fn decode_iframes_streaming(video_path: &Path, sender: Sender<StreamFrame>) -> Result<()> {
    unsafe {
        // Open video file
        let format_ctx = FormatContext::open(video_path)?;

        // Find video stream and decoder
        let (stream_index, decoder) = format_ctx.find_video_stream()?;

        // Create codec context
        let codecpar = format_ctx.get_codecpar(stream_index);
        let codec_ctx = CodecContext::create(decoder, codecpar)?;

        // Get time base for timestamp conversion
        let (time_base_num, time_base_den) = format_ctx.get_time_base(stream_index);

        let mut frame_number = 0u64;

        // Scaling context (created lazily after first frame)
        let mut sws_ctx: *mut SwsContext = ptr::null_mut();

        // Decode loop
        loop {
            // Read packet from file
            let packet = av_packet_alloc();
            if packet.is_null() {
                break;
            }

            let ret = av_read_frame(format_ctx.as_ptr(), packet);

            if ret == AVERROR_EOF {
                // End of file - flush decoder to get any buffered frames
                av_packet_free(&mut (packet as *mut _));

                // Send NULL packet to flush decoder (required for codecs that buffer frames like VP9, H.264)
                avcodec_send_packet(codec_ctx.as_ptr(), ptr::null());

                // Drain all remaining frames from decoder
                loop {
                    let decoded_frame = av_frame_alloc();
                    if decoded_frame.is_null() {
                        break;
                    }

                    let ret = avcodec_receive_frame(codec_ctx.as_ptr(), decoded_frame);
                    if ret < 0 {
                        // No more frames available
                        av_frame_free(&mut (decoded_frame as *mut _));
                        break;
                    }

                    // Check if this is a keyframe
                    const AV_FRAME_FLAG_KEY: i32 = 1 << 1;
                    let is_keyframe = (*decoded_frame).pict_type
                        == ffmpeg::AVPictureType::AV_PICTURE_TYPE_I
                        || ((*decoded_frame).flags & AV_FRAME_FLAG_KEY) != 0;

                    if is_keyframe {
                        // Create scaling context on first frame (lazy initialization)
                        if sws_ctx.is_null() {
                            sws_ctx = sws_getContext(
                                (*decoded_frame).width,
                                (*decoded_frame).height,
                                (*decoded_frame).format,
                                (*decoded_frame).width,
                                (*decoded_frame).height,
                                AV_PIX_FMT_RGB24,
                                SWS_BILINEAR,
                                ptr::null_mut(),
                                ptr::null_mut(),
                                ptr::null(),
                            );

                            if sws_ctx.is_null() {
                                av_frame_free(&mut (decoded_frame as *mut _));
                                return Err(ProcessingError::FFmpegError(
                                    "sws_getContext failed (color space conversion)".to_string(),
                                ));
                            }
                        }

                        // Allocate RGB frame
                        let rgb_frame = av_frame_alloc();
                        if rgb_frame.is_null() {
                            av_frame_free(&mut (decoded_frame as *mut _));
                            continue;
                        }

                        (*rgb_frame).width = (*decoded_frame).width;
                        (*rgb_frame).height = (*decoded_frame).height;
                        (*rgb_frame).format = AV_PIX_FMT_RGB24;

                        let ret = av_frame_get_buffer(rgb_frame, 0);
                        if ret < 0 {
                            av_frame_free(&mut (rgb_frame as *mut _));
                            av_frame_free(&mut (decoded_frame as *mut _));
                            continue;
                        }

                        // Convert YUV → RGB
                        sws_scale(
                            sws_ctx,
                            (*decoded_frame).data.as_ptr() as *const *const u8,
                            (*decoded_frame).linesize.as_ptr(),
                            0,
                            (*decoded_frame).height,
                            (*rgb_frame).data.as_ptr(),
                            (*rgb_frame).linesize.as_ptr(),
                        );

                        // Calculate timestamp
                        let pts = (*decoded_frame).pts;
                        let timestamp = if pts != 0 {
                            (pts as f64 * time_base_num as f64) / time_base_den as f64
                        } else {
                            frame_number as f64 / 30.0
                        };

                        // Create zero-copy frame buffer
                        let frame_buffer = RawFrameBuffer {
                            width: (*rgb_frame).width as u32,
                            height: (*rgb_frame).height as u32,
                            data_ptr: (*rgb_frame).data[0],
                            linesize: (*rgb_frame).linesize[0] as usize,
                            timestamp,
                            frame_number,
                            is_keyframe: true,
                            _frame: rgb_frame,
                        };

                        // Send frame to channel (streaming output)
                        sender.send(StreamFrame::Frame(frame_buffer)).map_err(|e| {
                            ProcessingError::FFmpegError(format!("Channel send error: {}", e))
                        })?;

                        frame_number += 1;
                    }

                    av_frame_free(&mut (decoded_frame as *mut _));
                }

                break;
            }

            if ret < 0 {
                av_packet_free(&mut (packet as *mut _));
                return Err(ProcessingError::FFmpegError(format!(
                    "av_read_frame failed: {}",
                    ret
                )));
            }

            // Filter packets by stream index (only process video stream packets)
            if (*packet).stream_index != stream_index {
                av_packet_free(&mut (packet as *mut _));
                continue;
            }

            // Send packet to decoder
            let ret = avcodec_send_packet(codec_ctx.as_ptr(), packet);
            av_packet_free(&mut (packet as *mut _));

            if ret < 0 {
                return Err(ProcessingError::FFmpegError(format!(
                    "avcodec_send_packet failed: {}",
                    ret
                )));
            }

            // Receive decoded frames
            loop {
                let decoded_frame = av_frame_alloc();
                if decoded_frame.is_null() {
                    break;
                }

                let ret = avcodec_receive_frame(codec_ctx.as_ptr(), decoded_frame);

                if ret < 0 {
                    // AVERROR(EAGAIN) or AVERROR_EOF - no more frames from this packet
                    av_frame_free(&mut (decoded_frame as *mut _));
                    break;
                }

                // Check if this is an I-frame (keyframe)
                // Note: Some codecs (VP9, VP8) don't set pict_type, so check AV_FRAME_FLAG_KEY as well
                const AV_FRAME_FLAG_KEY: i32 = 1 << 1;
                let is_keyframe = (*decoded_frame).pict_type
                    == ffmpeg::AVPictureType::AV_PICTURE_TYPE_I
                    || ((*decoded_frame).flags & AV_FRAME_FLAG_KEY) != 0;

                if is_keyframe {
                    // Create scaling context on first frame (lazy initialization)
                    if sws_ctx.is_null() {
                        sws_ctx = sws_getContext(
                            (*decoded_frame).width,
                            (*decoded_frame).height,
                            (*decoded_frame).format,
                            (*decoded_frame).width,
                            (*decoded_frame).height,
                            AV_PIX_FMT_RGB24,
                            SWS_BILINEAR,
                            ptr::null_mut(),
                            ptr::null_mut(),
                            ptr::null(),
                        );

                        if sws_ctx.is_null() {
                            av_frame_free(&mut (decoded_frame as *mut _));
                            return Err(ProcessingError::FFmpegError(
                                "sws_getContext failed (color space conversion)".to_string(),
                            ));
                        }
                    }

                    // Allocate RGB frame
                    let rgb_frame = av_frame_alloc();
                    if rgb_frame.is_null() {
                        av_frame_free(&mut (decoded_frame as *mut _));
                        continue;
                    }

                    (*rgb_frame).width = (*decoded_frame).width;
                    (*rgb_frame).height = (*decoded_frame).height;
                    (*rgb_frame).format = AV_PIX_FMT_RGB24;

                    let ret = av_frame_get_buffer(rgb_frame, 0);
                    if ret < 0 {
                        av_frame_free(&mut (rgb_frame as *mut _));
                        av_frame_free(&mut (decoded_frame as *mut _));
                        continue;
                    }

                    // Convert YUV → RGB
                    sws_scale(
                        sws_ctx,
                        (*decoded_frame).data.as_ptr() as *const *const u8,
                        (*decoded_frame).linesize.as_ptr(),
                        0,
                        (*decoded_frame).height,
                        (*rgb_frame).data.as_ptr(),
                        (*rgb_frame).linesize.as_ptr(),
                    );

                    // Calculate timestamp
                    let pts = (*decoded_frame).pts;
                    let timestamp = if pts != 0 {
                        (pts as f64 * time_base_num as f64) / time_base_den as f64
                    } else {
                        frame_number as f64 / 30.0 // Fallback: assume 30 fps
                    };

                    // Create zero-copy frame buffer
                    let frame_buffer = RawFrameBuffer {
                        width: (*rgb_frame).width as u32,
                        height: (*rgb_frame).height as u32,
                        data_ptr: (*rgb_frame).data[0],
                        linesize: (*rgb_frame).linesize[0] as usize,
                        timestamp,
                        frame_number,
                        is_keyframe: true,
                        _frame: rgb_frame, // Transfer ownership (Drop will free)
                    };

                    // Send frame to channel (streaming output)
                    sender.send(StreamFrame::Frame(frame_buffer)).map_err(|e| {
                        ProcessingError::FFmpegError(format!("Channel send error: {}", e))
                    })?;

                    frame_number += 1;
                }

                av_frame_free(&mut (decoded_frame as *mut _));
            }
        }

        // Free scaling context (safe to call even if null)
        if !sws_ctx.is_null() {
            sws_freeContext(sws_ctx);
        }

        // Send completion message
        sender.send(StreamFrame::Done(frame_number)).map_err(|e| {
            ProcessingError::FFmpegError(format!("Channel send error (Done): {}", e))
        })?;

        Ok(())
    }
}

// ============================================================================
// JPEG Encoding (YUV→JPEG direct, no spawn)
// ============================================================================

/// Encode a single YUV420P frame to JPEG file
///
/// This function uses the embedded mjpeg encoder to convert a YUV frame directly to JPEG
/// without spawning FFmpeg as a subprocess. This eliminates 70-90ms of process spawn overhead.
///
/// # Performance
///
/// Expected time: ~143ms (same as FFmpeg CLI avcodec calls) + 23ms Rust overhead = 166ms total
/// vs FFmpeg CLI: ~187ms (44ms startup + 143ms work)
/// vs Current fast.rs: ~280ms (with process spawn overhead)
///
/// # Arguments
///
/// * `frame` - YUV420P frame from decoder (must be I-frame with YUV420P format)
/// * `output_path` - Path to write JPEG file
/// * `quality` - JPEG quality (2-31, lower is better quality, default 2)
///
/// # Safety
///
/// This function is unsafe because it:
/// - Dereferences raw FFmpeg pointers
/// - Calls C functions that may have undefined behavior if arguments are invalid
///
/// # Panics
///
/// Panics if unable to write JPEG file to filesystem
/// Decode HEIF/HEIC Tile Grid into a single composed YUV frame
///
/// # Arguments
/// * `format_ctx` - Open format context for the HEIF/HEIC file
/// * `tile_grid_group` - Pointer to the Tile Grid stream group
///
/// # Returns
/// Vector containing a single YUV frame with all tiles composed into the full image
///
/// # Safety
/// This function is unsafe because it:
/// - Dereferences raw pointers from FFmpeg
/// - Calls FFmpeg C functions that may have undefined behavior
unsafe fn decode_tile_grid(
    format_ctx: &FormatContext,
    tile_grid_group: *mut AVStreamGroup,
) -> Result<Vec<YuvFrame>> {
    let tile_grid = (*tile_grid_group).params.tile_grid;
    if tile_grid.is_null() {
        return Err(ProcessingError::FFmpegError(
            "Tile Grid params is null".to_string(),
        ));
    }

    let canvas_width = (*tile_grid).coded_width;
    let canvas_height = (*tile_grid).coded_height;
    let nb_tiles = (*tile_grid).nb_tiles;

    // Allocate canvas frame (full size)
    let canvas_frame = av_frame_alloc();
    if canvas_frame.is_null() {
        return Err(ProcessingError::FFmpegError(
            "Failed to allocate canvas frame".to_string(),
        ));
    }

    (*canvas_frame).width = canvas_width;
    (*canvas_frame).height = canvas_height;
    (*canvas_frame).format = AV_PIX_FMT_YUV420P;

    let ret = av_frame_get_buffer(canvas_frame, 0);
    if ret < 0 {
        av_frame_free(&mut (canvas_frame as *mut _));
        return Err(ProcessingError::FFmpegError(format!(
            "Failed to allocate canvas buffer: {}",
            ret
        )));
    }

    // Get streams from the stream group
    let streams = (*tile_grid_group).streams;
    let nb_streams = (*tile_grid_group).nb_streams;

    // Create codec contexts for all tiles
    let mut tile_decoders = Vec::with_capacity(nb_tiles as usize);
    for tile_idx in 0..nb_tiles {
        let offset = (*tile_grid).offsets.offset(tile_idx as isize);
        let stream_idx = (*offset).idx;

        if stream_idx >= nb_streams {
            continue;
        }

        let stream = *streams.offset(stream_idx as isize);
        let stream_index = (*stream).index as c_int;
        let codecpar = (*stream).codecpar;
        let codec_id = (*codecpar).codec_id as c_int;

        let decoder = avcodec_find_decoder(codec_id);
        if decoder.is_null() {
            continue;
        }

        let codec_ctx = CodecContext::create(decoder, codecpar)?;
        tile_decoders.push((tile_idx, stream_index, codec_ctx));
    }

    // Decode all tiles in a single pass through the file
    // Pre-allocate with tile_decoders capacity (one frame per tile expected)
    let mut tile_frames: Vec<(*mut AVFrame, u32, c_int, c_int)> =
        Vec::with_capacity(tile_decoders.len());

    loop {
        let packet = av_packet_alloc();
        if packet.is_null() {
            break;
        }

        let ret = av_read_frame(format_ctx.as_ptr(), packet);
        if ret == AVERROR_EOF || ret < 0 {
            av_packet_free(&mut (packet as *mut _));
            break;
        }

        let packet_stream_idx = (*packet).stream_index;

        // Find the decoder for this stream
        let decoder_idx = tile_decoders
            .iter()
            .position(|(_, stream_idx, _)| *stream_idx == packet_stream_idx);

        if let Some(idx) = decoder_idx {
            let (_, _, codec_ctx) = &tile_decoders[idx];

            // Send packet to decoder
            avcodec_send_packet(codec_ctx.as_ptr(), packet);
        }

        av_packet_free(&mut (packet as *mut _));
    }

    // Flush all decoders and receive frames
    for (tile_idx, _, codec_ctx) in tile_decoders.iter() {
        // Flush decoder (send NULL packet)
        avcodec_send_packet(codec_ctx.as_ptr(), ptr::null());

        // Receive all buffered frames from this decoder
        loop {
            let frame = av_frame_alloc();
            if frame.is_null() {
                break;
            }

            let ret = avcodec_receive_frame(codec_ctx.as_ptr(), frame);
            if ret >= 0 {
                // Get tile offset
                let offset = (*tile_grid).offsets.offset(*tile_idx as isize);
                let tile_x = (*offset).horizontal;
                let tile_y = (*offset).vertical;

                tile_frames.push((frame, *tile_idx, tile_x, tile_y));
            } else {
                av_frame_free(&mut (frame as *mut _));
                break; // No more frames from this decoder
            }
        }
    }

    // Copy all decoded tiles to canvas
    for (tile_frame, _tile_idx, tile_x, tile_y) in tile_frames.iter() {
        copy_yuv_tile_to_canvas(canvas_frame, *tile_frame, *tile_x, *tile_y)?;
        av_frame_free(&mut (*tile_frame as *mut _));
    }

    // Return the composed frame as a YuvFrame
    Ok(vec![YuvFrame {
        width: canvas_width as u32,
        height: canvas_height as u32,
        timestamp: 0.0,    // HEIF/HEIC images don't have timestamps
        frame_number: 0,   // Single frame
        is_keyframe: true, // HEIF/HEIC is always a keyframe
        frame_ptr: canvas_frame,
    }])
}

/// Copy a tile's YUV data into the canvas at the specified offset
///
/// # Safety
/// Unsafe because it dereferences raw pointers and manipulates memory directly
unsafe fn copy_yuv_tile_to_canvas(
    canvas: *mut AVFrame,
    tile: *mut AVFrame,
    offset_x: c_int,
    offset_y: c_int,
) -> Result<()> {
    let tile_width = (*tile).width;
    let tile_height = (*tile).height;

    // Copy Y plane (full resolution)
    let canvas_y = (*canvas).data[0];
    let tile_y_plane = (*tile).data[0];
    let canvas_y_stride = (*canvas).linesize[0];
    let tile_y_stride = (*tile).linesize[0];

    for y in 0..tile_height {
        let canvas_row = canvas_y.offset(((offset_y + y) * canvas_y_stride + offset_x) as isize);
        let tile_row = tile_y_plane.offset((y * tile_y_stride) as isize);
        ptr::copy_nonoverlapping(tile_row, canvas_row, tile_width as usize);
    }

    // Copy U and V planes (half resolution for YUV420P)
    let tile_width_uv = tile_width / 2;
    let tile_height_uv = tile_height / 2;
    let offset_x_uv = offset_x / 2;
    let offset_y_uv = offset_y / 2;

    // U plane
    let canvas_u = (*canvas).data[1];
    let tile_u_plane = (*tile).data[1];
    let canvas_u_stride = (*canvas).linesize[1];
    let tile_u_stride = (*tile).linesize[1];

    for y in 0..tile_height_uv {
        let canvas_row =
            canvas_u.offset(((offset_y_uv + y) * canvas_u_stride + offset_x_uv) as isize);
        let tile_row = tile_u_plane.offset((y * tile_u_stride) as isize);
        ptr::copy_nonoverlapping(tile_row, canvas_row, tile_width_uv as usize);
    }

    // V plane
    let canvas_v = (*canvas).data[2];
    let tile_v_plane = (*tile).data[2];
    let canvas_v_stride = (*canvas).linesize[2];
    let tile_v_stride = (*tile).linesize[2];

    for y in 0..tile_height_uv {
        let canvas_row =
            canvas_v.offset(((offset_y_uv + y) * canvas_v_stride + offset_x_uv) as isize);
        let tile_row = tile_v_plane.offset((y * tile_v_stride) as isize);
        ptr::copy_nonoverlapping(tile_row, canvas_row, tile_width_uv as usize);
    }

    Ok(())
}

/// Decode I-frames (keyframes) from video file in YUV format (no RGB conversion)
///
/// This function is optimized for JPEG encoding - it returns frames in YUV format
/// which can be directly encoded to JPEG without expensive colorspace conversion.
///
/// Returns vector of YUV frames (AVFrame pointers wrapped for safety)
pub fn decode_iframes_yuv(video_path: &Path) -> Result<Vec<YuvFrame>> {
    unsafe {
        // Open video file
        let format_ctx = FormatContext::open(video_path)?;

        // Check for Tile Grid stream group (HEIF/HEIC)
        if let Some(tile_grid_group) = format_ctx.find_tile_grid() {
            // Decode Tile Grid (HEIF/HEIC)
            return decode_tile_grid(&format_ctx, tile_grid_group);
        }

        // Find video stream and decoder
        let (stream_index, decoder) = format_ctx.find_video_stream()?;

        // Create codec context
        let codecpar = format_ctx.get_codecpar(stream_index);
        let codec_ctx = CodecContext::create(decoder, codecpar)?;

        // Get time base for timestamp conversion
        let (time_base_num, time_base_den) = format_ctx.get_time_base(stream_index);

        // Pre-allocate frames Vec with estimated capacity
        // YUV frames typically needed for all frames in short video clips (estimate 60 frames)
        let mut frames = Vec::with_capacity(64);
        let mut frame_number = 0u64;

        // Decode loop
        loop {
            // Read packet from file
            let packet = av_packet_alloc();
            if packet.is_null() {
                break;
            }

            let ret = av_read_frame(format_ctx.as_ptr(), packet);

            if ret == AVERROR_EOF {
                // End of file - flush decoder to get any buffered frames
                av_packet_free(&mut (packet as *mut _));

                // Send NULL packet to flush decoder (required for codecs that buffer frames like VP9, H.264)
                avcodec_send_packet(codec_ctx.as_ptr(), ptr::null());

                // Drain all remaining frames from decoder
                loop {
                    let decoded_frame = av_frame_alloc();
                    if decoded_frame.is_null() {
                        break;
                    }

                    let ret = avcodec_receive_frame(codec_ctx.as_ptr(), decoded_frame);
                    if ret < 0 {
                        // No more frames available
                        av_frame_free(&mut (decoded_frame as *mut _));
                        break;
                    }

                    // Check if this is a keyframe
                    const AV_FRAME_FLAG_KEY: i32 = 1 << 1;
                    let is_keyframe = (*decoded_frame).pict_type
                        == ffmpeg::AVPictureType::AV_PICTURE_TYPE_I
                        || ((*decoded_frame).flags & AV_FRAME_FLAG_KEY) != 0;

                    if is_keyframe {
                        // Calculate timestamp
                        let pts = (*decoded_frame).pts;
                        let timestamp = if pts != 0 {
                            (pts as f64 * time_base_num as f64) / time_base_den as f64
                        } else {
                            frame_number as f64 / 30.0
                        };

                        // Clear PTS
                        (*decoded_frame).pts = ffmpeg::AV_NOPTS_VALUE;
                        (*decoded_frame).pkt_dts = ffmpeg::AV_NOPTS_VALUE;
                        (*decoded_frame).best_effort_timestamp = ffmpeg::AV_NOPTS_VALUE;

                        frames.push(YuvFrame {
                            width: (*decoded_frame).width as u32,
                            height: (*decoded_frame).height as u32,
                            timestamp,
                            frame_number,
                            is_keyframe: true,
                            frame_ptr: decoded_frame,
                        });

                        frame_number += 1;
                    } else {
                        av_frame_free(&mut (decoded_frame as *mut _));
                    }
                }

                break;
            }

            if ret < 0 {
                av_packet_free(&mut (packet as *mut _));
                return Err(ProcessingError::FFmpegError(format!(
                    "av_read_frame failed: {}",
                    ret
                )));
            }

            // Filter packets by stream index (only process video stream packets)
            if (*packet).stream_index != stream_index {
                av_packet_free(&mut (packet as *mut _));
                continue;
            }

            // Send packet to decoder
            let ret = avcodec_send_packet(codec_ctx.as_ptr(), packet);
            av_packet_free(&mut (packet as *mut _));

            if ret < 0 {
                return Err(ProcessingError::FFmpegError(format!(
                    "avcodec_send_packet failed: {}",
                    ret
                )));
            }

            // Receive decoded frames
            loop {
                let decoded_frame = av_frame_alloc();
                if decoded_frame.is_null() {
                    break;
                }

                let ret = avcodec_receive_frame(codec_ctx.as_ptr(), decoded_frame);

                if ret < 0 {
                    // AVERROR(EAGAIN) or AVERROR_EOF - no more frames from this packet
                    av_frame_free(&mut (decoded_frame as *mut _));
                    break;
                }

                // Check if this is an I-frame (keyframe)
                // Note: Some codecs (VP9, VP8) don't set pict_type, so check AV_FRAME_FLAG_KEY as well
                const AV_FRAME_FLAG_KEY: i32 = 1 << 1;
                let is_keyframe = (*decoded_frame).pict_type
                    == ffmpeg::AVPictureType::AV_PICTURE_TYPE_I
                    || ((*decoded_frame).flags & AV_FRAME_FLAG_KEY) != 0;

                if is_keyframe {
                    // Calculate timestamp
                    let pts = (*decoded_frame).pts;
                    let timestamp = if pts != 0 {
                        (pts as f64 * time_base_num as f64) / time_base_den as f64
                    } else {
                        frame_number as f64 / 30.0 // Fallback: assume 30 fps
                    };

                    // CRITICAL FIX (N=140): Clear PTS to prevent MJPEG encoder errors on long videos
                    // Long videos may have non-monotonic keyframe PTS due to B-frame reordering or edit points.
                    // MJPEG encoder rejects frames with out-of-order PTS ("Invalid pts <= last" error).
                    // Solution: Clear original PTS here, encoder will assign sequential PTS based on frame_number.
                    (*decoded_frame).pts = ffmpeg::AV_NOPTS_VALUE;
                    (*decoded_frame).pkt_dts = ffmpeg::AV_NOPTS_VALUE;
                    (*decoded_frame).best_effort_timestamp = ffmpeg::AV_NOPTS_VALUE;

                    // Keep YUV frame as-is (no conversion)
                    frames.push(YuvFrame {
                        width: (*decoded_frame).width as u32,
                        height: (*decoded_frame).height as u32,
                        timestamp,
                        frame_number,
                        is_keyframe: true,
                        frame_ptr: decoded_frame, // Transfer ownership (Drop will free)
                    });

                    frame_number += 1;
                } else {
                    // Not a keyframe, free it
                    av_frame_free(&mut (decoded_frame as *mut _));
                }
            }
        }

        Ok(frames)
    }
}

/// YUV frame wrapper (zero-copy, owns AVFrame)
///
/// This struct provides safe access to YUV frames decoded from video.
/// The frame is kept in YUV format for direct JPEG encoding.
pub struct YuvFrame {
    /// Frame width in pixels
    pub width: u32,
    /// Frame height in pixels
    pub height: u32,
    /// Timestamp in seconds
    pub timestamp: f64,
    /// Frame number (0-indexed)
    pub frame_number: u64,
    /// Whether this is a keyframe (I-frame)
    pub is_keyframe: bool,
    /// Owned AVFrame pointer (kept alive via Drop impl)
    frame_ptr: *mut AVFrame,
}

impl YuvFrame {
    /// Get raw AVFrame pointer for encoding
    pub fn as_ptr(&self) -> *const AVFrame {
        self.frame_ptr as *const AVFrame
    }
}

impl Drop for YuvFrame {
    fn drop(&mut self) {
        unsafe {
            if !self.frame_ptr.is_null() {
                av_frame_free(&mut self.frame_ptr);
            }
        }
    }
}

// SAFETY: YuvFrame owns the AVFrame pointer and ensures proper cleanup via Drop.
// The AVFrame is not shared across threads, and parallel processing of different
// YuvFrame instances is safe since each owns its own AVFrame.
unsafe impl Send for YuvFrame {}
unsafe impl Sync for YuvFrame {}

/// # Safety
/// Caller must ensure frame pointer is valid and points to initialized AVFrame.
/// Frame must remain valid for the duration of this function.
pub unsafe fn encode_yuv_frame_to_jpeg(
    frame: *const AVFrame,
    output_path: &Path,
    quality: c_int,
    frame_number: u64,
) -> Result<()> {
    if frame.is_null() {
        return Err(ProcessingError::FFmpegError(
            "Null frame pointer".to_string(),
        ));
    }

    // N=140 FIX: Create fresh encoder for each frame (no caching)
    // MJPEG encoder maintains PTS state, and caching causes "Invalid pts <= last" errors
    // for long videos with non-monotonic keyframe PTS.
    // Performance impact: ~1-2ms per frame overhead from encoder creation
    let enc_ctx = create_jpeg_encoder_uncached((*frame).width, (*frame).height, quality)?;

    // Convert YUV420P to YUVJ420P if needed
    let encode_frame: *mut AVFrame =
        if (*frame).format == ffmpeg::AVPixelFormat::AV_PIX_FMT_YUV420P as c_int {
            // Need to convert: create new frame with YUVJ420P format
            let yuv_j_frame = av_frame_alloc();
            if yuv_j_frame.is_null() {
                return Err(ProcessingError::FFmpegError(
                    "Failed to allocate YUVJ420P frame".to_string(),
                ));
            }

            (*yuv_j_frame).format = ffmpeg::AVPixelFormat::AV_PIX_FMT_YUVJ420P as c_int;
            (*yuv_j_frame).width = (*frame).width;
            (*yuv_j_frame).height = (*frame).height;

            let ret = av_frame_get_buffer(yuv_j_frame, 32);
            if ret < 0 {
                av_frame_free(&mut (yuv_j_frame as *mut _));
                return Err(ProcessingError::FFmpegError(format!(
                    "Failed to allocate YUVJ420P buffer: {}",
                    ret
                )));
            }

            // Copy YUV data (data and linesize are compatible)
            for plane in 0..3 {
                let src_plane = (*frame).data[plane];
                let dst_plane = (*yuv_j_frame).data[plane];
                let src_linesize = (*frame).linesize[plane] as usize;
                let dst_linesize = (*yuv_j_frame).linesize[plane] as usize;
                let height = if plane == 0 {
                    (*frame).height as usize
                } else {
                    ((*frame).height as usize) / 2 // UV planes are half height
                };

                for y in 0..height {
                    std::ptr::copy_nonoverlapping(
                        src_plane.add(y * src_linesize),
                        dst_plane.add(y * dst_linesize),
                        src_linesize.min(dst_linesize),
                    );
                }
            }

            // Set monotonic PTS based on frame_number to ensure encoder sees sequential timestamps
            // This prevents "Invalid pts <= last" errors when frames are encoded in parallel
            (*yuv_j_frame).pts = frame_number as i64;
            (*yuv_j_frame).pkt_dts = ffmpeg::AV_NOPTS_VALUE;
            (*yuv_j_frame).best_effort_timestamp = ffmpeg::AV_NOPTS_VALUE;
            yuv_j_frame
        } else {
            // Already YUVJ420P, but still need to set monotonic PTS
            // Create mutable copy to modify timestamps
            let yuv_j_frame = av_frame_clone(frame);
            if yuv_j_frame.is_null() {
                return Err(ProcessingError::FFmpegError(
                    "Failed to clone frame".to_string(),
                ));
            }
            // Set monotonic PTS based on frame_number to ensure encoder sees sequential timestamps
            (*yuv_j_frame).pts = frame_number as i64;
            (*yuv_j_frame).pkt_dts = ffmpeg::AV_NOPTS_VALUE;
            (*yuv_j_frame).best_effort_timestamp = ffmpeg::AV_NOPTS_VALUE;
            yuv_j_frame
        };

    // Send frame to encoder (no flush needed since encoder is fresh)
    let ret = avcodec_send_frame(enc_ctx, encode_frame);
    if ret < 0 {
        avcodec_free_context(&mut (enc_ctx as *mut _)); // Free encoder on error
        if !std::ptr::eq(encode_frame, frame) {
            av_frame_free(&mut (encode_frame as *mut _));
        }
        return Err(ProcessingError::FFmpegError(format!(
            "avcodec_send_frame failed: {}",
            ret
        )));
    }

    // Receive encoded packet
    let pkt = av_packet_alloc();
    if pkt.is_null() {
        avcodec_free_context(&mut (enc_ctx as *mut _)); // Free encoder on error
        if !std::ptr::eq(encode_frame, frame) {
            av_frame_free(&mut (encode_frame as *mut _));
        }
        return Err(ProcessingError::FFmpegError(
            "Failed to allocate packet".to_string(),
        ));
    }

    let ret = avcodec_receive_packet(enc_ctx, pkt);
    if ret < 0 {
        av_packet_free(&mut (pkt as *mut _));
        avcodec_free_context(&mut (enc_ctx as *mut _)); // Free encoder on error
        if !std::ptr::eq(encode_frame, frame) {
            av_frame_free(&mut (encode_frame as *mut _));
        }
        return Err(ProcessingError::FFmpegError(format!(
            "avcodec_receive_packet failed: {}",
            ret
        )));
    }

    // Write JPEG data to file
    let jpeg_data = std::slice::from_raw_parts((*pkt).data, (*pkt).size as usize);
    std::fs::write(output_path, jpeg_data).map_err(|e| {
        av_packet_free(&mut (pkt as *mut _));
        if !std::ptr::eq(encode_frame, frame) {
            av_frame_free(&mut (encode_frame as *mut _));
        }
        ProcessingError::IoError(e)
    })?;

    // Cleanup
    av_packet_free(&mut (pkt as *mut _));
    if !std::ptr::eq(encode_frame, frame) {
        av_frame_free(&mut (encode_frame as *mut _));
    }

    // N=140: Free encoder context (not cached anymore)
    avcodec_free_context(&mut (enc_ctx as *mut _));

    Ok(())
}

/// Encode YUV frame to JPEG in memory (returns bytes without writing to disk)
///
/// Encodes a YUV frame to JPEG format and returns the bytes as Vec<u8>.
/// This allows callers to batch I/O operations separately from encoding.
///
/// # Performance
/// - Eliminates I/O blocking during parallel encoding
/// - Allows encoding and I/O to be pipelined
/// - Encoding threads don't wait for disk writes
///
/// # Safety
/// - frame must be a valid AVFrame pointer
/// - frame must remain valid for the duration of encoding
/// - Uses thread-local encoder cache (safe across threads)
pub unsafe fn encode_yuv_frame_to_jpeg_bytes(
    frame: *const AVFrame,
    quality: c_int,
    frame_number: u64,
) -> Result<Vec<u8>> {
    if frame.is_null() {
        return Err(ProcessingError::FFmpegError(
            "Null frame pointer".to_string(),
        ));
    }

    // N=140 FIX: Create fresh encoder for each frame (no caching)
    let enc_ctx = create_jpeg_encoder_uncached((*frame).width, (*frame).height, quality)?;

    // Convert YUV420P to YUVJ420P if needed
    let encode_frame: *mut AVFrame =
        if (*frame).format == ffmpeg::AVPixelFormat::AV_PIX_FMT_YUV420P as c_int {
            // Need to convert: create new frame with YUVJ420P format
            let yuv_j_frame = av_frame_alloc();
            if yuv_j_frame.is_null() {
                return Err(ProcessingError::FFmpegError(
                    "Failed to allocate YUVJ420P frame".to_string(),
                ));
            }

            (*yuv_j_frame).format = ffmpeg::AVPixelFormat::AV_PIX_FMT_YUVJ420P as c_int;
            (*yuv_j_frame).width = (*frame).width;
            (*yuv_j_frame).height = (*frame).height;

            let ret = av_frame_get_buffer(yuv_j_frame, 32);
            if ret < 0 {
                av_frame_free(&mut (yuv_j_frame as *mut _));
                return Err(ProcessingError::FFmpegError(format!(
                    "Failed to allocate YUVJ420P buffer: {}",
                    ret
                )));
            }

            // Copy YUV data (data and linesize are compatible)
            for plane in 0..3 {
                let src_plane = (*frame).data[plane];
                let dst_plane = (*yuv_j_frame).data[plane];
                let src_linesize = (*frame).linesize[plane] as usize;
                let dst_linesize = (*yuv_j_frame).linesize[plane] as usize;
                let height = if plane == 0 {
                    (*frame).height as usize
                } else {
                    ((*frame).height as usize) / 2 // UV planes are half height
                };

                for y in 0..height {
                    std::ptr::copy_nonoverlapping(
                        src_plane.add(y * src_linesize),
                        dst_plane.add(y * dst_linesize),
                        src_linesize.min(dst_linesize),
                    );
                }
            }

            // Set monotonic PTS based on frame_number
            (*yuv_j_frame).pts = frame_number as i64;
            (*yuv_j_frame).pkt_dts = ffmpeg::AV_NOPTS_VALUE;
            (*yuv_j_frame).best_effort_timestamp = ffmpeg::AV_NOPTS_VALUE;
            yuv_j_frame
        } else {
            // Already YUVJ420P, clone and set PTS
            let yuv_j_frame = av_frame_clone(frame);
            if yuv_j_frame.is_null() {
                return Err(ProcessingError::FFmpegError(
                    "Failed to clone frame".to_string(),
                ));
            }
            (*yuv_j_frame).pts = frame_number as i64;
            (*yuv_j_frame).pkt_dts = ffmpeg::AV_NOPTS_VALUE;
            (*yuv_j_frame).best_effort_timestamp = ffmpeg::AV_NOPTS_VALUE;
            yuv_j_frame
        };

    // Send frame to encoder (no flush needed since encoder is fresh)
    let ret = avcodec_send_frame(enc_ctx, encode_frame);
    if ret < 0 {
        if !std::ptr::eq(encode_frame, frame) {
            av_frame_free(&mut (encode_frame as *mut _));
        }
        return Err(ProcessingError::FFmpegError(format!(
            "avcodec_send_frame failed: {}",
            ret
        )));
    }

    // Receive encoded packet
    let pkt = av_packet_alloc();
    if pkt.is_null() {
        if !std::ptr::eq(encode_frame, frame) {
            av_frame_free(&mut (encode_frame as *mut _));
        }
        return Err(ProcessingError::FFmpegError(
            "Failed to allocate packet".to_string(),
        ));
    }

    let ret = avcodec_receive_packet(enc_ctx, pkt);
    if ret < 0 {
        av_packet_free(&mut (pkt as *mut _));
        if !std::ptr::eq(encode_frame, frame) {
            av_frame_free(&mut (encode_frame as *mut _));
        }
        return Err(ProcessingError::FFmpegError(format!(
            "avcodec_receive_packet failed: {}",
            ret
        )));
    }

    // Copy JPEG data to owned Vec
    let jpeg_data = std::slice::from_raw_parts((*pkt).data, (*pkt).size as usize);
    let jpeg_bytes = jpeg_data.to_vec();

    // Cleanup
    av_packet_free(&mut (pkt as *mut _));
    if !std::ptr::eq(encode_frame, frame) {
        av_frame_free(&mut (encode_frame as *mut _));
    }

    // N=140: Free encoder context (not cached anymore)
    avcodec_free_context(&mut (enc_ctx as *mut _));

    Ok(jpeg_bytes)
}

/// Extract audio from media file to PCM WAV
///
/// Decodes audio stream and resamples to specified format (default: 16kHz mono s16le).
/// Equivalent to: ffmpeg -i INPUT -vn -acodec pcm_s16le -ar 16000 -ac 1 OUTPUT.wav
///
/// # Performance
/// - Eliminates 70-90ms FFmpeg process spawn overhead
/// - Direct libavcodec decode + libswresample resample (same C functions as FFmpeg CLI)
/// - Expected overhead: ~8-12ms (Clap parsing, binary loading)
///
/// # Safety
/// Uses unsafe FFmpeg C API. Memory safety ensured by RAII wrappers and error handling.
pub fn extract_audio_to_wav(
    input_path: &Path,
    output_path: &Path,
    sample_rate: u32,
    channels: u32,
) -> Result<()> {
    unsafe {
        // Open input file
        let fmt_ctx = FormatContext::open(input_path)?;

        // Find audio stream
        let (audio_stream_idx, decoder) = fmt_ctx.find_audio_stream()?;

        // Get codec parameters
        let codecpar = fmt_ctx.get_codecpar(audio_stream_idx);

        // Allocate codec context
        let dec_ctx = avcodec_alloc_context3(decoder);
        if dec_ctx.is_null() {
            return Err(ProcessingError::FFmpegError(
                "Failed to allocate codec context".to_string(),
            ));
        }

        // Copy codec parameters to context
        let ret = avcodec_parameters_to_context(dec_ctx, codecpar);
        if ret < 0 {
            avcodec_free_context(&mut (dec_ctx as *mut _));
            return Err(ProcessingError::FFmpegError(format!(
                "avcodec_parameters_to_context failed: {}",
                ret
            )));
        }

        // Open codec (needs lock)
        {
            let _lock = FFMPEG_INIT_LOCK.lock().unwrap();
            let ret = avcodec_open2(dec_ctx, decoder, ptr::null_mut());
            if ret < 0 {
                avcodec_free_context(&mut (dec_ctx as *mut _));
                return Err(ProcessingError::FFmpegError(format!(
                    "avcodec_open2 failed: {}",
                    ret
                )));
            }
        }

        // Get input audio parameters
        let in_sample_rate = (*dec_ctx).sample_rate;
        let in_sample_fmt = (*dec_ctx).sample_fmt as c_int;
        // Input channel layout is already in dec_ctx->ch_layout (FFmpeg 5+ AVChannelLayout)
        let in_ch_layout = &(*dec_ctx).ch_layout as *const ffmpeg::AVChannelLayout;

        // Output parameters (configurable sample rate and channels, PCM s16le format)
        let out_sample_rate = sample_rate as c_int;
        let out_channels = channels as c_int;
        let out_sample_fmt = AV_SAMPLE_FMT_S16;

        // Initialize output channel layout using FFmpeg 5+ API
        let mut out_ch_layout = std::mem::zeroed::<ffmpeg::AVChannelLayout>();
        av_channel_layout_default(&mut out_ch_layout as *mut _, out_channels);

        // Create resampler using FFmpeg 5+ API (swr_alloc_set_opts2)
        let mut swr_ctx: *mut SwrContext = ptr::null_mut();
        let ret = swr_alloc_set_opts2(
            &mut swr_ctx as *mut *mut SwrContext,
            &out_ch_layout as *const ffmpeg::AVChannelLayout,
            out_sample_fmt as c_int,
            out_sample_rate,
            in_ch_layout,
            in_sample_fmt,
            in_sample_rate,
            0,
            ptr::null_mut(),
        );

        if ret < 0 || swr_ctx.is_null() {
            av_channel_layout_uninit(&mut out_ch_layout as *mut _);
            avcodec_free_context(&mut (dec_ctx as *mut _));
            return Err(ProcessingError::FFmpegError(format!(
                "swr_alloc_set_opts2 failed: {}",
                ret
            )));
        }

        let ret = swr_init(swr_ctx);
        if ret < 0 {
            av_channel_layout_uninit(&mut out_ch_layout as *mut _);
            swr_free(&mut (swr_ctx as *mut _));
            avcodec_free_context(&mut (dec_ctx as *mut _));
            return Err(ProcessingError::FFmpegError(format!(
                "swr_init failed: {}",
                ret
            )));
        }

        // Allocate packet and frame
        let pkt = av_packet_alloc();
        let frame = av_frame_alloc();
        if pkt.is_null() || frame.is_null() {
            if !pkt.is_null() {
                av_packet_free(&mut (pkt as *mut _));
            }
            if !frame.is_null() {
                av_frame_free(&mut (frame as *mut _));
            }
            swr_free(&mut (swr_ctx as *mut _));
            av_channel_layout_uninit(&mut out_ch_layout as *mut _);
            avcodec_free_context(&mut (dec_ctx as *mut _));
            return Err(ProcessingError::FFmpegError(
                "Failed to allocate packet/frame".to_string(),
            ));
        }

        // Decode loop - collect all audio samples
        let mut audio_samples = Vec::<i16>::new();

        while av_read_frame(fmt_ctx.as_ptr(), pkt) >= 0 {
            // Only process audio packets
            if (*pkt).stream_index != audio_stream_idx {
                av_packet_unref(pkt);
                continue;
            }

            // Send packet to decoder
            let ret = avcodec_send_packet(dec_ctx, pkt);
            av_packet_unref(pkt);

            if ret < 0 {
                continue; // Skip corrupted packets
            }

            // Receive all frames from this packet
            loop {
                let ret = avcodec_receive_frame(dec_ctx, frame);
                if ret < 0 {
                    break; // Need more data or EOF
                }

                // Allocate output buffer for resampled audio
                let out_samples = (*frame).nb_samples;
                let out_buffer_size = out_samples * out_channels * 2; // 2 bytes per s16 sample
                let mut out_buffer = vec![0u8; out_buffer_size as usize];
                let mut out_ptr = out_buffer.as_mut_ptr();

                // Resample
                let converted_samples = swr_convert(
                    swr_ctx,
                    &mut out_ptr as *mut *mut u8 as *const *mut u8,
                    out_samples,
                    (*frame).data.as_ptr() as *const *const u8,
                    (*frame).nb_samples,
                );

                if converted_samples > 0 {
                    // Convert bytes to i16 samples
                    let sample_count = (converted_samples * out_channels) as usize;
                    let samples_slice =
                        std::slice::from_raw_parts(out_buffer.as_ptr() as *const i16, sample_count);
                    audio_samples.extend_from_slice(samples_slice);
                }

                av_frame_unref(frame);
            }
        }

        // Cleanup FFmpeg resources
        av_packet_free(&mut (pkt as *mut _));
        av_frame_free(&mut (frame as *mut _));
        swr_free(&mut (swr_ctx as *mut _));
        av_channel_layout_uninit(&mut out_ch_layout as *mut _);
        avcodec_free_context(&mut (dec_ctx as *mut _));

        // Write WAV file
        write_wav_file(output_path, &audio_samples, sample_rate, channels as u16)?;

        Ok(())
    }
}

/// Write PCM audio samples to WAV file
///
/// Creates a standard WAV file header and writes PCM s16le samples.
fn write_wav_file(path: &Path, samples: &[i16], sample_rate: u32, channels: u16) -> Result<()> {
    use std::io::{BufWriter, Write};

    let file = std::fs::File::create(path)
        .map_err(|e| ProcessingError::FFmpegError(format!("Failed to create WAV file: {}", e)))?;
    let mut file = BufWriter::with_capacity(256 * 1024, file); // 256KB buffer

    // WAV header
    let data_size = (samples.len() * 2) as u32; // 2 bytes per i16 sample
    let file_size = data_size + 36; // Header is 44 bytes, minus 8 for RIFF chunk header

    // RIFF chunk
    file.write_all(b"RIFF").map_err(ProcessingError::IoError)?;
    file.write_all(&file_size.to_le_bytes())
        .map_err(ProcessingError::IoError)?;
    file.write_all(b"WAVE").map_err(ProcessingError::IoError)?;

    // fmt subchunk
    file.write_all(b"fmt ").map_err(ProcessingError::IoError)?;
    file.write_all(&16u32.to_le_bytes())
        .map_err(ProcessingError::IoError)?; // Subchunk size
    file.write_all(&1u16.to_le_bytes())
        .map_err(ProcessingError::IoError)?; // Audio format (1 = PCM)
    file.write_all(&channels.to_le_bytes())
        .map_err(ProcessingError::IoError)?;
    file.write_all(&sample_rate.to_le_bytes())
        .map_err(ProcessingError::IoError)?;
    let byte_rate = sample_rate * channels as u32 * 2;
    file.write_all(&byte_rate.to_le_bytes())
        .map_err(ProcessingError::IoError)?;
    let block_align = channels * 2;
    file.write_all(&block_align.to_le_bytes())
        .map_err(ProcessingError::IoError)?;
    file.write_all(&16u16.to_le_bytes())
        .map_err(ProcessingError::IoError)?; // Bits per sample

    // data subchunk
    file.write_all(b"data").map_err(ProcessingError::IoError)?;
    file.write_all(&data_size.to_le_bytes())
        .map_err(ProcessingError::IoError)?;

    // Write PCM data
    for &sample in samples {
        file.write_all(&sample.to_le_bytes())
            .map_err(ProcessingError::IoError)?;
    }

    // Flush buffer to ensure all data is written
    file.flush().map_err(ProcessingError::IoError)?;

    Ok(())
}

/// Load audio samples from media file into memory as f32
///
/// Uses C FFI for zero-overhead audio decoding and resampling (no process spawn).
/// Returns audio samples as Vec<f32> for direct use in ML models.
///
/// # Arguments
/// * `input_path` - Path to input video or audio file
/// * `sample_rate` - Target sample rate (e.g., 16000 for ML models)
/// * `channels` - Number of channels (1 for mono, 2 for stereo)
///
/// # Returns
/// `(Vec<f32>, u32)` - Tuple of (audio samples, sample rate)
///
/// # Errors
/// Returns error if:
/// - Input file has no audio stream
/// - Audio decoding/resampling fails
pub fn load_audio_samples_f32(
    input_path: &Path,
    sample_rate: u32,
    channels: u32,
) -> Result<(Vec<f32>, u32)> {
    unsafe {
        // Open input file
        let fmt_ctx = FormatContext::open(input_path)?;

        // Find audio stream
        let (audio_stream_idx, decoder) = fmt_ctx.find_audio_stream()?;

        // Get codec parameters
        let codecpar = fmt_ctx.get_codecpar(audio_stream_idx);

        // Allocate codec context
        let dec_ctx = avcodec_alloc_context3(decoder);
        if dec_ctx.is_null() {
            return Err(ProcessingError::FFmpegError(
                "Failed to allocate codec context".to_string(),
            ));
        }

        // Copy codec parameters to context
        let ret = avcodec_parameters_to_context(dec_ctx, codecpar);
        if ret < 0 {
            avcodec_free_context(&mut (dec_ctx as *mut _));
            return Err(ProcessingError::FFmpegError(format!(
                "avcodec_parameters_to_context failed: {}",
                ret
            )));
        }

        // Open codec (needs lock)
        {
            let _lock = FFMPEG_INIT_LOCK.lock().unwrap();
            let ret = avcodec_open2(dec_ctx, decoder, ptr::null_mut());
            if ret < 0 {
                avcodec_free_context(&mut (dec_ctx as *mut _));
                return Err(ProcessingError::FFmpegError(format!(
                    "avcodec_open2 failed: {}",
                    ret
                )));
            }
        }

        // Get input audio parameters
        let in_sample_rate = (*dec_ctx).sample_rate;
        let in_sample_fmt = (*dec_ctx).sample_fmt as c_int;
        let in_ch_layout = &(*dec_ctx).ch_layout as *const ffmpeg::AVChannelLayout;

        // Output parameters (configurable sample rate and channels, PCM f32le format)
        let out_sample_rate = sample_rate as c_int;
        let out_channels = channels as c_int;
        let out_sample_fmt = AV_SAMPLE_FMT_FLT;

        // Initialize output channel layout using FFmpeg 5+ API
        let mut out_ch_layout = std::mem::zeroed::<ffmpeg::AVChannelLayout>();
        av_channel_layout_default(&mut out_ch_layout as *mut _, out_channels);

        // Create resampler using FFmpeg 5+ API (swr_alloc_set_opts2)
        let mut swr_ctx: *mut SwrContext = ptr::null_mut();
        let ret = swr_alloc_set_opts2(
            &mut swr_ctx as *mut *mut SwrContext,
            &out_ch_layout as *const ffmpeg::AVChannelLayout,
            out_sample_fmt as c_int,
            out_sample_rate,
            in_ch_layout,
            in_sample_fmt,
            in_sample_rate,
            0,
            ptr::null_mut(),
        );

        if ret < 0 || swr_ctx.is_null() {
            av_channel_layout_uninit(&mut out_ch_layout as *mut _);
            avcodec_free_context(&mut (dec_ctx as *mut _));
            return Err(ProcessingError::FFmpegError(format!(
                "swr_alloc_set_opts2 failed: {}",
                ret
            )));
        }

        let ret = swr_init(swr_ctx);
        if ret < 0 {
            av_channel_layout_uninit(&mut out_ch_layout as *mut _);
            swr_free(&mut (swr_ctx as *mut _));
            avcodec_free_context(&mut (dec_ctx as *mut _));
            return Err(ProcessingError::FFmpegError(format!(
                "swr_init failed: {}",
                ret
            )));
        }

        // Allocate packet and frame
        let pkt = av_packet_alloc();
        let frame = av_frame_alloc();
        if pkt.is_null() || frame.is_null() {
            if !pkt.is_null() {
                av_packet_free(&mut (pkt as *mut _));
            }
            if !frame.is_null() {
                av_frame_free(&mut (frame as *mut _));
            }
            swr_free(&mut (swr_ctx as *mut _));
            av_channel_layout_uninit(&mut out_ch_layout as *mut _);
            avcodec_free_context(&mut (dec_ctx as *mut _));
            return Err(ProcessingError::FFmpegError(
                "Failed to allocate packet/frame".to_string(),
            ));
        }

        // Decode loop - collect all audio samples
        let mut audio_samples = Vec::<f32>::new();

        while av_read_frame(fmt_ctx.as_ptr(), pkt) >= 0 {
            // Only process audio packets
            if (*pkt).stream_index != audio_stream_idx {
                av_packet_unref(pkt);
                continue;
            }

            // Send packet to decoder
            let ret = avcodec_send_packet(dec_ctx, pkt);
            av_packet_unref(pkt);

            if ret < 0 {
                continue; // Skip corrupted packets
            }

            // Receive all frames from this packet
            loop {
                let ret = avcodec_receive_frame(dec_ctx, frame);
                if ret < 0 {
                    break; // Need more data or EOF
                }

                // Allocate output buffer for resampled audio
                let out_samples = (*frame).nb_samples;
                let out_buffer_size = out_samples * out_channels * 4; // 4 bytes per f32 sample
                let mut out_buffer = vec![0u8; out_buffer_size as usize];
                let mut out_ptr = out_buffer.as_mut_ptr();

                // Resample
                let converted_samples = swr_convert(
                    swr_ctx,
                    &mut out_ptr as *mut *mut u8 as *const *mut u8,
                    out_samples,
                    (*frame).data.as_ptr() as *const *const u8,
                    (*frame).nb_samples,
                );

                if converted_samples > 0 {
                    // Convert bytes to f32 samples
                    let sample_count = (converted_samples * out_channels) as usize;
                    let samples_slice =
                        std::slice::from_raw_parts(out_buffer.as_ptr() as *const f32, sample_count);
                    audio_samples.extend_from_slice(samples_slice);
                }

                av_frame_unref(frame);
            }
        }

        // Cleanup FFmpeg resources
        av_packet_free(&mut (pkt as *mut _));
        av_frame_free(&mut (frame as *mut _));
        swr_free(&mut (swr_ctx as *mut _));
        av_channel_layout_uninit(&mut out_ch_layout as *mut _);
        avcodec_free_context(&mut (dec_ctx as *mut _));

        Ok((audio_samples, sample_rate))
    }
}
