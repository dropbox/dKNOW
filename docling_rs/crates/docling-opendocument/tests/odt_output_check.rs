//! Check ODT parsing output

use docling_opendocument::odt::parse_odt_file;
use std::path::Path;

#[test]
fn print_simple_text_output() {
    let path = Path::new("../../test-corpus/opendocument/odt/simple_text.odt");
    let doc = parse_odt_file(path).expect("Failed to parse");

    println!("\n=== simple_text.odt ===");
    println!("Title: {:?}", doc.title);
    println!("Author: {:?}", doc.author);
    println!("Paragraphs: {}", doc.paragraph_count);
    println!("Tables: {}", doc.table_count);
    println!("\nText output:");
    println!("{}", doc.text);
    println!("=== END ===\n");
}

#[test]
fn print_report_output() {
    let path = Path::new("../../test-corpus/opendocument/odt/report.odt");
    let doc = parse_odt_file(path).expect("Failed to parse");

    println!("\n=== report.odt ===");
    println!("Title: {:?}", doc.title);
    println!("Author: {:?}", doc.author);
    println!("Paragraphs: {}", doc.paragraph_count);
    println!("Tables: {}", doc.table_count);
    println!("\nText output:");
    println!("{}", doc.text);
    println!("=== END ===\n");
}
