/// `CoreML` vs CPU Benchmark Test for OCR Pipeline
///
/// Compares performance between CPU and `CoreML` (Apple Neural Engine) backends
/// for the `RapidOcrPure` OCR pipeline on macOS Apple Silicon.
///
/// Expected: 2-3x speedup with `CoreML` on Apple Silicon (M1/M2/M3)
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::time::{Duration, Instant};

const WARMUP_ITERATIONS: usize = 2;
const BENCHMARK_ITERATIONS: usize = 5;

#[test]
fn test_coreml_vs_cpu_benchmark() -> Result<()> {
    println!("\n{}", "=".repeat(80));
    println!("CoreML vs CPU Benchmark - RapidOcrPure OCR Pipeline");
    println!("{}", "=".repeat(80));

    // Load a real test image
    let home = std::env::var("HOME").context("HOME not set")?;
    let image_path = PathBuf::from(&home)
        .join("docling_debug_pdf_parsing/ml_model_inputs/rapid_ocr/test_image_input.npy");

    // Check if test image exists
    if !image_path.exists() {
        // Fallback: create a synthetic test image
        println!("\n[!] Real test image not found, using synthetic image");
        return run_benchmark_with_synthetic_image();
    }

    // Load real image
    println!("\n[1] Loading test image...");
    let input_array = load_npy_image(&image_path)?;
    let input_image = array_to_dynamic_image(&input_array)?;
    println!(
        "  Loaded image: {}x{}",
        input_image.width(),
        input_image.height()
    );

    run_benchmark(&input_image)
}

fn run_benchmark_with_synthetic_image() -> Result<()> {
    use image::{DynamicImage, RgbImage};

    println!("\n[1] Creating synthetic test image...");

    // Create a larger image to better measure performance (800x600)
    let mut img = RgbImage::new(800, 600);

    // Add some "text-like" patterns (white rectangles on dark background)
    for y in 0..600 {
        for x in 0..800 {
            // Background: dark gray
            let gray = 50u8;
            img.put_pixel(x, y, image::Rgb([gray, gray, gray]));
        }
    }

    // Add several "text line" rectangles
    for row in 0..10 {
        let y_start = 50 + row * 50;
        let y_end = y_start + 20;
        for y in y_start..y_end.min(600) {
            for x in 50..750 {
                img.put_pixel(x, y, image::Rgb([255, 255, 255]));
            }
        }
    }

    let input_image = DynamicImage::ImageRgb8(img);
    println!(
        "  Created synthetic image: {}x{}",
        input_image.width(),
        input_image.height()
    );

    run_benchmark(&input_image)
}

fn run_benchmark(input_image: &image::DynamicImage) -> Result<()> {
    use docling_pdf_ml::ocr::types::OcrParams;
    use docling_pdf_ml::ocr::RapidOcrPure;

    let model_dir = "models/rapidocr";
    let params = OcrParams::default();

    // ============================================================================
    // CPU Backend Benchmark
    // ============================================================================
    println!("\n[2] Benchmarking CPU backend...");

    let mut ocr_cpu =
        RapidOcrPure::new(model_dir).context("Failed to load RapidOcrPure with CPU backend")?;
    println!("  Loaded RapidOcrPure (CPU)");

    // Warmup
    println!("  Warming up ({WARMUP_ITERATIONS} iterations)...");
    for _ in 0..WARMUP_ITERATIONS {
        let _ = ocr_cpu.detect(input_image, &params)?;
    }

    // Benchmark
    println!("  Running benchmark ({BENCHMARK_ITERATIONS} iterations)...");
    let mut cpu_times: Vec<Duration> = Vec::with_capacity(BENCHMARK_ITERATIONS);
    let mut cpu_text_count = 0;

    for i in 0..BENCHMARK_ITERATIONS {
        let start = Instant::now();
        let cells = ocr_cpu.detect(input_image, &params)?;
        let elapsed = start.elapsed();
        cpu_times.push(elapsed);
        cpu_text_count = cells.len();
        println!(
            "    Iteration {}: {:?} ({} cells)",
            i + 1,
            elapsed,
            cells.len()
        );
    }

    let cpu_avg = cpu_times.iter().sum::<Duration>() / BENCHMARK_ITERATIONS as u32;
    let cpu_min = *cpu_times.iter().min().unwrap();
    let cpu_max = *cpu_times.iter().max().unwrap();

    // ============================================================================
    // CoreML Backend Benchmark
    // ============================================================================
    println!("\n[3] Benchmarking CoreML backend...");

    let mut ocr_coreml = RapidOcrPure::new_with_coreml(model_dir)
        .context("Failed to load RapidOcrPure with CoreML backend")?;
    println!("  Loaded RapidOcrPure (CoreML)");

    // Warmup
    println!("  Warming up ({WARMUP_ITERATIONS} iterations)...");
    for _ in 0..WARMUP_ITERATIONS {
        let _ = ocr_coreml.detect(input_image, &params)?;
    }

    // Benchmark
    println!("  Running benchmark ({BENCHMARK_ITERATIONS} iterations)...");
    let mut coreml_times: Vec<Duration> = Vec::with_capacity(BENCHMARK_ITERATIONS);
    let mut coreml_text_count = 0;

    for i in 0..BENCHMARK_ITERATIONS {
        let start = Instant::now();
        let cells = ocr_coreml.detect(input_image, &params)?;
        let elapsed = start.elapsed();
        coreml_times.push(elapsed);
        coreml_text_count = cells.len();
        println!(
            "    Iteration {}: {:?} ({} cells)",
            i + 1,
            elapsed,
            cells.len()
        );
    }

    let coreml_avg = coreml_times.iter().sum::<Duration>() / BENCHMARK_ITERATIONS as u32;
    let coreml_min = *coreml_times.iter().min().unwrap();
    let coreml_max = *coreml_times.iter().max().unwrap();

    // ============================================================================
    // Results Summary
    // ============================================================================
    println!("\n{}", "=".repeat(80));
    println!("BENCHMARK RESULTS");
    println!("{}", "=".repeat(80));
    println!(
        "\nImage size: {}x{}",
        input_image.width(),
        input_image.height()
    );
    println!("\nCPU Backend:");
    println!("  Average: {cpu_avg:?}");
    println!("  Min:     {cpu_min:?}");
    println!("  Max:     {cpu_max:?}");
    println!("  Detected: {cpu_text_count} text cells");

    println!("\nCoreML Backend (ANE):");
    println!("  Average: {coreml_avg:?}");
    println!("  Min:     {coreml_min:?}");
    println!("  Max:     {coreml_max:?}");
    println!("  Detected: {coreml_text_count} text cells");

    let speedup = cpu_avg.as_secs_f64() / coreml_avg.as_secs_f64();
    println!("\nSpeedup: {speedup:.2}x");

    // Verify correctness: same number of detections
    if cpu_text_count != coreml_text_count {
        println!(
            "\n[WARN] Detection count differs: CPU={cpu_text_count}, CoreML={coreml_text_count}"
        );
    } else {
        println!("\n[OK] Detection count matches: {cpu_text_count}");
    }

    println!("{}", "=".repeat(80));

    Ok(())
}

// Helper: Load .npy image file
fn load_npy_image(path: &PathBuf) -> Result<ndarray::Array3<u8>> {
    // Use ndarray-npy to read directly from path
    let array: ndarray::Array3<u8> =
        ndarray_npy::read_npy(path).context("Failed to read npy array")?;

    Ok(array)
}

// Helper: Convert ndarray to DynamicImage
fn array_to_dynamic_image(array: &ndarray::Array3<u8>) -> Result<image::DynamicImage> {
    use image::{DynamicImage, RgbImage};

    let shape = array.shape();
    let height = shape[0];
    let width = shape[1];
    let channels = shape[2];

    if channels != 3 {
        anyhow::bail!("Expected 3 channels, got {channels}");
    }

    let mut img = RgbImage::new(width as u32, height as u32);

    for y in 0..height {
        for x in 0..width {
            let r = array[[y, x, 0]];
            let g = array[[y, x, 1]];
            let b = array[[y, x, 2]];
            img.put_pixel(x as u32, y as u32, image::Rgb([r, g, b]));
        }
    }

    Ok(DynamicImage::ImageRgb8(img))
}

#[test]
fn test_coreml_detection_model_only() -> Result<()> {
    //! Benchmark just the detection model (`DbNetPure`) - the most expensive stage
    use docling_pdf_ml::ocr::types::DetectionParams;
    use docling_pdf_ml::ocr::DbNetPure;
    use image::{DynamicImage, RgbImage};

    println!("\n{}", "=".repeat(80));
    println!("CoreML vs CPU Benchmark - Detection Model Only (DbNetPure)");
    println!("{}", "=".repeat(80));

    // Create test image
    let mut img = RgbImage::new(800, 600);
    for y in 0..600 {
        for x in 0..800 {
            img.put_pixel(x, y, image::Rgb([200, 200, 200]));
        }
    }
    let input_image = DynamicImage::ImageRgb8(img);
    let params = DetectionParams::default();

    // CPU benchmark
    println!("\n[1] Detection model - CPU backend...");
    let mut det_cpu = DbNetPure::new("models/rapidocr/ch_PP-OCRv4_det_infer.onnx")?;

    // Warmup
    for _ in 0..2 {
        let _ = det_cpu.detect(&input_image, &params)?;
    }

    let start = Instant::now();
    for _ in 0..5 {
        let _ = det_cpu.detect(&input_image, &params)?;
    }
    let cpu_time = start.elapsed() / 5;
    println!("  CPU average: {cpu_time:?}");

    // CoreML benchmark
    println!("\n[2] Detection model - CoreML backend...");
    let mut det_coreml = DbNetPure::new_with_coreml("models/rapidocr/ch_PP-OCRv4_det_infer.onnx")?;

    // Warmup
    for _ in 0..2 {
        let _ = det_coreml.detect(&input_image, &params)?;
    }

    let start = Instant::now();
    for _ in 0..5 {
        let _ = det_coreml.detect(&input_image, &params)?;
    }
    let coreml_time = start.elapsed() / 5;
    println!("  CoreML average: {coreml_time:?}");

    let speedup = cpu_time.as_secs_f64() / coreml_time.as_secs_f64();
    println!("\n  Detection speedup: {speedup:.2}x");
    println!("{}", "=".repeat(80));

    Ok(())
}
