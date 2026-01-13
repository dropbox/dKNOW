use docling_backend::{DocumentBackend, DocxBackend};
use std::fs;

fn main() {
    let backend = DocxBackend;
    let result = backend
        .parse_file("test-corpus/docx/word_sample.docx", &Default::default())
        .expect("Failed to parse DOCX");

    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize");
    fs::write("/tmp/word_sample_docitems.json", &json).expect("Failed to write JSON");
    println!("Exported to /tmp/word_sample_docitems.json");
    println!("JSON size: {} bytes", json.len());

    if let Some(blocks) = &result.content_blocks {
        println!("DocItem count: {}", blocks.len());

        // Count by type
        let mut counts = std::collections::HashMap::new();
        for item in blocks {
            let typ = match item {
                docling_core::content::DocItem::Text { .. } => "Text",
                docling_core::content::DocItem::Title { .. } => "Title",
                docling_core::content::DocItem::SectionHeader { .. } => "SectionHeader",
                docling_core::content::DocItem::ListItem { .. } => "ListItem",
                docling_core::content::DocItem::Table { .. } => "Table",
                docling_core::content::DocItem::Picture { .. } => "Picture",
                _ => "Other",
            };
            *counts.entry(typ).or_insert(0) += 1;
        }

        println!("\nDocItem types:");
        let mut types: Vec<_> = counts.iter().collect();
        types.sort_by_key(|(k, _)| *k);
        for (typ, count) in types {
            println!("  {}: {}", typ, count);
        }
    }
}
