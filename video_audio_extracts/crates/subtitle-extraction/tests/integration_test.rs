use std::path::PathBuf;
use video_audio_subtitle::{extract_subtitles, SubtitleConfig};

#[test]
fn test_extract_subtitles_from_test_video() {
    let test_video = PathBuf::from("../../test_edge_cases/video_with_subtitles__subtitle_test.mp4");

    if !test_video.exists() {
        eprintln!("Test video not found, skipping test");
        return;
    }

    let config = SubtitleConfig::default();
    let result = extract_subtitles(&test_video, config);

    match result {
        Ok(subtitles) => {
            println!("Extracted {} subtitle tracks", subtitles.tracks.len());
            println!("Total entries: {}", subtitles.total_entries);

            for track in &subtitles.tracks {
                println!("\nTrack {}: {} entries", track.index, track.entries.len());
                for entry in track.entries.iter().take(3) {
                    println!(
                        "  [{:.2}s -> {:.2}s] {}",
                        entry.start_time, entry.end_time, entry.text
                    );
                }
            }

            assert!(
                subtitles.total_entries > 0,
                "Should have extracted subtitle entries"
            );
            assert_eq!(subtitles.tracks.len(), 1, "Should have one subtitle track");
            assert_eq!(
                subtitles.tracks[0].entries.len(),
                4,
                "Should have 4 subtitle entries"
            );
        }
        Err(e) => {
            panic!("Failed to extract subtitles: {}", e);
        }
    }
}
