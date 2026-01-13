//! Integration tests for motion tracking

use video_audio_motion_tracking::{BoundingBox, Detection, MotionTracker, MotionTrackingConfig};

#[allow(clippy::too_many_arguments)]
fn create_detection(
    class_id: u8,
    class_name: &str,
    confidence: f32,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    frame_idx: u32,
) -> Detection {
    Detection {
        class_id,
        class_name: class_name.to_string(),
        confidence,
        bbox: BoundingBox::new(x, y, w, h),
        frame_idx,
    }
}

#[test]
fn test_tracker_creation() {
    let config = MotionTrackingConfig::default();
    let tracker = MotionTracker::new(config);
    assert!(tracker.get_all_tracks().is_empty());
}

#[test]
fn test_single_frame_tracking() {
    let config = MotionTrackingConfig {
        min_hits: 1,
        ..Default::default()
    };
    let mut tracker = MotionTracker::new(config);

    let det = create_detection(0, "person", 0.8, 0.1, 0.2, 0.3, 0.4, 0);
    let tracks = tracker.update(&[det]).unwrap();

    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].class_name, "person");
}

#[test]
fn test_multi_frame_tracking() {
    let config = MotionTrackingConfig {
        min_hits: 2,
        ..Default::default()
    };
    let mut tracker = MotionTracker::new(config);

    // Frame 0
    let det0 = create_detection(0, "person", 0.8, 0.1, 0.2, 0.3, 0.4, 0);
    let tracks0 = tracker.update(&[det0]).unwrap();
    assert_eq!(tracks0.len(), 0); // Not confirmed yet

    // Frame 1: Should match (IoU > threshold)
    let det1 = create_detection(0, "person", 0.8, 0.12, 0.21, 0.3, 0.4, 1);
    let tracks1 = tracker.update(&[det1]).unwrap();
    assert_eq!(tracks1.len(), 1); // Now confirmed
    assert_eq!(tracks1[0].detections.len(), 2);
    assert_eq!(tracks1[0].start_frame, 0);
    assert_eq!(tracks1[0].end_frame, 1);
}

#[test]
fn test_multiple_object_tracking() {
    let config = MotionTrackingConfig {
        min_hits: 1,
        ..Default::default()
    };
    let mut tracker = MotionTracker::new(config);

    // Frame 0: Two separate objects
    let det0_a = create_detection(0, "person", 0.8, 0.1, 0.2, 0.2, 0.3, 0);
    let det0_b = create_detection(1, "car", 0.9, 0.6, 0.5, 0.3, 0.4, 0);
    let tracks0 = tracker.update(&[det0_a, det0_b]).unwrap();

    assert_eq!(tracks0.len(), 2);
    assert_eq!(tracks0[0].id, 1);
    assert_eq!(tracks0[1].id, 2);
}

#[test]
fn test_track_continuity_across_frames() {
    let config = MotionTrackingConfig {
        min_hits: 1,
        ..Default::default()
    };
    let mut tracker = MotionTracker::new(config);

    // Frame 0
    let det0 = create_detection(0, "person", 0.8, 0.1, 0.2, 0.3, 0.4, 0);
    tracker.update(&[det0]).unwrap();

    // Frame 1: Moving slightly
    let det1 = create_detection(0, "person", 0.8, 0.12, 0.21, 0.3, 0.4, 1);
    tracker.update(&[det1]).unwrap();

    // Frame 2: Moving more
    let det2 = create_detection(0, "person", 0.8, 0.15, 0.23, 0.3, 0.4, 2);
    let tracks = tracker.update(&[det2]).unwrap();

    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].detections.len(), 3);
    assert_eq!(tracks[0].start_frame, 0);
    assert_eq!(tracks[0].end_frame, 2);
}

#[test]
fn test_track_death_after_missing_frames() {
    let config = MotionTrackingConfig {
        min_hits: 1,
        max_age: 2,
        ..Default::default()
    };
    let mut tracker = MotionTracker::new(config);

    // Frame 0: Create track
    let det0 = create_detection(0, "person", 0.8, 0.1, 0.2, 0.3, 0.4, 0);
    let tracks0 = tracker.update(&[det0]).unwrap();
    assert_eq!(tracks0.len(), 1);

    // Frame 1-2: No detections (track ages)
    tracker.update(&[]).unwrap();
    tracker.update(&[]).unwrap();

    // Frame 3: Track should be dead
    let tracks3 = tracker.update(&[]).unwrap();
    assert_eq!(tracks3.len(), 0);
}

#[test]
fn test_high_confidence_matching() {
    let config = MotionTrackingConfig {
        min_hits: 1,
        detection_threshold_high: 0.7,
        high_confidence_threshold: 0.5,
        ..Default::default()
    };
    let mut tracker = MotionTracker::new(config);

    // Frame 0: High confidence
    let det0 = create_detection(0, "person", 0.8, 0.1, 0.2, 0.3, 0.4, 0);
    let tracks0 = tracker.update(&[det0]).unwrap();
    assert_eq!(tracks0.len(), 1);

    // Frame 1: High confidence, overlapping
    let det1 = create_detection(0, "person", 0.85, 0.11, 0.21, 0.3, 0.4, 1);
    let tracks1 = tracker.update(&[det1]).unwrap();
    assert_eq!(tracks1.len(), 1);
    assert_eq!(tracks1[0].id, 1); // Same track ID
}

#[test]
fn test_low_confidence_matching() {
    let config = MotionTrackingConfig {
        min_hits: 1,
        detection_threshold_high: 0.6,
        detection_threshold_low: 0.3,
        low_confidence_threshold: 0.3,
        ..Default::default()
    };
    let mut tracker = MotionTracker::new(config);

    // Frame 0: High confidence
    let det0 = create_detection(0, "person", 0.8, 0.1, 0.2, 0.3, 0.4, 0);
    tracker.update(&[det0]).unwrap();

    // Frame 1: Low confidence, overlapping (should still match)
    let det1 = create_detection(0, "person", 0.4, 0.12, 0.21, 0.3, 0.4, 1);
    let tracks1 = tracker.update(&[det1]).unwrap();
    assert_eq!(tracks1.len(), 1);
    assert_eq!(tracks1[0].detections.len(), 2); // Track continues
}

#[test]
fn test_track_trajectory() {
    let config = MotionTrackingConfig {
        min_hits: 1,
        high_confidence_threshold: 0.3, // Lower threshold for test
        ..Default::default()
    };
    let mut tracker = MotionTracker::new(config);

    // Create a track across 3 frames with overlapping boxes
    tracker
        .update(&[create_detection(0, "person", 0.8, 0.1, 0.2, 0.3, 0.4, 0)])
        .unwrap();
    tracker
        .update(&[create_detection(0, "person", 0.8, 0.15, 0.22, 0.3, 0.4, 1)])
        .unwrap();
    tracker
        .update(&[create_detection(0, "person", 0.8, 0.20, 0.24, 0.3, 0.4, 2)])
        .unwrap();

    // Get all tracks to see full trajectory
    let tracks = tracker.get_all_tracks();
    assert_eq!(tracks.len(), 1, "Expected 1 track, got {}", tracks.len());
    let trajectory = tracks[0].trajectory();
    assert_eq!(trajectory.len(), 3);

    // Check trajectory centers
    let (x0, y0) = trajectory[0];
    let (x1, y1) = trajectory[1];
    let (x2, y2) = trajectory[2];

    // Should show movement
    assert!(x1 > x0);
    assert!(y1 > y0);
    assert!(x2 > x1);
    assert!(y2 > y1);
}

#[test]
fn test_track_duration() {
    let config = MotionTrackingConfig {
        min_hits: 1,
        ..Default::default()
    };
    let mut tracker = MotionTracker::new(config);

    tracker
        .update(&[create_detection(0, "person", 0.8, 0.1, 0.2, 0.3, 0.4, 0)])
        .unwrap();
    tracker
        .update(&[create_detection(0, "person", 0.8, 0.12, 0.21, 0.3, 0.4, 1)])
        .unwrap();
    tracker
        .update(&[create_detection(0, "person", 0.8, 0.14, 0.22, 0.3, 0.4, 2)])
        .unwrap();
    let tracks = tracker
        .update(&[create_detection(0, "person", 0.8, 0.16, 0.23, 0.3, 0.4, 3)])
        .unwrap();

    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].duration(), 4); // Frames 0-3 inclusive
}

#[test]
fn test_iou_calculation() {
    let bbox1 = BoundingBox::new(0.0, 0.0, 0.5, 0.5);
    let bbox2 = BoundingBox::new(0.25, 0.25, 0.5, 0.5); // 50% overlap

    let iou = bbox1.iou(&bbox2);
    assert!(iou > 0.0);
    assert!(iou < 1.0);

    // Exact same box should have IoU = 1.0
    let bbox3 = BoundingBox::new(0.0, 0.0, 0.5, 0.5);
    assert!((bbox1.iou(&bbox3) - 1.0).abs() < 0.01);

    // Non-overlapping boxes should have IoU = 0.0
    let bbox4 = BoundingBox::new(0.6, 0.6, 0.3, 0.3);
    assert!((bbox1.iou(&bbox4) - 0.0).abs() < 0.01);
}
