//! Dump KML to JSON for inspection

use docling_backend::{DocumentBackend, KmlBackend};
use docling_core::InputFormat;

fn main() {
    let backend = KmlBackend::new(InputFormat::Kml);
    let result = backend
        .parse_file("test-corpus/gps/kml/hiking_path.kml", &Default::default())
        .expect("Failed to parse KML");

    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize to JSON");
    eprintln!("JSON length: {} chars", json.len());
    eprintln!("Has content_blocks: {}", result.content_blocks.is_some());

    if let Some(blocks) = &result.content_blocks {
        eprintln!("Number of DocItems: {}", blocks.len());
    }

    println!("{json}");
}
