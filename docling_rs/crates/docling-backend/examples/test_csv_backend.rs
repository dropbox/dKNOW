use docling_backend::{BackendOptions, CsvBackend, DocumentBackend};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let csv_path = Path::new("test-corpus/csv/csv-comma.csv");
    let expected_path = Path::new("test-corpus/groundtruth/docling_v2/csv-comma.csv.md");

    println!("🧪 Testing CSV Backend\n");
    println!("Input: {}", csv_path.display());

    // Parse CSV
    let backend = CsvBackend::new();
    let options = BackendOptions::default();
    let document = backend.parse_file(csv_path, &options)?;

    // Get markdown from document
    let markdown = &document.markdown;

    println!("✅ Parsed successfully");
    println!("   Format: {:?}", document.format);
    println!("   Markdown output: {} chars", markdown.len());

    // Read expected output
    let expected = std::fs::read_to_string(expected_path)?;
    println!("\n📋 Expected output: {} chars", expected.len());

    // Compare
    if markdown == &expected {
        println!("\n✅ PASS: Output matches expected!");
        Ok(())
    } else {
        println!("\n❌ FAIL: Output differs from expected");
        println!("\n--- First 500 chars of actual ---");
        println!("{}", &markdown.chars().take(500).collect::<String>());
        println!("\n--- First 500 chars of expected ---");
        println!("{}", &expected.chars().take(500).collect::<String>());

        // Character-level diff summary
        let min_len = markdown.len().min(expected.len());
        let mut first_diff = None;
        for (i, (a, b)) in markdown.chars().zip(expected.chars()).enumerate() {
            if a != b {
                first_diff = Some(i);
                break;
            }
        }

        if let Some(pos) = first_diff {
            println!("\n❗ First difference at position {pos}");
            let context_start = pos.saturating_sub(50);
            let context_end = (pos + 50).min(min_len);
            println!(
                "   Context: ...{}...",
                &markdown[context_start..context_end]
            );
        }

        Err("Output mismatch".into())
    }
}
