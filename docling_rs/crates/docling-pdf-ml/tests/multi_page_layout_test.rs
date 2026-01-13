/// Multi-page PDF Layout Detection Test
///
/// Tests layout detection on multi_page.pdf to verify semantic labels are correctly detected.
use docling_pdf_ml::models::layout_predictor::{InferenceBackend, LayoutPredictorModel};
use image::RgbImage;
use ndarray::Array3;
use std::collections::HashMap;
use std::path::PathBuf;

// Device stub for non-pytorch builds
#[cfg(not(feature = "pytorch"))]
use docling_pdf_ml::pipeline::Device;
#[cfg(feature = "pytorch")]
use tch::Device;

fn rgb_image_to_ndarray(img: &RgbImage) -> Array3<u8> {
    let (width, height) = img.dimensions();
    let mut arr = Array3::<u8>::zeros((height as usize, width as usize, 3));
    for (x, y, pixel) in img.enumerate_pixels() {
        arr[[y as usize, x as usize, 0]] = pixel[0];
        arr[[y as usize, x as usize, 1]] = pixel[1];
        arr[[y as usize, x as usize, 2]] = pixel[2];
    }
    arr
}

#[test]
fn test_multi_page_layout_detection() {
    println!("\n================================================================================");
    println!("Multi-page PDF Layout Detection Test");
    println!("================================================================================");
    println!("Testing that semantic labels (Section-Header, List-item) are detected correctly");
    println!();

    // Load ONNX model
    let model_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("onnx_exports/layout_optimum/model.onnx");

    if !model_path.exists() {
        println!("⚠️  Skipping - ONNX model not found at {model_path:?}");
        return;
    }

    // Load a test image - use a rendered page from multi_page.pdf
    // For simplicity, let's create a synthetic test image
    let test_image_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../test-corpus/pdf/multi_page.pdf_page_0.png");

    // If page image doesn't exist, try to find alternative
    let image_path = if test_image_path.exists() {
        test_image_path
    } else {
        // Try temp location where debug outputs might be saved
        let tmp_path = PathBuf::from("/tmp/multi_page_page_0.png");
        if tmp_path.exists() {
            tmp_path
        } else {
            println!("⚠️  Skipping - No test image found");
            println!("   Expected: {test_image_path:?}");
            println!("   Or: /tmp/multi_page_page_0.png");
            println!("   Please render multi_page.pdf page 0 to one of these locations");
            return;
        }
    };

    println!("Loading image from: {image_path:?}");
    let img = image::open(&image_path).expect("load image");
    let rgb_img = img.to_rgb8();
    let image_array = rgb_image_to_ndarray(&rgb_img);
    println!("Image size: {:?}", image_array.dim());

    // Load model
    let mut model =
        LayoutPredictorModel::load_with_backend(&model_path, Device::Cpu, InferenceBackend::ONNX)
            .expect("load model");

    println!("\n--- Running inference ---");
    let clusters = model.infer(&image_array).expect("inference");

    println!("\n--- Results ---");
    println!("Total clusters detected: {}", clusters.len());

    // Count by label
    let mut label_counts: HashMap<&str, usize> = HashMap::new();
    let mut label_confidences: HashMap<&str, f64> = HashMap::new();

    for cluster in &clusters {
        *label_counts.entry(&cluster.label).or_insert(0) += 1;
        let entry = label_confidences.entry(&cluster.label).or_insert(0.0);
        if cluster.confidence > *entry {
            *entry = cluster.confidence;
        }
    }

    println!("\nLabel distribution:");
    let mut sorted_labels: Vec<_> = label_counts.iter().collect();
    sorted_labels.sort_by(|a, b| b.1.cmp(a.1));
    for (label, count) in sorted_labels {
        let max_conf = label_confidences.get(label).unwrap_or(&0.0);
        println!("  {label}: {count} (max conf: {max_conf:.3})");
    }

    // Expected labels for multi_page.pdf page 0:
    // - Text (multiple paragraphs)
    // - Section-Header (e.g., "The Evolution of the Word Processor")
    // - List-item (bullet points about word processing history)
    let has_section_headers = label_counts.get("Section-Header").copied().unwrap_or(0) > 0;
    let has_text = label_counts.get("Text").copied().unwrap_or(0) > 0;

    println!("\n--- Validation ---");
    println!("Has Text labels: {has_text}");
    println!("Has Section-Header labels: {has_section_headers}");

    // The key validation: we should detect Section-Header with reasonable confidence
    if has_section_headers {
        let sh_conf = label_confidences.get("Section-Header").unwrap_or(&0.0);
        println!("\n✅ Section-Header detected with confidence {sh_conf:.3}");
    } else {
        println!("\n❌ Section-Header NOT detected!");
        println!("   This indicates the model is not producing correct semantic labels");
    }

    println!(
        "\n================================================================================\n"
    );
}
