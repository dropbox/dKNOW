/// Integration test for C FFI zero-copy decoder and audio extraction
use std::path::Path;
use video_audio_decoder::{decode_iframes_zero_copy, extract_audio_to_wav};

#[test]
#[ignore] // Requires test video file
fn test_c_ffi_decoder_vfr() {
    let test_video =
        Path::new("../../test_edge_cases/video_variable_framerate_vfr__timing_test.mp4");

    if !test_video.exists() {
        panic!("Test video not found: {}", test_video.display());
    }

    let result = decode_iframes_zero_copy(test_video);
    assert!(
        result.is_ok(),
        "C FFI decoder should succeed: {:?}",
        result.err()
    );

    let frames = result.unwrap();
    assert!(!frames.is_empty(), "Should extract at least one I-frame");

    println!("✅ Extracted {} I-frames", frames.len());

    // Verify first frame has valid dimensions
    let first_frame = &frames[0];
    assert!(first_frame.width > 0, "Frame width should be positive");
    assert!(first_frame.height > 0, "Frame height should be positive");
    assert!(
        !first_frame.data_ptr.is_null(),
        "Data pointer should not be null"
    );
    assert!(first_frame.linesize > 0, "Linesize should be positive");
    assert!(first_frame.is_keyframe, "Should be marked as keyframe");

    println!(
        "✅ First frame: {}x{} pixels, {:.2}s timestamp, linesize={}",
        first_frame.width, first_frame.height, first_frame.timestamp, first_frame.linesize
    );

    // Verify data pointer is readable (sample first pixel)
    unsafe {
        let first_pixel_r = *first_frame.data_ptr.offset(0);
        let first_pixel_g = *first_frame.data_ptr.offset(1);
        let first_pixel_b = *first_frame.data_ptr.offset(2);
        println!(
            "✅ First pixel RGB: ({}, {}, {})",
            first_pixel_r, first_pixel_g, first_pixel_b
        );
        // Pixels are u8, so they're always in valid range [0, 255]
    }

    println!("✅ C FFI decoder test PASSED");
}

#[test]
#[ignore] // Requires test video file with audio
fn test_c_ffi_audio_extraction() {
    let test_video = Path::new("../../test_media/test_120fps_10s.mp4");
    let output_wav = Path::new("/tmp/test_audio_cffi_output.wav");

    // Skip if test file doesn't exist
    if !test_video.exists() {
        eprintln!(
            "Skipping test: test file not found at {}",
            test_video.display()
        );
        return;
    }

    // Remove old output if exists
    let _ = std::fs::remove_file(output_wav);

    // Extract audio: 16kHz mono PCM
    let result = extract_audio_to_wav(test_video, output_wav, 16000, 1);

    match result {
        Ok(()) => {
            assert!(output_wav.exists(), "Output file should exist");
            let metadata = std::fs::metadata(output_wav).unwrap();
            assert!(metadata.len() > 0, "Output file should not be empty");

            println!("✅ Extracted audio: {} bytes", metadata.len());

            // Cleanup
            let _ = std::fs::remove_file(output_wav);

            println!("✅ C FFI audio extraction test PASSED");
        }
        Err(e) => {
            panic!("Audio extraction failed: {}", e);
        }
    }
}
