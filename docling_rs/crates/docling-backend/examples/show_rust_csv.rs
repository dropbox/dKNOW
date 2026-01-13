use docling_backend::{BackendOptions, CsvBackend, DocumentBackend};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let csv_path = Path::new("test-corpus/csv/csv-comma.csv");

    let backend = CsvBackend::new();
    let options = BackendOptions::default();
    let document = backend.parse_file(csv_path, &options)?;

    // Print just the Rust output
    print!("{}", document.markdown);

    Ok(())
}
