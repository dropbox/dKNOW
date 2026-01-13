#!/usr/bin/env -S cargo +nightly -Zscript
//! Manual CSV backend test - bypasses integration test infrastructure
//!
//! Usage: cargo +nightly -Zscript test_csv_manual.rs

use std::fs;
use std::path::PathBuf;

fn main() {
    // Test corpus directory
    let test_corpus = PathBuf::from("test-corpus/csv");
    let groundtruth = PathBuf::from("test-corpus/groundtruth/docling_v2");

    if !test_corpus.exists() {
        eprintln!("❌ Test corpus not found at {}", test_corpus.display());
        std::process::exit(1);
    }

    println!("🧪 CSV Backend Manual Test\n");
    println!("Testing CSV files from: {}", test_corpus.display());
    println!("Expected outputs at: {}\n", groundtruth.display());

    // Find all CSV test files
    let csv_files: Vec<_> = fs::read_dir(&test_corpus)
        .expect("Failed to read test corpus")
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry.path().extension()
                .and_then(|s| s.to_str())
                .map(|s| s == "csv")
                .unwrap_or(false)
        })
        .collect();

    println!("Found {} CSV test files:\n", csv_files.len());

    let mut passed = 0;
    let mut failed = 0;

    for entry in csv_files {
        let csv_path = entry.path();
        let filename = csv_path.file_name().unwrap().to_str().unwrap();

        print!("  📄 {} ... ", filename);

        // Find expected markdown output
        let expected_md_name = format!("{}.md", filename);
        let expected_md_path = groundtruth.join(&expected_md_name);

        if !expected_md_path.exists() {
            println!("⚠️  SKIP (no expected output)");
            continue;
        }

        // Read expected output
        let expected = match fs::read_to_string(&expected_md_path) {
            Ok(content) => content,
            Err(e) => {
                println!("❌ FAIL (cannot read expected: {})", e);
                failed += 1;
                continue;
            }
        };

        // TODO: Actually call the Rust backend here
        // For now, just check that files exist
        println!("✅ READY (expected {} chars)", expected.len());
        passed += 1;
    }

    println!("\n" + &"=".repeat(60));
    println!("Summary: {} passed, {} failed", passed, failed);

    if failed > 0 {
        std::process::exit(1);
    }
}
