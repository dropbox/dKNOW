# API Cookbook - Common Usage Patterns

This cookbook provides practical examples of common tasks using the docling-core API.

## Table of Contents

1. [Basic Conversion](#basic-conversion)
2. [Batch Processing](#batch-processing)
3. [OCR Processing](#ocr-processing)
4. [Error Handling](#error-handling)
5. [Custom Serialization](#custom-serialization)
6. [Format Detection](#format-detection)
7. [Metadata Extraction](#metadata-extraction)
8. [Streaming API](#streaming-api)
9. [Performance Optimization](#performance-optimization)
10. [Integration Patterns](#integration-patterns)
11. [Testing Patterns](#testing-patterns)
12. [Production Deployment](#production-deployment)

---

## Basic Conversion

### Convert a Single Document

```rust
// Note: DocumentConverter is in docling-backend crate
use docling_backend::DocumentConverter;
use docling_core::Result;

fn convert_document(path: &str) -> Result<String> {
    let converter = DocumentConverter::new()?;
    let result = converter.convert(path)?;
    Ok(result.document.markdown)
}

fn main() -> Result<()> {
    let markdown = convert_document("document.pdf")?;
    println!("{}", markdown);
    Ok(())
}
```

### Convert and Save to File

```rust
// Note: DocumentConverter is in docling-backend crate
use docling_backend::DocumentConverter;
use docling_core::Result;
use std::fs;

fn convert_and_save(input: &str, output: &str) -> Result<()> {
    let converter = DocumentConverter::new()?;
    let result = converter.convert(input)?;

    fs::write(output, result.document.markdown)?;
    println!("Saved to {}", output);

    Ok(())
}

fn main() -> Result<()> {
    convert_and_save("report.pdf", "report.md")?;
    Ok(())
}
```

### Convert Multiple Formats

```rust
// Note: DocumentConverter is in docling-backend crate
use docling_backend::DocumentConverter;
use docling_core::Result;

fn convert_various_formats() -> Result<()> {
    let converter = DocumentConverter::new()?;

    // PDF
    let pdf = converter.convert("document.pdf")?;
    println!("PDF: {} chars", pdf.document.metadata.num_characters);

    // Word document
    let docx = converter.convert("letter.docx")?;
    println!("DOCX: {} chars", docx.document.metadata.num_characters);

    // HTML
    let html = converter.convert("webpage.html")?;
    println!("HTML: {} chars", html.document.metadata.num_characters);

    Ok(())
}
```

---

## Batch Processing

### Process Multiple Files

```rust
// Note: DocumentConverter is in docling-backend crate
use docling_backend::DocumentConverter;
use docling_core::Result;
use std::path::Path;

fn batch_convert(files: &[&str]) -> Result<Vec<String>> {
    let converter = DocumentConverter::new()?;
    let mut results = Vec::new();

    for file in files {
        match converter.convert(file) {
            Ok(result) => {
                println!("✓ {}: {} chars", file, result.document.metadata.num_characters);
                results.push(result.document.markdown);
            }
            Err(e) => {
                eprintln!("✗ {}: {}", file, e);
            }
        }
    }

    Ok(results)
}

fn main() -> Result<()> {
    let files = vec!["doc1.pdf", "doc2.docx", "doc3.html"];
    let markdowns = batch_convert(&files)?;
    println!("Converted {} files", markdowns.len());
    Ok(())
}
```

### Process Directory Recursively

```rust
// Note: DocumentConverter is in docling-backend crate
use docling_backend::DocumentConverter;
use docling_core::Result;
use std::fs;
use std::path::Path;

fn process_directory(dir: &Path, output_dir: &Path) -> Result<usize> {
    let converter = DocumentConverter::new()?;
    let mut count = 0;

    fs::create_dir_all(output_dir)?;

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension() {
                if matches!(ext.to_str(), Some("pdf") | Some("docx") | Some("html")) {
                    match converter.convert(&path) {
                        Ok(result) => {
                            let output_path = output_dir.join(
                                path.file_stem().unwrap()
                            ).with_extension("md");

                            fs::write(&output_path, result.document.markdown)?;
                            println!("✓ {:?} -> {:?}", path, output_path);
                            count += 1;
                        }
                        Err(e) => {
                            eprintln!("✗ {:?}: {}", path, e);
                        }
                    }
                }
            }
        }
    }

    Ok(count)
}

fn main() -> Result<()> {
    let count = process_directory(
        Path::new("input_docs"),
        Path::new("output_markdown")
    )?;
    println!("Converted {} files", count);
    Ok(())
}
```

### Parallel Batch Processing

```rust
// Note: DocumentConverter is in docling-backend crate
use docling_backend::DocumentConverter;
use docling_core::Result;
use std::sync::{Arc, Mutex};
use std::thread;

fn parallel_convert(files: Vec<String>) -> Result<Vec<String>> {
    let results = Arc::new(Mutex::new(Vec::new()));
    let mut handles = vec![];

    for file in files {
        let results = Arc::clone(&results);

        let handle = thread::spawn(move || {
            // Each thread gets its own converter
            let converter = DocumentConverter::new().unwrap();

            match converter.convert(&file) {
                Ok(result) => {
                    results.lock().unwrap().push(result.document.markdown);
                    println!("✓ {}", file);
                }
                Err(e) => {
                    eprintln!("✗ {}: {}", file, e);
                }
            }
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let results = results.lock().unwrap().clone();
    Ok(results)
}

fn main() -> Result<()> {
    let files = vec![
        "doc1.pdf".to_string(),
        "doc2.pdf".to_string(),
        "doc3.pdf".to_string(),
    ];

    let markdowns = parallel_convert(files)?;
    println!("Converted {} files", markdowns.len());
    Ok(())
}
```

---

## OCR Processing

### Convert Scanned PDF

```rust
// Note: DocumentConverter is in docling-backend crate
use docling_backend::DocumentConverter;
use docling_core::Result;

fn convert_scanned_pdf(path: &str) -> Result<String> {
    // Enable OCR for scanned documents
    let converter = DocumentConverter::with_ocr(true)?;
    let result = converter.convert(path)?;

    println!("Extracted {} characters", result.document.metadata.num_characters);
    println!("Processing took {:?}", result.latency);

    Ok(result.document.markdown)
}

fn main() -> Result<()> {
    let text = convert_scanned_pdf("scanned_invoice.pdf")?;
    println!("{}", text);
    Ok(())
}
```

### Convert Images with OCR

```rust
// Note: DocumentConverter is in docling-backend crate
use docling_backend::DocumentConverter;
use docling_core::Result;

fn extract_text_from_images(images: &[&str]) -> Result<Vec<String>> {
    let converter = DocumentConverter::with_ocr(true)?;
    let mut texts = Vec::new();

    for image in images {
        match converter.convert(image) {
            Ok(result) => {
                println!("✓ {}: {} chars", image, result.document.metadata.num_characters);
                texts.push(result.document.markdown);
            }
            Err(e) => {
                eprintln!("✗ {}: {}", image, e);
            }
        }
    }

    Ok(texts)
}

fn main() -> Result<()> {
    let images = vec!["scan1.png", "scan2.jpg", "scan3.tiff"];
    let texts = extract_text_from_images(&images)?;
    println!("Extracted text from {} images", texts.len());
    Ok(())
}
```

### Conditional OCR Based on Content

```rust
// Note: DocumentConverter is in docling-backend crate
use docling_backend::DocumentConverter;
use docling_core::Result;

fn smart_convert(path: &str) -> Result<String> {
    // Try without OCR first (faster)
    let converter = DocumentConverter::new()?;
    let result = converter.convert(path)?;

    // Check if we got meaningful content
    if result.document.metadata.num_characters < 100 {
        println!("Low character count, retrying with OCR...");

        // Retry with OCR
        let converter_ocr = DocumentConverter::with_ocr(true)?;
        let result_ocr = converter_ocr.convert(path)?;

        return Ok(result_ocr.document.markdown);
    }

    Ok(result.document.markdown)
}

fn main() -> Result<()> {
    let text = smart_convert("document.pdf")?;
    println!("{}", text);
    Ok(())
}
```

---

## Error Handling

### Pattern Matching on Error Types

```rust
// Note: DocumentConverter is in docling-backend crate
use docling_backend::DocumentConverter;
use docling_core::{DoclingError, Result};

fn convert_with_detailed_errors(path: &str) -> Result<String> {
    let converter = DocumentConverter::new()?;

    match converter.convert(path) {
        Ok(result) => Ok(result.document.markdown),
        Err(DoclingError::IoError(e)) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                eprintln!("File not found: {}", path);
                Err(DoclingError::IoError(e))
            } else {
                eprintln!("IO error: {}", e);
                Err(DoclingError::IoError(e))
            }
        }
        Err(DoclingError::FormatError(msg)) => {
            eprintln!("Unsupported format: {}", msg);
            Err(DoclingError::FormatError(msg))
        }
        Err(DoclingError::ConversionError(msg)) => {
            eprintln!("Conversion failed: {}", msg);
            Err(DoclingError::ConversionError(msg))
        }
        Err(e) => {
            eprintln!("Unexpected error: {}", e);
            Err(e)
        }
    }
}

fn main() -> Result<()> {
    convert_with_detailed_errors("document.pdf")?;
    Ok(())
}
```

### Graceful Degradation

```rust
// Note: DocumentConverter is in docling-backend crate
use docling_backend::DocumentConverter;
use docling_core::DoclingError;

fn convert_with_fallback(path: &str) -> String {
    let converter = match DocumentConverter::new() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to create converter: {}", e);
            return String::from("Error: Converter initialization failed");
        }
    };

    match converter.convert(path) {
        Ok(result) => result.document.markdown,
        Err(DoclingError::FormatError(msg)) => {
            format!("Unsupported format: {}", msg)
        }
        Err(e) => {
            format!("Conversion error: {}", e)
        }
    }
}

fn main() {
    let result = convert_with_fallback("document.pdf");
    println!("{}", result);
}
```

### Retry Logic

```rust
// Note: DocumentConverter is in docling-backend crate
use docling_backend::DocumentConverter;
use docling_core::Result;
use std::thread;
use std::time::Duration;

fn convert_with_retry(path: &str, max_retries: usize) -> Result<String> {
    let converter = DocumentConverter::new()?;

    for attempt in 1..=max_retries {
        match converter.convert(path) {
            Ok(result) => {
                return Ok(result.document.markdown);
            }
            Err(e) => {
                eprintln!("Attempt {}/{} failed: {}", attempt, max_retries, e);

                if attempt < max_retries {
                    println!("Retrying in 2 seconds...");
                    thread::sleep(Duration::from_secs(2));
                } else {
                    return Err(e);
                }
            }
        }
    }

    unreachable!()
}

fn main() -> Result<()> {
    let markdown = convert_with_retry("document.pdf", 3)?;
    println!("{}", markdown);
    Ok(())
}
```

---

## Custom Serialization

### Custom Markdown Options

```rust
use docling_core::{MarkdownSerializer, MarkdownOptions};

fn create_custom_serializer() -> MarkdownSerializer {
    let options = MarkdownOptions {
        indent: 2,                    // 2-space indentation
        escape_underscores: false,    // Don't escape underscores
        escape_html: true,            // Escape HTML
        ..Default::default()          // Use defaults for other fields
    };

    MarkdownSerializer::with_options(options)
}

fn main() {
    let serializer = create_custom_serializer();
    println!("Created custom serializer");
}
```

---

## Format Detection

### Detect Format from Extension

```rust
// Note: DocumentConverter is in docling-backend crate
use docling_backend::DocumentConverter;
use docling_core::{InputFormat, Result};
use std::path::Path;

fn detect_and_convert(path: &Path) -> Result<String> {
    let ext = path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    let format = InputFormat::from_extension(ext);

    match format {
        Some(fmt) => {
            println!("Detected format: {:?}", fmt);
            let converter = DocumentConverter::new()?;
            let result = converter.convert(path)?;
            Ok(result.document.markdown)
        }
        None => {
            Err(docling_core::DoclingError::FormatError(
                format!("Unknown format: {}", ext)
            ))
        }
    }
}

fn main() -> Result<()> {
    let markdown = detect_and_convert(Path::new("document.pdf"))?;
    println!("{}", markdown);
    Ok(())
}
```

---

## Metadata Extraction

### Extract Document Metadata

```rust
// Note: DocumentConverter is in docling-backend crate
use docling_backend::DocumentConverter;
use docling_core::Result;

fn extract_metadata(path: &str) -> Result<()> {
    let converter = DocumentConverter::new()?;
    let result = converter.convert(path)?;
    let metadata = &result.document.metadata;

    println!("=== Document Metadata ===");
    println!("Characters: {}", metadata.num_characters);

    if let Some(pages) = metadata.num_pages {
        println!("Pages: {}", pages);
    }

    if let Some(title) = &metadata.title {
        println!("Title: {}", title);
    }

    if let Some(author) = &metadata.author {
        println!("Author: {}", author);
    }

    if let Some(created) = metadata.created {
        println!("Created: {}", created);
    }

    if let Some(modified) = metadata.modified {
        println!("Modified: {}", modified);
    }

    if let Some(language) = &metadata.language {
        println!("Language: {}", language);
    }

    Ok(())
}

fn main() -> Result<()> {
    extract_metadata("document.pdf")?;
    Ok(())
}
```

---

## Streaming API

### Process Large Batch with Iterator

```rust
// Note: DocumentConverter is in docling-backend crate
use docling_backend::DocumentConverter;
use docling_core::{ConversionConfig, Result};
use std::path::PathBuf;

fn stream_convert(files: Vec<PathBuf>) -> Result<()> {
    let converter = DocumentConverter::new()?;

    let config = ConversionConfig {
        paths: files,
        enable_ocr: false,
    };

    // Process as iterator (memory efficient)
    for (i, result) in converter.convert_all(config).enumerate() {
        match result {
            Ok(doc) => {
                println!("#{}: {} chars", i + 1, doc.metadata.num_characters);
            }
            Err(e) => {
                eprintln!("#{}: Error: {}", i + 1, e);
            }
        }
    }

    Ok(())
}

fn main() -> Result<()> {
    let files = vec![
        PathBuf::from("doc1.pdf"),
        PathBuf::from("doc2.pdf"),
        PathBuf::from("doc3.pdf"),
    ];

    stream_convert(files)?;
    Ok(())
}
```

---

## Performance Optimization

### Reuse Converter Instance

```rust
// Note: DocumentConverter is in docling-backend crate
use docling_backend::DocumentConverter;
use docling_core::Result;

fn optimize_batch_conversion(files: &[&str]) -> Result<Vec<String>> {
    // Create converter once and reuse
    let converter = DocumentConverter::new()?;
    let mut results = Vec::with_capacity(files.len());

    for file in files {
        let result = converter.convert(file)?;
        results.push(result.document.markdown);
    }

    Ok(results)
}

fn main() -> Result<()> {
    let files = vec!["doc1.pdf", "doc2.pdf", "doc3.pdf"];
    let markdowns = optimize_batch_conversion(&files)?;
    println!("Converted {} files", markdowns.len());
    Ok(())
}
```

### Measure Conversion Time

```rust
// Note: DocumentConverter is in docling-backend crate
use docling_backend::DocumentConverter;
use docling_core::Result;
use std::time::Instant;

fn benchmark_conversion(path: &str) -> Result<()> {
    let converter = DocumentConverter::new()?;

    let start = Instant::now();
    let result = converter.convert(path)?;
    let duration = start.elapsed();

    println!("=== Performance Metrics ===");
    println!("File: {}", path);
    println!("Characters: {}", result.document.metadata.num_characters);
    println!("Conversion time: {:?}", duration);
    println!("Reported latency: {:?}", result.latency);

    let chars_per_sec = result.document.metadata.num_characters as f64
        / duration.as_secs_f64();
    println!("Throughput: {:.0} chars/sec", chars_per_sec);

    Ok(())
}

fn main() -> Result<()> {
    benchmark_conversion("large_document.pdf")?;
    Ok(())
}
```

---

## Integration Patterns

### REST API Endpoint

```rust
// Note: DocumentConverter is in docling-backend crate
use docling_backend::DocumentConverter;
use docling_core::Result;
use std::fs;
use std::path::Path;

struct ConversionRequest {
    file_path: String,
    enable_ocr: bool,
}

struct ConversionResponse {
    markdown: String,
    char_count: usize,
    duration_ms: u64,
}

fn handle_conversion_request(req: ConversionRequest) -> Result<ConversionResponse> {
    let converter = if req.enable_ocr {
        DocumentConverter::with_ocr(true)?
    } else {
        DocumentConverter::new()?
    };

    let result = converter.convert(&req.file_path)?;

    Ok(ConversionResponse {
        markdown: result.document.markdown.clone(),
        char_count: result.document.metadata.num_characters,
        duration_ms: result.latency.as_millis() as u64,
    })
}

fn main() -> Result<()> {
    let req = ConversionRequest {
        file_path: "document.pdf".to_string(),
        enable_ocr: false,
    };

    let resp = handle_conversion_request(req)?;
    println!("Converted {} chars in {}ms", resp.char_count, resp.duration_ms);
    Ok(())
}
```

### Command-Line Tool

```rust
// Note: DocumentConverter is in docling-backend crate
use docling_backend::DocumentConverter;
use docling_core::Result;
use std::env;
use std::fs;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        eprintln!("Usage: {} <input> <output>", args[0]);
        std::process::exit(1);
    }

    let input = &args[1];
    let output = &args[2];

    println!("Converting {} to {}", input, output);

    let converter = DocumentConverter::new()?;
    let result = converter.convert(input)?;

    fs::write(output, result.document.markdown)?;

    println!("✓ Converted {} characters in {:?}",
        result.document.metadata.num_characters,
        result.latency
    );

    Ok(())
}
```

---

## Testing Patterns

### Mock Converter for Tests

```rust
use docling_core::{Document, InputFormat, DocumentMetadata};

fn create_mock_document(content: &str) -> Document {
    Document::from_markdown(content.to_string(), InputFormat::Markdown)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_document() {
        let doc = create_mock_document("# Test\n\nContent");

        assert_eq!(doc.to_markdown(), "# Test\n\nContent");
        assert_eq!(doc.format, InputFormat::Markdown);
        assert!(doc.metadata.num_characters > 0);
    }
}
```

---

## Production Deployment

### Graceful Shutdown Handler

```rust
// Note: DocumentConverter is in docling-backend crate
use docling_backend::DocumentConverter;
use docling_core::Result;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

fn conversion_worker(shutdown: Arc<AtomicBool>) -> Result<()> {
    let converter = DocumentConverter::new()?;

    while !shutdown.load(Ordering::Relaxed) {
        // Process conversion requests
        // Check shutdown flag regularly
    }

    println!("Worker shutting down gracefully");
    Ok(())
}

fn main() -> Result<()> {
    let shutdown = Arc::new(AtomicBool::new(false));

    // Setup signal handler
    // ctrlc::set_handler(move || {
    //     shutdown.store(true, Ordering::Relaxed);
    // }).expect("Error setting Ctrl-C handler");

    conversion_worker(shutdown)?;
    Ok(())
}
```

### Health Check

```rust
// Note: DocumentConverter is in docling-backend crate
use docling_backend::DocumentConverter;
use docling_core::Result;

fn health_check() -> Result<bool> {
    // Try to create converter
    match DocumentConverter::new() {
        Ok(_) => {
            println!("✓ Health check passed");
            Ok(true)
        }
        Err(e) => {
            eprintln!("✗ Health check failed: {}", e);
            Ok(false)
        }
    }
}

fn main() -> Result<()> {
    let healthy = health_check()?;
    std::process::exit(if healthy { 0 } else { 1 });
}
```

---

## See Also

- [User Guide](USER_GUIDE.md) - Comprehensive usage documentation
- [Format Guides](formats/) - Format-specific documentation
- [Performance Guide](guides/performance.md) - Optimization techniques
- [Migration Guide](guides/migration.md) - Python to Rust migration
