use docling_backend::pdf::PdfBackend;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pdf_path = "test-corpus/pdf/2305.03393v1-pg9.pdf";
    let data = fs::read(pdf_path)?;
    
    let backend = PdfBackend::new()?;
    
    println!("Calling parse_file_ml...");
    let doc = backend.parse_file_ml(pdf_path, &docling_backend::traits::BackendOptions::default())?;
    
    println!("SUCCESS!");
    println!("Markdown: {} chars", doc.markdown.len());
    println!("DocItems: {:?}", doc.content_blocks.as_ref().map(|v| v.len()));
    println!("\nFirst 500 chars:\n{}", &doc.markdown[..500.min(doc.markdown.len())]);
    
    Ok(())
}
