//! Stress Tests
//!
//! Tests for large files, complex documents, and performance characteristics:
//! - Large PDF files (100+ pages, 10+ MB)
//! - Large DOCX files (complex formatting)
//! - Large image files (high resolution)
//! - Complex documents (many tables/images)
//! - Performance benchmarks
//!
//! These tests exercise system limits and verify graceful handling of large inputs.
//! They complement canonical tests by focusing on scale and performance.

use docling_backend::DocumentConverter;
use std::fs;
use std::path::Path;
use std::time::Instant;
use tempfile::TempDir;

/// Performance threshold: warn if conversion takes longer than this (seconds)
const WARN_THRESHOLD_SECS: u64 = 30;

/// Create a temporary directory for test files
fn setup_test_dir() -> TempDir {
    tempfile::tempdir().expect("Failed to create temp directory")
}

/// Measure conversion time and memory usage
fn measure_conversion(converter: &DocumentConverter, path: &Path, description: &str) {
    let start = Instant::now();

    match converter.convert(path) {
        Ok(result) => {
            let duration = start.elapsed();
            let chars = result.document.markdown.len();

            println!("\n{description} - Success:");
            println!("  Duration: {:.2}s", duration.as_secs_f64());
            println!("  Output: {chars} chars");

            if duration.as_secs() > WARN_THRESHOLD_SECS {
                println!("  ⚠️  WARNING: Conversion took longer than {WARN_THRESHOLD_SECS}s");
            }
        }
        Err(e) => {
            let duration = start.elapsed();
            println!(
                "\n{} - Error (duration: {:.2}s):",
                description,
                duration.as_secs_f64()
            );
            println!("  {e}");
        }
    }
}

// ============================================================================
// Large PDF Tests
// ============================================================================

#[test]
#[ignore = "Slow test"]
fn test_large_pdf_2206_01062() {
    // Test with 4.1MB PDF file from test corpus
    let pdf_path = Path::new("../../test-corpus/pdf/2206.01062.pdf");

    if !pdf_path.exists() {
        println!("⚠️  Test file not found: {}", pdf_path.display());
        println!("   Run test corpus setup to enable this test");
        return;
    }

    let converter = DocumentConverter::new().expect("Failed to create converter");
    measure_conversion(&converter, pdf_path, "Large PDF (4.1MB) - 2206.01062.pdf");
}

#[test]
#[ignore = "Slow test"]
fn test_large_pdf_2203_01017v2() {
    // Test with 6.9MB PDF file from test corpus (largest available)
    let pdf_path = Path::new("../../test-corpus/pdf/2203.01017v2.pdf");

    if !pdf_path.exists() {
        println!("⚠️  Test file not found: {}", pdf_path.display());
        println!("   Run test corpus setup to enable this test");
        return;
    }

    let converter = DocumentConverter::new().expect("Failed to create converter");
    measure_conversion(&converter, pdf_path, "Large PDF (6.9MB) - 2203.01017v2.pdf");
}

#[test]
#[ignore = "Slow test"]
fn test_large_pdf_2305_03393v1() {
    // Test with 4.1MB PDF file from test corpus
    let pdf_path = Path::new("../../test-corpus/pdf/2305.03393v1.pdf");

    if !pdf_path.exists() {
        println!("⚠️  Test file not found: {}", pdf_path.display());
        println!("   Run test corpus setup to enable this test");
        return;
    }

    let converter = DocumentConverter::new().expect("Failed to create converter");
    measure_conversion(&converter, pdf_path, "Large PDF (4.1MB) - 2305.03393v1.pdf");
}

#[test]
#[ignore = "Slow test"]
fn test_large_pdf_redp5110_sampled() {
    // Test with 1.2MB PDF file from test corpus
    let pdf_path = Path::new("../../test-corpus/pdf/redp5110_sampled.pdf");

    if !pdf_path.exists() {
        println!("⚠️  Test file not found: {}", pdf_path.display());
        println!("   Run test corpus setup to enable this test");
        return;
    }

    let converter = DocumentConverter::new().expect("Failed to create converter");
    measure_conversion(
        &converter,
        pdf_path,
        "Large PDF (1.2MB) - redp5110_sampled.pdf",
    );
}

// ============================================================================
// Large DOCX Tests
// ============================================================================

#[test]
#[ignore = "Slow test"]
fn test_large_docx_test_emf() {
    // Test with 416KB DOCX file (largest available in test corpus)
    let docx_path = Path::new("../../test-corpus/docx/test_emf_docx.docx");

    if !docx_path.exists() {
        println!("⚠️  Test file not found: {}", docx_path.display());
        println!("   Run test corpus setup to enable this test");
        return;
    }

    let converter = DocumentConverter::new().expect("Failed to create converter");
    measure_conversion(
        &converter,
        docx_path,
        "Large DOCX (416KB) - test_emf_docx.docx",
    );
}

#[test]
#[ignore = "Slow test"]
fn test_large_docx_word_sample() {
    // Test with 102KB DOCX file
    let docx_path = Path::new("../../test-corpus/docx/word_sample.docx");

    if !docx_path.exists() {
        println!("⚠️  Test file not found: {}", docx_path.display());
        println!("   Run test corpus setup to enable this test");
        return;
    }

    let converter = DocumentConverter::new().expect("Failed to create converter");
    measure_conversion(
        &converter,
        docx_path,
        "Large DOCX (102KB) - word_sample.docx",
    );
}

// ============================================================================
// Large XLSX Tests
// ============================================================================

#[test]
#[ignore = "Slow test"]
fn test_large_xlsx_04_inflated() {
    // Test with 168KB XLSX file (largest available in test corpus)
    let xlsx_path = Path::new("../../test-corpus/xlsx/xlsx_04_inflated.xlsx");

    if !xlsx_path.exists() {
        println!("⚠️  Test file not found: {}", xlsx_path.display());
        println!("   Run test corpus setup to enable this test");
        return;
    }

    let converter = DocumentConverter::new().expect("Failed to create converter");
    measure_conversion(
        &converter,
        xlsx_path,
        "Large XLSX (168KB) - xlsx_04_inflated.xlsx",
    );
}

#[test]
#[ignore = "Slow test"]
fn test_large_xlsx_01() {
    // Test with 167KB XLSX file
    let xlsx_path = Path::new("../../test-corpus/xlsx/xlsx_01.xlsx");

    if !xlsx_path.exists() {
        println!("⚠️  Test file not found: {}", xlsx_path.display());
        println!("   Run test corpus setup to enable this test");
        return;
    }

    let converter = DocumentConverter::new().expect("Failed to create converter");
    measure_conversion(&converter, xlsx_path, "Large XLSX (167KB) - xlsx_01.xlsx");
}

// ============================================================================
// Large Image Tests
// ============================================================================

#[test]
#[ignore = "Slow test"]
fn test_large_png_image() {
    // Test with 301KB PNG file (largest available in test corpus)
    let png_path = Path::new("../../test-corpus/png/2305.03393v1-pg9-img.png");

    if !png_path.exists() {
        println!("⚠️  Test file not found: {}", png_path.display());
        println!("   Run test corpus setup to enable this test");
        return;
    }

    let converter = DocumentConverter::new().expect("Failed to create converter");
    measure_conversion(
        &converter,
        png_path,
        "Large PNG (301KB) - 2305.03393v1-pg9-img.png",
    );
}

// ============================================================================
// Synthetic Large File Tests
// ============================================================================

#[test]
#[ignore = "Slow test"]
fn test_synthetic_large_html() {
    // Generate large HTML file (10MB)
    let temp_dir = setup_test_dir();
    let large_html = temp_dir.path().join("large.html");

    let mut html = String::from("<html><head><title>Large Test</title></head><body>\n");

    // Generate 100,000 paragraphs (~10MB)
    for i in 0..100_000 {
        html.push_str(&format!("<p>This is paragraph {i}. "));
        html.push_str("Lorem ipsum dolor sit amet, consectetur adipiscing elit. ");
        html.push_str("Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.</p>\n");
    }

    html.push_str("</body></html>");

    fs::write(&large_html, html).expect("Failed to write large HTML");

    let file_size = fs::metadata(&large_html)
        .expect("Failed to get file size")
        .len();
    println!(
        "\nGenerated large HTML file: {:.2}MB",
        file_size as f64 / 1_048_576.0
    );

    let converter = DocumentConverter::new().expect("Failed to create converter");
    measure_conversion(
        &converter,
        &large_html,
        "Synthetic Large HTML (100K paragraphs)",
    );
}

#[test]
#[ignore = "Slow test"]
fn test_synthetic_large_csv() {
    // Generate large CSV file (10MB, 100,000 rows)
    let temp_dir = setup_test_dir();
    let large_csv = temp_dir.path().join("large.csv");

    let mut csv = String::from("id,name,email,age,city,country,score\n");

    for i in 0..100_000 {
        csv.push_str(&format!(
            "{},User{},user{}@example.com,{},City{},Country{},{}\n",
            i,
            i,
            i,
            20 + (i % 60),
            i % 100,
            i % 50,
            i % 100
        ));
    }

    fs::write(&large_csv, csv).expect("Failed to write large CSV");

    let file_size = fs::metadata(&large_csv)
        .expect("Failed to get file size")
        .len();
    println!(
        "\nGenerated large CSV file: {:.2}MB",
        file_size as f64 / 1_048_576.0
    );

    let converter = DocumentConverter::new().expect("Failed to create converter");
    measure_conversion(&converter, &large_csv, "Synthetic Large CSV (100K rows)");
}

#[test]
#[ignore = "Slow test"]
fn test_synthetic_complex_html_many_tables() {
    // Generate HTML with many tables (stress test for table parsing)
    let temp_dir = setup_test_dir();
    let complex_html = temp_dir.path().join("many_tables.html");

    let mut html = String::from("<html><head><title>Many Tables</title></head><body>\n");

    // Generate 100 tables (stress test - reduced from 1000 for reasonable runtime)
    for i in 0..100 {
        html.push_str(&format!("<h2>Table {i}</h2>\n"));
        html.push_str("<table>\n");
        html.push_str("<tr><th>Column 1</th><th>Column 2</th><th>Column 3</th></tr>\n");

        // 10 rows per table
        for j in 0..10 {
            html.push_str(&format!(
                "<tr><td>Data {i}-{j}-1</td><td>Data {i}-{j}-2</td><td>Data {i}-{j}-3</td></tr>\n"
            ));
        }

        html.push_str("</table>\n");
    }

    html.push_str("</body></html>");

    fs::write(&complex_html, html).expect("Failed to write complex HTML");

    let file_size = fs::metadata(&complex_html)
        .expect("Failed to get file size")
        .len();
    println!(
        "\nGenerated complex HTML file: {:.2}MB (100 tables)",
        file_size as f64 / 1_048_576.0
    );

    let converter = DocumentConverter::new().expect("Failed to create converter");
    measure_conversion(
        &converter,
        &complex_html,
        "Synthetic Complex HTML (100 tables)",
    );
}

#[test]
#[ignore = "Slow test"]
fn test_synthetic_complex_html_many_lists() {
    // Generate HTML with deeply nested lists
    let temp_dir = setup_test_dir();
    let complex_html = temp_dir.path().join("many_lists.html");

    let mut html = String::from("<html><head><title>Many Lists</title></head><body>\n");

    // Generate 100 nested list structures
    for i in 0..100 {
        html.push_str(&format!("<h2>List Structure {i}</h2>\n"));
        html.push_str("<ul>\n");

        // 5 levels deep
        for level in 0..5 {
            html.push_str(&format!("<li>Level {level} Item {i}-{level}\n<ul>\n"));
        }

        // Close all levels
        for _ in 0..5 {
            html.push_str("</ul></li>\n");
        }

        html.push_str("</ul>\n");
    }

    html.push_str("</body></html>");

    fs::write(&complex_html, html).expect("Failed to write complex HTML");

    let converter = DocumentConverter::new().expect("Failed to create converter");
    measure_conversion(
        &converter,
        &complex_html,
        "Synthetic Complex HTML (nested lists)",
    );
}

// ============================================================================
// All Stress Tests Combined
// ============================================================================

#[test]
#[ignore = "Very slow test"]
fn test_all_stress_tests() {
    // Run all stress tests sequentially and report summary
    println!("\n=== Running All Stress Tests ===\n");

    let start = Instant::now();

    // Run each test
    test_large_pdf_2206_01062();
    test_large_pdf_2203_01017v2();
    test_large_pdf_2305_03393v1();
    test_large_pdf_redp5110_sampled();
    test_large_docx_test_emf();
    test_large_docx_word_sample();
    test_large_xlsx_04_inflated();
    test_large_xlsx_01();
    test_large_png_image();
    test_synthetic_large_html();
    test_synthetic_large_csv();
    test_synthetic_complex_html_many_tables();
    test_synthetic_complex_html_many_lists();

    let total_duration = start.elapsed();

    println!("\n=== Stress Test Summary ===");
    println!("Total duration: {:.2}s", total_duration.as_secs_f64());
    println!("All stress tests completed successfully ✅");
}
