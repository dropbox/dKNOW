//! Property-Based Tests
//!
//! Tests using property-based testing (proptest) to verify invariants:
//! - Serialization doesn't panic with arbitrary inputs
//! - Data structures maintain invariants
//! - Boundary conditions handled correctly
//!
//! These tests complement unit tests by exploring the input space automatically.

use docling_core::content::BoundingBox;
use docling_core::document::Document;
use docling_core::format::InputFormat;
use docling_core::serializer::JsonSerializer;
use proptest::prelude::*;

// ============================================================================
// JSON Serialization Properties
// ============================================================================

/// Property: Any markdown document should serialize to JSON without panic
#[test]
fn proptest_json_no_panic() {
    proptest!(|(text in ".*{0,500}")| {
        let doc = Document::from_markdown(text, InputFormat::Md);
        let serializer = JsonSerializer::new();

        // Should not panic
        let result = serializer.serialize_document(&doc);
        prop_assert!(result.is_ok(), "JSON serialization should not fail");

        // Result should be valid JSON
        if let Ok(json_str) = result {
            let parsed: Result<serde_json::Value, _> = serde_json::from_str(&json_str);
            prop_assert!(parsed.is_ok(), "Result should be valid JSON");
        }
    });
}

/// Property: Empty document produces valid JSON
#[test]
fn proptest_empty_document_json() {
    proptest!(|(_ in 0u8..10u8)| {
        let doc = Document::from_markdown(String::new(), InputFormat::Md);
        let serializer = JsonSerializer::new();

        let json = serializer.serialize_document(&doc);
        prop_assert!(json.is_ok(), "Empty document should serialize");
    });
}

/// Property: Unicode text should be handled
#[test]
fn proptest_unicode_handling() {
    proptest!(|(text in "\\PC{0,200}")| {
        let doc = Document::from_markdown(text, InputFormat::Md);
        let serializer = JsonSerializer::new();

        let json = serializer.serialize_document(&doc);
        prop_assert!(json.is_ok(), "Should handle Unicode");
    });
}

/// Property: Large documents should not panic
#[test]
fn proptest_large_document() {
    proptest!(ProptestConfig::with_cases(10), |(text in ".*{5000,10000}")| {
        let doc = Document::from_markdown(text, InputFormat::Md);
        let serializer = JsonSerializer::new();

        let json = serializer.serialize_document(&doc);
        prop_assert!(json.is_ok(), "Should handle large documents");
    });
}

// ============================================================================
// BoundingBox Properties
// ============================================================================

/// Property: BoundingBox should maintain valid coordinates
#[test]
fn proptest_bounding_box_validity() {
    proptest!(|(
        l in 0.0f64..1000.0,
        t in 0.0f64..1000.0,
        r in 0.0f64..1000.0,
        b in 0.0f64..1000.0
    )| {
        let (left, right) = if l <= r { (l, r) } else { (r, l) };
        let (top, bottom) = if t <= b { (t, b) } else { (b, t) };

        let bbox = BoundingBox {
            l: left,
            t: top,
            r: right,
            b: bottom,
            coord_origin: docling_core::content::CoordOrigin::BottomLeft,
        };

        prop_assert!(bbox.r >= bbox.l, "Right should be >= left");
        prop_assert!(bbox.b >= bbox.t, "Bottom should be >= top");

        let json = serde_json::to_string(&bbox);
        prop_assert!(json.is_ok(), "Should serialize BoundingBox");
    });
}

/// Property: Extreme coordinates should be handled
#[test]
fn proptest_extreme_coordinates() {
    proptest!(|(
        x in -10000.0f64..10000.0,
        y in -10000.0f64..10000.0,
        w in 0.0f64..10000.0,
        h in 0.0f64..10000.0
    )| {
        let bbox = BoundingBox {
            l: x,
            t: y,
            r: x + w,
            b: y + h,
            coord_origin: docling_core::content::CoordOrigin::BottomLeft,
        };

        let json = serde_json::to_string(&bbox);
        prop_assert!(json.is_ok(), "Should handle extreme coordinates");
        prop_assert!(bbox.r >= bbox.l, "Right >= left");
        prop_assert!(bbox.b >= bbox.t, "Bottom >= top");
    });
}

/// Property: Special markdown characters don't cause panics
#[test]
fn proptest_markdown_special_chars() {
    proptest!(|(text in "[a-zA-Z0-9_\\[\\]\\(\\)\\*\\-\\#\\.\\!\\s\\n]{0,300}")| {
        let doc = Document::from_markdown(text, InputFormat::Md);
        let serializer = JsonSerializer::new();

        let result = serializer.serialize_document(&doc);
        prop_assert!(result.is_ok(), "Should handle special markdown characters");
    });
}

/// Property: Whitespace variations don't cause panics
#[test]
fn proptest_whitespace_handling() {
    proptest!(|(
        text in ".*{0,200}",
        leading_spaces in 0usize..20,
        trailing_spaces in 0usize..20
    )| {
        let text_with_spaces = format!("{}{}{}",
            " ".repeat(leading_spaces),
            text,
            " ".repeat(trailing_spaces)
        );

        let doc = Document::from_markdown(text_with_spaces, InputFormat::Md);
        let serializer = JsonSerializer::new();

        let result = serializer.serialize_document(&doc);
        prop_assert!(result.is_ok(), "Should handle whitespace variations");
    });
}
