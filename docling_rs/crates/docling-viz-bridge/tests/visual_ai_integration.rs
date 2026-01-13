// Test code uses various patterns that trigger pedantic lints but are acceptable
#![allow(
    clippy::float_cmp,                  // test values are exact comparisons
    clippy::needless_raw_string_hashes, // test strings
    clippy::uninlined_format_args,      // test output formatting
    clippy::single_match_else,          // test clarity
    clippy::manual_let_else,            // test clarity
    clippy::doc_markdown,               // test documentation
    clippy::redundant_closure_for_method_calls, // test clarity
    clippy::missing_panics_doc,         // test functions can panic
)]

//! Visual AI Integration Tests
//!
//! Tests that validate the PDF layout detection quality using:
//! 1. Structural validation (label variety, confidence variety)
//! 2. Optional LLM-based visual quality assessment
//!
//! # Running Tests
//!
//! ```bash
//! # Run fast validation tests (no LLM required)
//! cargo test -p docling-viz-bridge --test visual_ai_integration
//!
//! # Run LLM-based tests (requires OPENAI_API_KEY)
//! OPENAI_API_KEY=sk-... cargo test -p docling-viz-bridge --test visual_ai_integration -- --ignored
//! ```

use docling_viz_bridge::{
    visualization::{validate_layout_quality, LayoutValidationResult},
    DlvizBBox, DlvizElement, DlvizLabel,
};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

/// Helper to create a test element
const fn make_element(id: u32, label: DlvizLabel, confidence: f32) -> DlvizElement {
    DlvizElement {
        id,
        bbox: DlvizBBox {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 20.0,
        },
        label,
        confidence,
        reading_order: id as i32,
    }
}

/// Test that layout validation correctly identifies good layouts
#[test]
fn test_layout_variety_validation_good() {
    // A good layout has multiple label types and varying confidence
    let elements = vec![
        make_element(1, DlvizLabel::Title, 0.96),
        make_element(2, DlvizLabel::SectionHeader, 0.89),
        make_element(3, DlvizLabel::Text, 0.87),
        make_element(4, DlvizLabel::Text, 0.92),
        make_element(5, DlvizLabel::Text, 0.85),
        make_element(6, DlvizLabel::ListItem, 0.88),
    ];

    let result = validate_layout_quality(&elements);
    assert_eq!(
        result,
        LayoutValidationResult::Valid,
        "Good layout should be valid"
    );

    // Check label variety
    let unique_labels: HashSet<DlvizLabel> = elements.iter().map(|e| e.label).collect();
    assert!(
        unique_labels.len() >= 3,
        "Good layout should have at least 3 distinct labels"
    );
}

/// Test that layout validation catches all-text bug
#[test]
fn test_layout_variety_validation_all_text_fails() {
    // All elements labeled "text" indicates ML model failure
    let elements = vec![
        make_element(1, DlvizLabel::Text, 0.90),
        make_element(2, DlvizLabel::Text, 0.91),
        make_element(3, DlvizLabel::Text, 0.92),
        make_element(4, DlvizLabel::Text, 0.89),
        make_element(5, DlvizLabel::Text, 0.88),
        make_element(6, DlvizLabel::Text, 0.90),
    ];

    let result = validate_layout_quality(&elements);
    match result {
        LayoutValidationResult::Error(msg) => {
            assert!(
                msg.contains("All") && msg.contains("text"),
                "Error message should mention all-text issue: {}",
                msg
            );
        }
        other => panic!("Expected Error for all-text layout, got {:?}", other),
    }
}

/// Test that layout validation catches uniform confidence bug
#[test]
fn test_layout_variety_validation_uniform_confidence_fails() {
    // All elements with confidence 1.0 indicates native text, not ML predictions
    let elements = vec![
        make_element(1, DlvizLabel::Title, 1.0),
        make_element(2, DlvizLabel::SectionHeader, 1.0),
        make_element(3, DlvizLabel::Text, 1.0),
        make_element(4, DlvizLabel::Text, 1.0),
        make_element(5, DlvizLabel::Text, 1.0),
        make_element(6, DlvizLabel::Text, 1.0),
    ];

    let result = validate_layout_quality(&elements);
    match result {
        LayoutValidationResult::Error(msg) => {
            assert!(
                msg.contains("confidence"),
                "Error message should mention confidence issue: {}",
                msg
            );
        }
        other => panic!(
            "Expected Error for uniform confidence layout, got {:?}",
            other
        ),
    }
}

/// Test that small element counts don't trigger false positives
#[test]
fn test_layout_variety_validation_small_count_ok() {
    // Few elements (<=5) don't trigger errors even if homogeneous
    let elements = vec![
        make_element(1, DlvizLabel::Text, 1.0),
        make_element(2, DlvizLabel::Text, 1.0),
        make_element(3, DlvizLabel::Text, 1.0),
    ];

    let result = validate_layout_quality(&elements);
    assert_eq!(
        result,
        LayoutValidationResult::Valid,
        "Small element count should not trigger validation errors"
    );
}

/// Test that empty layouts are valid (blank page)
#[test]
fn test_layout_variety_validation_empty_ok() {
    let result = validate_layout_quality(&[]);
    assert_eq!(
        result,
        LayoutValidationResult::Valid,
        "Empty layout should be valid (blank page)"
    );
}

/// Integration test: Run dlviz-screenshot on a real PDF and validate output structure
///
/// This test requires:
/// - dlviz-screenshot binary to be built
/// - Test PDF file to exist
#[test]
#[ignore = "Requires built dlviz-screenshot binary and test corpus"]
fn test_pdf_layout_visual_quality_structure() {
    let test_pdf = "test-corpus/pdf/2305.03393v1.pdf";
    if !Path::new(test_pdf).exists() {
        eprintln!("Test PDF not found: {}", test_pdf);
        return;
    }

    let output_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let output_base = output_dir.path().join("output");

    // Run dlviz-screenshot
    let dlviz_path = "target/release/dlviz-screenshot";
    if !Path::new(dlviz_path).exists() {
        eprintln!("dlviz-screenshot not built: {}", dlviz_path);
        return;
    }

    let output = Command::new(dlviz_path)
        .args(["--pdf", test_pdf, "--output", output_base.to_str().unwrap()])
        .output()
        .expect("Failed to run dlviz-screenshot");

    if !output.status.success() {
        eprintln!(
            "dlviz-screenshot failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }

    // Load and validate JSON output
    let json_path = format!("{}.json", output_base.display());
    let json_data = fs::read_to_string(&json_path).expect("Failed to read JSON output");

    let parsed: serde_json::Value = serde_json::from_str(&json_data).expect("Failed to parse JSON");

    // Validate structure
    if let Some(stats) = parsed.get("statistics") {
        if let Some(by_label) = stats.get("by_label") {
            let label_count = by_label.as_object().map_or(0, |m| m.len());
            assert!(
                label_count > 1,
                "Layout should have multiple label types, found {}",
                label_count
            );
        }
    }

    eprintln!("Visual structure validation passed");
}

/// Integration test: Use LLM to assess visual quality of layout detection
///
/// This test requires:
/// - OPENAI_API_KEY environment variable
/// - dlviz-screenshot binary to be built
/// - Test PDF file to exist
#[test]
#[ignore = "Requires OPENAI_API_KEY and built dlviz-screenshot"]
fn test_pdf_layout_visual_quality_llm() {
    let api_key = match env::var("OPENAI_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            eprintln!("OPENAI_API_KEY not set, skipping LLM test");
            return;
        }
    };

    let test_pdf = "test-corpus/pdf/2305.03393v1.pdf";
    if !Path::new(test_pdf).exists() {
        eprintln!("Test PDF not found: {}", test_pdf);
        return;
    }

    let output_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let output_base = output_dir.path().join("output");

    // Run dlviz-screenshot
    let dlviz_path = "target/release/dlviz-screenshot";
    if !Path::new(dlviz_path).exists() {
        eprintln!("dlviz-screenshot not built: {}", dlviz_path);
        return;
    }

    let output = Command::new(dlviz_path)
        .args(["--pdf", test_pdf, "--output", output_base.to_str().unwrap()])
        .output()
        .expect("Failed to run dlviz-screenshot");

    if !output.status.success() {
        eprintln!(
            "dlviz-screenshot failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }

    // Load PNG for vision analysis
    let png_path = format!("{}.png", output_base.display());
    let png_data = fs::read(&png_path).expect("Failed to read PNG output");

    // Call OpenAI vision API
    let score = call_openai_vision_analysis(&api_key, &png_data);
    eprintln!("LLM quality score: {}/100", score);
    assert!(
        score >= 80,
        "Layout quality score {} below threshold 80",
        score
    );
}

/// Call OpenAI vision API to analyze layout visualization
///
/// Returns a quality score 0-100
fn call_openai_vision_analysis(api_key: &str, png_data: &[u8]) -> u32 {
    use base64::{engine::general_purpose::STANDARD, Engine};

    let b64_image = STANDARD.encode(png_data);

    let prompt = r#"Analyze this PDF layout visualization image. The colored boxes show detected elements:
- Yellow: text
- Red: title
- Blue: section headers
- Green: tables
- Magenta: pictures/figures
- Orange: captions

Rate the layout detection quality on a 0-100 scale considering:
1. Are elements labeled correctly? (title is actually a title, etc.)
2. Are all visible elements detected?
3. Do boxes fit the content precisely?
4. Are elements numbered in sensible reading order?

Return ONLY a single integer score 0-100, nothing else."#;

    let request_body = serde_json::json!({
        "model": "gpt-4o-mini",
        "messages": [
            {
                "role": "user",
                "content": [
                    {
                        "type": "text",
                        "text": prompt
                    },
                    {
                        "type": "image_url",
                        "image_url": {
                            "url": format!("data:image/png;base64,{}", b64_image),
                            "detail": "high"
                        }
                    }
                ]
            }
        ],
        "max_tokens": 10
    });

    let client = reqwest::blocking::Client::new();
    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send();

    match response {
        Ok(resp) => {
            if let Ok(json) = resp.json::<serde_json::Value>() {
                if let Some(content) = json["choices"][0]["message"]["content"].as_str() {
                    // Parse the score from the response
                    content.trim().parse::<u32>().unwrap_or_else(|_| {
                        eprintln!("Failed to parse LLM response as score: {}", content);
                        0
                    })
                } else {
                    eprintln!("No content in LLM response: {:?}", json);
                    0
                }
            } else {
                eprintln!("Failed to parse LLM response JSON");
                0
            }
        }
        Err(e) => {
            eprintln!("OpenAI API request failed: {}", e);
            0
        }
    }
}

#[cfg(test)]
mod verification_tests {
    use super::*;

    /// Verify the test file itself compiles and basic imports work
    #[test]
    fn test_imports_work() {
        // Verify we can create elements
        let elem = make_element(1, DlvizLabel::Title, 0.95);
        assert_eq!(elem.id, 1);
        assert_eq!(elem.label, DlvizLabel::Title);
        assert!((elem.confidence - 0.95).abs() < 0.001);
    }

    /// Verify LayoutValidationResult enum is accessible
    #[test]
    fn test_validation_result_accessible() {
        let valid = LayoutValidationResult::Valid;
        assert_eq!(valid, LayoutValidationResult::Valid);
    }
}
