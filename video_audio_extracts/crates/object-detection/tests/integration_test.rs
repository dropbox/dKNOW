use image::{Rgb, RgbImage};
use video_audio_object_detection::{ObjectDetectionConfig, ObjectDetector, YOLOModel};

const MODEL_PATH: &str = "models/yolov8n.onnx";

/// Create a test image with solid color
fn create_test_image(width: u32, height: u32, color: Rgb<u8>) -> RgbImage {
    RgbImage::from_fn(width, height, |_, _| color)
}

#[test]
#[ignore] // Requires yolov8n.onnx model to be downloaded
fn test_detector_loads_model() {
    let config = ObjectDetectionConfig::default();
    let detector = ObjectDetector::new(MODEL_PATH, config);
    assert!(
        detector.is_ok(),
        "Failed to load YOLOv8 model from {MODEL_PATH}"
    );
}

#[test]
#[ignore] // Requires yolov8n.onnx model to be downloaded
fn test_detect_on_blank_image() {
    let config = ObjectDetectionConfig::default();
    let mut detector = ObjectDetector::new(MODEL_PATH, config).unwrap();

    // Create a blank image (should detect nothing)
    let img = create_test_image(640, 480, Rgb([255, 255, 255]));

    let detections = detector.detect(&img).unwrap();

    // Blank image should have few or no detections
    assert!(
        detections.len() < 5,
        "Unexpected detections on blank image: {detections:?}"
    );
}

#[test]
#[ignore] // Requires yolov8n.onnx model to be downloaded
fn test_detect_with_different_configs() {
    let configs = vec![
        ObjectDetectionConfig::default(),
        ObjectDetectionConfig::fast(),
        ObjectDetectionConfig::accurate(),
        ObjectDetectionConfig::person_only(),
    ];

    let img = create_test_image(640, 480, Rgb([128, 128, 128]));

    for config in configs {
        let config_clone = config.clone();
        let mut detector = ObjectDetector::new(MODEL_PATH, config).unwrap();
        let result = detector.detect(&img);
        assert!(
            result.is_ok(),
            "Detection failed with config: {config_clone:?}"
        );
    }
}

#[test]
#[ignore] // Requires yolov8n.onnx model and test image
fn test_detect_on_real_image() {
    // This test requires a real test image with known objects
    let test_image_path = "test_images/sample.jpg";

    if !std::path::Path::new(test_image_path).exists() {
        eprintln!("Skipping test: {test_image_path} not found");
        return;
    }

    let config = ObjectDetectionConfig::default();
    let mut detector = ObjectDetector::new(MODEL_PATH, config).unwrap();

    let img = image::open(test_image_path).unwrap().to_rgb8();
    let detections = detector.detect(&img).unwrap();

    println!("Detected {} objects:", detections.len());
    for (i, det) in detections.iter().enumerate() {
        println!(
            "  {}: {} ({:.2}% confidence) at ({:.3}, {:.3}, {:.3}, {:.3})",
            i + 1,
            det.class_name,
            det.confidence * 100.0,
            det.bbox.x,
            det.bbox.y,
            det.bbox.width,
            det.bbox.height
        );
    }

    // At least one object should be detected in a real image
    assert!(!detections.is_empty(), "Expected at least one detection");
}

#[test]
#[ignore] // Requires yolov8n.onnx model
fn test_batch_detection() {
    let config = ObjectDetectionConfig::default();
    let mut detector = ObjectDetector::new(MODEL_PATH, config).unwrap();

    let images = vec![
        create_test_image(640, 480, Rgb([255, 0, 0])),
        create_test_image(640, 480, Rgb([0, 255, 0])),
        create_test_image(640, 480, Rgb([0, 0, 255])),
    ];

    let results = detector.detect_batch(&images).unwrap();

    assert_eq!(results.len(), 3, "Expected 3 detection results");
    for (i, detections) in results.iter().enumerate() {
        println!("Image {}: {} detections", i, detections.len());
    }
}

#[test]
#[ignore] // Requires yolov8n.onnx model
fn test_confidence_threshold_filtering() {
    let low_threshold = ObjectDetectionConfig {
        confidence_threshold: 0.1,
        ..Default::default()
    };

    let high_threshold = ObjectDetectionConfig {
        confidence_threshold: 0.9,
        ..Default::default()
    };

    let img = create_test_image(640, 480, Rgb([128, 128, 128]));

    let mut detector_low = ObjectDetector::new(MODEL_PATH, low_threshold).unwrap();
    let mut detector_high = ObjectDetector::new(MODEL_PATH, high_threshold).unwrap();

    let detections_low = detector_low.detect(&img).unwrap();
    let detections_high = detector_high.detect(&img).unwrap();

    // Lower threshold should detect more (or equal) objects
    assert!(
        detections_low.len() >= detections_high.len(),
        "Low threshold detected {} objects, high threshold detected {}",
        detections_low.len(),
        detections_high.len()
    );
}

#[test]
#[ignore] // Requires yolov8n.onnx model
fn test_class_filtering() {
    let config_all = ObjectDetectionConfig::default();
    let config_person_only = ObjectDetectionConfig::person_only();

    let img = create_test_image(640, 480, Rgb([128, 128, 128]));

    let mut detector_all = ObjectDetector::new(MODEL_PATH, config_all).unwrap();
    let mut detector_person = ObjectDetector::new(MODEL_PATH, config_person_only).unwrap();

    let detections_all = detector_all.detect(&img).unwrap();
    let detections_person = detector_person.detect(&img).unwrap();

    // Person-only detector should only find person class (class_id = 0)
    for det in &detections_person {
        assert_eq!(
            det.class_id, 0,
            "Expected only person class (0), got {}",
            det.class_id
        );
        assert_eq!(det.class_name, "person");
    }

    // Person-only should detect fewer or equal objects
    assert!(detections_person.len() <= detections_all.len());
}

#[test]
fn test_yolo_model_enum() {
    assert_eq!(YOLOModel::Nano.filename(), "yolov8n.onnx");
    assert_eq!(YOLOModel::Small.filename(), "yolov8s.onnx");
    assert_eq!(YOLOModel::Medium.filename(), "yolov8m.onnx");
    assert_eq!(YOLOModel::Large.filename(), "yolov8l.onnx");
    assert_eq!(YOLOModel::XLarge.filename(), "yolov8x.onnx");

    assert_eq!(YOLOModel::Nano.size_bytes(), 6_000_000);
    assert_eq!(YOLOModel::XLarge.size_bytes(), 136_000_000);
}
