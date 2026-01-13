//! Benchmark for thumbnail generation pipeline
//!
//! Measures time breakdown:
//! - Decode (zero-copy C FFI)
//! - Resize (Lanczos3)
//! - JPEG encode (mozjpeg)
//! - Disk I/O
//!
//! Run with:
//! ```bash
//! cargo bench --bench thumbnail_pipeline_bench
//! ```

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use image::{imageops, RgbImage};
use std::path::PathBuf;
use video_audio_decoder::decode_iframes_zero_copy;
use video_extract_core::image_io::save_image;

fn benchmark_decode(c: &mut Criterion) {
    let test_video = PathBuf::from("test_media_generated/test_120fps_10s.mp4");

    c.bench_function("decode_iframes_zero_copy", |b| {
        b.iter(|| {
            let frames =
                decode_iframes_zero_copy(black_box(&test_video)).expect("Failed to decode frames");
            black_box(frames);
        })
    });
}

fn benchmark_resize(c: &mut Criterion) {
    // Create a sample 1920x1080 RGB image
    let img = RgbImage::from_vec(1920, 1080, vec![128u8; 1920 * 1080 * 3])
        .expect("Failed to create test image");

    c.bench_function("resize_1920x1080_to_640x480_lanczos3", |b| {
        b.iter(|| {
            let resized =
                imageops::resize(black_box(&img), 640, 480, imageops::FilterType::Lanczos3);
            black_box(resized);
        })
    });

    c.bench_function("resize_1920x1080_to_640x480_triangle", |b| {
        b.iter(|| {
            let resized =
                imageops::resize(black_box(&img), 640, 480, imageops::FilterType::Triangle);
            black_box(resized);
        })
    });

    c.bench_function("resize_1920x1080_to_640x480_nearest", |b| {
        b.iter(|| {
            let resized =
                imageops::resize(black_box(&img), 640, 480, imageops::FilterType::Nearest);
            black_box(resized);
        })
    });
}

fn benchmark_jpeg_encode(c: &mut Criterion) {
    // Create a sample 640x480 RGB image
    let img = RgbImage::from_vec(640, 480, vec![128u8; 640 * 480 * 3])
        .expect("Failed to create test image");

    let output_path = PathBuf::from("/tmp/bench_thumbnail.jpg");

    c.bench_function("jpeg_encode_mozjpeg_quality85", |b| {
        b.iter(|| {
            save_image(black_box(&img), black_box(&output_path), 85).expect("Failed to save JPEG");
        })
    });

    c.bench_function("jpeg_encode_mozjpeg_quality75", |b| {
        b.iter(|| {
            save_image(black_box(&img), black_box(&output_path), 75).expect("Failed to save JPEG");
        })
    });

    c.bench_function("jpeg_encode_mozjpeg_quality60", |b| {
        b.iter(|| {
            save_image(black_box(&img), black_box(&output_path), 60).expect("Failed to save JPEG");
        })
    });

    // Clean up
    let _ = std::fs::remove_file(&output_path);
}

fn benchmark_full_pipeline(c: &mut Criterion) {
    let test_video = PathBuf::from("test_media_generated/test_120fps_10s.mp4");
    let output_path = PathBuf::from("/tmp/bench_thumbnail_full.jpg");

    c.bench_function("full_pipeline_single_frame", |b| {
        b.iter(|| {
            // Decode first I-frame
            let frames =
                decode_iframes_zero_copy(black_box(&test_video)).expect("Failed to decode frames");
            let first_frame = &frames[0];

            // Convert to RgbImage
            let data_slice = unsafe {
                std::slice::from_raw_parts(
                    first_frame.data_ptr,
                    (first_frame.width * first_frame.height * 3) as usize,
                )
            };
            let img =
                RgbImage::from_vec(first_frame.width, first_frame.height, data_slice.to_vec())
                    .expect("Failed to create image");

            // Resize
            let resized = imageops::resize(&img, 640, 480, imageops::FilterType::Lanczos3);

            // JPEG encode and save
            save_image(&resized, black_box(&output_path), 85).expect("Failed to save JPEG");
        })
    });

    // Clean up
    let _ = std::fs::remove_file(&output_path);
}

criterion_group!(
    benches,
    benchmark_decode,
    benchmark_resize,
    benchmark_jpeg_encode,
    benchmark_full_pipeline
);
criterion_main!(benches);
