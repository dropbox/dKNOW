use std::path::PathBuf;

fn main() {
    println!("Testing FB2 files...");
    test_fb2_files();

    println!("\nTesting EPUB files...");
    test_epub_files();

    println!("\nTesting MOBI files...");
    test_mobi_files();
}

fn test_fb2_files() {
    let files = vec![
        "test-corpus/ebooks/fb2/simple.fb2",
        "test-corpus/ebooks/fb2/fiction_novel.fb2",
        "test-corpus/ebooks/fb2/technical_book.fb2",
        "test-corpus/ebooks/fb2/poetry.fb2",
        "test-corpus/ebooks/fb2/multilingual.fb2",
    ];

    for file in files {
        let path = PathBuf::from(file);
        print!("  Testing {file}... ");
        match docling_ebook::parse_fb2(&path) {
            Ok(ebook) => {
                println!(
                    "✓ (title: {})",
                    ebook
                        .metadata
                        .title
                        .unwrap_or_else(|| "Unknown".to_string())
                );
            }
            Err(e) => {
                println!("✗ Error: {e}");
            }
        }
    }
}

fn test_epub_files() {
    let files = vec![
        "test-corpus/ebooks/epub/simple.epub",
        "test-corpus/ebooks/epub/complex.epub",
        "test-corpus/ebooks/epub/with_images.epub",
        "test-corpus/ebooks/epub/large.epub",
        "test-corpus/ebooks/epub/non_english.epub",
    ];

    for file in files {
        let path = PathBuf::from(file);
        print!("  Testing {file}... ");
        match docling_ebook::parse_epub(&path) {
            Ok(ebook) => {
                println!(
                    "✓ (title: {})",
                    ebook
                        .metadata
                        .title
                        .unwrap_or_else(|| "Unknown".to_string())
                );
            }
            Err(e) => {
                println!("✗ Error: {e}");
            }
        }
    }
}

fn test_mobi_files() {
    let files = vec![
        "test-corpus/ebooks/mobi/simple_text.mobi",
        "test-corpus/ebooks/mobi/formatted.mobi",
        "test-corpus/ebooks/mobi/multi_chapter.mobi",
        "test-corpus/ebooks/mobi/with_metadata.mobi",
        "test-corpus/ebooks/mobi/large_content.mobi",
    ];

    for file in files {
        let path = PathBuf::from(file);
        print!("  Testing {file}... ");
        let bytes = std::fs::read(&path).unwrap();
        match docling_ebook::parse_mobi(&bytes) {
            Ok(ebook) => {
                println!(
                    "✓ (title: {})",
                    ebook
                        .metadata
                        .title
                        .unwrap_or_else(|| "Unknown".to_string())
                );
            }
            Err(e) => {
                println!("✗ Error: {e}");
            }
        }
    }
}
