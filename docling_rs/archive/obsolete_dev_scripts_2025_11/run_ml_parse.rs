#[cfg(feature = "pdf-ml")]
use docling_backend::pdf::PdfBackend;

#[cfg(feature = "pdf-ml")]
use docling_backend::traits::BackendOptions;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "pdf-ml")]
    {
        let backend = PdfBackend::new()?;
        let result = backend.parse_file_ml(
            "test-corpus/pdf/2305.03393v1-pg9.pdf",
            &BackendOptions::default()
        )?;
        
        println!("✅ SUCCESS - Parsed PDF with Rust ML");
        println!("Markdown: {} chars", result.markdown.len());
        if let Some(items) = &result.content_blocks {
            println!("DocItems: {}", items.len());
        }
        println!("\nFirst 300 chars:\n{}", &result.markdown[..300.min(result.markdown.len())]);
    }
    
    #[cfg(not(feature = "pdf-ml"))]
    {
        println!("Build with --features pdf-ml");
    }
    
    Ok(())
}
