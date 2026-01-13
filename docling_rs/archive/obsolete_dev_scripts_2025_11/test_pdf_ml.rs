use docling_backend::pdf::PdfBackend;
use docling_backend::traits::{BackendOptions, DocumentBackend};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    
    let backend = PdfBackend::new()?;
    let options = BackendOptions::default();
    
    let test_pdf = "test-corpus/pdf/2305.03393v1.pdf";
    println!("Testing PDF ML pipeline with: {}", test_pdf);
    
    let doc = backend.parse_file(test_pdf, &options)?;
    
    println!("✅ Success!");
    println!("  Pages: {}", doc.metadata.num_pages.unwrap_or(0));
    println!("  Chars: {}", doc.metadata.num_characters);
    println!("  DocItems: {}", doc.content_blocks.as_ref().map(|v| v.len()).unwrap_or(0));
    println!("  Markdown preview: {}", &doc.markdown[..200.min(doc.markdown.len())]);
    
    Ok(())
}
