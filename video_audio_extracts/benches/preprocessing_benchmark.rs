// Preprocessing benchmark - measure resize + normalize time for vision models
//
// Run with: cargo bench --bench preprocessing_benchmark

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use image::{ImageBuffer, Rgb, RgbImage};
use ndarray::Array;

/// Simulate current preprocessing: resize + normalize
fn preprocess_current(image: &RgbImage, input_size: u32) -> Array<f32, ndarray::Dim<[usize; 4]>> {
    // Resize image to input size (letterbox)
    let resized = image::imageops::resize(
        image,
        input_size,
        input_size,
        image::imageops::FilterType::Triangle,
    );

    // Convert to CHW format and normalize to [0, 1]
    let mut input_array = Array::zeros((1, 3, input_size as usize, input_size as usize));

    for y in 0..input_size as usize {
        for x in 0..input_size as usize {
            let pixel = resized.get_pixel(x as u32, y as u32);
            input_array[[0, 0, y, x]] = f32::from(pixel[0]) / 255.0;
            input_array[[0, 1, y, x]] = f32::from(pixel[1]) / 255.0;
            input_array[[0, 2, y, x]] = f32::from(pixel[2]) / 255.0;
        }
    }

    input_array
}

/// Benchmark preprocessing at different resolutions
fn bench_preprocessing(c: &mut Criterion) {
    let mut group = c.benchmark_group("preprocessing");

    // Test resolutions: 224x224 (small), 640x640 (YOLO), 1280x1280 (large)
    let resolutions = vec![(224, "224x224"), (640, "640x640"), (1280, "1280x1280")];

    for (resolution, name) in resolutions {
        // Create test image (random pattern)
        let test_image: RgbImage = ImageBuffer::from_fn(resolution, resolution, |x, y| {
            Rgb([
                ((x + y) % 256) as u8,
                ((x * 2) % 256) as u8,
                ((y * 2) % 256) as u8,
            ])
        });

        group.bench_with_input(BenchmarkId::new("current", name), &test_image, |b, img| {
            b.iter(|| {
                let result = preprocess_current(black_box(img), resolution);
                black_box(result);
            });
        });
    }

    group.finish();
}

/// Benchmark individual stages: resize vs normalize
fn bench_stages(c: &mut Criterion) {
    let mut group = c.benchmark_group("preprocessing_stages");

    let input_size = 640u32;

    // Create test image
    let test_image: RgbImage = ImageBuffer::from_fn(input_size, input_size, |x, y| {
        Rgb([
            ((x + y) % 256) as u8,
            ((x * 2) % 256) as u8,
            ((y * 2) % 256) as u8,
        ])
    });

    // Benchmark resize only
    group.bench_function("resize_640x640", |b| {
        b.iter(|| {
            let resized = image::imageops::resize(
                black_box(&test_image),
                input_size,
                input_size,
                image::imageops::FilterType::Triangle,
            );
            black_box(resized);
        });
    });

    // Benchmark normalize only (after resize)
    let resized = image::imageops::resize(
        &test_image,
        input_size,
        input_size,
        image::imageops::FilterType::Triangle,
    );

    group.bench_function("normalize_640x640", |b| {
        b.iter(|| {
            let mut input_array = Array::zeros((1, 3, input_size as usize, input_size as usize));

            for y in 0..input_size as usize {
                for x in 0..input_size as usize {
                    let pixel = resized.get_pixel(x as u32, y as u32);
                    input_array[[0, 0, y, x]] = f32::from(pixel[0]) / 255.0;
                    input_array[[0, 1, y, x]] = f32::from(pixel[1]) / 255.0;
                    input_array[[0, 2, y, x]] = f32::from(pixel[2]) / 255.0;
                }
            }

            black_box(input_array);
        });
    });

    group.finish();
}

/// Benchmark different resize filter types
fn bench_resize_filters(c: &mut Criterion) {
    let mut group = c.benchmark_group("resize_filters");

    let target_size = 640u32;
    let source_size = 1920u32; // Typical 1080p video width

    // Create test image (1920x1920) to simulate actual downscaling
    let test_image: RgbImage = ImageBuffer::from_fn(source_size, source_size, |x, y| {
        Rgb([
            ((x + y) % 256) as u8,
            ((x * 2) % 256) as u8,
            ((y * 2) % 256) as u8,
        ])
    });

    // Benchmark Nearest (fastest, lowest quality)
    group.bench_function("nearest_1920to640", |b| {
        b.iter(|| {
            let resized = image::imageops::resize(
                black_box(&test_image),
                target_size,
                target_size,
                image::imageops::FilterType::Nearest,
            );
            black_box(resized);
        });
    });

    // Benchmark Triangle (current - bilinear, medium speed)
    group.bench_function("triangle_1920to640", |b| {
        b.iter(|| {
            let resized = image::imageops::resize(
                black_box(&test_image),
                target_size,
                target_size,
                image::imageops::FilterType::Triangle,
            );
            black_box(resized);
        });
    });

    // Benchmark CatmullRom (slower, better quality)
    group.bench_function("catmull_rom_1920to640", |b| {
        b.iter(|| {
            let resized = image::imageops::resize(
                black_box(&test_image),
                target_size,
                target_size,
                image::imageops::FilterType::CatmullRom,
            );
            black_box(resized);
        });
    });

    // Benchmark Lanczos3 (slowest, highest quality)
    group.bench_function("lanczos3_1920to640", |b| {
        b.iter(|| {
            let resized = image::imageops::resize(
                black_box(&test_image),
                target_size,
                target_size,
                image::imageops::FilterType::Lanczos3,
            );
            black_box(resized);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_preprocessing, bench_stages, bench_resize_filters);
criterion_main!(benches);
