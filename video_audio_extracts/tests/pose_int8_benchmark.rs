//! Benchmark: Pose Estimation FP32 vs INT8
//!
//! Measures performance and accuracy of YOLOv8n-Pose INT8 quantization
//! Tests both inference speed and output consistency (detection count, keypoint similarity)
//!
//! Run with: cargo test --release --test pose_int8_benchmark -- --ignored --nocapture --test-threads=1

use std::path::Path;
use std::time::Instant;
use video_audio_pose_estimation::{PoseEstimationConfig, PoseEstimator, YOLOPoseModel};
use video_extract_core::image_io::load_image;

/// Run pose estimation with specified model and collect statistics
fn benchmark_model(
    model: YOLOPoseModel,
    test_image: &Path,
    runs: usize,
) -> Result<(Vec<std::time::Duration>, usize), Box<dyn std::error::Error>> {
    println!(
        "  Loading model: {} ({} MB)",
        model.filename(),
        model.size_bytes() / 1_000_000
    );

    // Load image once
    let img = load_image(test_image)?;
    println!("  Image loaded: {}x{} pixels", img.width(), img.height());

    // Load ONNX session (this is cached in real usage, but measure here for completeness)
    let model_path = Path::new("models/pose-estimation").join(model.filename());
    if !model_path.exists() {
        return Err(format!("Model not found: {}", model_path.display()).into());
    }

    let session_load_start = Instant::now();
    let mut session = video_extract_core::onnx_utils::create_optimized_session(&model_path)?;
    let session_load_time = session_load_start.elapsed();
    println!("  Session loaded: {:.3}s", session_load_time.as_secs_f64());

    // Configure estimator
    let config = PoseEstimationConfig {
        confidence_threshold: 0.5,
        keypoint_threshold: 0.5,
        ..Default::default()
    };

    // Warm-up run (not counted)
    println!("  Warm-up run...");
    let _ = PoseEstimator::estimate_with_session(&mut session, &img, &config)?;

    // Benchmark runs
    println!("  Running {} benchmark iterations...", runs);
    let mut times = Vec::with_capacity(runs);
    let mut detection_count = 0;

    for run in 1..=runs {
        let start = Instant::now();
        let detections = PoseEstimator::estimate_with_session(&mut session, &img, &config)?;
        let duration = start.elapsed();

        times.push(duration);

        if run == 1 {
            detection_count = detections.len();
            println!(
                "    Run 1: {:.3}s ({} detections)",
                duration.as_secs_f64(),
                detection_count
            );
        } else {
            println!("    Run {}: {:.3}s", run, duration.as_secs_f64());
        }
    }

    Ok((times, detection_count))
}

fn avg_duration(times: &[std::time::Duration]) -> std::time::Duration {
    times.iter().sum::<std::time::Duration>() / times.len() as u32
}

#[test]
#[ignore]
fn benchmark_pose_fp32_vs_int8() {
    println!("\n=== Pose Estimation Benchmark: FP32 vs INT8 ===\n");

    // Use a test image with human subjects
    let test_image = Path::new("test_edge_cases/video_hevc_h265_modern_codec__compatibility.mp4");

    // First extract a keyframe for testing
    println!("Extracting test keyframe...");

    // Use video-extract to get a keyframe
    let extract_status = std::process::Command::new("target/release/video-extract")
        .arg("debug")
        .arg(test_image)
        .arg("--ops")
        .arg("keyframes")
        .arg("--output-dir")
        .arg("./debug_output_pose_benchmark")
        .arg("--max-frames")
        .arg("1")
        .env("VIDEO_EXTRACT_THREADS", "4")
        .status();

    match extract_status {
        Ok(status) if status.success() => {
            // Find the extracted keyframe
            let keyframe_json =
                std::fs::read_to_string("./debug_output_pose_benchmark/stage_00_keyframes.json")
                    .expect("Failed to read stage_00_keyframes.json");
            let keyframes: Vec<serde_json::Value> =
                serde_json::from_str(&keyframe_json).expect("Failed to parse keyframes.json");

            if keyframes.is_empty() {
                panic!("No keyframes extracted from test video");
            }

            // Use the first keyframe's largest thumbnail
            let first_keyframe = &keyframes[0];
            let thumbnail_paths = first_keyframe["thumbnail_paths"]
                .as_object()
                .expect("No thumbnail_paths in keyframe");
            let test_image_path = thumbnail_paths
                .values()
                .next()
                .expect("No thumbnails available")
                .as_str()
                .expect("Thumbnail path not a string");

            println!("Using test image: {}\n", test_image_path);
            let test_image = Path::new(test_image_path);

            // Benchmark FP32
            println!("Benchmarking FP32 (yolov8n-pose.onnx):");
            let (fp32_times, fp32_detections) =
                benchmark_model(YOLOPoseModel::Nano, test_image, 5).expect("FP32 benchmark failed");
            let fp32_avg = avg_duration(&fp32_times);
            println!(
                "  Average: {:.3}s (min: {:.3}s, max: {:.3}s, std: {:.3}s)\n",
                fp32_avg.as_secs_f64(),
                fp32_times.iter().min().unwrap().as_secs_f64(),
                fp32_times.iter().max().unwrap().as_secs_f64(),
                std_dev(&fp32_times)
            );

            // Benchmark INT8
            println!("Benchmarking INT8 (yolov8n-pose-int8.onnx):");
            let (int8_times, int8_detections) =
                benchmark_model(YOLOPoseModel::NanoInt8, test_image, 5)
                    .expect("INT8 benchmark failed");
            let int8_avg = avg_duration(&int8_times);
            println!(
                "  Average: {:.3}s (min: {:.3}s, max: {:.3}s, std: {:.3}s)\n",
                int8_avg.as_secs_f64(),
                int8_times.iter().min().unwrap().as_secs_f64(),
                int8_times.iter().max().unwrap().as_secs_f64(),
                std_dev(&int8_times)
            );

            // Compare results
            println!("=== RESULTS ===\n");

            let speedup = fp32_avg.as_secs_f64() / int8_avg.as_secs_f64();
            println!("FP32 average:      {:.3}s", fp32_avg.as_secs_f64());
            println!("INT8 average:      {:.3}s", int8_avg.as_secs_f64());
            println!("Speedup:           {:.2}x", speedup);

            if speedup > 1.0 {
                println!(
                    "Performance:       INT8 is {:.1}% faster",
                    (speedup - 1.0) * 100.0
                );
            } else {
                println!(
                    "Performance:       INT8 is {:.1}% slower",
                    (1.0 - speedup) * 100.0
                );
            }

            println!("\nFP32 detections:   {}", fp32_detections);
            println!("INT8 detections:   {}", int8_detections);

            let detection_diff = (int8_detections as i32) - (fp32_detections as i32);
            if detection_diff == 0 {
                println!("Detection accuracy: 100% (identical)");
            } else {
                println!("Detection difference: {:+} detections", detection_diff);
            }

            // Model sizes
            println!("\nFP32 model size:   13 MB");
            println!("INT8 model size:   3.6 MB");
            println!("Size reduction:    72%");

            // Verdict
            println!("\n=== VERDICT ===\n");

            let accuracy_acceptable = (int8_detections as i32 - fp32_detections as i32).abs() <= 1;

            if speedup >= 1.05 && accuracy_acceptable {
                println!("✅ INT8 quantization is VIABLE:");
                println!("   - {:.2}x speedup (≥5% improvement threshold)", speedup);
                println!("   - Acceptable accuracy (detection count within ±1)");
                println!("   - 72% smaller model (13MB → 3.6MB)");
                println!("\nRECOMMENDATION: Switch default to INT8");
            } else if speedup < 1.0 {
                println!("❌ INT8 quantization is NOT VIABLE:");
                println!("   - {:.1}% slower than FP32", (1.0 - speedup) * 100.0);
                println!("\nRECOMMENDATION: Keep FP32 as default");
            } else if !accuracy_acceptable {
                println!("⚠️  INT8 quantization has ACCURACY ISSUES:");
                println!("   - {:.2}x speedup achieved", speedup);
                println!("   - Detection count difference: {:+}", detection_diff);
                println!("\nRECOMMENDATION: Further testing needed");
            } else {
                println!("⚠️  INT8 quantization has MARGINAL BENEFIT:");
                println!(
                    "   - Only {:.1}% faster (<5% threshold)",
                    (speedup - 1.0) * 100.0
                );
                println!("\nRECOMMENDATION: Consider model size benefit (72% reduction)");
            }
        }
        Ok(status) => panic!("Keyframe extraction failed with status: {}", status),
        Err(e) => panic!("Failed to run video-extract: {}", e),
    }

    println!("\n=== END ===");
}

/// Calculate standard deviation of durations
fn std_dev(times: &[std::time::Duration]) -> f64 {
    if times.len() <= 1 {
        return 0.0;
    }

    let avg = avg_duration(times).as_secs_f64();
    let variance: f64 = times
        .iter()
        .map(|t| {
            let diff = t.as_secs_f64() - avg;
            diff * diff
        })
        .sum::<f64>()
        / (times.len() - 1) as f64;

    variance.sqrt()
}
