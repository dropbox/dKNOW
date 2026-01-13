//! Benchmark scene detection with and without `keyframes_only` optimization

use std::env;
use std::path::Path;
use std::time::Instant;
use video_audio_scene::{detect_scenes, SceneDetectorConfig};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <video_path> [keyframes_only=true]", args[0]);
        eprintln!("Example: {} video.mp4 true", args[0]);
        std::process::exit(1);
    }

    let video_path = &args[1];
    let keyframes_only = args.len() > 2 && args[2] == "true";

    println!("Scene Detection Benchmark");
    println!("========================");
    println!("File: {video_path}");
    println!("Mode: keyframes_only = {keyframes_only}");
    println!();

    let config = SceneDetectorConfig {
        threshold: 10.0,
        min_scene_duration: 0.0,
        keyframes_only,
    };

    println!("Starting scene detection...");
    let start = Instant::now();

    match detect_scenes(Path::new(video_path), &config) {
        Ok(result) => {
            let elapsed = start.elapsed();
            println!();
            println!("✓ SUCCESS");
            println!("  Duration: {:.2}s", elapsed.as_secs_f64());
            println!("  Scene boundaries: {}", result.boundaries.len());
            println!("  Total scenes: {}", result.num_scenes);
            println!();

            if !result.boundaries.is_empty() {
                println!("Scene boundaries detected:");
                for (i, boundary) in result.boundaries.iter().enumerate() {
                    println!(
                        "  #{}: {:.2}s (score: {:.2})",
                        i + 1,
                        boundary.timestamp,
                        boundary.score
                    );
                }
            }

            println!();
            println!("Scenes:");
            for (i, scene) in result.scenes.iter().enumerate() {
                println!(
                    "  Scene #{}: {:.2}s - {:.2}s ({:.2}s duration, {} frames)",
                    i + 1,
                    scene.start_time,
                    scene.end_time,
                    scene.end_time - scene.start_time,
                    scene.frame_count
                );
            }
        }
        Err(e) => {
            let elapsed = start.elapsed();
            eprintln!();
            eprintln!("✗ FAILED after {:.2}s", elapsed.as_secs_f64());
            eprintln!("  Error: {e}");
            std::process::exit(1);
        }
    }
}
