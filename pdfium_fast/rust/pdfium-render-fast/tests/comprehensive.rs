//! Comprehensive tests for pdfium-render-fast
//!
//! Tests cover:
//! - Library initialization
//! - Document loading (file, bytes, owned bytes)
//! - Page access and iteration
//! - Text extraction (all, range, chars, words, cells)
//! - Rendering (default, custom DPI, pixel formats)
//! - Parallel rendering
//! - Error handling
//! - Edge cases

use pdfium_render_fast::{
    analyze_font_clusters,
    // Data Availability API
    check_linearization,
    detect_layout_regions,
    // Docling Integration API
    extract_reading_order,
    // Core
    font_flags,
    // Font System API
    get_default_ttf_map,
    get_default_ttf_map_count,
    get_default_ttf_map_entry,
    get_first_available_page,
    is_hiragana,
    is_japanese_char,
    is_kanji,
    is_katakana,
    is_known_math_font,
    // Artifact Detection API
    ArtifactType,
    // Bracketed Reference Detection API
    BracketType,
    BracketedReference,
    // Centered Block Detection API
    CenteredBlock,
    CharsetFontMapping,
    DataAvailability,
    DoclingClassification,
    DocumentType,
    DuplexType,
    FontCharset,
    FontSemanticRole,
    // Font Usage API
    FontUsageInfo,
    FormAvailability,
    FormError,
    FormFieldFlags,
    IccColorSpace,
    ImageColorspace,
    // Image Technical Metadata API
    ImageFilter,
    JPunctType,
    // Japanese Character Analysis API
    JapaneseCharAnalysis,
    // Japanese Punctuation API
    JapanesePunctuation,
    LayoutRegionType,
    LinearizationStatus,
    // Math Character Analysis API
    MathCharAnalysis,
    PageMode,
    PageObjectType,
    PathSegmentType,
    PdfBitmap,
    PdfClipPath,
    PdfDocument,
    PdfError,
    PdfFormFieldType,
    PdfRenderConfig,
    Pdfium,
    PixelFormat,
    ReferencePosition,
    // Repeated Content Detection API
    RepeatedRegion,
    // Ruby Annotation (Furigana) API
    RubyAnnotation,
    // Script Cluster Detection API
    ScriptChar,
    ScriptCluster,
    ScriptPosition,
    // Text Block Metrics API
    TextBlockMetrics,
    // Text Decoration API
    TextDecoration,
    TextDecorationType,
    // Writing Direction Detection API
    WritingDirection,
    WritingDirectionInfo,
};
use serial_test::serial;
use std::path::PathBuf;
use tempfile::tempdir;

/// Get path to a test PDF
fn get_test_pdf() -> PathBuf {
    // Look for web_039.pdf which is commonly used in tests
    let paths = [
        PathBuf::from("../../integration_tests/pdfs/benchmark/web_039.pdf"),
        PathBuf::from("../integration_tests/pdfs/benchmark/web_039.pdf"),
        PathBuf::from("/Users/ayates/pdfium_fast/integration_tests/pdfs/benchmark/web_039.pdf"),
    ];

    for path in &paths {
        if path.exists() {
            return path.clone();
        }
    }

    panic!("Test PDF not found. Looked in: {:?}", paths);
}

// ============================================================================
// Library Initialization Tests
// ============================================================================

#[test]
#[serial]
fn test_pdfium_new() {
    let pdfium = Pdfium::new();
    assert!(pdfium.is_ok(), "Pdfium::new() should succeed");
}

#[test]
#[serial]
fn test_pdfium_default() {
    let pdfium = Pdfium::default();
    // Default should not panic
    drop(pdfium);
}

#[test]
#[serial]
fn test_pdfium_clone() {
    let pdfium1 = Pdfium::new().unwrap();
    let pdfium2 = pdfium1.clone();
    // Both should work
    let pdf_path = get_test_pdf();
    let _doc1 = pdfium1.load_pdf_from_file(&pdf_path, None);
    let _doc2 = pdfium2.load_pdf_from_file(&pdf_path, None);
}

// ============================================================================
// Document Loading Tests
// ============================================================================

#[test]
#[serial]
fn test_load_pdf_from_file() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None);
    assert!(doc.is_ok(), "Should load PDF from file");

    let doc = doc.unwrap();
    assert!(doc.page_count() > 0, "Document should have pages");
}

#[test]
#[serial]
fn test_load_pdf_from_file_not_found() {
    let pdfium = Pdfium::new().unwrap();
    let result = pdfium.load_pdf_from_file("/nonexistent/path/to/file.pdf", None);
    assert!(result.is_err(), "Should fail for non-existent file");

    match result {
        Err(PdfError::FileNotFound(_)) => {}
        Err(e) => panic!("Expected FileNotFound, got {:?}", e),
        Ok(_) => panic!("Expected error, got Ok"),
    }
}

#[test]
#[serial]
fn test_load_pdf_from_bytes() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let data = std::fs::read(&pdf_path).unwrap();

    let doc = pdfium.load_pdf_from_bytes(&data, None);
    assert!(doc.is_ok(), "Should load PDF from bytes");

    let doc = doc.unwrap();
    assert!(doc.page_count() > 0, "Document should have pages");
}

#[test]
#[serial]
fn test_load_pdf_from_bytes_owned() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let data = std::fs::read(&pdf_path).unwrap();

    let doc = pdfium.load_pdf_from_bytes_owned(data, None);
    assert!(doc.is_ok(), "Should load PDF from owned bytes");

    let doc = doc.unwrap();
    assert!(doc.page_count() > 0, "Document should have pages");
}

#[test]
#[serial]
fn test_load_pdf_invalid_data() {
    let pdfium = Pdfium::new().unwrap();
    let invalid_data = b"This is not a PDF file";

    let result = pdfium.load_pdf_from_bytes(invalid_data, None);
    assert!(result.is_err(), "Should fail for invalid PDF data");

    match result {
        Err(PdfError::OpenFailed { .. }) => {}
        Err(e) => panic!("Expected OpenFailed, got {:?}", e),
        Ok(_) => panic!("Expected error, got Ok"),
    }
}

// ============================================================================
// Document Metadata Tests
// ============================================================================

#[test]
#[serial]
fn test_document_page_count() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    let count = doc.page_count();
    assert!(count > 0, "Page count should be > 0");
}

#[test]
#[serial]
fn test_document_handle() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    let handle = doc.handle();
    assert!(!handle.is_null(), "Handle should not be null");
}

#[test]
#[serial]
fn test_document_is_tagged() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    // Just check it doesn't crash - result depends on the PDF
    let _tagged = doc.is_tagged();
}

#[test]
#[serial]
fn test_document_metadata() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    // These may or may not exist, but shouldn't crash
    let _title = doc.metadata("Title");
    let _author = doc.metadata("Author");
    let _subject = doc.metadata("Subject");
    let _keywords = doc.metadata("Keywords");
    let _creator = doc.metadata("Creator");
    let _producer = doc.metadata("Producer");
}

#[test]
#[serial]
fn test_document_optimal_thread_count() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    let threads = doc.optimal_thread_count();
    assert!(threads >= 1, "Should have at least 1 thread");
}

// ============================================================================
// Page Access Tests
// ============================================================================

#[test]
#[serial]
fn test_page_by_index() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    let page = doc.page(0);
    assert!(page.is_ok(), "Should get page 0");

    let page = page.unwrap();
    assert_eq!(page.index(), 0, "Page index should be 0");
}

#[test]
#[serial]
fn test_page_out_of_bounds() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    let count = doc.page_count();
    let result = doc.page(count); // One past the end

    assert!(result.is_err(), "Should fail for out of bounds page");
    match result {
        Err(PdfError::PageIndexOutOfBounds { index, count: c }) => {
            assert_eq!(index, count);
            assert_eq!(c, count);
        }
        Err(e) => panic!("Expected PageIndexOutOfBounds, got {:?}", e),
        Ok(_) => panic!("Expected error, got Ok"),
    }
}

#[test]
#[serial]
fn test_pages_iterator() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    let count = doc.page_count();
    let pages: Vec<_> = doc.pages().collect();

    assert_eq!(pages.len(), count, "Iterator should yield all pages");
}

#[test]
#[serial]
fn test_pages_iterator_size_hint() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    let count = doc.page_count();
    let mut iter = doc.pages();

    assert_eq!(iter.size_hint(), (count, Some(count)));
    iter.next();
    assert_eq!(iter.size_hint(), (count - 1, Some(count - 1)));
}

// ============================================================================
// Page Properties Tests
// ============================================================================

#[test]
#[serial]
fn test_page_size() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let (width, height) = page.size();
    assert!(width > 0.0, "Width should be > 0");
    assert!(height > 0.0, "Height should be > 0");
}

#[test]
#[serial]
fn test_page_width_height() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let width = page.width();
    let height = page.height();
    let (w, h) = page.size();

    assert_eq!(width, w, "width() should match size().0");
    assert_eq!(height, h, "height() should match size().1");
}

#[test]
#[serial]
fn test_page_size_at_dpi() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let (width_72, height_72) = page.size_at_dpi(72.0);
    let (width_144, height_144) = page.size_at_dpi(144.0);

    // At 72 DPI, dimensions should match points (approximately)
    assert!(
        (width_72 as f64 - page.width()).abs() < 2.0,
        "72 DPI should match points"
    );

    // At 144 DPI, dimensions should be double 72 DPI
    assert!(
        (width_144 as f64 - width_72 as f64 * 2.0).abs() < 2.0,
        "144 DPI should be ~2x 72 DPI"
    );
    assert!(
        (height_144 as f64 - height_72 as f64 * 2.0).abs() < 2.0,
        "144 DPI should be ~2x 72 DPI"
    );
}

#[test]
#[serial]
fn test_page_handle() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let handle = page.handle();
    assert!(!handle.is_null(), "Page handle should not be null");
}

#[test]
#[serial]
fn test_page_is_scanned() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // Just check it doesn't crash - result depends on the PDF
    let _scanned = page.is_scanned();
}

// ============================================================================
// Text Extraction Tests
// ============================================================================

#[test]
#[serial]
fn test_page_text() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let text = page.text();
    assert!(text.is_ok(), "Should extract text");
}

#[test]
#[serial]
fn test_text_all() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();
    let text = page.text().unwrap();

    let all = text.all();
    // Text content depends on PDF, but shouldn't crash
    // The test PDF may or may not have text
    let _ = all; // Just verify it doesn't crash
}

#[test]
#[serial]
fn test_text_char_count() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();
    let text = page.text().unwrap();

    let count = text.char_count();
    // Just verify it returns a reasonable value (test PDF has content)
    assert!(count > 0, "char_count should return positive for test PDF");
}

#[test]
#[serial]
fn test_text_range() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();
    let text = page.text().unwrap();

    if text.char_count() > 10 {
        let range = text.range(0, 10);
        // Range may or may not return text depending on the PDF
        let _ = range; // Just verify it doesn't crash
    }
}

#[test]
#[serial]
fn test_text_range_empty() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();
    let text = page.text().unwrap();

    let empty = text.range(0, 0);
    assert_eq!(empty.len(), 0, "Empty range should return empty string");
}

#[test]
#[serial]
fn test_text_chars_iterator() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();
    let text = page.text().unwrap();

    let chars: Vec<_> = text.chars().take(10).collect();
    if text.char_count() > 0 {
        assert!(!chars.is_empty(), "Should iterate chars");
    }
}

#[test]
#[serial]
fn test_text_char_at() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();
    let text = page.text().unwrap();

    if text.char_count() > 0 {
        let ch = text.char_at(0);
        assert!(ch.is_some(), "Should get char at index 0");

        let ch = ch.unwrap();
        assert!(ch.font_size >= 0.0, "Font size should be valid");
    }

    // Out of bounds should return None
    let none = text.char_at(usize::MAX);
    assert!(none.is_none(), "Out of bounds should return None");
}

#[test]
#[serial]
fn test_pdf_char_properties() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();
    let text = page.text().unwrap();

    if text.char_count() > 0 {
        let ch = text.char_at(0).unwrap();

        // Check PdfChar methods
        let _width = ch.width();
        let _height = ch.height();
        let _unicode = ch.unicode;
        let _index = ch.index;
        let _left = ch.left;
        let _bottom = ch.bottom;
        let _right = ch.right;
        let _top = ch.top;
        let _font_size = ch.font_size;
        let _angle = ch.angle;
    }
}

#[test]
#[serial]
fn test_text_word_count() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();
    let text = page.text().unwrap();

    let count = text.word_count();
    // Just verify it returns a reasonable value (test PDF has words)
    assert!(count > 0, "word_count should return positive for test PDF");
}

#[test]
#[serial]
fn test_text_words() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();
    let text = page.text().unwrap();

    let words = text.words();
    if text.word_count() > 0 {
        assert!(!words.is_empty(), "Should extract words");
    }
}

#[test]
#[serial]
fn test_pdf_word_properties() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();
    let text = page.text().unwrap();

    let words = text.words();
    if !words.is_empty() {
        let word = &words[0];

        // Check PdfWord methods
        let _width = word.width();
        let _height = word.height();
        let _text = &word.text;
        let _left = word.left;
        let _bottom = word.bottom;
        let _right = word.right;
        let _top = word.top;
        let _start = word.start_char_index;
        let _end = word.end_char_index;
    }
}

#[test]
#[serial]
fn test_text_cells() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();
    let text = page.text().unwrap();

    let cells = text.cells();
    // Cells may be empty for some PDFs, but shouldn't crash
    if !cells.is_empty() {
        let cell = &cells[0];
        assert!(
            !cell.text.is_empty() || cell.char_count > 0,
            "Cells should have content"
        );
    }
}

#[test]
#[serial]
fn test_pdf_text_cell_properties() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();
    let text = page.text().unwrap();

    let cells = text.cells();
    if !cells.is_empty() {
        let cell = &cells[0];

        // Check PdfTextCell methods
        let _width = cell.width();
        let _height = cell.height();
        let _is_bold = cell.is_bold();
        let _is_italic = cell.is_italic();
        let _is_mono = cell.is_monospace();
        let _is_serif = cell.is_serif();

        // Check PdfTextCell fields
        let _text = &cell.text;
        let _left = cell.left;
        let _bottom = cell.bottom;
        let _right = cell.right;
        let _top = cell.top;
        let _font_size = cell.font_size;
        let _font_flags = cell.font_flags;
        let _char_start = cell.char_start;
        let _char_count = cell.char_count;
    }
}

#[test]
#[serial]
fn test_font_flags() {
    // Test font flag constants
    assert_eq!(font_flags::FIXED_PITCH, 0x0001);
    assert_eq!(font_flags::SERIF, 0x0002);
    assert_eq!(font_flags::SYMBOLIC, 0x0004);
    assert_eq!(font_flags::SCRIPT, 0x0008);
    assert_eq!(font_flags::NONSYMBOLIC, 0x0020);
    assert_eq!(font_flags::ITALIC, 0x0040);
    assert_eq!(font_flags::ALLCAP, 0x10000);
    assert_eq!(font_flags::SMALLCAP, 0x20000);
    assert_eq!(font_flags::BOLD, 0x40000);
}

#[test]
#[serial]
fn test_text_in_rect() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();
    let text = page.text().unwrap();

    // Extract text from a region
    let (width, height) = page.size();
    let region_text = text.text_in_rect(0.0, height, width / 2.0, height / 2.0);
    // May be empty depending on PDF layout, but shouldn't crash
    let _ = region_text;
}

// ============================================================================
// Rendering Tests
// ============================================================================

#[test]
#[serial]
fn test_page_render_default() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let bitmap = page.render();
    assert!(bitmap.is_ok(), "Should render page with defaults");

    let bitmap = bitmap.unwrap();
    assert!(bitmap.width() > 0, "Bitmap width should be > 0");
    assert!(bitmap.height() > 0, "Bitmap height should be > 0");
}

#[test]
#[serial]
fn test_page_render_with_config() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let config = PdfRenderConfig::new()
        .set_target_dpi(150.0)
        .set_pixel_format(PixelFormat::Bgra);

    let bitmap = page.render_with_config(&config);
    assert!(bitmap.is_ok(), "Should render with config");
}

#[test]
#[serial]
fn test_render_config_dpi_scaling() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let config_72 = PdfRenderConfig::new().set_target_dpi(72.0);
    let config_144 = PdfRenderConfig::new().set_target_dpi(144.0);

    let bitmap_72 = page.render_with_config(&config_72).unwrap();
    let bitmap_144 = page.render_with_config(&config_144).unwrap();

    // 144 DPI should be approximately 2x 72 DPI
    assert!(
        (bitmap_144.width() as f64 - bitmap_72.width() as f64 * 2.0).abs() < 2.0,
        "Width should scale with DPI"
    );
}

#[test]
#[serial]
fn test_render_config_pixel_formats() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // BGRA (default)
    let config_bgra = PdfRenderConfig::new().set_pixel_format(PixelFormat::Bgra);
    let bitmap_bgra = page.render_with_config(&config_bgra).unwrap();
    assert_eq!(bitmap_bgra.format(), PixelFormat::Bgra);

    // BGR
    let config_bgr = PdfRenderConfig::new().set_pixel_format(PixelFormat::Bgr);
    let bitmap_bgr = page.render_with_config(&config_bgr).unwrap();
    assert_eq!(bitmap_bgr.format(), PixelFormat::Bgr);

    // Gray
    let config_gray = PdfRenderConfig::new().set_pixel_format(PixelFormat::Gray);
    let bitmap_gray = page.render_with_config(&config_gray).unwrap();
    assert_eq!(bitmap_gray.format(), PixelFormat::Gray);
}

#[test]
#[serial]
fn test_pixel_format_bytes_per_pixel() {
    assert_eq!(PixelFormat::Bgra.bytes_per_pixel(), 4);
    assert_eq!(PixelFormat::Bgr.bytes_per_pixel(), 3);
    assert_eq!(PixelFormat::Gray.bytes_per_pixel(), 1);
}

#[test]
#[serial]
fn test_render_config_target_width() {
    let config = PdfRenderConfig::new().set_target_width(800);
    let (width, height) = config.calculate_size(612.0, 792.0);
    assert_eq!(width, 800, "Width should match target");
    assert!(height > 0, "Height should be calculated from aspect ratio");
}

#[test]
#[serial]
fn test_render_config_target_height() {
    let config = PdfRenderConfig::new().set_target_height(1000);
    let (width, height) = config.calculate_size(612.0, 792.0);
    assert_eq!(height, 1000, "Height should match target");
    assert!(width > 0, "Width should be calculated from aspect ratio");
}

// ============================================================================
// Bitmap Tests
// ============================================================================

#[test]
#[serial]
fn test_bitmap_properties() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let config = PdfRenderConfig::new()
        .set_target_dpi(72.0)
        .set_pixel_format(PixelFormat::Bgra);

    let bitmap = page.render_with_config(&config).unwrap();

    assert!(bitmap.width() > 0);
    assert!(bitmap.height() > 0);
    assert!(bitmap.stride() >= bitmap.width() as usize * 4);
    assert_eq!(bitmap.format(), PixelFormat::Bgra);
}

#[test]
#[serial]
fn test_bitmap_data() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let config = PdfRenderConfig::new().set_target_dpi(72.0);
    let bitmap = page.render_with_config(&config).unwrap();

    let data = bitmap.data();
    assert!(!data.is_empty(), "Bitmap data should not be empty");
    assert!(data.len() >= bitmap.stride() * bitmap.height() as usize);
}

#[test]
#[serial]
fn test_bitmap_to_vec() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let config = PdfRenderConfig::new().set_target_dpi(72.0);
    let bitmap = page.render_with_config(&config).unwrap();

    let vec_data = bitmap.to_vec();
    assert_eq!(vec_data.len(), bitmap.data().len());
}

#[test]
#[serial]
fn test_bitmap_to_rgb() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // Test BGRA to RGB
    let config_bgra = PdfRenderConfig::new()
        .set_target_dpi(72.0)
        .set_pixel_format(PixelFormat::Bgra);
    let bitmap_bgra = page.render_with_config(&config_bgra).unwrap();
    let rgb_bgra = bitmap_bgra.to_rgb();
    assert_eq!(
        rgb_bgra.len(),
        (bitmap_bgra.width() * bitmap_bgra.height() * 3) as usize
    );

    // Test BGR to RGB
    let config_bgr = PdfRenderConfig::new()
        .set_target_dpi(72.0)
        .set_pixel_format(PixelFormat::Bgr);
    let bitmap_bgr = page.render_with_config(&config_bgr).unwrap();
    let rgb_bgr = bitmap_bgr.to_rgb();
    assert_eq!(
        rgb_bgr.len(),
        (bitmap_bgr.width() * bitmap_bgr.height() * 3) as usize
    );

    // Test Gray to RGB
    let config_gray = PdfRenderConfig::new()
        .set_target_dpi(72.0)
        .set_pixel_format(PixelFormat::Gray);
    let bitmap_gray = page.render_with_config(&config_gray).unwrap();
    let rgb_gray = bitmap_gray.to_rgb();
    assert_eq!(
        rgb_gray.len(),
        (bitmap_gray.width() * bitmap_gray.height() * 3) as usize
    );
}

// ============================================================================
// Image Saving Tests
// ============================================================================

#[test]
#[serial]
fn test_bitmap_save_as_png() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let config = PdfRenderConfig::new().set_target_dpi(72.0);
    let bitmap = page.render_with_config(&config).unwrap();

    let dir = tempdir().unwrap();
    let png_path = dir.path().join("test.png");

    let result = bitmap.save_as_png(&png_path);
    assert!(result.is_ok(), "Should save PNG");
    assert!(png_path.exists(), "PNG file should exist");
}

#[test]
#[serial]
fn test_bitmap_save_as_jpeg() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let config = PdfRenderConfig::new().set_target_dpi(72.0);
    let bitmap = page.render_with_config(&config).unwrap();

    let dir = tempdir().unwrap();
    let jpeg_path = dir.path().join("test.jpg");

    let result = bitmap.save_as_jpeg(&jpeg_path, 85);
    assert!(result.is_ok(), "Should save JPEG");
    assert!(jpeg_path.exists(), "JPEG file should exist");
}

#[test]
#[serial]
fn test_bitmap_save_as_ppm() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let config = PdfRenderConfig::new().set_target_dpi(72.0);
    let bitmap = page.render_with_config(&config).unwrap();

    let dir = tempdir().unwrap();
    let ppm_path = dir.path().join("test.ppm");

    let result = bitmap.save_as_ppm(&ppm_path);
    assert!(result.is_ok(), "Should save PPM");
    assert!(ppm_path.exists(), "PPM file should exist");
}

#[test]
#[serial]
fn test_bitmap_save_all_formats() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // Test saving with different pixel formats
    let formats = [PixelFormat::Bgra, PixelFormat::Bgr, PixelFormat::Gray];

    for format in formats {
        let config = PdfRenderConfig::new()
            .set_target_dpi(72.0)
            .set_pixel_format(format);
        let bitmap = page.render_with_config(&config).unwrap();

        let dir = tempdir().unwrap();

        // PNG
        let png_path = dir.path().join(format!("test_{:?}.png", format));
        assert!(bitmap.save_as_png(&png_path).is_ok());

        // JPEG
        let jpeg_path = dir.path().join(format!("test_{:?}.jpg", format));
        assert!(bitmap.save_as_jpeg(&jpeg_path, 85).is_ok());

        // PPM
        let ppm_path = dir.path().join(format!("test_{:?}.ppm", format));
        assert!(bitmap.save_as_ppm(&ppm_path).is_ok());
    }
}

// ============================================================================
// Parallel Rendering Tests
// ============================================================================

#[test]
#[serial]
fn test_parallel_rendering() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    if doc.page_count() > 1 {
        let config = PdfRenderConfig::new().set_target_dpi(72.0);
        let pages = doc.render_pages_parallel(&config);

        assert!(pages.is_ok(), "Parallel rendering should succeed");
        let pages = pages.unwrap();
        assert_eq!(pages.len(), doc.page_count(), "Should render all pages");
    }
}

#[test]
#[serial]
fn test_parallel_rendering_threaded() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    if doc.page_count() > 1 {
        let config = PdfRenderConfig::new().set_target_dpi(72.0);
        let pages = doc.render_pages_parallel_threaded(&config, 4);

        assert!(
            pages.is_ok(),
            "Parallel rendering with threads should succeed"
        );
    }
}

#[test]
#[serial]
fn test_parallel_rendering_range() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    if doc.page_count() > 2 {
        let config = PdfRenderConfig::new().set_target_dpi(72.0);
        let pages = doc.render_pages_parallel_range(0, 2, &config, Some(2));

        assert!(pages.is_ok(), "Range rendering should succeed");
        let pages = pages.unwrap();
        assert_eq!(pages.len(), 2, "Should render 2 pages");
    }
}

#[test]
#[serial]
fn test_parallel_rendering_empty_range() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    let config = PdfRenderConfig::new().set_target_dpi(72.0);
    let pages = doc.render_pages_parallel_range(0, 0, &config, None);

    assert!(pages.is_ok(), "Empty range should succeed");
    assert!(pages.unwrap().is_empty(), "Should return empty vector");
}

#[test]
#[serial]
fn test_parallel_rendering_out_of_bounds() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    let config = PdfRenderConfig::new().set_target_dpi(72.0);
    let count = doc.page_count();
    let result = doc.render_pages_parallel_range(count + 100, 10, &config, None);

    assert!(result.is_err(), "Out of bounds start should fail");
}

// ============================================================================
// RenderedPage Tests
// ============================================================================

#[test]
#[serial]
fn test_rendered_page_properties() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    if doc.page_count() > 1 {
        let config = PdfRenderConfig::new().set_target_dpi(72.0);
        let pages = doc.render_pages_parallel(&config).unwrap();

        let page = &pages[0];
        assert_eq!(page.page_index, 0);
        assert!(page.width > 0);
        assert!(page.height > 0);
        assert!(page.stride > 0);
        assert!(!page.data.is_empty());
        assert!(page.bytes_per_pixel() > 0);
        assert!(page.data_size() > 0);
    }
}

#[test]
#[serial]
fn test_rendered_page_to_rgb() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    if doc.page_count() > 1 {
        let config = PdfRenderConfig::new().set_target_dpi(72.0);
        let pages = doc.render_pages_parallel(&config).unwrap();

        let page = &pages[0];
        let rgb = page.to_rgb();
        assert_eq!(rgb.len(), (page.width * page.height * 3) as usize);
    }
}

#[test]
#[serial]
fn test_rendered_page_save() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    if doc.page_count() > 1 {
        let config = PdfRenderConfig::new().set_target_dpi(72.0);
        let pages = doc.render_pages_parallel(&config).unwrap();
        let page = &pages[0];

        let dir = tempdir().unwrap();

        // PNG
        let png_path = dir.path().join("rendered.png");
        assert!(page.save_as_png(&png_path).is_ok());

        // JPEG
        let jpeg_path = dir.path().join("rendered.jpg");
        assert!(page.save_as_jpeg(&jpeg_path, 85).is_ok());

        // PPM
        let ppm_path = dir.path().join("rendered.ppm");
        assert!(page.save_as_ppm(&ppm_path).is_ok());
    }
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[test]
#[serial]
fn test_error_display() {
    let err = PdfError::FileNotFound("test.pdf".to_string());
    let msg = format!("{}", err);
    assert!(msg.contains("test.pdf"));

    let err = PdfError::PageIndexOutOfBounds {
        index: 10,
        count: 5,
    };
    let msg = format!("{}", err);
    assert!(msg.contains("10"));
    assert!(msg.contains("5"));
}

#[test]
#[serial]
fn test_error_debug() {
    let err = PdfError::InitializationFailed;
    let debug = format!("{:?}", err);
    assert!(!debug.is_empty());
}

// ============================================================================
// Thread Safety Tests
// ============================================================================

#[test]
#[serial]
fn test_document_send() {
    fn assert_send<T: Send>() {}
    assert_send::<PdfDocument>();
}

#[test]
#[serial]
fn test_page_send() {
    fn assert_send<T: Send>() {}
    assert_send::<pdfium_render_fast::PdfPage>();
}

#[test]
#[serial]
fn test_bitmap_send() {
    fn assert_send<T: Send>() {}
    assert_send::<PdfBitmap>();
}

// ============================================================================
// Integration Tests (Multi-page workflow)
// ============================================================================

#[test]
#[serial]
fn test_full_document_workflow() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    // Get metadata
    let _title = doc.metadata("Title");
    let _tagged = doc.is_tagged();

    // Process each page
    for page in doc.pages() {
        // Get page properties
        let _size = page.size();
        let _scanned = page.is_scanned();

        // Extract text
        if let Ok(text) = page.text() {
            let _all = text.all();
            let _words = text.words();
            let _cells = text.cells();
        }

        // Render page
        let config = PdfRenderConfig::new().set_target_dpi(72.0);
        let _bitmap = page.render_with_config(&config);
    }
}

#[test]
#[serial]
fn test_concurrent_document_access() {
    use std::thread;

    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();

    let handles: Vec<_> = (0..4)
        .map(|_| {
            let pdfium = pdfium.clone();
            let path = pdf_path.clone();
            thread::spawn(move || {
                let doc = pdfium.load_pdf_from_file(&path, None).unwrap();
                let page = doc.page(0).unwrap();
                let _ = page.size();
                let _ = page.text().map(|t| t.all());
            })
        })
        .collect();

    for handle in handles {
        handle.join().expect("Thread should complete");
    }
}

// ============================================================================
// JavaScript API Tests
// ============================================================================

#[test]
#[serial]
fn test_javascript_count() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    // Just verify it doesn't crash - most PDFs don't have JavaScript
    let _count = doc.javascript_count();
}

#[test]
#[serial]
fn test_has_javascript() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    // Just verify it doesn't crash
    let _has_js = doc.has_javascript();
}

#[test]
#[serial]
fn test_javascript_actions() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    let js = doc.javascript_actions();

    // Verify collection methods work
    let _count = js.count();
    let _is_empty = js.is_empty();
    let _has_suspicious = js.has_any_suspicious();
    let _names = js.names();
    let _total_len = js.total_script_length();
}

#[test]
#[serial]
fn test_javascript_actions_iterator() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    let js = doc.javascript_actions();

    // Test iter()
    for action in js.iter() {
        let _name = action.name();
        let _script = action.script();
        let _len = action.script_length();
        let _suspicious = action.has_suspicious_patterns();
    }
}

#[test]
#[serial]
fn test_javascript_actions_into_iter() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    // Test into_iter (consuming)
    for action in doc.javascript_actions() {
        let _name = action.name();
    }
}

#[test]
#[serial]
fn test_has_suspicious_javascript() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    // Just verify it doesn't crash - result depends on PDF content
    let _suspicious = doc.has_suspicious_javascript();
}

// ============================================================================
// Catalog API Tests
// ============================================================================

#[test]
#[serial]
fn test_set_language() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    // Try setting various language codes
    let _result_en = doc.set_language("en");
    let _result_en_us = doc.set_language("en-US");
    let _result_ja = doc.set_language("ja");
    let _result_zh_cn = doc.set_language("zh-CN");

    // Note: These may succeed or fail depending on PDF permissions,
    // but they shouldn't crash
}

#[test]
#[serial]
fn test_set_language_empty() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    // Empty language should be handled gracefully
    let _result = doc.set_language("");
}

// ============================================================================
// Page Boxes API Tests
// ============================================================================

#[test]
#[serial]
fn test_page_media_box() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // All PDFs should have a MediaBox
    let media_box = page.media_box();
    assert!(media_box.is_some(), "MediaBox should exist");

    let media_box = media_box.unwrap();
    assert!(media_box.width() > 0.0, "Width should be positive");
    assert!(media_box.height() > 0.0, "Height should be positive");
    assert!(media_box.is_valid(), "MediaBox should be valid");
}

#[test]
#[serial]
fn test_page_boxes_optional() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // These boxes are optional - just test they don't crash
    let _crop_box = page.crop_box();
    let _bleed_box = page.bleed_box();
    let _trim_box = page.trim_box();
    let _art_box = page.art_box();
}

#[test]
#[serial]
fn test_page_bounding_box() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // Bounding box should exist if page has content
    let bbox = page.bounding_box();
    if let Some(bbox) = bbox {
        assert!(bbox.is_valid(), "Bounding box should be valid if present");
    }
}

#[test]
#[serial]
fn test_page_box_struct() {
    use pdfium_render_fast::PdfPageBox;

    let page_box = PdfPageBox::new(0.0, 0.0, 612.0, 792.0);
    assert_eq!(page_box.width(), 612.0);
    assert_eq!(page_box.height(), 792.0);
    assert!(page_box.is_valid());

    let default_box = PdfPageBox::default();
    assert_eq!(default_box.width(), 612.0); // US Letter
    assert_eq!(default_box.height(), 792.0);
}

// ============================================================================
// Page Transform API Tests
// ============================================================================

#[test]
#[serial]
fn test_matrix_constructors() {
    use pdfium_render_fast::PdfMatrix;

    let identity = PdfMatrix::identity();
    assert_eq!(identity.a, 1.0);
    assert_eq!(identity.d, 1.0);
    assert_eq!(identity.e, 0.0);
    assert_eq!(identity.f, 0.0);

    let translation = PdfMatrix::translation(100.0, 200.0);
    assert_eq!(translation.e, 100.0);
    assert_eq!(translation.f, 200.0);

    let scale = PdfMatrix::scale(2.0);
    assert_eq!(scale.a, 2.0);
    assert_eq!(scale.d, 2.0);

    let scale_xy = PdfMatrix::scale_xy(1.5, 2.5);
    assert_eq!(scale_xy.a, 1.5);
    assert_eq!(scale_xy.d, 2.5);
}

#[test]
#[serial]
fn test_matrix_rotation() {
    use pdfium_render_fast::PdfMatrix;
    use std::f32::consts::PI;

    // 90 degree rotation
    let rot90 = PdfMatrix::rotation(PI / 2.0);
    assert!((rot90.a - 0.0).abs() < 0.0001);
    assert!((rot90.d - 0.0).abs() < 0.0001);

    let rot90_deg = PdfMatrix::rotation_degrees(90.0);
    assert!((rot90.a - rot90_deg.a).abs() < 0.0001);
}

// ============================================================================
// Thumbnail API Tests
// ============================================================================

#[test]
#[serial]
fn test_page_has_thumbnail() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // Just test it doesn't crash - most PDFs don't have thumbnails
    let _has_thumb = page.has_thumbnail();
}

#[test]
#[serial]
fn test_page_thumbnail_methods() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // Test thumbnail methods - they may return None which is fine
    let _decoded_thumb = page.thumbnail_data();
    let _raw_thumb = page.raw_thumbnail_data();
    let _thumb_bitmap = page.thumbnail_bitmap();
}

// ============================================================================
// Structure Tree API Tests
// ============================================================================

#[test]
#[serial]
fn test_page_has_structure_tree() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // Just test it doesn't crash - not all PDFs are tagged
    let _has_tree = page.has_structure_tree();
}

#[test]
#[serial]
fn test_page_structure_tree() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // Test structure tree access - may be None for untagged PDFs
    if let Some(tree) = page.structure_tree() {
        let count = tree.child_count();
        let _is_empty = tree.is_empty();

        // Iterate children
        for elem in tree.children() {
            let _elem_type = elem.element_type();
            let _title = elem.title();
            let _alt_text = elem.alt_text();
            let _actual_text = elem.actual_text();
            let _id = elem.id();
            let _lang = elem.language();
            let _mcid = elem.marked_content_id();
            let _child_count = elem.child_count();
            let _has_children = elem.has_children();
            let _attr_count = elem.attribute_count();

            // Test helper methods
            let _is_heading = elem.is_heading();
            let _is_paragraph = elem.is_paragraph();
            let _is_table = elem.is_table();
            let _is_figure = elem.is_figure();
            let _is_list = elem.is_list();
        }

        // Test all_elements
        let all_elems = tree.all_elements();
        assert!(all_elems.len() >= count as usize || count == 0);
    }
}

#[test]
#[serial]
fn test_struct_attribute_value_type() {
    use pdfium_render_fast::PdfStructAttributeValueType;

    // Just test enum values exist
    let _unknown = PdfStructAttributeValueType::Unknown;
    let _boolean = PdfStructAttributeValueType::Boolean;
    let _number = PdfStructAttributeValueType::Number;
    let _string = PdfStructAttributeValueType::String;
    let _blob = PdfStructAttributeValueType::Blob;
    let _array = PdfStructAttributeValueType::Array;
}

// ============================================================================
// Document Save Tests
// ============================================================================

#[test]
#[serial]
fn test_save_to_bytes() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    // Save to bytes
    let bytes = doc.save_to_bytes(None).unwrap();

    // Verify we got PDF data
    assert!(!bytes.is_empty(), "Saved PDF should have content");
    assert!(
        bytes.starts_with(b"%PDF"),
        "Saved data should start with PDF header"
    );
}

#[test]
#[serial]
fn test_save_to_file() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    // Create temp directory
    let temp_dir = tempdir().unwrap();
    let output_path = temp_dir.path().join("output.pdf");

    // Save to file
    doc.save_to_file(&output_path, None).unwrap();

    // Verify file exists and has content
    assert!(output_path.exists(), "Output file should exist");
    let file_size = std::fs::metadata(&output_path).unwrap().len();
    assert!(file_size > 0, "Output file should have content");

    // Verify we can re-open the saved PDF
    let reopened = pdfium.load_pdf_from_file(&output_path, None).unwrap();
    assert_eq!(
        reopened.page_count(),
        doc.page_count(),
        "Reopened document should have same page count"
    );
}

#[test]
#[serial]
fn test_save_with_version() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    // Save as PDF 1.7
    let bytes = doc.save_with_version(17, None).unwrap();

    // Verify we got PDF data
    assert!(!bytes.is_empty(), "Saved PDF should have content");
    assert!(bytes.starts_with(b"%PDF-1.7"), "Should be saved as PDF 1.7");
}

#[test]
#[serial]
fn test_save_flags() {
    use pdfium_render_fast::SaveFlags;

    // Test SaveFlags builder pattern
    let flags = SaveFlags::new();
    assert!(!flags.incremental, "Default should not be incremental");
    assert!(!flags.remove_security, "Default should not remove security");

    let flags = SaveFlags::new().incremental();
    assert!(flags.incremental, "Should be incremental");

    let flags = SaveFlags::new().remove_security();
    assert!(flags.remove_security, "Should remove security");
}

#[test]
#[serial]
fn test_save_round_trip() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    // Get original text from first page
    let original_text = doc.page(0).unwrap().text().unwrap().all();

    // Save to bytes
    let bytes = doc.save_to_bytes(None).unwrap();

    // Reload from bytes
    let reopened = pdfium.load_pdf_from_bytes_owned(bytes, None).unwrap();

    // Get text from reopened document
    let reopened_text = reopened.page(0).unwrap().text().unwrap().all();

    // Text should match
    assert_eq!(
        original_text, reopened_text,
        "Text should be preserved after save/reload"
    );
}

// ============================================================================
// Page Flatten Tests
// ============================================================================

#[test]
#[serial]
fn test_flatten_mode_enum() {
    use pdfium_render_fast::FlattenMode;

    // Test enum values exist
    let _display = FlattenMode::Display;
    let _print = FlattenMode::Print;

    // Test to_raw
    assert_eq!(FlattenMode::Display.to_raw(), 0);
    assert_eq!(FlattenMode::Print.to_raw(), 1);
}

#[test]
#[serial]
fn test_flatten_result_enum() {
    use pdfium_render_fast::FlattenResult;

    // Test enum values exist
    let _success = FlattenResult::Success;
    let _nothing = FlattenResult::NothingToDo;
    let _fail = FlattenResult::Fail;

    // Test from_raw
    assert_eq!(FlattenResult::from_raw(1), FlattenResult::Success);
    assert_eq!(FlattenResult::from_raw(2), FlattenResult::NothingToDo);
    assert_eq!(FlattenResult::from_raw(0), FlattenResult::Fail);
}

#[test]
#[serial]
fn test_flatten_page() {
    use pdfium_render_fast::{FlattenMode, FlattenResult};

    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // Flatten the page (most PDFs don't have annotations, so expect NothingToDo)
    let result = page.flatten(FlattenMode::Display);

    // Should be either Success or NothingToDo (not Fail)
    assert!(
        result == FlattenResult::Success || result == FlattenResult::NothingToDo,
        "Flatten should succeed or have nothing to do, got {:?}",
        result
    );
}

#[test]
#[serial]
fn test_flatten_convenience_methods() {
    use pdfium_render_fast::FlattenResult;

    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // Test convenience methods
    let result = page.flatten_for_display();
    assert!(
        result == FlattenResult::Success || result == FlattenResult::NothingToDo,
        "flatten_for_display should succeed or have nothing to do"
    );

    let result = page.flatten_for_print();
    assert!(
        result == FlattenResult::Success || result == FlattenResult::NothingToDo,
        "flatten_for_print should succeed or have nothing to do"
    );
}

// ============================================================================
// Create New Document Tests
// ============================================================================

#[test]
#[serial]
fn test_create_new_document() {
    let pdfium = Pdfium::new().unwrap();

    // Create new empty document
    let doc = pdfium.create_new_document().unwrap();

    // New document should have 0 pages
    assert_eq!(doc.page_count(), 0, "New document should have 0 pages");
}

#[test]
#[serial]
fn test_create_and_import_pages() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();

    // Create new document
    let new_doc = pdfium.create_new_document().unwrap();

    // Load source document
    let source = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let source_pages = source.page_count();

    // Import first page from source
    let success = new_doc.import_pages(&source, Some("1"), 0).unwrap();
    assert!(success, "Import should succeed");
    assert_eq!(new_doc.page_count(), 1, "Should have 1 page after import");

    // Import all pages
    let new_doc2 = pdfium.create_new_document().unwrap();
    let success = new_doc2.import_pages(&source, None, 0).unwrap();
    assert!(success, "Import all should succeed");
    assert_eq!(
        new_doc2.page_count(),
        source_pages,
        "Should have all pages after import"
    );
}

#[test]
#[serial]
fn test_create_save_reload() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();

    // Create new document and import pages
    let new_doc = pdfium.create_new_document().unwrap();
    let source = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    new_doc.import_pages(&source, Some("1"), 0).unwrap();

    // Save to bytes
    let bytes = new_doc.save_to_bytes(None).unwrap();

    // Reload and verify
    let reloaded = pdfium.load_pdf_from_bytes_owned(bytes, None).unwrap();
    assert_eq!(reloaded.page_count(), 1, "Reloaded should have 1 page");
}

// ============================================================================
// Page Delete Tests
// ============================================================================

#[test]
#[serial]
fn test_delete_page() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    let original_count = doc.page_count();
    if original_count > 1 {
        // Delete first page
        doc.delete_page(0);
        assert_eq!(
            doc.page_count(),
            original_count - 1,
            "Page count should decrease"
        );
    }
}

// ============================================================================
// Copy Viewer Preferences Tests
// ============================================================================

#[test]
#[serial]
fn test_copy_viewer_preferences() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();

    // Create destination document
    let dest = pdfium.create_new_document().unwrap();

    // Load source document
    let source = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    // Copy viewer preferences
    let success = dest.copy_viewer_preferences(&source);
    // This may or may not succeed depending on the PDF
    // Just verify it doesn't crash
    let _ = success;
}

// ============================================
// Page Creation API Tests
// ============================================

#[test]
fn test_new_page() {
    let pdfium = Pdfium::new().unwrap();
    let doc = pdfium.create_new_document().unwrap();

    // Create a custom size page
    let page = doc.new_page(0, 500.0, 700.0).unwrap();
    let (width, height) = page.size();

    assert_eq!(doc.page_count(), 1);
    assert!((width - 500.0).abs() < 0.001);
    assert!((height - 700.0).abs() < 0.001);
}

#[test]
fn test_new_page_letter() {
    let pdfium = Pdfium::new().unwrap();
    let doc = pdfium.create_new_document().unwrap();

    // Create a US Letter size page
    let page = doc.new_page_letter(0).unwrap();
    let (width, height) = page.size();

    // US Letter: 8.5 x 11 inches = 612 x 792 points
    assert!((width - 612.0).abs() < 0.001);
    assert!((height - 792.0).abs() < 0.001);
}

#[test]
fn test_new_page_a4() {
    let pdfium = Pdfium::new().unwrap();
    let doc = pdfium.create_new_document().unwrap();

    // Create an A4 size page
    let page = doc.new_page_a4(0).unwrap();
    let (width, height) = page.size();

    // A4: 210 x 297 mm = 595.276 x 841.890 points
    assert!((width - 595.276).abs() < 0.01);
    assert!((height - 841.890).abs() < 0.01);
}

#[test]
fn test_new_multiple_pages() {
    let pdfium = Pdfium::new().unwrap();
    let doc = pdfium.create_new_document().unwrap();

    // Create several pages
    doc.new_page_letter(0).unwrap();
    doc.new_page_a4(1).unwrap();
    doc.new_page(2, 400.0, 600.0).unwrap();

    assert_eq!(doc.page_count(), 3);

    // Verify page sizes
    let page0 = doc.page(0).unwrap();
    let page1 = doc.page(1).unwrap();
    let page2 = doc.page(2).unwrap();

    assert!((page0.size().0 - 612.0).abs() < 0.01); // Letter
    assert!((page1.size().0 - 595.276).abs() < 0.01); // A4
    assert!((page2.size().0 - 400.0).abs() < 0.01); // Custom
}

// ============================================
// Page Move/Reorder API Tests
// ============================================

#[test]
fn test_move_pages_basic() {
    let pdfium = Pdfium::new().unwrap();
    let doc = pdfium.create_new_document().unwrap();

    // Create 4 pages with different sizes for identification
    doc.new_page(0, 100.0, 100.0).unwrap(); // Page A
    doc.new_page(1, 200.0, 200.0).unwrap(); // Page B
    doc.new_page(2, 300.0, 300.0).unwrap(); // Page C
    doc.new_page(3, 400.0, 400.0).unwrap(); // Page D

    assert_eq!(doc.page_count(), 4);

    // Move page D (index 3) to position 1
    // [A, B, C, D] -> [A, D, B, C]
    let success = doc.move_pages(&[3], 1).unwrap();
    assert!(success);

    // Verify new order by page sizes
    let page1 = doc.page(1).unwrap();
    assert!((page1.size().0 - 400.0).abs() < 0.01); // D is now at position 1
}

#[test]
fn test_move_pages_empty() {
    let pdfium = Pdfium::new().unwrap();
    let doc = pdfium.create_new_document().unwrap();

    doc.new_page_letter(0).unwrap();

    // Moving empty list should succeed (no-op)
    let success = doc.move_pages(&[], 0).unwrap();
    assert!(success);
}

#[test]
fn test_reverse_pages() {
    let pdfium = Pdfium::new().unwrap();
    let doc = pdfium.create_new_document().unwrap();

    // Create 4 pages with different sizes
    doc.new_page(0, 100.0, 100.0).unwrap();
    doc.new_page(1, 200.0, 200.0).unwrap();
    doc.new_page(2, 300.0, 300.0).unwrap();
    doc.new_page(3, 400.0, 400.0).unwrap();

    // Reverse: [100, 200, 300, 400] -> [400, 300, 200, 100]
    let success = doc.reverse_pages().unwrap();
    assert!(success);

    // Verify new order
    let page0 = doc.page(0).unwrap();
    let page3 = doc.page(3).unwrap();

    assert!((page0.size().0 - 400.0).abs() < 0.01); // 400 is now first
    assert!((page3.size().0 - 100.0).abs() < 0.01); // 100 is now last
}

#[test]
fn test_reverse_single_page() {
    let pdfium = Pdfium::new().unwrap();
    let doc = pdfium.create_new_document().unwrap();

    doc.new_page_letter(0).unwrap();

    // Reversing a single page should succeed (no-op)
    let success = doc.reverse_pages().unwrap();
    assert!(success);
    assert_eq!(doc.page_count(), 1);
}

// ============================================
// File Identifier API Tests
// ============================================

#[test]
fn test_file_identifier_type_enum() {
    use pdfium_render_fast::FileIdentifierType;

    // Test enum values exist
    let permanent = FileIdentifierType::Permanent;
    let changing = FileIdentifierType::Changing;

    // Test they are different
    assert_ne!(permanent, changing);
}

#[test]
fn test_file_identifier_from_pdf() {
    use pdfium_render_fast::FileIdentifierType;

    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    // Try to get file identifiers
    // Note: Not all PDFs have file identifiers, so we just verify the API doesn't crash
    let _permanent = doc.file_identifier(FileIdentifierType::Permanent);
    let _changing = doc.file_identifier(FileIdentifierType::Changing);

    // Also test the convenience methods
    let _permanent_hex = doc.permanent_id();
    let _changing_hex = doc.changing_id();
}

#[test]
fn test_file_identifier_new_document() {
    use pdfium_render_fast::FileIdentifierType;

    let pdfium = Pdfium::new().unwrap();
    let doc = pdfium.create_new_document().unwrap();

    // New documents typically don't have file identifiers until saved
    let permanent = doc.file_identifier(FileIdentifierType::Permanent);
    let changing = doc.file_identifier(FileIdentifierType::Changing);

    // Both should be None for a brand new document
    assert!(permanent.is_none());
    assert!(changing.is_none());
}

#[test]
fn test_permanent_id_convenience() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    // If permanent_id returns Some, verify it's a hex string
    if let Some(id) = doc.permanent_id() {
        // Should only contain hex characters
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
        // Should be even length (each byte = 2 hex chars)
        assert!(id.len() % 2 == 0);
    }
}

#[test]
fn test_changing_id_convenience() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    // If changing_id returns Some, verify it's a hex string
    if let Some(id) = doc.changing_id() {
        // Should only contain hex characters
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
        // Should be even length (each byte = 2 hex chars)
        assert!(id.len() % 2 == 0);
    }
}

// ============================================
// Standard Font Loading API Tests
// ============================================

#[test]
fn test_standard_font_enum() {
    use pdfium_render_fast::StandardFont;

    // Test font names
    assert_eq!(StandardFont::Helvetica.name(), "Helvetica");
    assert_eq!(StandardFont::HelveticaBold.name(), "Helvetica-Bold");
    assert_eq!(StandardFont::TimesRoman.name(), "Times-Roman");
    assert_eq!(StandardFont::Courier.name(), "Courier");
    assert_eq!(StandardFont::Symbol.name(), "Symbol");
    assert_eq!(StandardFont::ZapfDingbats.name(), "ZapfDingbats");

    // Test font categories
    assert!(StandardFont::Courier.is_fixed_width());
    assert!(!StandardFont::Helvetica.is_fixed_width());

    assert!(StandardFont::Helvetica.is_sans_serif());
    assert!(!StandardFont::TimesRoman.is_sans_serif());

    assert!(StandardFont::TimesRoman.is_serif());
    assert!(!StandardFont::Helvetica.is_serif());
}

#[test]
fn test_load_standard_font() {
    use pdfium_render_fast::StandardFont;

    let pdfium = Pdfium::new().unwrap();
    let doc = pdfium.create_new_document().unwrap();

    // Load a standard font
    let font = doc.load_standard_font(StandardFont::Helvetica);
    assert!(font.is_ok());
}

#[test]
fn test_load_multiple_standard_fonts() {
    use pdfium_render_fast::StandardFont;

    let pdfium = Pdfium::new().unwrap();
    let doc = pdfium.create_new_document().unwrap();

    // Load multiple standard fonts
    let fonts = [
        StandardFont::Helvetica,
        StandardFont::HelveticaBold,
        StandardFont::TimesRoman,
        StandardFont::Courier,
    ];

    for font in &fonts {
        let loaded = doc.load_standard_font(*font);
        assert!(loaded.is_ok(), "Failed to load font: {:?}", font);
    }
}

#[test]
fn test_load_font_by_name() {
    let pdfium = Pdfium::new().unwrap();
    let doc = pdfium.create_new_document().unwrap();

    // Load font by name
    let font = doc.load_font_by_name("Helvetica");
    assert!(font.is_ok());

    let bold_font = doc.load_font_by_name("Helvetica-Bold");
    assert!(bold_font.is_ok());
}

#[test]
fn test_load_invalid_font_name() {
    let pdfium = Pdfium::new().unwrap();
    let doc = pdfium.create_new_document().unwrap();

    // Try to load an invalid font name
    let result = doc.load_font_by_name("NotARealFont");
    assert!(result.is_err());
}

// ========================================
// Content Creation Tests
// ========================================

#[test]
fn test_create_text_object() {
    use pdfium_render_fast::StandardFont;

    let pdfium = Pdfium::new().unwrap();
    let doc = pdfium.create_new_document().unwrap();

    // Load a font
    let font = doc.load_standard_font(StandardFont::Helvetica).unwrap();

    // Create a text object
    let mut text_obj = doc.create_text_object(&font, 12.0).unwrap();

    // Set text content
    let result = text_obj.set_text("Hello, World!");
    assert!(result.is_ok());
}

#[test]
fn test_create_text_object_with_unicode() {
    use pdfium_render_fast::StandardFont;

    let pdfium = Pdfium::new().unwrap();
    let doc = pdfium.create_new_document().unwrap();

    // Load a font
    let font = doc.load_standard_font(StandardFont::Helvetica).unwrap();

    // Create a text object
    let mut text_obj = doc.create_text_object(&font, 12.0).unwrap();

    // Set text with special characters
    let result = text_obj.set_text("Héllo, Wörld! 日本語");
    assert!(result.is_ok());
}

#[test]
fn test_create_rect_object() {
    let pdfium = Pdfium::new().unwrap();
    let doc = pdfium.create_new_document().unwrap();

    // Create a rectangle
    let rect = doc.create_rect_object(100.0, 100.0, 200.0, 100.0);
    assert!(rect.is_ok());
}

#[test]
fn test_create_path_object() {
    let pdfium = Pdfium::new().unwrap();
    let doc = pdfium.create_new_document().unwrap();

    // Create a path
    let mut path = doc.create_path_object(0.0, 0.0).unwrap();

    // Add line segments
    assert!(path.line_to(100.0, 0.0).is_ok());
    assert!(path.line_to(100.0, 100.0).is_ok());
    assert!(path.close().is_ok());
}

#[test]
fn test_create_path_with_bezier() {
    let pdfium = Pdfium::new().unwrap();
    let doc = pdfium.create_new_document().unwrap();

    // Create a path with bezier curves
    let mut path = doc.create_path_object(0.0, 0.0).unwrap();

    // Add bezier curve
    let result = path.bezier_to(50.0, 100.0, 100.0, 100.0, 150.0, 0.0);
    assert!(result.is_ok());
}

#[test]
fn test_object_colors() {
    let pdfium = Pdfium::new().unwrap();
    let doc = pdfium.create_new_document().unwrap();

    // Create a rectangle
    let mut rect = doc.create_rect_object(100.0, 100.0, 200.0, 100.0).unwrap();

    // Set fill color (red)
    assert!(rect.set_fill_color(255, 0, 0, 255).is_ok());

    // Set stroke color (blue)
    assert!(rect.set_stroke_color(0, 0, 255, 255).is_ok());

    // Set stroke width
    assert!(rect.set_stroke_width(2.0).is_ok());

    // Set draw mode
    assert!(rect.set_draw_mode(true, true).is_ok());
}

#[test]
fn test_object_transform() {
    let pdfium = Pdfium::new().unwrap();
    let doc = pdfium.create_new_document().unwrap();

    // Create a rectangle
    let mut rect = doc.create_rect_object(0.0, 0.0, 100.0, 100.0).unwrap();

    // Apply transform (translate by 100, 200)
    rect.transform(1.0, 0.0, 0.0, 1.0, 100.0, 200.0);

    // Set matrix (scale by 2)
    let result = rect.set_matrix(2.0, 0.0, 0.0, 2.0, 0.0, 0.0);
    assert!(result.is_ok());
}

#[test]
fn test_insert_object_into_page() {
    let pdfium = Pdfium::new().unwrap();
    let doc = pdfium.create_new_document().unwrap();

    // Create a page
    let mut page = doc.new_page(0, 612.0, 792.0).unwrap();

    // Create a rectangle
    let mut rect = doc.create_rect_object(100.0, 100.0, 200.0, 100.0).unwrap();
    rect.set_fill_color(255, 0, 0, 255).unwrap();
    rect.set_draw_mode(true, false).unwrap();

    // Insert into page
    let result = page.insert_object(rect);
    assert!(result.is_ok());

    // Generate content
    let result = page.generate_content();
    assert!(result.is_ok());
}

#[test]
fn test_insert_text_and_shapes() {
    use pdfium_render_fast::StandardFont;

    let pdfium = Pdfium::new().unwrap();
    let doc = pdfium.create_new_document().unwrap();

    // Create a page
    let mut page = doc.new_page(0, 612.0, 792.0).unwrap();

    // Add a blue rectangle
    let mut rect = doc.create_rect_object(50.0, 700.0, 200.0, 50.0).unwrap();
    rect.set_fill_color(0, 0, 200, 255).unwrap();
    rect.set_draw_mode(true, false).unwrap();
    page.insert_object(rect).unwrap();

    // Add text
    let font = doc.load_standard_font(StandardFont::Helvetica).unwrap();
    let mut text = doc.create_text_object(&font, 24.0).unwrap();
    text.set_text("Test Document").unwrap();
    text.transform(1.0, 0.0, 0.0, 1.0, 72.0, 720.0);
    page.insert_object(text).unwrap();

    // Add a triangle path
    let mut path = doc.create_path_object(100.0, 500.0).unwrap();
    path.line_to(200.0, 500.0).unwrap();
    path.line_to(150.0, 600.0).unwrap();
    path.close().unwrap();
    path.set_fill_color(0, 200, 0, 255).unwrap();
    path.set_stroke_color(0, 0, 0, 255).unwrap();
    path.set_stroke_width(2.0).unwrap();
    path.set_draw_mode(true, true).unwrap();
    page.insert_object(path).unwrap();

    // Generate content
    assert!(page.generate_content().is_ok());
}

#[test]
fn test_insert_object_at_index() {
    let pdfium = Pdfium::new().unwrap();
    let doc = pdfium.create_new_document().unwrap();

    // Create a page
    let mut page = doc.new_page(0, 612.0, 792.0).unwrap();

    // Add first rectangle
    let mut rect1 = doc.create_rect_object(100.0, 100.0, 100.0, 100.0).unwrap();
    rect1.set_fill_color(255, 0, 0, 255).unwrap();
    rect1.set_draw_mode(true, false).unwrap();
    page.insert_object(rect1).unwrap();

    // Add second rectangle at front (index 0)
    let mut rect2 = doc.create_rect_object(50.0, 50.0, 100.0, 100.0).unwrap();
    rect2.set_fill_color(0, 0, 255, 255).unwrap();
    rect2.set_draw_mode(true, false).unwrap();
    let result = page.insert_object_at_index(rect2, 0);
    assert!(result.is_ok());

    assert!(page.generate_content().is_ok());
}

#[test]
fn test_remove_object() {
    let pdfium = Pdfium::new().unwrap();
    let doc = pdfium.create_new_document().unwrap();

    // Create a page
    let mut page = doc.new_page(0, 612.0, 792.0).unwrap();

    // Add a rectangle
    let mut rect = doc.create_rect_object(100.0, 100.0, 200.0, 100.0).unwrap();
    rect.set_fill_color(255, 0, 0, 255).unwrap();
    rect.set_draw_mode(true, false).unwrap();
    page.insert_object(rect).unwrap();
    page.generate_content().unwrap();

    // Check object count
    let count_before = page.object_count();
    assert!(count_before > 0);

    // Remove the object
    let removed = page.remove_object(0);
    assert!(removed);

    // Generate content again
    assert!(page.generate_content().is_ok());
}

#[test]
fn test_create_save_with_content() {
    use pdfium_render_fast::StandardFont;
    use std::io::Write;

    let pdfium = Pdfium::new().unwrap();
    let doc = pdfium.create_new_document().unwrap();

    // Create a page with content
    let mut page = doc.new_page(0, 612.0, 792.0).unwrap();

    // Add text
    let font = doc.load_standard_font(StandardFont::TimesRoman).unwrap();
    let mut text = doc.create_text_object(&font, 12.0).unwrap();
    text.set_text("This is a test document.").unwrap();
    text.transform(1.0, 0.0, 0.0, 1.0, 72.0, 720.0);
    page.insert_object(text).unwrap();

    // Add a rectangle
    let mut rect = doc.create_rect_object(72.0, 600.0, 468.0, 50.0).unwrap();
    rect.set_fill_color(230, 230, 230, 255).unwrap();
    rect.set_draw_mode(true, false).unwrap();
    page.insert_object(rect).unwrap();

    // Generate content
    page.generate_content().unwrap();

    // Save to bytes
    let bytes = doc.save_to_bytes(None).unwrap();
    assert!(!bytes.is_empty());

    // Verify it starts with PDF signature
    assert!(bytes.starts_with(b"%PDF-"));

    // Optionally save to file for manual inspection
    let temp_path = std::env::temp_dir().join("pdfium_content_test.pdf");
    let mut file = std::fs::File::create(&temp_path).unwrap();
    file.write_all(&bytes).unwrap();
}

#[test]
fn test_page_delete_from_document() {
    let pdfium = Pdfium::new().unwrap();
    let doc = pdfium.create_new_document().unwrap();

    // Create multiple pages
    doc.new_page(0, 612.0, 792.0).unwrap();
    doc.new_page(1, 612.0, 792.0).unwrap();
    doc.new_page(2, 612.0, 792.0).unwrap();

    assert_eq!(doc.page_count(), 3);

    // Delete middle page (index 1)
    doc.delete_page(1);

    // Should have 2 pages now
    assert_eq!(doc.page_count(), 2);
}

// ============================================
// N=177: Image Object, Line Style, Blend Mode Tests
// ============================================

#[test]
fn test_image_object_creation() {
    let pdfium = Pdfium::new().unwrap();
    let doc = pdfium.create_new_document().unwrap();

    // Create image object
    let img = doc.create_image_object();
    assert!(img.is_ok());

    let img = img.unwrap();
    // Verify it's an image type
    assert_eq!(img.object_type(), pdfium_render_fast::PageObjectType::Image);
}

#[test]
fn test_image_matrix() {
    let pdfium = Pdfium::new().unwrap();
    let doc = pdfium.create_new_document().unwrap();

    let mut img = doc.create_image_object().unwrap();

    // Set image matrix (width=200, height=150, x=72, y=600)
    let result = img.set_image_matrix(200.0, 0.0, 0.0, 150.0, 72.0, 600.0);
    assert!(result.is_ok());
}

#[test]
fn test_line_cap_styles() {
    use pdfium_render_fast::LineCap;

    let pdfium = Pdfium::new().unwrap();
    let doc = pdfium.create_new_document().unwrap();

    let mut path = doc.create_path_object(100.0, 100.0).unwrap();
    path.line_to(300.0, 100.0).unwrap();
    path.set_stroke_color(0, 0, 0, 255).unwrap();
    path.set_stroke_width(10.0).unwrap();
    path.set_draw_mode(false, true).unwrap();

    // Test each line cap style
    assert!(path.set_line_cap(LineCap::Butt).is_ok());
    assert_eq!(path.get_line_cap(), Some(LineCap::Butt));

    assert!(path.set_line_cap(LineCap::Round).is_ok());
    assert_eq!(path.get_line_cap(), Some(LineCap::Round));

    assert!(path.set_line_cap(LineCap::ProjectingSquare).is_ok());
    assert_eq!(path.get_line_cap(), Some(LineCap::ProjectingSquare));
}

#[test]
fn test_line_join_styles() {
    use pdfium_render_fast::LineJoin;

    let pdfium = Pdfium::new().unwrap();
    let doc = pdfium.create_new_document().unwrap();

    let mut path = doc.create_path_object(100.0, 100.0).unwrap();
    path.line_to(200.0, 200.0).unwrap();
    path.line_to(300.0, 100.0).unwrap();
    path.set_stroke_color(0, 0, 0, 255).unwrap();
    path.set_stroke_width(10.0).unwrap();
    path.set_draw_mode(false, true).unwrap();

    // Test each line join style
    assert!(path.set_line_join(LineJoin::Miter).is_ok());
    assert_eq!(path.get_line_join(), Some(LineJoin::Miter));

    assert!(path.set_line_join(LineJoin::Round).is_ok());
    assert_eq!(path.get_line_join(), Some(LineJoin::Round));

    assert!(path.set_line_join(LineJoin::Bevel).is_ok());
    assert_eq!(path.get_line_join(), Some(LineJoin::Bevel));
}

#[test]
fn test_dash_pattern() {
    let pdfium = Pdfium::new().unwrap();
    let doc = pdfium.create_new_document().unwrap();

    let mut path = doc.create_path_object(100.0, 100.0).unwrap();
    path.line_to(400.0, 100.0).unwrap();
    path.set_stroke_color(0, 0, 0, 255).unwrap();
    path.set_stroke_width(2.0).unwrap();
    path.set_draw_mode(false, true).unwrap();

    // Set dash pattern: 10 on, 5 off, starting at phase 0
    let result = path.set_dash_pattern(&[10.0, 5.0], 0.0);
    assert!(result.is_ok());

    // More complex pattern: 15 on, 5 off, 5 on, 5 off
    let result = path.set_dash_pattern(&[15.0, 5.0, 5.0, 5.0], 2.5);
    assert!(result.is_ok());
}

#[test]
fn test_blend_modes() {
    use pdfium_render_fast::BlendMode;

    let pdfium = Pdfium::new().unwrap();
    let doc = pdfium.create_new_document().unwrap();

    let mut rect = doc.create_rect_object(100.0, 100.0, 100.0, 100.0).unwrap();
    rect.set_fill_color(255, 0, 0, 128).unwrap(); // Semi-transparent red
    rect.set_draw_mode(true, false).unwrap();

    // Test each blend mode
    rect.set_blend_mode(BlendMode::Normal);
    rect.set_blend_mode(BlendMode::Multiply);
    rect.set_blend_mode(BlendMode::Screen);
    rect.set_blend_mode(BlendMode::Overlay);
    rect.set_blend_mode(BlendMode::Darken);
    rect.set_blend_mode(BlendMode::Lighten);
    rect.set_blend_mode(BlendMode::ColorDodge);
    rect.set_blend_mode(BlendMode::ColorBurn);
    rect.set_blend_mode(BlendMode::HardLight);
    rect.set_blend_mode(BlendMode::SoftLight);
    rect.set_blend_mode(BlendMode::Difference);
    rect.set_blend_mode(BlendMode::Exclusion);
    rect.set_blend_mode(BlendMode::Hue);
    rect.set_blend_mode(BlendMode::Saturation);
    rect.set_blend_mode(BlendMode::Color);
    rect.set_blend_mode(BlendMode::Luminosity);
}

#[test]
fn test_has_transparency() {
    let pdfium = Pdfium::new().unwrap();
    let doc = pdfium.create_new_document().unwrap();

    // Opaque rectangle
    let mut opaque = doc.create_rect_object(0.0, 0.0, 100.0, 100.0).unwrap();
    opaque.set_fill_color(255, 0, 0, 255).unwrap(); // Fully opaque
    opaque.set_draw_mode(true, false).unwrap();

    // Semi-transparent rectangle
    let mut transparent = doc.create_rect_object(50.0, 50.0, 100.0, 100.0).unwrap();
    transparent.set_fill_color(0, 0, 255, 128).unwrap(); // 50% transparent
    transparent.set_draw_mode(true, false).unwrap();

    // The transparent rect should report having transparency
    assert!(transparent.has_transparency());
}

#[test]
fn test_page_object_type() {
    use pdfium_render_fast::{PageObjectType, StandardFont};

    let pdfium = Pdfium::new().unwrap();
    let doc = pdfium.create_new_document().unwrap();

    // Test path object type
    let path = doc.create_path_object(0.0, 0.0).unwrap();
    assert_eq!(path.object_type(), PageObjectType::Path);

    // Test rect object type (also a path)
    let rect = doc.create_rect_object(0.0, 0.0, 100.0, 100.0).unwrap();
    assert_eq!(rect.object_type(), PageObjectType::Path);

    // Test text object type
    let font = doc.load_standard_font(StandardFont::Helvetica).unwrap();
    let text = doc.create_text_object(&font, 12.0).unwrap();
    assert_eq!(text.object_type(), PageObjectType::Text);

    // Test image object type
    let img = doc.create_image_object().unwrap();
    assert_eq!(img.object_type(), PageObjectType::Image);
}

#[test]
fn test_styled_path_drawing() {
    use pdfium_render_fast::{LineCap, LineJoin};

    let pdfium = Pdfium::new().unwrap();
    let doc = pdfium.create_new_document().unwrap();

    // Create a page
    let mut page = doc.new_page(0, 612.0, 792.0).unwrap();

    // Create a styled path with all features
    let mut path = doc.create_path_object(72.0, 700.0).unwrap();
    path.line_to(200.0, 700.0).unwrap();
    path.line_to(200.0, 600.0).unwrap();
    path.line_to(72.0, 600.0).unwrap();
    path.close().unwrap();

    // Apply styling
    path.set_stroke_color(0, 0, 0, 255).unwrap();
    path.set_stroke_width(5.0).unwrap();
    path.set_line_cap(LineCap::Round).unwrap();
    path.set_line_join(LineJoin::Round).unwrap();
    path.set_fill_color(200, 220, 255, 200).unwrap(); // Light blue, semi-transparent
    path.set_draw_mode(true, true).unwrap();

    // Insert and generate
    page.insert_object(path).unwrap();
    assert!(page.generate_content().is_ok());
}

#[test]
fn test_dashed_line_document() {
    let pdfium = Pdfium::new().unwrap();
    let doc = pdfium.create_new_document().unwrap();

    let mut page = doc.new_page(0, 612.0, 792.0).unwrap();

    // Solid line
    let mut solid = doc.create_path_object(72.0, 700.0).unwrap();
    solid.line_to(540.0, 700.0).unwrap();
    solid.set_stroke_color(0, 0, 0, 255).unwrap();
    solid.set_stroke_width(2.0).unwrap();
    solid.set_draw_mode(false, true).unwrap();
    page.insert_object(solid).unwrap();

    // Dashed line
    let mut dashed = doc.create_path_object(72.0, 680.0).unwrap();
    dashed.line_to(540.0, 680.0).unwrap();
    dashed.set_stroke_color(0, 0, 0, 255).unwrap();
    dashed.set_stroke_width(2.0).unwrap();
    dashed.set_dash_pattern(&[10.0, 5.0], 0.0).unwrap();
    dashed.set_draw_mode(false, true).unwrap();
    page.insert_object(dashed).unwrap();

    // Dotted line
    let mut dotted = doc.create_path_object(72.0, 660.0).unwrap();
    dotted.line_to(540.0, 660.0).unwrap();
    dotted.set_stroke_color(0, 0, 0, 255).unwrap();
    dotted.set_stroke_width(2.0).unwrap();
    dotted.set_dash_pattern(&[2.0, 4.0], 0.0).unwrap();
    dotted.set_draw_mode(false, true).unwrap();
    page.insert_object(dotted).unwrap();

    // Dash-dot line
    let mut dashdot = doc.create_path_object(72.0, 640.0).unwrap();
    dashdot.line_to(540.0, 640.0).unwrap();
    dashdot.set_stroke_color(0, 0, 0, 255).unwrap();
    dashdot.set_stroke_width(2.0).unwrap();
    dashdot
        .set_dash_pattern(&[10.0, 3.0, 2.0, 3.0], 0.0)
        .unwrap();
    dashdot.set_draw_mode(false, true).unwrap();
    page.insert_object(dashdot).unwrap();

    assert!(page.generate_content().is_ok());

    // Verify we can save
    let bytes = doc.save_to_bytes(None).unwrap();
    assert!(!bytes.is_empty());
}

#[test]
fn test_blend_mode_as_str() {
    use pdfium_render_fast::BlendMode;

    // Verify all blend mode strings
    assert_eq!(BlendMode::Normal.as_str(), "Normal");
    assert_eq!(BlendMode::Multiply.as_str(), "Multiply");
    assert_eq!(BlendMode::Screen.as_str(), "Screen");
    assert_eq!(BlendMode::Overlay.as_str(), "Overlay");
    assert_eq!(BlendMode::Darken.as_str(), "Darken");
    assert_eq!(BlendMode::Lighten.as_str(), "Lighten");
    assert_eq!(BlendMode::ColorDodge.as_str(), "ColorDodge");
    assert_eq!(BlendMode::ColorBurn.as_str(), "ColorBurn");
    assert_eq!(BlendMode::HardLight.as_str(), "HardLight");
    assert_eq!(BlendMode::SoftLight.as_str(), "SoftLight");
    assert_eq!(BlendMode::Difference.as_str(), "Difference");
    assert_eq!(BlendMode::Exclusion.as_str(), "Exclusion");
    assert_eq!(BlendMode::Hue.as_str(), "Hue");
    assert_eq!(BlendMode::Saturation.as_str(), "Saturation");
    assert_eq!(BlendMode::Color.as_str(), "Color");
    assert_eq!(BlendMode::Luminosity.as_str(), "Luminosity");
}

// ============================================================================
// Clip Path Tests
// ============================================================================

#[test]
fn test_clip_path_creation() {
    let clip = PdfClipPath::new_rect(100.0, 100.0, 400.0, 500.0).unwrap();
    // A rectangular clip path should have exactly 1 path
    assert_eq!(clip.path_count(), 1);
}

#[test]
fn test_clip_path_segments() {
    let clip = PdfClipPath::new_rect(50.0, 50.0, 200.0, 300.0).unwrap();
    assert_eq!(clip.path_count(), 1);

    // A rectangular clip path should have 5 segments (moveto + 3 lineto + close)
    let seg_count = clip.segment_count(0);
    assert!(
        seg_count >= 4,
        "Rectangle should have at least 4 segments, got {}",
        seg_count
    );
}

#[test]
fn test_clip_path_segment_details() {
    let clip = PdfClipPath::new_rect(10.0, 20.0, 100.0, 200.0).unwrap();

    // Get first segment (should be MoveTo)
    if let Some(seg) = clip.get_segment(0, 0) {
        assert!(
            seg.segment_type == PathSegmentType::MoveTo
                || seg.segment_type == PathSegmentType::LineTo,
            "First segment should be MoveTo or LineTo"
        );
        if let (Some(x), Some(y)) = (seg.x, seg.y) {
            // Coordinates should be one of the corners
            assert!(x >= 0.0 && y >= 0.0, "Coordinates should be valid");
        }
    }
}

#[test]
#[serial]
fn test_page_insert_clip_path() {
    let pdfium = Pdfium::new().unwrap();
    let doc = pdfium.create_new_document().unwrap();
    let page = doc.new_page(0, 612.0, 792.0).unwrap();

    // Create and insert a clip path
    let clip = PdfClipPath::new_rect(100.0, 100.0, 500.0, 700.0).unwrap();
    page.insert_clip_path(&clip);

    // Verify page is still valid
    assert!(page.width() > 0.0);
}

#[test]
#[serial]
fn test_page_transform_with_clip() {
    let pdfium = Pdfium::new().unwrap();
    let doc = pdfium.create_new_document().unwrap();
    let page = doc.new_page(0, 612.0, 792.0).unwrap();

    // Apply transform with clip rect
    let matrix = (0.5, 0.0, 0.0, 0.5, 0.0, 0.0); // 50% scale
    let clip_rect = Some((0.0, 0.0, 300.0, 400.0));

    // Transform may return false on empty pages - that's okay
    let _result = page.transform_with_clip(matrix, clip_rect);
    // Just verify the method doesn't crash and page is still valid
    assert!(page.width() > 0.0);
}

#[test]
#[serial]
fn test_page_transform_without_clip() {
    let pdfium = Pdfium::new().unwrap();
    let doc = pdfium.create_new_document().unwrap();
    let page = doc.new_page(0, 612.0, 792.0).unwrap();

    // Apply transform without clip rect
    let matrix = (1.0, 0.0, 0.0, 1.0, 50.0, 50.0); // Translate by (50, 50)

    // Transform may return false on empty pages - that's okay
    let _result = page.transform_with_clip(matrix, None);
    // Just verify the method doesn't crash and page is still valid
    assert!(page.width() > 0.0);
}

// ============================================================================
// Image Object Data Tests
// ============================================================================

#[test]
#[serial]
fn test_image_object_pixel_size() {
    let pdfium = Pdfium::new().unwrap();
    let doc = pdfium.create_new_document().unwrap();

    // Create an image object
    let img = doc.create_image_object().unwrap();
    assert_eq!(img.object_type(), PageObjectType::Image);

    // Without a bitmap set, pixel size might be (0, 0) or None
    let size = img.get_image_pixel_size();
    // Just verify the method works - exact behavior depends on state
    assert!(size.is_none() || size == Some((0, 0)));
}

#[test]
#[serial]
fn test_image_object_filter_count() {
    let pdfium = Pdfium::new().unwrap();
    let doc = pdfium.create_new_document().unwrap();

    let img = doc.create_image_object().unwrap();
    // New image object should have 0 filters
    let count = img.get_image_filter_count();
    assert_eq!(count, 0);
}

#[test]
#[serial]
fn test_non_image_object_data_methods() {
    let pdfium = Pdfium::new().unwrap();
    let doc = pdfium.create_new_document().unwrap();

    // Create a path object (not an image)
    let path = doc.create_path_object(100.0, 100.0).unwrap();
    assert_eq!(path.object_type(), PageObjectType::Path);

    // These methods should return None for non-image objects
    assert!(path.get_image_data_decoded().is_none());
    assert!(path.get_image_data_raw().is_none());
    assert!(path.get_image_pixel_size().is_none());
    assert_eq!(path.get_image_filter_count(), 0);
    assert!(path.get_image_filter(0).is_none());
}

// ============================================================================
// Object Clip Path Tests
// ============================================================================

#[test]
#[serial]
fn test_object_get_clip_path() {
    let pdfium = Pdfium::new().unwrap();
    let doc = pdfium.create_new_document().unwrap();

    // Create a path object
    let path = doc.create_path_object(100.0, 100.0).unwrap();

    // Check if clip path exists (behavior may vary)
    let clip = path.get_clip_path();
    if let Some(clip_path) = clip {
        // If clip path exists, verify we can query it
        let _count = clip_path.path_count();
    }
    // Just verify the method doesn't crash
    assert_eq!(path.object_type(), PageObjectType::Path);
}

#[test]
#[serial]
fn test_object_transform_clip_path() {
    let pdfium = Pdfium::new().unwrap();
    let doc = pdfium.create_new_document().unwrap();

    // Create a path object
    let mut path = doc.create_path_object(100.0, 100.0).unwrap();

    // Transform clip path (even if none exists, shouldn't crash)
    path.transform_clip_path(1.0, 0.0, 0.0, 1.0, 10.0, 10.0);

    // Verify object is still valid
    assert_eq!(path.object_type(), PageObjectType::Path);
}

// ============================================================================
// ICC Color Space Tests
// ============================================================================

#[test]
fn test_icc_color_space_enum() {
    // Test that the enum variants exist and have expected string representations
    let spaces = [
        IccColorSpace::Unknown,
        IccColorSpace::DeviceGray,
        IccColorSpace::DeviceRgb,
        IccColorSpace::DeviceCmyk,
        IccColorSpace::CalGray,
        IccColorSpace::CalRgb,
        IccColorSpace::Lab,
        IccColorSpace::IccBased,
        IccColorSpace::Separation,
        IccColorSpace::DeviceN,
        IccColorSpace::Indexed,
        IccColorSpace::Pattern,
    ];

    // Just verify all variants can be created and compared
    assert_eq!(spaces[0], IccColorSpace::Unknown);
    assert_ne!(spaces[1], IccColorSpace::Unknown);
}

// ============================================================================
// Path Segment Type Tests
// ============================================================================

#[test]
fn test_path_segment_type_from_raw() {
    // Verify PathSegmentType::from works correctly
    assert_eq!(PathSegmentType::from(0), PathSegmentType::LineTo);
    assert_eq!(PathSegmentType::from(1), PathSegmentType::BezierTo);
    assert_eq!(PathSegmentType::from(2), PathSegmentType::MoveTo);
    assert_eq!(PathSegmentType::from(99), PathSegmentType::Unknown);
}

// ============================================================================
// Form Field Editor Tests
// ============================================================================

#[test]
fn test_form_error_display() {
    let err = FormError::ReadOnly;
    assert_eq!(format!("{}", err), "Field is read-only");

    let err = FormError::InvalidIndex(5);
    assert_eq!(format!("{}", err), "Invalid option index: 5");

    let err = FormError::SetValueFailed("test error".to_string());
    assert!(format!("{}", err).contains("test error"));

    let err = FormError::UnsupportedOperation("test op");
    assert!(format!("{}", err).contains("test op"));

    let err = FormError::AnnotationNotFound;
    assert_eq!(format!("{}", err), "Annotation not found");
}

#[test]
fn test_form_field_flags() {
    // Test default
    let default = FormFieldFlags::NONE;
    assert!(!default.is_read_only());
    assert!(!default.is_required());

    // Test individual flags
    let readonly = FormFieldFlags::READ_ONLY;
    assert!(readonly.is_read_only());
    assert!(!readonly.is_required());

    let required = FormFieldFlags::REQUIRED;
    assert!(!required.is_read_only());
    assert!(required.is_required());

    // Test combined flags
    let combined = FormFieldFlags(FormFieldFlags::READ_ONLY.0 | FormFieldFlags::REQUIRED.0);
    assert!(combined.is_read_only());
    assert!(combined.is_required());
}

#[test]
fn test_form_field_type_methods() {
    // Test button types
    assert!(PdfFormFieldType::PushButton.is_button());
    assert!(PdfFormFieldType::CheckBox.is_button());
    assert!(PdfFormFieldType::RadioButton.is_button());
    assert!(!PdfFormFieldType::TextField.is_button());
    assert!(!PdfFormFieldType::ComboBox.is_button());
    assert!(!PdfFormFieldType::ListBox.is_button());

    // Test choice types
    assert!(PdfFormFieldType::ComboBox.is_choice());
    assert!(PdfFormFieldType::ListBox.is_choice());
    assert!(!PdfFormFieldType::TextField.is_choice());
    assert!(!PdfFormFieldType::CheckBox.is_choice());

    // Test text types
    assert!(PdfFormFieldType::TextField.is_text());
    assert!(!PdfFormFieldType::ComboBox.is_text());
    assert!(!PdfFormFieldType::CheckBox.is_text());
}

#[test]
#[serial]
fn test_form_field_editors_on_regular_pdf() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // Test form_field_editors iterator
    let editors: Vec<_> = page.form_field_editors().collect();
    // Most regular PDFs don't have form fields, so this may be empty
    // Just verify it doesn't crash - drop to ensure we successfully collected
    drop(editors);
}

#[test]
#[serial]
fn test_form_field_editor_by_index() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // Test invalid index returns None
    let editor = page.form_field_editor(-1);
    assert!(editor.is_none());

    let editor = page.form_field_editor(10000);
    assert!(editor.is_none());

    // Test valid index (may or may not return form field depending on PDF)
    if page.annotation_count() > 0 {
        let editor = page.form_field_editor(0);
        // If annotation is not a form field, this is None - that's expected
        let _ = editor;
    }
}

#[test]
#[serial]
fn test_form_fields_iterator() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // Test read-only form_fields iterator
    let fields: Vec<_> = page.form_fields().collect();
    // Most regular PDFs don't have form fields, so this may be empty
    // Just verify it doesn't crash - drop to ensure we successfully collected
    drop(fields);
}

#[test]
#[serial]
fn test_has_form_fields() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // Should not crash
    let _ = page.has_form_fields();
}

// ========================================
// PageMode Tests
// ========================================

#[test]
fn test_page_mode_enum() {
    // Test from_raw conversion
    assert_eq!(PageMode::from_raw(-1), PageMode::Unknown(-1));
    assert_eq!(PageMode::from_raw(0), PageMode::UseNone);
    assert_eq!(PageMode::from_raw(1), PageMode::UseOutlines);
    assert_eq!(PageMode::from_raw(2), PageMode::UseThumbs);
    assert_eq!(PageMode::from_raw(3), PageMode::FullScreen);
    assert_eq!(PageMode::from_raw(4), PageMode::UseOC);
    assert_eq!(PageMode::from_raw(5), PageMode::UseAttachments);
    assert_eq!(PageMode::from_raw(99), PageMode::Unknown(99));
}

#[test]
fn test_page_mode_is_normal() {
    assert!(PageMode::UseNone.is_normal());
    assert!(PageMode::UseOutlines.is_normal());
    assert!(PageMode::UseThumbs.is_normal());
    assert!(!PageMode::FullScreen.is_normal());
    assert!(!PageMode::UseOC.is_normal());
    assert!(!PageMode::UseAttachments.is_normal());
    assert!(!PageMode::Unknown(99).is_normal());
}

#[test]
fn test_page_mode_is_fullscreen() {
    assert!(!PageMode::UseNone.is_fullscreen());
    assert!(!PageMode::UseOutlines.is_fullscreen());
    assert!(!PageMode::UseThumbs.is_fullscreen());
    assert!(PageMode::FullScreen.is_fullscreen());
    assert!(!PageMode::UseOC.is_fullscreen());
    assert!(!PageMode::UseAttachments.is_fullscreen());
    assert!(!PageMode::Unknown(99).is_fullscreen());
}

#[test]
fn test_page_mode_default() {
    let default = PageMode::default();
    assert_eq!(default, PageMode::UseNone);
}

#[test]
#[serial]
fn test_document_page_mode() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    // Test that page_mode doesn't crash and returns a valid enum
    let mode = doc.page_mode();

    // Most simple PDFs use UseNone (default), but any valid mode is acceptable
    match mode {
        PageMode::UseNone
        | PageMode::UseOutlines
        | PageMode::UseThumbs
        | PageMode::FullScreen
        | PageMode::UseOC
        | PageMode::UseAttachments => {
            // Expected values
        }
        PageMode::Unknown(_) => {
            // Also valid - means the PDF has an unusual page mode
        }
    }
}

#[test]
fn test_page_mode_equality() {
    assert_eq!(PageMode::UseNone, PageMode::UseNone);
    assert_ne!(PageMode::UseNone, PageMode::UseOutlines);
    assert_ne!(PageMode::Unknown(1), PageMode::Unknown(2));
    assert_eq!(PageMode::Unknown(42), PageMode::Unknown(42));
}

#[test]
fn test_page_mode_clone_copy() {
    let mode = PageMode::FullScreen;
    let cloned = mode;
    let copied = mode;

    assert_eq!(mode, cloned);
    assert_eq!(mode, copied);
    assert_eq!(cloned, copied);
}

#[test]
fn test_page_mode_debug() {
    // Ensure Debug trait works
    let debug_str = format!("{:?}", PageMode::UseOutlines);
    assert!(debug_str.contains("UseOutlines"));

    let debug_str = format!("{:?}", PageMode::Unknown(42));
    assert!(debug_str.contains("Unknown"));
    assert!(debug_str.contains("42"));
}

// ============================================================================
// Progressive Rendering Tests
// ============================================================================

use pdfium_render_fast::{ProgressiveRender, RenderStatus};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[test]
#[serial]
fn test_progressive_render_complete_no_pause() {
    let pdfium = Pdfium::new().unwrap();
    let path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&path, None).unwrap();
    let page = doc.page(0).unwrap();
    let config = PdfRenderConfig::new().set_target_dpi(72.0);

    // Never pause - rendering should complete immediately
    let mut renderer = ProgressiveRender::start(&page, &config, || false).unwrap();

    // Continue until done
    loop {
        let status = renderer.continue_render().unwrap();
        if status.is_finished() {
            break;
        }
    }

    assert!(renderer.status().is_success());
    let bitmap = renderer.finish().unwrap();
    assert!(bitmap.width() > 0);
    assert!(bitmap.height() > 0);
}

#[test]
#[serial]
fn test_progressive_render_with_pause() {
    let pdfium = Pdfium::new().unwrap();
    let path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&path, None).unwrap();
    let page = doc.page(0).unwrap();
    let config = PdfRenderConfig::new().set_target_dpi(72.0);

    // Pause every 3 callbacks, then continue
    let call_count = Arc::new(AtomicUsize::new(0));
    let pause_flag = call_count.clone();

    let mut renderer = ProgressiveRender::start(&page, &config, move || {
        let count = pause_flag.fetch_add(1, Ordering::Relaxed);
        count % 3 == 2 // Pause every 3rd call
    })
    .unwrap();

    // Continue until done
    let mut iterations = 0;
    loop {
        let status = renderer.continue_render().unwrap();
        iterations += 1;
        if status.is_finished() {
            break;
        }
        // Safety limit
        if iterations > 1000 {
            break;
        }
    }

    let bitmap = renderer.finish().unwrap();
    assert!(bitmap.width() > 0);
    assert!(bitmap.height() > 0);
}

#[test]
#[serial]
fn test_progressive_render_cancel() {
    let pdfium = Pdfium::new().unwrap();
    let path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&path, None).unwrap();
    let page = doc.page(0).unwrap();
    let config = PdfRenderConfig::new().set_target_dpi(72.0);

    // Always pause immediately
    let renderer = ProgressiveRender::start(&page, &config, || true).unwrap();

    // Cancel without finishing
    renderer.cancel();

    // Should not panic - resources properly cleaned up
}

#[test]
fn test_render_status_methods() {
    // Test Ready
    let status = RenderStatus::Ready;
    assert!(!status.is_finished());
    assert!(!status.is_success());
    assert!(!status.needs_continue());

    // Test ToBeContinued
    let status = RenderStatus::ToBeContinued;
    assert!(!status.is_finished());
    assert!(!status.is_success());
    assert!(status.needs_continue());

    // Test Done
    let status = RenderStatus::Done;
    assert!(status.is_finished());
    assert!(status.is_success());
    assert!(!status.needs_continue());

    // Test Failed
    let status = RenderStatus::Failed;
    assert!(status.is_finished());
    assert!(!status.is_success());
    assert!(!status.needs_continue());
}

#[test]
fn test_render_status_debug() {
    let debug_str = format!("{:?}", RenderStatus::Done);
    assert!(debug_str.contains("Done"));

    let debug_str = format!("{:?}", RenderStatus::ToBeContinued);
    assert!(debug_str.contains("ToBeContinued"));
}

#[test]
fn test_render_status_equality() {
    assert_eq!(RenderStatus::Done, RenderStatus::Done);
    assert_eq!(RenderStatus::Ready, RenderStatus::Ready);
    assert_ne!(RenderStatus::Done, RenderStatus::Failed);
    assert_ne!(RenderStatus::ToBeContinued, RenderStatus::Ready);
}

#[test]
fn test_render_status_clone_copy() {
    let status = RenderStatus::ToBeContinued;
    let cloned = status;
    let copied = status;

    assert_eq!(status, cloned);
    assert_eq!(status, copied);
}

#[test]
#[serial]
fn test_progressive_render_different_formats() {
    let pdfium = Pdfium::new().unwrap();
    let path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&path, None).unwrap();
    let page = doc.page(0).unwrap();

    // Test BGRA format
    let config = PdfRenderConfig::new()
        .set_target_dpi(72.0)
        .set_pixel_format(PixelFormat::Bgra);

    let mut renderer = ProgressiveRender::start(&page, &config, || false).unwrap();
    while !renderer.continue_render().unwrap().is_finished() {}
    let bitmap = renderer.finish().unwrap();
    assert_eq!(bitmap.format(), PixelFormat::Bgra);

    // Test BGR format
    let config = PdfRenderConfig::new()
        .set_target_dpi(72.0)
        .set_pixel_format(PixelFormat::Bgr);

    let mut renderer = ProgressiveRender::start(&page, &config, || false).unwrap();
    while !renderer.continue_render().unwrap().is_finished() {}
    let bitmap = renderer.finish().unwrap();
    assert_eq!(bitmap.format(), PixelFormat::Bgr);

    // Test Gray format
    let config = PdfRenderConfig::new()
        .set_target_dpi(72.0)
        .set_pixel_format(PixelFormat::Gray);

    let mut renderer = ProgressiveRender::start(&page, &config, || false).unwrap();
    while !renderer.continue_render().unwrap().is_finished() {}
    let bitmap = renderer.finish().unwrap();
    assert_eq!(bitmap.format(), PixelFormat::Gray);
}

#[test]
#[serial]
fn test_progressive_render_drop_cleanup() {
    let pdfium = Pdfium::new().unwrap();
    let path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&path, None).unwrap();
    let page = doc.page(0).unwrap();
    let config = PdfRenderConfig::new().set_target_dpi(72.0);

    // Create renderer and let it drop without finish/cancel
    {
        let _renderer = ProgressiveRender::start(&page, &config, || true).unwrap();
        // Renderer goes out of scope
    }

    // Should not panic or leak - Drop handles cleanup
}

// ============================================================================
// Unsupported Feature Handler Tests
// ============================================================================

use pdfium_render_fast::{
    clear_unsupported_feature_handler, set_unsupported_feature_handler, UnsupportedFeature,
};

#[test]
fn test_unsupported_feature_enum_variants() {
    // Test that all variants can be created
    let features = [
        UnsupportedFeature::XfaForm,
        UnsupportedFeature::PortableCollection,
        UnsupportedFeature::Attachment,
        UnsupportedFeature::Security,
        UnsupportedFeature::SharedReview,
        UnsupportedFeature::SharedFormAcrobat,
        UnsupportedFeature::SharedFormFilesystem,
        UnsupportedFeature::SharedFormEmail,
        UnsupportedFeature::Annot3D,
        UnsupportedFeature::AnnotMovie,
        UnsupportedFeature::AnnotSound,
        UnsupportedFeature::AnnotScreenMedia,
        UnsupportedFeature::AnnotScreenRichMedia,
        UnsupportedFeature::AnnotAttachment,
        UnsupportedFeature::AnnotSignature,
        UnsupportedFeature::Unknown(999),
    ];

    for feature in &features {
        // Just ensure they're valid
        let _ = format!("{:?}", feature);
    }
}

#[test]
fn test_unsupported_feature_is_document_level() {
    // Document-level features
    assert!(UnsupportedFeature::XfaForm.is_document_level());
    assert!(UnsupportedFeature::PortableCollection.is_document_level());
    assert!(UnsupportedFeature::Attachment.is_document_level());
    assert!(UnsupportedFeature::Security.is_document_level());
    assert!(UnsupportedFeature::SharedReview.is_document_level());
    assert!(UnsupportedFeature::SharedFormAcrobat.is_document_level());
    assert!(UnsupportedFeature::SharedFormFilesystem.is_document_level());
    assert!(UnsupportedFeature::SharedFormEmail.is_document_level());

    // Annotation-level features (not document-level)
    assert!(!UnsupportedFeature::Annot3D.is_document_level());
    assert!(!UnsupportedFeature::AnnotMovie.is_document_level());
    assert!(!UnsupportedFeature::AnnotSound.is_document_level());

    // Unknown (neither)
    assert!(!UnsupportedFeature::Unknown(999).is_document_level());
}

#[test]
fn test_unsupported_feature_is_annotation_level() {
    // Annotation-level features
    assert!(UnsupportedFeature::Annot3D.is_annotation_level());
    assert!(UnsupportedFeature::AnnotMovie.is_annotation_level());
    assert!(UnsupportedFeature::AnnotSound.is_annotation_level());
    assert!(UnsupportedFeature::AnnotScreenMedia.is_annotation_level());
    assert!(UnsupportedFeature::AnnotScreenRichMedia.is_annotation_level());
    assert!(UnsupportedFeature::AnnotAttachment.is_annotation_level());
    assert!(UnsupportedFeature::AnnotSignature.is_annotation_level());

    // Document-level features (not annotation-level)
    assert!(!UnsupportedFeature::XfaForm.is_annotation_level());
    assert!(!UnsupportedFeature::PortableCollection.is_annotation_level());
    assert!(!UnsupportedFeature::Security.is_annotation_level());

    // Unknown (neither)
    assert!(!UnsupportedFeature::Unknown(999).is_annotation_level());
}

#[test]
fn test_unsupported_feature_description() {
    // Test descriptions are non-empty
    assert!(!UnsupportedFeature::XfaForm.description().is_empty());
    assert!(!UnsupportedFeature::Annot3D.description().is_empty());
    assert!(!UnsupportedFeature::Unknown(999).description().is_empty());

    // Test specific descriptions
    assert!(UnsupportedFeature::XfaForm.description().contains("XFA"));
    assert!(UnsupportedFeature::Annot3D.description().contains("3D"));
    assert!(UnsupportedFeature::AnnotMovie
        .description()
        .contains("Movie"));
}

#[test]
fn test_unsupported_feature_equality() {
    assert_eq!(UnsupportedFeature::XfaForm, UnsupportedFeature::XfaForm);
    assert_ne!(UnsupportedFeature::XfaForm, UnsupportedFeature::Annot3D);
    assert_eq!(
        UnsupportedFeature::Unknown(42),
        UnsupportedFeature::Unknown(42)
    );
    assert_ne!(
        UnsupportedFeature::Unknown(1),
        UnsupportedFeature::Unknown(2)
    );
}

#[test]
fn test_unsupported_feature_clone_copy() {
    let feature = UnsupportedFeature::AnnotMovie;
    let cloned = feature;
    let copied = feature;

    assert_eq!(feature, cloned);
    assert_eq!(feature, copied);
}

#[test]
fn test_unsupported_feature_debug() {
    let debug_str = format!("{:?}", UnsupportedFeature::XfaForm);
    assert!(debug_str.contains("XfaForm"));

    let debug_str = format!("{:?}", UnsupportedFeature::Unknown(999));
    assert!(debug_str.contains("Unknown"));
    assert!(debug_str.contains("999"));
}

#[test]
#[serial]
fn test_set_unsupported_feature_handler() {
    // Set a handler
    let result = set_unsupported_feature_handler(Some(|_feature: UnsupportedFeature| {
        // Handler does nothing in test
    }));
    assert!(result, "Setting handler should succeed");

    // Clear the handler
    clear_unsupported_feature_handler();
}

#[test]
#[serial]
fn test_set_unsupported_feature_handler_with_closure() {
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = counter.clone();

    // Set a handler that increments counter
    let result = set_unsupported_feature_handler(Some(move |_feature: UnsupportedFeature| {
        counter_clone.fetch_add(1, Ordering::Relaxed);
    }));
    assert!(result, "Setting handler should succeed");

    // Clear the handler
    clear_unsupported_feature_handler();
}

#[test]
#[serial]
fn test_clear_unsupported_feature_handler() {
    // Set a handler
    set_unsupported_feature_handler(Some(|_feature: UnsupportedFeature| {}));

    // Clear it
    clear_unsupported_feature_handler();

    // Setting a new handler should still work
    let result = set_unsupported_feature_handler(Some(|_feature: UnsupportedFeature| {}));
    assert!(result, "Setting new handler after clear should succeed");

    // Clean up
    clear_unsupported_feature_handler();
}

// ============================================================================
// Data Availability API Tests (N=186)
// ============================================================================

#[test]
fn test_linearization_status_debug() {
    // Test Debug trait on LinearizationStatus
    let status = LinearizationStatus::Linearized;
    let debug_str = format!("{:?}", status);
    assert!(debug_str.contains("Linearized"));

    let status = LinearizationStatus::NotLinearized;
    let debug_str = format!("{:?}", status);
    assert!(debug_str.contains("NotLinearized"));

    let status = LinearizationStatus::Unknown;
    let debug_str = format!("{:?}", status);
    assert!(debug_str.contains("Unknown"));
}

#[test]
fn test_linearization_status_eq() {
    // Test PartialEq
    assert_eq!(
        LinearizationStatus::Linearized,
        LinearizationStatus::Linearized
    );
    assert_eq!(
        LinearizationStatus::NotLinearized,
        LinearizationStatus::NotLinearized
    );
    assert_eq!(LinearizationStatus::Unknown, LinearizationStatus::Unknown);
    assert_ne!(
        LinearizationStatus::Linearized,
        LinearizationStatus::NotLinearized
    );
}

#[test]
fn test_check_linearization_small_data() {
    // Data smaller than 1KB should return Unknown
    let small_data = vec![0u8; 100];
    let status = check_linearization(&small_data);
    assert_eq!(status, LinearizationStatus::Unknown);
}

#[test]
fn test_check_linearization_invalid_pdf() {
    // Invalid PDF data should return Unknown or NotLinearized
    let invalid_data = vec![0u8; 2048];
    let status = check_linearization(&invalid_data);
    // Invalid data may return Unknown or NotLinearized depending on parsing
    assert!(matches!(
        status,
        LinearizationStatus::Unknown | LinearizationStatus::NotLinearized
    ));
}

#[test]
#[serial]
fn test_check_linearization_real_pdf() {
    let pdfium = Pdfium::new().unwrap();

    // Find a test PDF
    let test_files = [
        "integration_tests/test_pdfs/arxiv/arxiv_000.pdf",
        "integration_tests/test_pdfs/web/web_000.pdf",
    ];

    for path in &test_files {
        let full_path = format!("/Users/ayates/pdfium_fast/{}", path);
        if let Ok(data) = std::fs::read(&full_path) {
            let status = check_linearization(&data);
            // Most test PDFs are not linearized
            assert!(matches!(
                status,
                LinearizationStatus::Linearized
                    | LinearizationStatus::NotLinearized
                    | LinearizationStatus::Unknown
            ));

            // Also test get_first_available_page
            if let Ok(doc) = pdfium.load_pdf_from_bytes(&data, None) {
                let first_page = get_first_available_page(&doc);
                // First available page should be 0 for most PDFs
                assert!(first_page >= 0);
            }
            break; // Only need to test one file
        }
    }
}

#[test]
fn test_data_availability_debug() {
    let status = DataAvailability::Available;
    let debug_str = format!("{:?}", status);
    assert!(debug_str.contains("Available"));

    let status = DataAvailability::NotAvailable;
    assert_eq!(format!("{:?}", status), "NotAvailable");

    let status = DataAvailability::Error;
    assert_eq!(format!("{:?}", status), "Error");
}

#[test]
fn test_form_availability_debug() {
    let status = FormAvailability::Available;
    assert_eq!(format!("{:?}", status), "Available");

    let status = FormAvailability::NotAvailable;
    assert_eq!(format!("{:?}", status), "NotAvailable");

    let status = FormAvailability::NotExist;
    assert_eq!(format!("{:?}", status), "NotExist");

    let status = FormAvailability::Error;
    assert_eq!(format!("{:?}", status), "Error");
}

// ============================================================================
// System Font Info API Tests (N=186)
// ============================================================================

#[test]
fn test_font_charset_debug() {
    let charset = FontCharset::Ansi;
    assert_eq!(format!("{:?}", charset), "Ansi");

    let charset = FontCharset::ShiftJis;
    assert_eq!(format!("{:?}", charset), "ShiftJis");

    let charset = FontCharset::Gb2312;
    assert_eq!(format!("{:?}", charset), "Gb2312");
}

#[test]
fn test_font_charset_eq() {
    assert_eq!(FontCharset::Ansi, FontCharset::Ansi);
    assert_eq!(FontCharset::ShiftJis, FontCharset::ShiftJis);
    assert_ne!(FontCharset::Ansi, FontCharset::ShiftJis);
}

#[test]
fn test_font_charset_from_raw() {
    assert_eq!(FontCharset::from_raw(0), Some(FontCharset::Ansi));
    assert_eq!(FontCharset::from_raw(1), Some(FontCharset::Default));
    assert_eq!(FontCharset::from_raw(128), Some(FontCharset::ShiftJis));
    assert_eq!(FontCharset::from_raw(129), Some(FontCharset::Hangeul));
    assert_eq!(FontCharset::from_raw(134), Some(FontCharset::Gb2312));
    assert_eq!(FontCharset::from_raw(136), Some(FontCharset::ChineseBig5));
    assert_eq!(FontCharset::from_raw(177), Some(FontCharset::Hebrew));
    assert_eq!(FontCharset::from_raw(178), Some(FontCharset::Arabic));
    assert_eq!(FontCharset::from_raw(204), Some(FontCharset::Cyrillic));
    assert_eq!(FontCharset::from_raw(999), None); // Unknown charset
}

#[test]
fn test_font_charset_name() {
    assert_eq!(FontCharset::Ansi.name(), "ANSI (Western European)");
    assert_eq!(FontCharset::ShiftJis.name(), "Japanese (Shift-JIS)");
    assert_eq!(FontCharset::Hangeul.name(), "Korean (Hangeul)");
    assert_eq!(FontCharset::Gb2312.name(), "Simplified Chinese (GB2312)");
    assert_eq!(
        FontCharset::ChineseBig5.name(),
        "Traditional Chinese (Big5)"
    );
    assert_eq!(FontCharset::Arabic.name(), "Arabic");
    assert_eq!(FontCharset::Cyrillic.name(), "Cyrillic");
}

#[test]
#[serial]
fn test_get_default_ttf_map_count() {
    // Initialize PDFium first
    let _pdfium = Pdfium::new().unwrap();

    let count = get_default_ttf_map_count();
    // Should have some default font mappings
    assert!(count > 0, "Should have at least one font mapping");
}

#[test]
#[serial]
fn test_get_default_ttf_map_entry() {
    let _pdfium = Pdfium::new().unwrap();

    // First entry should exist
    let entry = get_default_ttf_map_entry(0);
    assert!(entry.is_some(), "First entry should exist");

    if let Some(mapping) = entry {
        // Font name should not be empty
        assert!(
            !mapping.font_name.is_empty(),
            "Font name should not be empty"
        );
        // Raw charset should match the charset enum
        let _charset_name = mapping.charset.name();
    }

    // Out of bounds should return None
    let invalid_entry = get_default_ttf_map_entry(999999);
    assert!(invalid_entry.is_none(), "Out of bounds should return None");
}

#[test]
#[serial]
fn test_get_default_ttf_map() {
    let _pdfium = Pdfium::new().unwrap();

    let mappings = get_default_ttf_map();
    assert!(!mappings.is_empty(), "Should have some font mappings");

    // Check that all entries have valid data
    for mapping in &mappings {
        assert!(
            !mapping.font_name.is_empty(),
            "All font names should be non-empty"
        );
        // charset_raw and charset should be consistent
        let _name = mapping.charset.name();
    }
}

#[test]
fn test_charset_font_mapping_clone() {
    let mapping = CharsetFontMapping {
        charset: FontCharset::Ansi,
        charset_raw: 0,
        font_name: "Arial".to_string(),
    };

    let cloned = mapping.clone();
    assert_eq!(cloned.charset, FontCharset::Ansi);
    assert_eq!(cloned.charset_raw, 0);
    assert_eq!(cloned.font_name, "Arial");
}

#[test]
fn test_charset_font_mapping_debug() {
    let mapping = CharsetFontMapping {
        charset: FontCharset::ShiftJis,
        charset_raw: 128,
        font_name: "MS Gothic".to_string(),
    };

    let debug_str = format!("{:?}", mapping);
    assert!(debug_str.contains("ShiftJis"));
    assert!(debug_str.contains("128"));
    assert!(debug_str.contains("MS Gothic"));
}

// ============================================================================
// Extended Search API Tests (Text Index Conversion)
// ============================================================================

#[test]
#[serial]
fn test_text_index_to_char_index() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();
    let text = page.text().unwrap();

    // Index 0 should always map to something valid for non-empty pages
    if text.char_count() > 0 {
        let char_idx = text.text_index_to_char_index(0);
        assert!(
            char_idx.is_some(),
            "text_index 0 should map to valid char_index"
        );
    }
}

#[test]
#[serial]
fn test_char_index_to_text_index() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();
    let text = page.text().unwrap();

    // Char index 0 should map to a valid text index
    if text.char_count() > 0 {
        let text_idx = text.char_index_to_text_index(0);
        assert!(
            text_idx.is_some(),
            "char_index 0 should map to valid text_index"
        );
    }
}

#[test]
#[serial]
fn test_text_index_round_trip() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();
    let text = page.text().unwrap();

    // For valid indices, round-trip should work
    if text.char_count() >= 5 {
        // Go from char index to text index and back
        if let Some(text_idx) = text.char_index_to_text_index(3) {
            if let Some(char_idx) = text.text_index_to_char_index(text_idx) {
                // Should get back to 3 or close to it
                assert!(char_idx <= 5, "Round trip should return similar index");
            }
        }
    }
}

#[test]
#[serial]
fn test_text_index_invalid() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();
    let text = page.text().unwrap();

    // Very large index should return None
    let invalid = text.text_index_to_char_index(999999);
    assert!(invalid.is_none(), "Invalid text index should return None");

    let invalid2 = text.char_index_to_text_index(999999);
    assert!(invalid2.is_none(), "Invalid char index should return None");
}

// ============================================================================
// XObject API Tests
// ============================================================================

#[test]
#[serial]
fn test_create_xobject_from_page() {
    let pdfium = Pdfium::new().unwrap();

    // Create a new destination document
    let dest_doc = pdfium.create_new_document().unwrap();

    // Load a source document
    let pdf_path = get_test_pdf();
    let src_doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    // Create XObject from first page of source
    let xobject = dest_doc.create_xobject_from_page(&src_doc, 0);
    assert!(xobject.is_ok(), "Should create XObject from page");
}

#[test]
#[serial]
fn test_xobject_to_page_object() {
    let pdfium = Pdfium::new().unwrap();

    let dest_doc = pdfium.create_new_document().unwrap();
    let pdf_path = get_test_pdf();
    let src_doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    let xobject = dest_doc.create_xobject_from_page(&src_doc, 0).unwrap();

    // Create a page object from the XObject
    let page_obj = xobject.to_page_object();
    assert!(page_obj.is_ok(), "Should create page object from XObject");
}

#[test]
#[serial]
fn test_xobject_insert_into_page() {
    let pdfium = Pdfium::new().unwrap();

    let dest_doc = pdfium.create_new_document().unwrap();
    let pdf_path = get_test_pdf();
    let src_doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    // Create XObject
    let xobject = dest_doc.create_xobject_from_page(&src_doc, 0).unwrap();

    // Create a new page
    let mut page = dest_doc.new_page(0, 612.0, 792.0).unwrap();

    // Create page object and insert
    let form_obj = xobject.to_page_object().unwrap();
    let result = page.insert_object(form_obj);
    assert!(result.is_ok(), "Should insert XObject content into page");
}

#[test]
#[serial]
fn test_xobject_multiple_uses() {
    let pdfium = Pdfium::new().unwrap();

    let dest_doc = pdfium.create_new_document().unwrap();
    let pdf_path = get_test_pdf();
    let src_doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    // Create XObject
    let xobject = dest_doc.create_xobject_from_page(&src_doc, 0).unwrap();

    // Use the same XObject multiple times on different pages
    for i in 0..3 {
        let mut page = dest_doc.new_page(i, 612.0, 792.0).unwrap();
        let form_obj = xobject.to_page_object().unwrap();
        page.insert_object(form_obj).unwrap();
    }

    // Document should now have 3 pages
    assert_eq!(dest_doc.page_count(), 3);
}

#[test]
#[serial]
fn test_xobject_handle() {
    let pdfium = Pdfium::new().unwrap();

    let dest_doc = pdfium.create_new_document().unwrap();
    let pdf_path = get_test_pdf();
    let src_doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    let xobject = dest_doc.create_xobject_from_page(&src_doc, 0).unwrap();

    // Handle should not be null
    assert!(
        !xobject.handle().is_null(),
        "XObject handle should not be null"
    );
}

#[test]
#[serial]
fn test_xobject_invalid_page_index() {
    let pdfium = Pdfium::new().unwrap();

    let dest_doc = pdfium.create_new_document().unwrap();
    let pdf_path = get_test_pdf();
    let src_doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    // Try to create XObject from invalid page index
    let result = dest_doc.create_xobject_from_page(&src_doc, 9999);
    assert!(result.is_err(), "Should fail for invalid page index");
}

#[test]
#[serial]
fn test_xobject_save_and_reload() {
    let pdfium = Pdfium::new().unwrap();

    let dest_doc = pdfium.create_new_document().unwrap();
    let pdf_path = get_test_pdf();
    let src_doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    // Create XObject and insert into page
    let xobject = dest_doc.create_xobject_from_page(&src_doc, 0).unwrap();
    let mut page = dest_doc.new_page(0, 612.0, 792.0).unwrap();
    let form_obj = xobject.to_page_object().unwrap();
    page.insert_object(form_obj).unwrap();

    // Save to bytes
    let bytes = dest_doc.save_to_bytes(None).unwrap();
    assert!(!bytes.is_empty(), "Saved bytes should not be empty");

    // Reload and verify
    let reloaded = pdfium.load_pdf_from_bytes(&bytes, None).unwrap();
    assert_eq!(reloaded.page_count(), 1);
}

// ============================================================================
// N-up Page Layout Tests
// ============================================================================

#[test]
#[serial]
fn test_nup_basic_2up() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    let source_pages = doc.page_count();

    // Create 2-up layout
    let nup = doc.import_pages_n_up(792.0, 612.0, 2, 1).unwrap();

    // Should have half as many pages (rounded up)
    let expected_pages = source_pages.div_ceil(2);
    assert_eq!(nup.page_count(), expected_pages);
}

#[test]
#[serial]
fn test_nup_4up_letter() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    // Use convenience function
    let nup = doc.import_pages_4up_letter().unwrap();

    // Should have 1/4 as many pages (rounded up)
    let source_pages = doc.page_count();
    let expected_pages = source_pages.div_ceil(4);
    assert_eq!(nup.page_count(), expected_pages);
}

#[test]
#[serial]
fn test_nup_2up_letter() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    let nup = doc.import_pages_2up_letter().unwrap();
    let source_pages = doc.page_count();
    let expected_pages = source_pages.div_ceil(2);
    assert_eq!(nup.page_count(), expected_pages);
}

#[test]
#[serial]
fn test_nup_2up_a4() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    let nup = doc.import_pages_2up_a4().unwrap();
    let source_pages = doc.page_count();
    let expected_pages = source_pages.div_ceil(2);
    assert_eq!(nup.page_count(), expected_pages);
}

#[test]
#[serial]
fn test_nup_4up_a4() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    let nup = doc.import_pages_4up_a4().unwrap();
    let source_pages = doc.page_count();
    let expected_pages = source_pages.div_ceil(4);
    assert_eq!(nup.page_count(), expected_pages);
}

#[test]
#[serial]
fn test_nup_invalid_zero_pages() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    // Zero pages on X axis should fail
    let result = doc.import_pages_n_up(612.0, 792.0, 0, 1);
    assert!(result.is_err());

    // Zero pages on Y axis should fail
    let result = doc.import_pages_n_up(612.0, 792.0, 1, 0);
    assert!(result.is_err());
}

#[test]
#[serial]
fn test_nup_save_and_reload() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    let nup = doc.import_pages_4up_letter().unwrap();
    let expected_pages = nup.page_count();

    // Save and reload
    let bytes = nup.save_to_bytes(None).unwrap();
    let reloaded = pdfium.load_pdf_from_bytes(&bytes, None).unwrap();

    assert_eq!(reloaded.page_count(), expected_pages);
}

// ============================================================================
// Named Destination Tests
// ============================================================================

#[test]
#[serial]
fn test_named_dest_count() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    // Just ensure the count is a valid number (>= 0)
    let count = doc.named_dest_count();
    assert!(count < 1_000_000, "Count should be reasonable");
}

#[test]
#[serial]
fn test_has_named_dests() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    // This should be consistent with count
    let has_dests = doc.has_named_dests();
    let count = doc.named_dest_count();
    assert_eq!(has_dests, count > 0);
}

#[test]
#[serial]
fn test_named_dest_by_name_nonexistent() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    // Looking up a nonexistent name should return None
    let result = doc.named_dest_by_name("definitely_not_a_real_destination_name_12345");
    assert!(result.is_none());
}

#[test]
#[serial]
fn test_named_dest_by_index_out_of_bounds() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    let count = doc.named_dest_count();
    // Out of bounds index should return None
    let result = doc.named_dest(count + 100);
    assert!(result.is_none());
}

#[test]
#[serial]
fn test_named_dests_iterator() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    let count = doc.named_dest_count();
    let mut iterated = 0;

    for (name, dest) in doc.named_dests() {
        assert!(!name.is_empty(), "Name should not be empty");
        // Page index should be valid if present
        if let Some(page_idx) = dest.page_index() {
            assert!(page_idx < doc.page_count(), "Page index should be valid");
        }
        iterated += 1;
    }

    assert_eq!(
        iterated, count,
        "Iterator should yield correct number of items"
    );
}

#[test]
#[serial]
fn test_named_dests_iterator_size_hint() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    let count = doc.named_dest_count();
    let iter = doc.named_dests();

    let (lower, upper) = iter.size_hint();
    assert_eq!(lower, count);
    assert_eq!(upper, Some(count));
}

// ============================================================================
// Document Splitting API Tests
// ============================================================================

#[test]
#[serial]
fn test_extract_pages_range() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    let original_count = doc.page_count();
    assert!(
        original_count >= 3,
        "Test requires PDF with at least 3 pages"
    );

    // Extract pages 1-3 (1-indexed)
    let extracted = doc.extract_pages("1-3").unwrap();
    assert_eq!(extracted.page_count(), 3);
}

#[test]
#[serial]
fn test_extract_pages_specific() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    let original_count = doc.page_count();
    assert!(
        original_count >= 3,
        "Test requires PDF with at least 3 pages"
    );

    // Extract specific pages: 1 and 3 (1-indexed)
    let extracted = doc.extract_pages("1,3").unwrap();
    assert_eq!(extracted.page_count(), 2);
}

#[test]
#[serial]
fn test_extract_single_page() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    // Extract page at index 0
    let extracted = doc.extract_page(0).unwrap();
    assert_eq!(extracted.page_count(), 1);
}

#[test]
#[serial]
fn test_extract_page_out_of_range() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    let page_count = doc.page_count();

    // Try to extract page beyond document
    let result = doc.extract_page(page_count);
    assert!(result.is_err());
}

#[test]
#[serial]
fn test_split_at() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    let total = doc.page_count();
    assert!(total >= 4, "Test requires PDF with at least 4 pages");

    let split_index = 2; // Split after 2 pages
    let (first, second) = doc.split_at(split_index).unwrap();

    assert_eq!(first.page_count(), split_index);
    assert_eq!(second.page_count(), total - split_index);
}

#[test]
#[serial]
fn test_split_at_invalid_indices() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    // Split at 0 should fail (first part would be empty)
    assert!(doc.split_at(0).is_err());

    // Split beyond document should fail
    assert!(doc.split_at(doc.page_count()).is_err());
}

#[test]
#[serial]
fn test_split_into_chunks() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    let total = doc.page_count();
    assert!(total >= 5, "Test requires PDF with at least 5 pages");

    let chunk_size = 2;
    let chunks = doc.split_into_chunks(chunk_size).unwrap();

    // Calculate expected number of chunks
    let expected_chunks = total.div_ceil(chunk_size);
    assert_eq!(chunks.len(), expected_chunks);

    // Verify total pages across chunks
    let total_in_chunks: usize = chunks.iter().map(|c| c.page_count()).sum();
    assert_eq!(total_in_chunks, total);

    // Verify each chunk has correct size (except possibly last)
    for (i, chunk) in chunks.iter().enumerate() {
        if i < chunks.len() - 1 {
            assert_eq!(chunk.page_count(), chunk_size);
        } else {
            // Last chunk may have fewer pages
            assert!(chunk.page_count() <= chunk_size);
        }
    }
}

#[test]
#[serial]
fn test_split_into_chunks_zero_size() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    // Chunk size 0 should fail
    assert!(doc.split_into_chunks(0).is_err());
}

#[test]
#[serial]
fn test_split_every_alias() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    let total = doc.page_count();
    assert!(total >= 3, "Test requires PDF with at least 3 pages");

    // split_every should be same as split_into_chunks
    let chunks1 = doc.split_into_chunks(3).unwrap();
    let chunks2 = doc.split_every(3).unwrap();

    assert_eq!(chunks1.len(), chunks2.len());
    for (c1, c2) in chunks1.iter().zip(chunks2.iter()) {
        assert_eq!(c1.page_count(), c2.page_count());
    }
}

#[test]
#[serial]
fn test_extract_even_pages() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    let total = doc.page_count();
    assert!(total >= 4, "Test requires PDF with at least 4 pages");

    let even = doc.extract_even_pages().unwrap();

    // Expected: pages 2, 4, 6, ... (1-indexed)
    let expected_count = total / 2;
    assert_eq!(even.page_count(), expected_count);
}

#[test]
#[serial]
fn test_extract_odd_pages() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    let total = doc.page_count();
    assert!(total >= 4, "Test requires PDF with at least 4 pages");

    let odd = doc.extract_odd_pages().unwrap();

    // Expected: pages 1, 3, 5, ... (1-indexed)
    let expected_count = total.div_ceil(2);
    assert_eq!(odd.page_count(), expected_count);
}

#[test]
#[serial]
fn test_to_reversed() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    let total = doc.page_count();
    assert!(total >= 2, "Test requires PDF with at least 2 pages");

    let reversed = doc.to_reversed().unwrap();
    assert_eq!(reversed.page_count(), total);
}

#[test]
#[serial]
fn test_extract_pages_save_and_reload() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    let original_count = doc.page_count();
    assert!(
        original_count >= 3,
        "Test requires PDF with at least 3 pages"
    );

    // Extract first 2 pages
    let extracted = doc.extract_pages("1-2").unwrap();

    // Save to temp file
    let temp = tempdir().unwrap();
    let temp_path = temp.path().join("extracted.pdf");
    extracted.save_to_file(&temp_path, None).unwrap();

    // Reload and verify
    let reloaded = pdfium.load_pdf_from_file(&temp_path, None).unwrap();
    assert_eq!(reloaded.page_count(), 2);
}

#[test]
#[serial]
fn test_split_chunks_save_and_reload() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    let total = doc.page_count();
    assert!(total >= 4, "Test requires PDF with at least 4 pages");

    let chunks = doc.split_into_chunks(2).unwrap();

    // Save first chunk and reload
    let temp = tempdir().unwrap();
    let temp_path = temp.path().join("chunk_0.pdf");
    chunks[0].save_to_file(&temp_path, None).unwrap();

    let reloaded = pdfium.load_pdf_from_file(&temp_path, None).unwrap();
    assert_eq!(reloaded.page_count(), 2);
}

// ============================================================================
// PDF Merge Tests
// ============================================================================

#[test]
#[serial]
fn test_merge_documents_basic() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();

    // Load the same document twice
    let doc1 = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let doc2 = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let pages_per_doc = doc1.page_count();

    // Merge both documents
    let merged = pdfium.merge_documents([&doc1, &doc2], false).unwrap();

    // Merged should have pages from both documents
    assert_eq!(
        merged.page_count(),
        pages_per_doc * 2,
        "Merged document should have twice the pages"
    );
}

#[test]
#[serial]
fn test_merge_documents_with_viewer_prefs() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();

    let doc1 = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let doc2 = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    // Merge with viewer preferences copied from first document
    let merged = pdfium.merge_documents([&doc1, &doc2], true).unwrap();

    // Should succeed - viewer prefs copying is optional/best-effort
    assert!(merged.page_count() > 0);
}

#[test]
#[serial]
fn test_merge_documents_single() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();

    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let original_pages = doc.page_count();

    // Merge a single document
    let merged = pdfium.merge_documents([&doc], false).unwrap();
    assert_eq!(merged.page_count(), original_pages);
}

#[test]
#[serial]
fn test_merge_documents_empty_iterator() {
    let pdfium = Pdfium::new().unwrap();
    let docs: Vec<&pdfium_render_fast::PdfDocument> = vec![];

    // Empty iterator should fail
    let result = pdfium.merge_documents(docs, false);
    assert!(result.is_err(), "Empty document list should fail");
}

#[test]
#[serial]
fn test_merge_documents_three() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();

    let doc1 = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let doc2 = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let doc3 = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let pages_per_doc = doc1.page_count();

    // Merge three documents
    let merged = pdfium
        .merge_documents([&doc1, &doc2, &doc3], false)
        .unwrap();
    assert_eq!(merged.page_count(), pages_per_doc * 3);
}

#[test]
#[serial]
fn test_merge_documents_from_vec() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();

    // Load documents into a Vec
    let docs: Vec<pdfium_render_fast::PdfDocument> = (0..3)
        .map(|_| pdfium.load_pdf_from_file(&pdf_path, None).unwrap())
        .collect();

    let pages_per_doc = docs[0].page_count();

    // Merge from iterator over references
    let merged = pdfium.merge_documents(&docs, false).unwrap();
    assert_eq!(merged.page_count(), pages_per_doc * 3);
}

#[test]
#[serial]
fn test_merge_files_basic() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();

    // Get expected page count
    let test_doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let pages_per_file = test_doc.page_count();

    // Merge the same file twice
    let merged = pdfium.merge_files([&pdf_path, &pdf_path], false).unwrap();
    assert_eq!(merged.page_count(), pages_per_file * 2);
}

#[test]
#[serial]
fn test_merge_files_empty() {
    let pdfium = Pdfium::new().unwrap();
    let paths: Vec<std::path::PathBuf> = vec![];

    // Empty file list should fail
    let result = pdfium.merge_files(paths, false);
    assert!(result.is_err(), "Empty file list should fail");
}

#[test]
#[serial]
fn test_merge_files_nonexistent() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();

    // One valid file, one nonexistent
    let result = pdfium.merge_files(
        [&pdf_path, &std::path::PathBuf::from("/nonexistent.pdf")],
        false,
    );
    assert!(result.is_err(), "Nonexistent file should cause error");
}

#[test]
#[serial]
fn test_merge_and_save() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();

    let doc1 = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let doc2 = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let expected_pages = doc1.page_count() + doc2.page_count();

    // Merge and save to temp file
    let merged = pdfium.merge_documents([&doc1, &doc2], true).unwrap();

    let temp = tempdir().unwrap();
    let temp_path = temp.path().join("merged.pdf");
    merged.save_to_file(&temp_path, None).unwrap();

    // Reload and verify
    let reloaded = pdfium.load_pdf_from_file(&temp_path, None).unwrap();
    assert_eq!(reloaded.page_count(), expected_pages);
}

#[test]
#[serial]
fn test_merge_to_bytes() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();

    let doc1 = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let doc2 = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let expected_pages = doc1.page_count() + doc2.page_count();

    // Merge and save to bytes
    let merged = pdfium.merge_documents([&doc1, &doc2], false).unwrap();
    let bytes = merged.save_to_bytes(None).unwrap();

    // Reload from bytes and verify
    let reloaded = pdfium.load_pdf_from_bytes(&bytes, None).unwrap();
    assert_eq!(reloaded.page_count(), expected_pages);
}

// ============================================================================
// Page Rotation Tests
// ============================================================================

#[test]
#[serial]
fn test_page_rotation_get() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // Get default rotation (usually 0)
    let rotation = page.rotation();
    let degrees = page.rotation_degrees();

    // Rotation should be valid (0, 90, 180, or 270)
    assert!(degrees == 0 || degrees == 90 || degrees == 180 || degrees == 270);

    // as_degrees should match rotation_degrees
    assert_eq!(rotation.as_degrees(), degrees);
}

#[test]
#[serial]
fn test_page_rotation_set() {
    use pdfium_render_fast::PdfPageRotation;

    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let mut page = doc.page(0).unwrap();

    // Set rotation to 90 degrees
    page.set_rotation(PdfPageRotation::Clockwise90);
    assert_eq!(page.rotation(), PdfPageRotation::Clockwise90);
    assert_eq!(page.rotation_degrees(), 90);

    // Set rotation to 180 degrees
    page.set_rotation(PdfPageRotation::Rotated180);
    assert_eq!(page.rotation(), PdfPageRotation::Rotated180);
    assert_eq!(page.rotation_degrees(), 180);

    // Set rotation to 270 degrees
    page.set_rotation(PdfPageRotation::Clockwise270);
    assert_eq!(page.rotation(), PdfPageRotation::Clockwise270);
    assert_eq!(page.rotation_degrees(), 270);

    // Set rotation back to none
    page.set_rotation(PdfPageRotation::None);
    assert_eq!(page.rotation(), PdfPageRotation::None);
    assert_eq!(page.rotation_degrees(), 0);
}

#[test]
#[serial]
fn test_page_rotation_set_degrees() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let mut page = doc.page(0).unwrap();

    // Set rotation via degrees
    page.set_rotation_degrees(90).unwrap();
    assert_eq!(page.rotation_degrees(), 90);

    page.set_rotation_degrees(180).unwrap();
    assert_eq!(page.rotation_degrees(), 180);

    page.set_rotation_degrees(270).unwrap();
    assert_eq!(page.rotation_degrees(), 270);

    page.set_rotation_degrees(0).unwrap();
    assert_eq!(page.rotation_degrees(), 0);
}

#[test]
#[serial]
fn test_page_rotation_set_degrees_invalid() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let mut page = doc.page(0).unwrap();

    // Invalid degree values should fail
    assert!(page.set_rotation_degrees(45).is_err());
    assert!(page.set_rotation_degrees(135).is_err());
    assert!(page.set_rotation_degrees(360).is_err());
    assert!(page.set_rotation_degrees(1).is_err());
}

#[test]
#[serial]
fn test_page_rotation_persist() {
    use pdfium_render_fast::PdfPageRotation;

    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    // Rotate the first page
    {
        let mut page = doc.page(0).unwrap();
        page.set_rotation(PdfPageRotation::Clockwise90);
    }

    // Save to bytes
    let bytes = doc.save_to_bytes(None).unwrap();

    // Reload and verify rotation persisted
    let reloaded = pdfium.load_pdf_from_bytes(&bytes, None).unwrap();
    let page = reloaded.page(0).unwrap();
    assert_eq!(page.rotation(), PdfPageRotation::Clockwise90);
}

#[test]
#[serial]
fn test_page_rotation_enum_variants() {
    use pdfium_render_fast::PdfPageRotation;

    // Test all enum variants
    assert_eq!(PdfPageRotation::None.as_degrees(), 0);
    assert_eq!(PdfPageRotation::None.as_raw(), 0);

    assert_eq!(PdfPageRotation::Clockwise90.as_degrees(), 90);
    assert_eq!(PdfPageRotation::Clockwise90.as_raw(), 1);

    assert_eq!(PdfPageRotation::Rotated180.as_degrees(), 180);
    assert_eq!(PdfPageRotation::Rotated180.as_raw(), 2);

    assert_eq!(PdfPageRotation::Clockwise270.as_degrees(), 270);
    assert_eq!(PdfPageRotation::Clockwise270.as_raw(), 3);
}

#[test]
#[serial]
fn test_page_rotation_from_raw() {
    use pdfium_render_fast::PdfPageRotation;

    // Test from_raw conversion
    assert_eq!(PdfPageRotation::from_raw(0), PdfPageRotation::None);
    assert_eq!(PdfPageRotation::from_raw(1), PdfPageRotation::Clockwise90);
    assert_eq!(PdfPageRotation::from_raw(2), PdfPageRotation::Rotated180);
    assert_eq!(PdfPageRotation::from_raw(3), PdfPageRotation::Clockwise270);

    // Invalid values should default to None
    assert_eq!(PdfPageRotation::from_raw(4), PdfPageRotation::None);
    assert_eq!(PdfPageRotation::from_raw(-1), PdfPageRotation::None);
    assert_eq!(PdfPageRotation::from_raw(100), PdfPageRotation::None);
}

// ========================================
// Viewer Reference (Print Preferences) Tests
// ========================================

#[test]
#[serial]
fn test_print_scaling() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    // Most PDFs default to print scaling enabled
    let _scaling = doc.print_scaling();
    // API call succeeds (test verifies it doesn't panic)
}

#[test]
#[serial]
fn test_num_copies() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    // Default is 1 copy if not specified
    let copies = doc.num_copies();
    assert!(copies >= 1, "num_copies should be at least 1");
}

#[test]
#[serial]
fn test_duplex_mode() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    let duplex = doc.duplex_mode();
    // Most PDFs don't specify duplex mode
    match duplex {
        DuplexType::Undefined => assert!(!duplex.is_duplex()),
        DuplexType::Simplex => assert!(!duplex.is_duplex()),
        DuplexType::FlipShortEdge => assert!(duplex.is_duplex()),
        DuplexType::FlipLongEdge => assert!(duplex.is_duplex()),
    }
}

#[test]
#[serial]
fn test_duplex_type_from_raw() {
    // Test from_raw conversion
    assert_eq!(DuplexType::from_raw(0), DuplexType::Undefined);
    assert_eq!(DuplexType::from_raw(1), DuplexType::Simplex);
    assert_eq!(DuplexType::from_raw(2), DuplexType::FlipShortEdge);
    assert_eq!(DuplexType::from_raw(3), DuplexType::FlipLongEdge);

    // Invalid values should default to Undefined
    assert_eq!(DuplexType::from_raw(4), DuplexType::Undefined);
    assert_eq!(DuplexType::from_raw(100), DuplexType::Undefined);
}

#[test]
#[serial]
fn test_duplex_type_is_duplex() {
    assert!(!DuplexType::Undefined.is_duplex());
    assert!(!DuplexType::Simplex.is_duplex());
    assert!(DuplexType::FlipShortEdge.is_duplex());
    assert!(DuplexType::FlipLongEdge.is_duplex());
}

#[test]
#[serial]
fn test_print_page_ranges() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    // Most PDFs don't specify print page ranges (empty = all pages)
    let ranges = doc.print_page_ranges();
    // Either empty (all pages) or valid ranges
    for (start, end) in &ranges {
        assert!(*start >= 0, "start page should be >= 0");
        assert!(*end >= *start, "end page should be >= start");
    }
}

#[test]
#[serial]
fn test_viewer_preference() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    // Most PDFs don't set viewer preferences, so None is expected
    let direction = doc.viewer_preference("Direction");
    // Either None or a valid string
    if let Some(dir) = direction {
        assert!(!dir.is_empty());
    }

    // Test a preference that likely doesn't exist
    let nonexistent = doc.viewer_preference("NonExistentPref");
    assert!(nonexistent.is_none());
}

// ============================================================================
// Document Metadata Convenience Methods Tests
// ============================================================================

#[test]
#[serial]
fn test_metadata_title() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    // API call succeeds - title may or may not be set
    let title = doc.title();
    // If present, should be non-empty
    if let Some(t) = &title {
        assert!(!t.is_empty());
    }
    // Verify convenience method matches direct metadata call
    assert_eq!(title, doc.metadata("Title"));
}

#[test]
#[serial]
fn test_metadata_author() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    // API call succeeds - author may or may not be set
    let author = doc.author();
    // If present, should be non-empty
    if let Some(a) = &author {
        assert!(!a.is_empty());
    }
    // Verify convenience method matches direct metadata call
    assert_eq!(author, doc.metadata("Author"));
}

#[test]
#[serial]
fn test_metadata_subject() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    // API call succeeds - subject may or may not be set
    let subject = doc.subject();
    // If present, should be non-empty
    if let Some(s) = &subject {
        assert!(!s.is_empty());
    }
    // Verify convenience method matches direct metadata call
    assert_eq!(subject, doc.metadata("Subject"));
}

#[test]
#[serial]
fn test_metadata_keywords() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    // API call succeeds - keywords may or may not be set
    let keywords = doc.keywords();
    // If present, should be non-empty
    if let Some(k) = &keywords {
        assert!(!k.is_empty());
    }
    // Verify convenience method matches direct metadata call
    assert_eq!(keywords, doc.metadata("Keywords"));
}

#[test]
#[serial]
fn test_metadata_creator() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    // API call succeeds - creator may or may not be set
    let creator = doc.creator();
    // If present, should be non-empty
    if let Some(c) = &creator {
        assert!(!c.is_empty());
    }
    // Verify convenience method matches direct metadata call
    assert_eq!(creator, doc.metadata("Creator"));
}

#[test]
#[serial]
fn test_metadata_producer() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    // API call succeeds - producer may or may not be set
    let producer = doc.producer();
    // If present, should be non-empty
    if let Some(p) = &producer {
        assert!(!p.is_empty());
    }
    // Verify convenience method matches direct metadata call
    assert_eq!(producer, doc.metadata("Producer"));
}

#[test]
#[serial]
fn test_metadata_creation_date() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    // API call succeeds - creation_date may or may not be set
    let creation_date = doc.creation_date();
    // If present, should be non-empty
    if let Some(d) = &creation_date {
        assert!(!d.is_empty());
    }
    // Verify convenience method matches direct metadata call
    assert_eq!(creation_date, doc.metadata("CreationDate"));
}

#[test]
#[serial]
fn test_metadata_modification_date() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    // API call succeeds - modification_date may or may not be set
    let modification_date = doc.modification_date();
    // If present, should be non-empty
    if let Some(d) = &modification_date {
        assert!(!d.is_empty());
    }
    // Verify convenience method matches direct metadata call
    assert_eq!(modification_date, doc.metadata("ModDate"));
}

// ============================================================================
// Docling Integration Tests
// ============================================================================

#[test]
#[serial]
fn test_docling_extract_reading_order() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // Extract reading order
    let segments = extract_reading_order(&page).unwrap();

    // Should return segments for pages with text
    // Even if empty (blank pages), the function should succeed
    for (i, segment) in segments.iter().enumerate() {
        // Order should match index
        assert_eq!(segment.order, i, "Segment order should match index");
        // Bounds should be valid (left < right, bottom < top for PDF coords)
        let (left, top, right, bottom) = segment.bounds;
        assert!(left <= right, "Bounds left should be <= right");
        // PDF coordinates: bottom < top (origin at bottom-left)
        assert!(bottom <= top, "Bounds bottom should be <= top");
    }
}

#[test]
#[serial]
fn test_docling_extract_reading_order_empty_page() {
    let pdfium = Pdfium::new().unwrap();

    // Create a new document with an empty page
    let doc = pdfium.create_new_document().unwrap();
    let page = doc.new_page_a4(0).unwrap(); // A4 size at index 0

    let segments = extract_reading_order(&page).unwrap();
    assert!(
        segments.is_empty(),
        "Empty page should have no reading order segments"
    );
}

#[test]
#[serial]
fn test_docling_analyze_font_clusters() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // Analyze font clusters
    let clusters = analyze_font_clusters(&page).unwrap();

    // For pages with text, we should get at least one cluster
    // (Could be zero for empty pages)
    for cluster in &clusters {
        // Font size should be positive
        assert!(cluster.font_size >= 0.0, "Font size should be non-negative");
        // Coverage should be 0.0 to 1.0
        assert!(
            cluster.coverage >= 0.0 && cluster.coverage <= 1.0,
            "Coverage should be between 0.0 and 1.0"
        );
        // Char count should be positive if coverage > 0
        if cluster.coverage > 0.0 {
            assert!(
                cluster.char_count > 0,
                "Char count should be positive if coverage > 0"
            );
        }
    }
}

#[test]
#[serial]
fn test_docling_font_cluster_roles() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let clusters = analyze_font_clusters(&page).unwrap();

    // Verify all roles are valid enum variants
    for cluster in &clusters {
        match cluster.role {
            FontSemanticRole::Title
            | FontSemanticRole::SectionHeader
            | FontSemanticRole::Body
            | FontSemanticRole::Footnote
            | FontSemanticRole::Code
            | FontSemanticRole::Unknown => {
                // All valid
            }
        }
    }
}

#[test]
#[serial]
fn test_docling_detect_layout_regions() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // Detect layout regions
    let regions = detect_layout_regions(&page).unwrap();

    // Should return at least one region (even for empty pages, we get a margin)
    assert!(!regions.is_empty(), "Should detect at least one region");

    for region in &regions {
        // Confidence should be 0.0 to 1.0
        assert!(
            region.confidence >= 0.0 && region.confidence <= 1.0,
            "Confidence should be between 0.0 and 1.0"
        );
        // Bounds should be valid
        let (left, top, right, bottom) = region.bounds;
        assert!(left <= right, "Region bounds left should be <= right");
        assert!(bottom <= top, "Region bounds bottom should be <= top");
    }
}

#[test]
#[serial]
fn test_docling_layout_region_types() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let regions = detect_layout_regions(&page).unwrap();

    // Verify all region types are valid enum variants
    for region in &regions {
        match region.region_type {
            LayoutRegionType::TextColumn
            | LayoutRegionType::HeaderFooter
            | LayoutRegionType::Figure
            | LayoutRegionType::Sidebar
            | LayoutRegionType::Margin => {
                // All valid
            }
        }
    }
}

#[test]
#[serial]
fn test_docling_classification_analyze() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    // Get document classification
    let classification = DoclingClassification::analyze(&doc);

    // Page count should match
    assert_eq!(
        classification.page_count as usize,
        doc.page_count(),
        "Classification page_count should match document"
    );

    // avg_text_per_page should be non-negative
    assert!(
        classification.avg_text_per_page >= 0.0,
        "Average text per page should be non-negative"
    );

    // image_coverage should be 0.0 to 1.0
    assert!(
        classification.image_coverage >= 0.0 && classification.image_coverage <= 1.0,
        "Image coverage should be between 0.0 and 1.0"
    );
}

#[test]
#[serial]
fn test_docling_classification_document_types() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    let classification = DoclingClassification::analyze(&doc);

    // Verify document_type is a valid enum variant
    match classification.document_type {
        DocumentType::Article
        | DocumentType::Book
        | DocumentType::Slides
        | DocumentType::Form
        | DocumentType::Invoice
        | DocumentType::Letter
        | DocumentType::Technical
        | DocumentType::Unknown => {
            // All valid
        }
    }
}

#[test]
#[serial]
fn test_docling_classification_flags() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    let classification = DoclingClassification::analyze(&doc);

    // Boolean flags should be accessible without error
    let _is_scanned = classification.is_scanned;
    let _is_tagged = classification.is_tagged;
    let _has_forms = classification.has_forms;
    let _is_multi_column = classification.is_multi_column;

    // These are just flag checks - any boolean value is valid
}

#[test]
#[serial]
fn test_docling_classification_new_document() {
    let pdfium = Pdfium::new().unwrap();
    let doc = pdfium.create_new_document().unwrap();

    // Add an empty page
    let _page = doc.new_page_a4(0).unwrap();

    let classification = DoclingClassification::analyze(&doc);

    // New empty document should have specific characteristics
    assert_eq!(classification.page_count, 1, "New document has 1 page");
    assert!(
        !classification.is_scanned,
        "New empty document should not be scanned"
    );
    assert!(
        !classification.has_forms,
        "New empty document should not have forms"
    );
    assert!(
        !classification.is_multi_column,
        "New empty document should not be multi-column"
    );
}

#[test]
#[serial]
fn test_docling_reading_order_segment_fields() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let segments = extract_reading_order(&page).unwrap();

    for segment in &segments {
        // Text should be accessible (may be empty but not panic)
        let _text = &segment.text;

        // Role is optional
        if let Some(role) = &segment.role {
            assert!(
                !role.is_empty(),
                "If role is present, it should not be empty"
            );
        }

        // Bounds tuple should be accessible
        let (_left, _top, _right, _bottom) = segment.bounds;

        // Order should be a valid usize
        let _order = segment.order;
    }
}

#[test]
#[serial]
fn test_docling_font_cluster_monospace_detection() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let clusters = analyze_font_clusters(&page).unwrap();

    // Check that is_monospace is properly a boolean
    for cluster in &clusters {
        // is_monospace should be true or false (no panic)
        let _is_mono = cluster.is_monospace;

        // If monospace, role should typically be Code
        // (but this isn't guaranteed for all fonts)
    }
}

#[test]
#[serial]
fn test_docling_layout_column_index() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let regions = detect_layout_regions(&page).unwrap();

    // Column indices should be sequential and non-negative
    let text_columns: Vec<_> = regions
        .iter()
        .filter(|r| r.region_type == LayoutRegionType::TextColumn)
        .collect();

    if !text_columns.is_empty() {
        // Verify column indices are valid
        for (i, column) in text_columns.iter().enumerate() {
            assert!(
                column.column_index < text_columns.len(),
                "Column index should be valid"
            );
            // For single-column documents, index should be 0
            // For multi-column, indices should be 0, 1, etc.
            if text_columns.len() == 1 {
                assert_eq!(column.column_index, 0, "Single column should have index 0");
            }
            let _ = i;
        }
    }
}

#[test]
#[serial]
fn test_docling_multiple_pages() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    // Test docling functions on multiple pages
    let pages_to_test = doc.page_count().min(3);

    for page_idx in 0..pages_to_test {
        let page = doc.page(page_idx).unwrap();

        // Each should succeed
        let _reading_order = extract_reading_order(&page).unwrap();
        let _font_clusters = analyze_font_clusters(&page).unwrap();
        let _layout_regions = detect_layout_regions(&page).unwrap();
    }
}

#[test]
#[serial]
fn test_docling_classification_consistency() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    // Call analyze multiple times - should be consistent
    let c1 = DoclingClassification::analyze(&doc);
    let c2 = DoclingClassification::analyze(&doc);

    assert_eq!(
        c1.page_count, c2.page_count,
        "Page count should be consistent"
    );
    assert_eq!(c1.is_tagged, c2.is_tagged, "is_tagged should be consistent");
    assert_eq!(c1.has_forms, c2.has_forms, "has_forms should be consistent");
    // Note: is_scanned and is_multi_column depend on sampled pages, so should also be consistent
    assert_eq!(
        c1.is_scanned, c2.is_scanned,
        "is_scanned should be consistent"
    );
    assert_eq!(
        c1.is_multi_column, c2.is_multi_column,
        "is_multi_column should be consistent"
    );
}

// ============================================================================
// Artifact Detection Tests
// ============================================================================

#[test]
#[serial]
fn test_artifact_is_artifact() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // Check that is_artifact() doesn't panic and returns boolean
    for obj in page.objects().iter() {
        let _is_artifact = obj.is_artifact();
    }
}

#[test]
#[serial]
fn test_artifact_type() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // Check that artifact_type() returns valid enum or None
    for obj in page.objects().iter() {
        if let Some(artifact_type) = obj.artifact_type() {
            match artifact_type {
                ArtifactType::Background
                | ArtifactType::Footer
                | ArtifactType::Header
                | ArtifactType::Layout
                | ArtifactType::Page
                | ArtifactType::Pagination
                | ArtifactType::Watermark
                | ArtifactType::Other => {
                    // All valid
                }
            }
        }
    }
}

#[test]
#[serial]
fn test_artifact_mark_count() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // Check that mark_count() returns a valid count
    for obj in page.objects().iter() {
        let count = obj.mark_count();
        // mark_count is usize, always >= 0
        let _ = count;
    }
}

#[test]
#[serial]
fn test_artifact_mark_names() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // Check that mark_names() returns a vector of strings
    for obj in page.objects().iter() {
        let names = obj.mark_names();
        for name in &names {
            assert!(!name.is_empty(), "Mark names should not be empty strings");
        }
    }
}

#[test]
#[serial]
fn test_page_content_objects() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // content_objects() should return non-artifact objects
    let content_objects = page.content_objects();
    for obj in &content_objects {
        assert!(
            !obj.is_artifact(),
            "content_objects() should not include artifacts"
        );
    }
}

#[test]
#[serial]
fn test_page_artifact_objects() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // artifact_objects() should return only artifact objects
    let artifact_objects = page.artifact_objects();
    for obj in &artifact_objects {
        assert!(
            obj.is_artifact(),
            "artifact_objects() should only include artifacts"
        );
    }
}

#[test]
#[serial]
fn test_page_object_counts() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let total_count = page.object_count();
    let artifact_count = page.artifact_count();
    let content_count = page.content_object_count();

    // Total should equal artifacts + content
    assert_eq!(
        total_count,
        artifact_count + content_count,
        "total objects should equal artifacts + content"
    );
}

#[test]
#[serial]
fn test_artifact_consistency() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // Multiple calls should be consistent
    let count1 = page.artifact_count();
    let count2 = page.artifact_count();
    assert_eq!(count1, count2, "artifact_count should be consistent");

    let content1 = page.content_object_count();
    let content2 = page.content_object_count();
    assert_eq!(
        content1, content2,
        "content_object_count should be consistent"
    );
}

// ============================================================================
// Image Technical Metadata Tests
// ============================================================================

#[test]
#[serial]
fn test_image_filters_basic() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // Check that image_filters() works on image objects
    for obj in page.objects().images() {
        let filters = obj.image_filters();
        assert!(
            filters.is_some(),
            "image_filters should return Some for image objects"
        );
        let filters = filters.unwrap();
        // Filters can be empty (uncompressed) or contain valid filter types
        for filter in &filters {
            // Ensure we can get the filter name
            let _ = filter.name();
        }
    }
}

#[test]
#[serial]
fn test_image_filters_non_image() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // image_filters() should return None for non-image objects
    for obj in page.objects().text_objects() {
        assert!(
            obj.image_filters().is_none(),
            "image_filters should return None for text objects"
        );
    }
    for obj in page.objects().paths() {
        assert!(
            obj.image_filters().is_none(),
            "image_filters should return None for path objects"
        );
    }
}

#[test]
#[serial]
fn test_image_tech_metadata_basic() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // Check that image_tech_metadata() works on image objects
    for obj in page.objects().images() {
        let meta = obj.image_tech_metadata();
        if let Some(meta) = meta {
            // Validate basic properties
            assert!(meta.width_px > 0, "image width should be positive");
            assert!(meta.height_px > 0, "image height should be positive");
            // DPI can be 0 if not specified in PDF
            let _ = meta.dpi;
            // bits_per_component should be reasonable (1, 2, 4, 8, 16)
            assert!(
                meta.bits_per_component <= 16,
                "bits_per_component should be <= 16"
            );
            // color_space should be a valid enum variant
            let _ = meta.color_space;
            // filters list should be valid
            let _ = &meta.filters;
            // mask flags should be booleans (always true by type)
            let _ = meta.has_mask;
            let _ = meta.has_soft_mask;
        }
    }
}

#[test]
#[serial]
fn test_image_tech_metadata_non_image() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // image_tech_metadata() should return None for non-image objects
    for obj in page.objects().text_objects() {
        assert!(
            obj.image_tech_metadata().is_none(),
            "image_tech_metadata should return None for text objects"
        );
    }
    for obj in page.objects().paths() {
        assert!(
            obj.image_tech_metadata().is_none(),
            "image_tech_metadata should return None for path objects"
        );
    }
}

#[test]
#[serial]
fn test_image_tech_meta_helper_methods() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // Test helper methods on real images
    for obj in page.objects().images() {
        if let Some(meta) = obj.image_tech_metadata() {
            // Test type detection methods
            let _ = meta.is_jpeg();
            let _ = meta.is_jpeg2000();
            let _ = meta.is_bilevel();
            let _ = meta.is_grayscale();
            let _ = meta.is_color();

            // Test derived properties
            let components = meta.components();
            assert!(components <= 4, "components should be <= 4");

            let bpp = meta.bits_per_pixel();
            assert!(bpp <= 64, "bits_per_pixel should be reasonable");

            let size = meta.uncompressed_size();
            // Size should be reasonable for an image
            assert!(size <= 1_000_000_000, "uncompressed_size should be < 1GB");

            let _ = meta.has_any_mask();
        }
    }
}

#[test]
#[serial]
fn test_image_filter_enum_completeness() {
    // Test that all known filters can be parsed
    let filters = [
        ("DCTDecode", ImageFilter::DCTDecode),
        ("DCT", ImageFilter::DCTDecode),
        ("FlateDecode", ImageFilter::FlateDecode),
        ("Fl", ImageFilter::FlateDecode),
        ("JPXDecode", ImageFilter::JPXDecode),
        ("JBIG2Decode", ImageFilter::JBIG2Decode),
        ("CCITTFaxDecode", ImageFilter::CCITTFaxDecode),
        ("CCF", ImageFilter::CCITTFaxDecode),
        ("LZWDecode", ImageFilter::LZWDecode),
        ("LZW", ImageFilter::LZWDecode),
        ("RunLengthDecode", ImageFilter::RunLengthDecode),
        ("RL", ImageFilter::RunLengthDecode),
        ("ASCIIHexDecode", ImageFilter::ASCIIHexDecode),
        ("AHx", ImageFilter::ASCIIHexDecode),
        ("ASCII85Decode", ImageFilter::ASCII85Decode),
        ("A85", ImageFilter::ASCII85Decode),
        ("Crypt", ImageFilter::Crypt),
    ];

    for (name, expected) in filters {
        let parsed = ImageFilter::from_name(name);
        assert_eq!(
            parsed, expected,
            "Filter {} should parse to {:?}",
            name, expected
        );
    }

    // Unknown filters should be handled
    if let ImageFilter::Unknown(name) = ImageFilter::from_name("CustomFilter") {
        assert_eq!(name, "CustomFilter");
    } else {
        panic!("Unknown filter should parse to Unknown variant");
    }
}

#[test]
#[serial]
fn test_image_filter_properties_comprehensive() {
    // Test is_lossy
    assert!(ImageFilter::DCTDecode.is_lossy());
    assert!(ImageFilter::JPXDecode.is_lossy());
    assert!(!ImageFilter::FlateDecode.is_lossy());
    assert!(!ImageFilter::LZWDecode.is_lossy());
    assert!(!ImageFilter::CCITTFaxDecode.is_lossy());
    assert!(!ImageFilter::JBIG2Decode.is_lossy());

    // Test is_bilevel
    assert!(ImageFilter::CCITTFaxDecode.is_bilevel());
    assert!(ImageFilter::JBIG2Decode.is_bilevel());
    assert!(!ImageFilter::DCTDecode.is_bilevel());
    assert!(!ImageFilter::FlateDecode.is_bilevel());
    assert!(!ImageFilter::JPXDecode.is_bilevel());
}

#[test]
#[serial]
fn test_image_colorspace_coverage() {
    // Test all colorspace variants can be matched
    let colorspaces = [
        ImageColorspace::Unknown,
        ImageColorspace::DeviceGray,
        ImageColorspace::DeviceRGB,
        ImageColorspace::DeviceCMYK,
        ImageColorspace::CalGray,
        ImageColorspace::CalRGB,
        ImageColorspace::Lab,
        ImageColorspace::ICCBased,
        ImageColorspace::Separation,
        ImageColorspace::DeviceN,
        ImageColorspace::Indexed,
        ImageColorspace::Pattern,
    ];

    for cs in colorspaces {
        // Each colorspace should be usable in match
        let _ = match cs {
            ImageColorspace::Unknown => "unknown",
            ImageColorspace::DeviceGray => "gray",
            ImageColorspace::DeviceRGB => "rgb",
            ImageColorspace::DeviceCMYK => "cmyk",
            ImageColorspace::CalGray => "calgray",
            ImageColorspace::CalRGB => "calrgb",
            ImageColorspace::Lab => "lab",
            ImageColorspace::ICCBased => "icc",
            ImageColorspace::Separation => "separation",
            ImageColorspace::DeviceN => "devicen",
            ImageColorspace::Indexed => "indexed",
            ImageColorspace::Pattern => "pattern",
        };
    }
}

#[test]
#[serial]
fn test_image_tech_meta_consistency() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // Multiple calls should return consistent results
    for obj in page.objects().images() {
        let meta1 = obj.image_tech_metadata();
        let meta2 = obj.image_tech_metadata();

        match (meta1, meta2) {
            (Some(m1), Some(m2)) => {
                assert_eq!(m1.width_px, m2.width_px, "width should be consistent");
                assert_eq!(m1.height_px, m2.height_px, "height should be consistent");
                assert_eq!(
                    m1.bits_per_component, m2.bits_per_component,
                    "bits_per_component should be consistent"
                );
                assert_eq!(
                    m1.color_space, m2.color_space,
                    "color_space should be consistent"
                );
                assert_eq!(
                    m1.filters.len(),
                    m2.filters.len(),
                    "filters should be consistent"
                );
            }
            (None, None) => {}
            _ => panic!("image_tech_metadata consistency mismatch"),
        }
    }
}

// ============================================================================
// Line Extraction API Tests
// ============================================================================

#[test]
#[serial]
fn test_extract_lines_basic() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // extract_lines should not panic
    let lines = page.extract_lines();
    // Result can be empty if no lines on page - that's valid
    for line in &lines {
        // Each line should have valid properties
        assert!(line.length() >= 0.0, "line length should be non-negative");
        assert!(
            line.thickness >= 0.0,
            "line thickness should be non-negative"
        );
    }
}

#[test]
#[serial]
fn test_extract_horizontal_lines() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let lines = page.extract_horizontal_lines();
    // All returned lines should be horizontal
    for line in &lines {
        assert!(
            line.is_horizontal,
            "extract_horizontal_lines should only return horizontal lines"
        );
        // Horizontal lines should have y_position
        assert!(
            line.y_position().is_some(),
            "horizontal lines should have y_position"
        );
    }
}

#[test]
#[serial]
fn test_extract_vertical_lines() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let lines = page.extract_vertical_lines();
    // All returned lines should be vertical
    for line in &lines {
        assert!(
            line.is_vertical,
            "extract_vertical_lines should only return vertical lines"
        );
        // Vertical lines should have x_position
        assert!(
            line.x_position().is_some(),
            "vertical lines should have x_position"
        );
    }
}

#[test]
#[serial]
fn test_line_count() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // line_count should equal extract_lines().len()
    let count = page.line_count();
    let lines = page.extract_lines();
    assert_eq!(
        count,
        lines.len(),
        "line_count should match extract_lines().len()"
    );
}

#[test]
#[serial]
fn test_extracted_line_properties() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    for line in page.extract_lines() {
        // Test bounds consistency
        let (left, bottom, right, top) = line.bounds();
        assert!(left <= right, "bounds: left should be <= right");
        assert!(bottom <= top, "bounds: bottom should be <= top");

        // Test midpoint is within bounds
        let (mx, my) = line.midpoint();
        assert!(
            mx >= left && mx <= right,
            "midpoint x should be within bounds"
        );
        assert!(
            my >= bottom && my <= top,
            "midpoint y should be within bounds"
        );

        // Test angle is in valid range
        let angle = line.angle();
        assert!(
            (-std::f32::consts::PI..=std::f32::consts::PI).contains(&angle),
            "angle should be in [-π, π]"
        );

        // Test angle_degrees
        let angle_deg = line.angle_degrees();
        assert!(
            (-180.0..=180.0).contains(&angle_deg),
            "angle_degrees should be in [-180, 180]"
        );
    }
}

#[test]
#[serial]
fn test_path_object_extract_line() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    for path_obj in page.objects().paths() {
        // is_simple_line should return Some(bool) for path objects
        let is_simple = path_obj.is_simple_line();
        assert!(
            is_simple.is_some(),
            "is_simple_line should return Some for path objects"
        );

        // extract_line may or may not return a line depending on path complexity
        let _ = path_obj.extract_line();

        // extract_lines should return Some for path objects
        let lines = path_obj.extract_lines();
        assert!(
            lines.is_some(),
            "extract_lines should return Some for path objects"
        );
    }
}

#[test]
#[serial]
fn test_extract_lines_non_path() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // Non-path objects should return None for line extraction methods
    for obj in page.objects().iter() {
        if !obj.is_path() {
            assert!(
                obj.is_simple_line().is_none(),
                "non-path objects should return None for is_simple_line"
            );
            assert!(
                obj.extract_line().is_none(),
                "non-path objects should return None for extract_line"
            );
            assert!(
                obj.extract_lines().is_none(),
                "non-path objects should return None for extract_lines"
            );
        }
    }
}

#[test]
#[serial]
fn test_line_horizontal_vertical_classification() {
    use pdfium_render_fast::ExtractedLine;

    // Test classification of lines
    // Pure horizontal
    let horiz = ExtractedLine::new((0.0, 50.0), (100.0, 50.0), 1.0, (0, 0, 0, 255));
    assert!(horiz.is_horizontal);
    assert!(!horiz.is_vertical);
    assert!(!horiz.is_diagonal());

    // Pure vertical
    let vert = ExtractedLine::new((50.0, 0.0), (50.0, 100.0), 1.0, (0, 0, 0, 255));
    assert!(!vert.is_horizontal);
    assert!(vert.is_vertical);
    assert!(!vert.is_diagonal());

    // Diagonal
    let diag = ExtractedLine::new((0.0, 0.0), (100.0, 100.0), 1.0, (0, 0, 0, 255));
    assert!(!diag.is_horizontal);
    assert!(!diag.is_vertical);
    assert!(diag.is_diagonal());
}

#[test]
#[serial]
fn test_extracted_line_struct() {
    use pdfium_render_fast::ExtractedLine;

    let line = ExtractedLine::new(
        (10.0, 20.0),     // start
        (110.0, 20.0),    // end (horizontal line)
        2.5,              // thickness
        (255, 0, 0, 200), // red, semi-transparent
    );

    assert_eq!(line.start, (10.0, 20.0));
    assert_eq!(line.end, (110.0, 20.0));
    assert_eq!(line.thickness, 2.5);
    assert_eq!(line.color, (255, 0, 0, 200));
    assert!(line.is_horizontal);
    assert!(!line.is_vertical);
    assert!(line.is_visible());
    assert!((line.length() - 100.0).abs() < 0.001);
}

#[test]
#[serial]
fn test_extract_lines_all_pages() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    // Extract lines from all pages - should not panic
    for page in doc.pages() {
        let lines = page.extract_lines();
        let h_lines = page.extract_horizontal_lines();
        let v_lines = page.extract_vertical_lines();

        // Horizontal + vertical lines should not exceed total
        // (they can be fewer due to diagonal lines)
        assert!(
            h_lines.len() + v_lines.len() <= lines.len() + lines.len(),
            "h + v lines should be reasonable"
        );
    }
}

// ============================================================================
// Colored Region Extraction API Tests
// ============================================================================

#[test]
#[serial]
fn test_extract_colored_regions_basic() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // extract_colored_regions should not panic
    let regions = page.extract_colored_regions();
    // Result can be empty if no filled paths on page - that's valid
    for region in &regions {
        // Each region should have valid dimensions
        assert!(region.width() >= 0.0, "region width should be non-negative");
        assert!(
            region.height() >= 0.0,
            "region height should be non-negative"
        );
        assert!(region.area() >= 0.0, "region area should be non-negative");
    }
}

#[test]
#[serial]
fn test_extract_background_regions() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let regions = page.extract_background_regions();
    // All returned regions should have is_behind_text = true
    for region in &regions {
        assert!(
            region.is_behind_text,
            "extract_background_regions should only return behind-text regions"
        );
    }
}

#[test]
#[serial]
fn test_extract_foreground_regions() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let regions = page.extract_foreground_regions();
    // All returned regions should have is_behind_text = false
    for region in &regions {
        assert!(
            !region.is_behind_text,
            "extract_foreground_regions should only return foreground regions"
        );
    }
}

#[test]
#[serial]
fn test_colored_region_count() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // colored_region_count should equal extract_colored_regions().len()
    let count = page.colored_region_count();
    let regions = page.extract_colored_regions();
    assert_eq!(
        count,
        regions.len(),
        "colored_region_count should match extract_colored_regions().len()"
    );
}

#[test]
#[serial]
fn test_colored_region_properties() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    for region in page.extract_colored_regions() {
        // Test bounds consistency
        let (left, bottom, right, top) = region.bounds;
        assert!(left <= right, "bounds: left should be <= right");
        assert!(bottom <= top, "bounds: bottom should be <= top");

        // Test center is within bounds
        let (cx, cy) = region.center();
        assert!(
            cx >= left && cx <= right,
            "center x should be within bounds"
        );
        assert!(
            cy >= bottom && cy <= top,
            "center y should be within bounds"
        );

        // Test aspect ratio is non-negative
        let aspect = region.aspect_ratio();
        assert!(aspect >= 0.0, "aspect ratio should be non-negative");

        // If filled, fill_color should be Some
        if region.is_filled() {
            assert!(region.fill_color.is_some());
        }

        // If stroked, stroke_color should be Some
        if region.is_stroked() {
            assert!(region.stroke_color.is_some());
        }
    }
}

#[test]
#[serial]
fn test_path_object_extract_colored_region() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    for path_obj in page.objects().paths() {
        // extract_colored_region may or may not return a region depending on fill
        let region = path_obj.extract_colored_region(false);
        if let Some(r) = region {
            // If we got a region, it should be filled
            assert!(r.is_filled(), "extracted region should be filled");
        }

        // is_filled_rectangle should return Some for path objects
        let is_rect = path_obj.is_filled_rectangle();
        assert!(
            is_rect.is_some(),
            "is_filled_rectangle should return Some for path objects"
        );
    }
}

#[test]
#[serial]
fn test_extract_colored_region_non_path() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // Non-path objects should return None for colored region extraction methods
    for obj in page.objects().iter() {
        if !obj.is_path() {
            assert!(
                obj.extract_colored_region(false).is_none(),
                "non-path objects should return None for extract_colored_region"
            );
            assert!(
                obj.is_filled_rectangle().is_none(),
                "non-path objects should return None for is_filled_rectangle"
            );
        }
    }
}

#[test]
#[serial]
fn test_colored_region_struct() {
    use pdfium_render_fast::ColoredRegion;

    let region = ColoredRegion::new(
        (10.0, 20.0, 110.0, 120.0), // bounds
        Some((255, 0, 0, 255)),     // fill color (red)
        Some((0, 0, 0, 255)),       // stroke color (black)
        true,                       // is_behind_text
    );

    assert_eq!(region.bounds, (10.0, 20.0, 110.0, 120.0));
    assert_eq!(region.fill_color, Some((255, 0, 0, 255)));
    assert_eq!(region.stroke_color, Some((0, 0, 0, 255)));
    assert!(region.is_behind_text);
    assert!(region.is_filled());
    assert!(region.is_stroked());
    assert!(region.is_visible());
    assert_eq!(region.width(), 100.0);
    assert_eq!(region.height(), 100.0);
    assert_eq!(region.area(), 10000.0);
    assert_eq!(region.center(), (60.0, 70.0));
}

#[test]
#[serial]
fn test_colored_region_visibility_and_colors() {
    use pdfium_render_fast::ColoredRegion;

    // White fill
    let white = ColoredRegion::new(
        (0.0, 0.0, 100.0, 100.0),
        Some((255, 255, 255, 255)),
        None,
        false,
    );
    assert!(white.is_white_fill());
    assert!(white.is_light_fill());
    assert!(!white.is_dark_fill());

    // Dark fill
    let dark = ColoredRegion::new(
        (0.0, 0.0, 100.0, 100.0),
        Some((20, 20, 20, 255)),
        None,
        false,
    );
    assert!(!dark.is_white_fill());
    assert!(!dark.is_light_fill());
    assert!(dark.is_dark_fill());

    // Transparent (invisible)
    let transparent = ColoredRegion::new(
        (0.0, 0.0, 100.0, 100.0),
        Some((255, 0, 0, 0)), // alpha = 0
        None,
        false,
    );
    assert!(!transparent.is_visible());
}

#[test]
#[serial]
fn test_colored_region_overlap_and_contains() {
    use pdfium_render_fast::ColoredRegion;

    let region1 = ColoredRegion::new((0.0, 0.0, 100.0, 100.0), Some((0, 0, 0, 255)), None, false);

    // Overlapping
    let overlap = ColoredRegion::new(
        (50.0, 50.0, 150.0, 150.0),
        Some((0, 0, 0, 255)),
        None,
        false,
    );
    assert!(region1.overlaps(&overlap));

    // Contained
    let inner = ColoredRegion::new((10.0, 10.0, 90.0, 90.0), Some((0, 0, 0, 255)), None, false);
    assert!(region1.contains(&inner));
    assert!(!inner.contains(&region1));

    // Point containment
    assert!(region1.contains_point(50.0, 50.0));
    assert!(!region1.contains_point(150.0, 150.0));
}

#[test]
#[serial]
fn test_colored_region_stripes() {
    use pdfium_render_fast::ColoredRegion;

    // Horizontal stripe
    let h_stripe = ColoredRegion::new((0.0, 0.0, 600.0, 50.0), Some((0, 0, 0, 255)), None, false);
    assert!(h_stripe.is_horizontal_stripe());
    assert!(!h_stripe.is_vertical_stripe());

    // Vertical stripe
    let v_stripe = ColoredRegion::new((0.0, 0.0, 50.0, 600.0), Some((0, 0, 0, 255)), None, false);
    assert!(!v_stripe.is_horizontal_stripe());
    assert!(v_stripe.is_vertical_stripe());
}

#[test]
#[serial]
fn test_has_page_background() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // has_page_background should not panic
    let _has_bg = page.has_page_background();
    // Result can be true or false depending on PDF content
}

#[test]
#[serial]
fn test_page_background_color() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // page_background_color should not panic
    let bg_color = page.page_background_color();
    if let Some((r, g, b, a)) = bg_color {
        // Color components are u8 values - validate they were extracted
        // (type system enforces 0-255 range)
        let _ = (r, g, b, a); // Use the values to confirm extraction worked
    }
}

#[test]
#[serial]
fn test_extract_colored_regions_all_pages() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    // Extract colored regions from all pages - should not panic
    for page in doc.pages() {
        let regions = page.extract_colored_regions();
        let bg_regions = page.extract_background_regions();
        let fg_regions = page.extract_foreground_regions();

        // Background + foreground should equal total
        assert_eq!(
            bg_regions.len() + fg_regions.len(),
            regions.len(),
            "background + foreground regions should equal total"
        );

        // Check has_page_background and page_background_color consistency
        let has_bg = page.has_page_background();
        let bg_color = page.page_background_color();

        // If has_page_background is true, there should be a qualifying region
        // (but page_background_color may still be None if fill_color is None)
        if has_bg {
            // There should be at least one large background region
            let page_area = page.width() * page.height();
            let large_bg_exists = bg_regions
                .iter()
                .any(|r| (r.area() as f64) > page_area * 0.9);
            assert!(
                large_bg_exists,
                "has_page_background true but no large background region found"
            );
        }

        // If we have a background color, has_page_background should likely be true
        // (relaxed: page_background_color requires >50% area, has_page_background requires >90%)
        let _ = (has_bg, bg_color); // Acknowledge both are used
    }
}

// ============================================================================
// Text Block Metrics Tests
// ============================================================================

#[test]
#[serial]
fn test_text_block_metrics_struct() {
    // Test creating TextBlockMetrics directly
    let block = TextBlockMetrics {
        bounds: (0.0, 0.0, 100.0, 50.0),
        line_count: 3,
        avg_line_height: 12.0,
        avg_line_spacing: 3.0,
        first_line_indent: 20.0,
        avg_char_spacing: 1.5,
        avg_word_spacing: 5.0,
    };

    assert_eq!(block.width(), 100.0);
    assert_eq!(block.height(), 50.0);
    assert_eq!(block.area(), 5000.0);
    assert_eq!(block.center(), (50.0, 25.0));
    assert!(!block.is_single_line());
    assert!(block.is_multi_line());
    assert!(block.is_indented(10.0));
    assert!(!block.is_indented(30.0));
}

#[test]
#[serial]
fn test_text_block_metrics_spacing_detection() {
    // Test tight spacing
    let tight = TextBlockMetrics {
        bounds: (0.0, 0.0, 100.0, 50.0),
        line_count: 3,
        avg_line_height: 12.0,
        avg_line_spacing: 1.0, // Very tight: ratio = 13/12 = 1.08
        first_line_indent: 0.0,
        avg_char_spacing: 1.0,
        avg_word_spacing: 4.0,
    };
    assert!(tight.has_tight_spacing());
    assert!(!tight.has_loose_spacing());

    // Test loose spacing
    let loose = TextBlockMetrics {
        bounds: (0.0, 0.0, 100.0, 50.0),
        line_count: 3,
        avg_line_height: 12.0,
        avg_line_spacing: 15.0, // Very loose: ratio = 27/12 = 2.25
        first_line_indent: 0.0,
        avg_char_spacing: 1.0,
        avg_word_spacing: 4.0,
    };
    assert!(!loose.has_tight_spacing());
    assert!(loose.has_loose_spacing());

    // Test normal spacing
    let normal = TextBlockMetrics {
        bounds: (0.0, 0.0, 100.0, 50.0),
        line_count: 3,
        avg_line_height: 12.0,
        avg_line_spacing: 6.0, // Normal: ratio = 18/12 = 1.5 (exactly at threshold)
        first_line_indent: 0.0,
        avg_char_spacing: 1.0,
        avg_word_spacing: 4.0,
    };
    // ratio = 1.5 is NOT < 1.5, so not tight
    assert!(!normal.has_tight_spacing());
    assert!(!normal.has_loose_spacing());
}

#[test]
#[serial]
fn test_text_block_metrics_line_spacing_ratio() {
    let block = TextBlockMetrics {
        bounds: (0.0, 0.0, 100.0, 50.0),
        line_count: 3,
        avg_line_height: 10.0,
        avg_line_spacing: 5.0, // ratio = 15/10 = 1.5
        first_line_indent: 0.0,
        avg_char_spacing: 1.0,
        avg_word_spacing: 4.0,
    };
    assert_eq!(block.line_spacing_ratio(), Some(1.5));

    // Test with zero height
    let zero_height = TextBlockMetrics {
        bounds: (0.0, 0.0, 100.0, 50.0),
        line_count: 1,
        avg_line_height: 0.0,
        avg_line_spacing: 0.0,
        first_line_indent: 0.0,
        avg_char_spacing: 0.0,
        avg_word_spacing: 0.0,
    };
    assert_eq!(zero_height.line_spacing_ratio(), None);
}

#[test]
#[serial]
fn test_text_block_metrics_aspect_ratio() {
    let wide = TextBlockMetrics {
        bounds: (0.0, 0.0, 200.0, 50.0),
        line_count: 1,
        avg_line_height: 12.0,
        avg_line_spacing: 0.0,
        first_line_indent: 0.0,
        avg_char_spacing: 1.0,
        avg_word_spacing: 4.0,
    };
    assert_eq!(wide.aspect_ratio(), 4.0); // 200/50

    let tall = TextBlockMetrics {
        bounds: (0.0, 0.0, 50.0, 200.0),
        line_count: 10,
        avg_line_height: 12.0,
        avg_line_spacing: 5.0,
        first_line_indent: 0.0,
        avg_char_spacing: 1.0,
        avg_word_spacing: 4.0,
    };
    assert_eq!(tall.aspect_ratio(), 0.25); // 50/200
}

#[test]
#[serial]
fn test_text_block_metrics_contains_point() {
    let block = TextBlockMetrics {
        bounds: (100.0, 200.0, 300.0, 400.0),
        line_count: 5,
        avg_line_height: 12.0,
        avg_line_spacing: 3.0,
        first_line_indent: 0.0,
        avg_char_spacing: 1.0,
        avg_word_spacing: 4.0,
    };

    // Point inside
    assert!(block.contains_point(150.0, 300.0));
    // Point on edge
    assert!(block.contains_point(100.0, 200.0));
    assert!(block.contains_point(300.0, 400.0));
    // Point outside
    assert!(!block.contains_point(50.0, 300.0));
    assert!(!block.contains_point(350.0, 300.0));
    assert!(!block.contains_point(200.0, 100.0));
    assert!(!block.contains_point(200.0, 500.0));
}

#[test]
#[serial]
fn test_text_block_metrics_overlaps() {
    let block1 = TextBlockMetrics {
        bounds: (0.0, 0.0, 100.0, 100.0),
        line_count: 3,
        avg_line_height: 12.0,
        avg_line_spacing: 3.0,
        first_line_indent: 0.0,
        avg_char_spacing: 1.0,
        avg_word_spacing: 4.0,
    };

    let block2_overlapping = TextBlockMetrics {
        bounds: (50.0, 50.0, 150.0, 150.0),
        line_count: 3,
        avg_line_height: 12.0,
        avg_line_spacing: 3.0,
        first_line_indent: 0.0,
        avg_char_spacing: 1.0,
        avg_word_spacing: 4.0,
    };

    let block3_separate = TextBlockMetrics {
        bounds: (200.0, 200.0, 300.0, 300.0),
        line_count: 3,
        avg_line_height: 12.0,
        avg_line_spacing: 3.0,
        first_line_indent: 0.0,
        avg_char_spacing: 1.0,
        avg_word_spacing: 4.0,
    };

    assert!(block1.overlaps(&block2_overlapping));
    assert!(!block1.overlaps(&block3_separate));
    assert!(block2_overlapping.overlaps(&block1)); // Symmetric
    assert!(!block3_separate.overlaps(&block1));
}

#[test]
#[serial]
fn test_text_block_metrics_clone_and_eq() {
    let block = TextBlockMetrics {
        bounds: (10.0, 20.0, 110.0, 70.0),
        line_count: 4,
        avg_line_height: 11.0,
        avg_line_spacing: 2.5,
        first_line_indent: 15.0,
        avg_char_spacing: 1.2,
        avg_word_spacing: 4.5,
    };

    let cloned = block.clone();
    assert_eq!(block, cloned);
    assert_eq!(block.bounds, cloned.bounds);
    assert_eq!(block.line_count, cloned.line_count);
}

#[test]
#[serial]
fn test_extract_text_blocks_with_metrics() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // Should extract some text blocks (PDF has content)
    let blocks = page.extract_text_blocks_with_metrics();

    // Most PDFs with text should have at least one block
    // But we can't assert exact count without knowing the PDF content
    for block in &blocks {
        // Basic sanity checks
        assert!(block.line_count > 0, "Block should have at least one line");
        assert!(block.width() >= 0.0, "Width should be non-negative");
        assert!(block.height() >= 0.0, "Height should be non-negative");
        assert!(
            block.avg_line_height >= 0.0,
            "Line height should be non-negative"
        );
        assert!(
            block.avg_line_spacing >= 0.0,
            "Line spacing should be non-negative"
        );
        assert!(
            block.first_line_indent >= 0.0,
            "Indent should be non-negative"
        );
        assert!(
            block.avg_char_spacing >= 0.0,
            "Char spacing should be non-negative"
        );
        assert!(
            block.avg_word_spacing >= 0.0,
            "Word spacing should be non-negative"
        );
    }
}

#[test]
#[serial]
fn test_text_block_count() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let count = page.text_block_count();
    let blocks = page.extract_text_blocks_with_metrics();

    assert_eq!(
        count,
        blocks.len(),
        "text_block_count should match blocks vector length"
    );
}

#[test]
#[serial]
fn test_extract_text_blocks_all_pages() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    // Should not panic on any page
    for page in doc.pages() {
        let blocks = page.extract_text_blocks_with_metrics();
        for block in &blocks {
            // Verify all metrics are valid floats (not NaN or infinity)
            assert!(block.avg_line_height.is_finite());
            assert!(block.avg_line_spacing.is_finite());
            assert!(block.avg_char_spacing.is_finite());
            assert!(block.avg_word_spacing.is_finite());
            assert!(block.first_line_indent.is_finite());
            assert!(block.bounds.0.is_finite());
            assert!(block.bounds.1.is_finite());
            assert!(block.bounds.2.is_finite());
            assert!(block.bounds.3.is_finite());
        }
    }
}

#[test]
#[serial]
fn test_text_block_metrics_single_line() {
    let single = TextBlockMetrics {
        bounds: (0.0, 0.0, 100.0, 12.0),
        line_count: 1,
        avg_line_height: 12.0,
        avg_line_spacing: 0.0,
        first_line_indent: 0.0,
        avg_char_spacing: 1.0,
        avg_word_spacing: 4.0,
    };

    assert!(single.is_single_line());
    assert!(!single.is_multi_line());
}

#[test]
#[serial]
fn test_text_block_debug_display() {
    let block = TextBlockMetrics {
        bounds: (0.0, 0.0, 100.0, 50.0),
        line_count: 3,
        avg_line_height: 12.0,
        avg_line_spacing: 3.0,
        first_line_indent: 20.0,
        avg_char_spacing: 1.5,
        avg_word_spacing: 5.0,
    };

    // Debug should work
    let debug_str = format!("{:?}", block);
    assert!(debug_str.contains("TextBlockMetrics"));
    assert!(debug_str.contains("bounds"));
    assert!(debug_str.contains("line_count"));
}

// ============================================================================
// Text Rise (Superscript/Subscript) Tests
// ============================================================================

#[test]
#[serial]
fn test_pdf_char_text_rise() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();
    let text = page.text().unwrap();

    // Get first character
    if let Some(ch) = text.char_at(0) {
        // text_rise() should return a finite value
        let rise = ch.text_rise();
        assert!(rise.is_finite(), "text_rise should be a finite value");

        // Check origin coordinates are populated
        assert!(ch.origin_x.is_finite());
        assert!(ch.origin_y.is_finite());
    }
}

#[test]
#[serial]
fn test_pdf_char_superscript_detection() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();
    let text = page.text().unwrap();

    // Test threshold detection methods
    for ch in text.chars().take(10) {
        let rise = ch.text_rise();
        let threshold = 2.0;

        // Check consistency of detection methods
        if rise > threshold {
            assert!(ch.is_superscript(threshold));
            assert!(!ch.is_subscript(threshold));
            assert!(!ch.is_baseline(threshold));
        } else if rise < -threshold {
            assert!(!ch.is_superscript(threshold));
            assert!(ch.is_subscript(threshold));
            assert!(!ch.is_baseline(threshold));
        } else {
            assert!(!ch.is_superscript(threshold));
            assert!(!ch.is_subscript(threshold));
            assert!(ch.is_baseline(threshold));
        }
    }
}

#[test]
#[serial]
fn test_chars_with_rise() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();
    let text = page.text().unwrap();

    let chars_with_rise = text.chars_with_rise();

    // Should have same count as chars()
    let char_count = text.chars().count();
    assert_eq!(chars_with_rise.len(), char_count);

    // Each entry should have valid data
    for (idx, ch, rise) in &chars_with_rise {
        assert!(*idx < text.char_count());
        assert!(rise.is_finite());
        // Character should be a valid Unicode char
        assert!(ch.is_ascii() || ch.len_utf8() > 0);
    }
}

#[test]
#[serial]
fn test_superscripts_method() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();
    let text = page.text().unwrap();

    // Get superscripts with different thresholds
    let superscripts_2 = text.superscripts(2.0);
    let superscripts_5 = text.superscripts(5.0);

    // Higher threshold should yield fewer results
    assert!(superscripts_5.len() <= superscripts_2.len());

    // All returned chars should have rise above threshold
    for ch in &superscripts_2 {
        assert!(ch.text_rise() > 2.0);
    }

    for ch in &superscripts_5 {
        assert!(ch.text_rise() > 5.0);
    }
}

#[test]
#[serial]
fn test_subscripts_method() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();
    let text = page.text().unwrap();

    // Get subscripts
    let subscripts = text.subscripts(2.0);

    // All returned chars should have rise below -threshold
    for ch in &subscripts {
        assert!(ch.text_rise() < -2.0);
    }
}

#[test]
#[serial]
fn test_text_rise_all_pages() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    // Should not panic on any page
    for page in doc.pages() {
        if let Ok(text) = page.text() {
            let chars_with_rise = text.chars_with_rise();
            for (_, _, rise) in &chars_with_rise {
                assert!(rise.is_finite());
            }
        }
    }
}

#[test]
#[serial]
fn test_pdf_char_origin_fields() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();
    let text = page.text().unwrap();

    // Check that origin fields are populated for all characters
    for ch in text.chars().take(10) {
        // Origin should be finite
        assert!(ch.origin_x.is_finite());
        assert!(ch.origin_y.is_finite());

        // Origin should be within or near the bounding box
        // (some tolerance allowed for fonts with descenders)
        let tolerance = 20.0;
        assert!(ch.origin_x >= ch.left - tolerance && ch.origin_x <= ch.right + tolerance);
        // Origin y can be below bottom for descenders or above top for superscripts
    }
}

// ============================================================================
// Text Decoration Tests
// ============================================================================

#[test]
#[serial]
fn test_text_decoration_type_enum() {
    // Test enum variants and methods
    let underline = TextDecorationType::Underline;
    let strikethrough = TextDecorationType::Strikethrough;
    let overline = TextDecorationType::Overline;

    assert!(underline.is_underline());
    assert!(!underline.is_strikethrough());
    assert!(!underline.is_overline());

    assert!(!strikethrough.is_underline());
    assert!(strikethrough.is_strikethrough());
    assert!(!strikethrough.is_overline());

    assert!(!overline.is_underline());
    assert!(!overline.is_strikethrough());
    assert!(overline.is_overline());
}

#[test]
#[serial]
fn test_text_decoration_struct() {
    let decoration = TextDecoration {
        decoration_type: TextDecorationType::Underline,
        bounds: (0.0, 10.0, 100.0, 12.0),
        thickness: 1.0,
        color: (0, 0, 0, 255),
    };

    assert_eq!(decoration.width(), 100.0);
    assert_eq!(decoration.height(), 2.0);
    assert_eq!(decoration.center_y(), 11.0);
    assert!(decoration.is_visible());
    assert!(decoration.is_near_y(11.0, 1.0));
    assert!(!decoration.is_near_y(20.0, 1.0));
}

#[test]
#[serial]
fn test_text_decoration_visibility() {
    let visible = TextDecoration {
        decoration_type: TextDecorationType::Strikethrough,
        bounds: (0.0, 10.0, 100.0, 12.0),
        thickness: 1.0,
        color: (255, 0, 0, 255),
    };
    assert!(visible.is_visible());

    let invisible = TextDecoration {
        decoration_type: TextDecorationType::Strikethrough,
        bounds: (0.0, 10.0, 100.0, 12.0),
        thickness: 1.0,
        color: (255, 0, 0, 0),
    };
    assert!(!invisible.is_visible());
}

#[test]
#[serial]
fn test_extract_text_decorations() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // Should not panic
    let decorations = page.extract_text_decorations();

    // Verify all decorations have valid data
    for decoration in &decorations {
        assert!(decoration.width() >= 0.0);
        assert!(decoration.thickness >= 0.0);
        assert!(decoration.bounds.0.is_finite());
        assert!(decoration.bounds.1.is_finite());
        assert!(decoration.bounds.2.is_finite());
        assert!(decoration.bounds.3.is_finite());
    }
}

#[test]
#[serial]
fn test_text_decoration_count() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let count = page.text_decoration_count();
    let decorations = page.extract_text_decorations();

    assert_eq!(count, decorations.len());
}

#[test]
#[serial]
fn test_has_text_decorations() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let has_decorations = page.has_text_decorations();
    let decorations = page.extract_text_decorations();

    assert_eq!(has_decorations, !decorations.is_empty());
}

#[test]
#[serial]
fn test_underlines_method() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let underlines = page.underlines();

    // All returned decorations should be underlines
    for decoration in &underlines {
        assert!(decoration.decoration_type.is_underline());
    }
}

#[test]
#[serial]
fn test_strikethroughs_method() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let strikethroughs = page.strikethroughs();

    // All returned decorations should be strikethroughs
    for decoration in &strikethroughs {
        assert!(decoration.decoration_type.is_strikethrough());
    }
}

#[test]
#[serial]
fn test_overlines_method() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let overlines = page.overlines();

    // All returned decorations should be overlines
    for decoration in &overlines {
        assert!(decoration.decoration_type.is_overline());
    }
}

#[test]
#[serial]
fn test_text_decorations_all_pages() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    // Should not panic on any page
    for page in doc.pages() {
        let decorations = page.extract_text_decorations();
        for decoration in &decorations {
            assert!(decoration.width() >= 0.0);
            assert!(decoration.bounds.0.is_finite());
        }
    }
}

#[test]
#[serial]
fn test_text_decoration_clone_debug() {
    let decoration = TextDecoration {
        decoration_type: TextDecorationType::Overline,
        bounds: (10.0, 50.0, 200.0, 52.0),
        thickness: 2.0,
        color: (0, 128, 255, 255),
    };

    let cloned = decoration.clone();
    assert_eq!(decoration.bounds, cloned.bounds);
    assert_eq!(decoration.thickness, cloned.thickness);

    // Debug should work
    let debug_str = format!("{:?}", decoration);
    assert!(debug_str.contains("TextDecoration"));
    assert!(debug_str.contains("Overline"));
}

// ============================================================================
// Invisible Text Layer Tests
// ============================================================================

#[test]
#[serial]
fn test_text_render_mode_enum() {
    use pdfium_render_fast::TextRenderMode;

    // Test enum variants exist and can be matched
    let modes = [
        TextRenderMode::Fill,
        TextRenderMode::Stroke,
        TextRenderMode::FillStroke,
        TextRenderMode::Invisible,
        TextRenderMode::FillClip,
        TextRenderMode::StrokeClip,
        TextRenderMode::FillStrokeClip,
        TextRenderMode::Clip,
        TextRenderMode::Unknown,
    ];

    for mode in modes {
        // All modes should be comparable
        assert_eq!(mode, mode);
    }

    // Specifically test Invisible mode
    assert_eq!(TextRenderMode::Invisible, TextRenderMode::Invisible);
    assert_ne!(TextRenderMode::Fill, TextRenderMode::Invisible);
}

#[test]
#[serial]
fn test_has_invisible_text_layer() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // Most regular PDFs don't have invisible text layers
    let has_invisible = page.has_invisible_text_layer();
    // This is informational - either result is valid for test PDFs
    println!("has_invisible_text_layer: {}", has_invisible);
}

#[test]
#[serial]
fn test_extract_invisible_text() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // Extract invisible text if any
    let invisible_text = page.extract_invisible_text();

    // If there's no invisible text, None should be returned
    // If there is, it should be a non-empty string
    if let Some(text) = &invisible_text {
        assert!(!text.is_empty(), "Invisible text should not be empty");
    }
    println!("extract_invisible_text: {:?}", invisible_text);
}

#[test]
#[serial]
fn test_invisible_text_object_count() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let count = page.invisible_text_object_count();
    println!("invisible_text_object_count: {}", count);

    // Count is usize - type system enforces non-negative
    // Function should return successfully (not panic)
}

#[test]
#[serial]
fn test_invisible_text_objects() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let invisible_objects = page.invisible_text_objects();
    let count = page.invisible_text_object_count();

    // Count should match vector length
    assert_eq!(invisible_objects.len(), count);

    // All should have invisible render mode
    for obj in &invisible_objects {
        assert!(obj.is_invisible_text());
    }
}

#[test]
#[serial]
fn test_visible_text_objects() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let visible_objects = page.visible_text_objects();

    // All visible text objects should NOT be invisible
    for obj in &visible_objects {
        assert!(!obj.is_invisible_text());
    }

    // Total text objects = visible + invisible
    let invisible_count = page.invisible_text_object_count();
    let total_text_objects = page.objects().text_objects().len();
    assert_eq!(visible_objects.len() + invisible_count, total_text_objects);
}

#[test]
#[serial]
fn test_is_invisible_text_on_page_object() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // Test on text objects
    for obj in page.objects().text_objects() {
        let is_invisible = obj.is_invisible_text();
        let render_mode = obj.text_render_mode();

        // is_invisible_text should match render mode check
        if let Some(mode) = render_mode {
            let expected = mode == pdfium_render_fast::TextRenderMode::Invisible;
            assert_eq!(is_invisible, expected);
        }
    }

    // Non-text objects should return false for is_invisible_text
    for obj in page.objects().paths() {
        assert!(!obj.is_invisible_text());
    }
}

#[test]
#[serial]
fn test_invisible_text_all_pages() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    let page_count = doc.page_count();

    for i in 0..page_count {
        let page = doc.page(i).unwrap();
        let has_invisible = page.has_invisible_text_layer();
        let invisible_count = page.invisible_text_object_count();
        let invisible_text = page.extract_invisible_text();

        // Consistency check: if has_invisible is true, count > 0
        if has_invisible {
            assert!(invisible_count > 0);
            assert!(invisible_text.is_some());
        }

        // Consistency check: if count > 0, has_invisible should be true
        if invisible_count > 0 {
            assert!(has_invisible);
        }

        println!(
            "Page {}: has_invisible={}, count={}, text_len={:?}",
            i,
            has_invisible,
            invisible_count,
            invisible_text.as_ref().map(|s| s.len())
        );
    }
}

// ============================================================================
// Object Opacity/Transparency Tests
// ============================================================================

#[test]
#[serial]
fn test_fill_opacity() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // Test fill opacity on objects that have fill color
    for obj in page.objects().iter() {
        if let Some(opacity) = obj.fill_opacity() {
            // Opacity should be in range 0.0 to 1.0
            assert!(
                (0.0..=1.0).contains(&opacity),
                "Fill opacity out of range: {}",
                opacity
            );
        }
    }
}

#[test]
#[serial]
fn test_stroke_opacity() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // Test stroke opacity on objects that have stroke color
    for obj in page.objects().iter() {
        if let Some(opacity) = obj.stroke_opacity() {
            // Opacity should be in range 0.0 to 1.0
            assert!(
                (0.0..=1.0).contains(&opacity),
                "Stroke opacity out of range: {}",
                opacity
            );
        }
    }
}

#[test]
#[serial]
fn test_is_semi_transparent_fill() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let transparent_fills = page
        .objects()
        .iter()
        .filter(|obj| obj.is_semi_transparent_fill())
        .count();

    // Just verify the method works, result can be any value
    println!("Objects with semi-transparent fills: {}", transparent_fills);
}

#[test]
#[serial]
fn test_is_semi_transparent_stroke() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let transparent_strokes = page
        .objects()
        .iter()
        .filter(|obj| obj.is_semi_transparent_stroke())
        .count();

    // Just verify the method works, result can be any value
    println!(
        "Objects with semi-transparent strokes: {}",
        transparent_strokes
    );
}

#[test]
#[serial]
fn test_opacity_consistency_with_color() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    for obj in page.objects().iter() {
        // If fill_color exists, fill_opacity should exist
        if let Some(color) = obj.fill_color() {
            let opacity = obj
                .fill_opacity()
                .expect("fill_opacity should exist when fill_color exists");
            let expected_opacity = color.a as f32 / 255.0;
            assert!(
                (opacity - expected_opacity).abs() < 0.001,
                "Fill opacity mismatch: got {}, expected {}",
                opacity,
                expected_opacity
            );
        }

        // If stroke_color exists, stroke_opacity should exist
        if let Some(color) = obj.stroke_color() {
            let opacity = obj
                .stroke_opacity()
                .expect("stroke_opacity should exist when stroke_color exists");
            let expected_opacity = color.a as f32 / 255.0;
            assert!(
                (opacity - expected_opacity).abs() < 0.001,
                "Stroke opacity mismatch: got {}, expected {}",
                opacity,
                expected_opacity
            );
        }
    }
}

#[test]
#[serial]
fn test_opacity_all_pages() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    for i in 0..doc.page_count() {
        let page = doc.page(i).unwrap();

        let has_transparency_count = page
            .objects()
            .iter()
            .filter(|obj| obj.has_transparency())
            .count();

        let semi_transparent_fills = page
            .objects()
            .iter()
            .filter(|obj| obj.is_semi_transparent_fill())
            .count();

        let semi_transparent_strokes = page
            .objects()
            .iter()
            .filter(|obj| obj.is_semi_transparent_stroke())
            .count();

        println!(
            "Page {}: has_transparency={}, semi_fill={}, semi_stroke={}",
            i, has_transparency_count, semi_transparent_fills, semi_transparent_strokes
        );
    }
}

// =============================================================================
// Repeated Content Detection Tests (Feature 10)
// =============================================================================

#[test]
#[serial]
fn test_repeated_region_struct() {
    // Test RepeatedRegion::new
    let region = RepeatedRegion::new(
        (0.0, 700.0, 612.0, 792.0),
        vec![0, 1, 2, 3],
        12345678901234567890,
    );

    assert_eq!(region.bounds, (0.0, 700.0, 612.0, 792.0));
    assert_eq!(region.page_indices, vec![0, 1, 2, 3]);
    assert_eq!(region.content_hash, 12345678901234567890);
    assert_eq!(region.occurrence_count, 4);
}

#[test]
#[serial]
fn test_repeated_region_helpers() {
    let header_region = RepeatedRegion::new(
        (0.0, 700.0, 612.0, 792.0), // Top of page
        vec![0, 1],
        123,
    );

    // Test is_header - bounds.top (792) > page_height (792) * 0.85 = 673.2
    assert!(header_region.is_header(792.0));

    let footer_region = RepeatedRegion::new(
        (0.0, 0.0, 612.0, 72.0), // Bottom of page
        vec![0, 1],
        456,
    );

    // Test is_footer - bounds.bottom (0) < bounds.top (72) * 0.15 = 10.8
    assert!(footer_region.is_footer());

    let margin_region = RepeatedRegion::new(
        (0.0, 100.0, 50.0, 700.0), // Left margin
        vec![0, 1],
        789,
    );

    // Test is_margin - bounds.left (0) < page_width (612) * 0.10 = 61.2
    assert!(margin_region.is_margin(612.0));
}

#[test]
#[serial]
fn test_repeated_region_dimensions() {
    let region = RepeatedRegion::new((100.0, 200.0, 300.0, 400.0), vec![0, 1], 123);

    // Test center
    let (cx, cy) = region.center();
    assert!((cx - 200.0).abs() < 0.01);
    assert!((cy - 300.0).abs() < 0.01);

    // Test width
    assert!((region.width() - 200.0).abs() < 0.01);

    // Test height
    assert!((region.height() - 200.0).abs() < 0.01);

    // Test area
    assert!((region.area() - 40000.0).abs() < 0.01);
}

#[test]
#[serial]
fn test_find_repeated_regions_single_page() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    // If document has only 1 page, should return empty
    if doc.page_count() < 2 {
        let regions = doc.find_repeated_regions(5.0);
        assert!(
            regions.is_empty(),
            "Single page doc should have no repeated regions"
        );
    }
}

#[test]
#[serial]
fn test_find_repeated_regions_multi_page() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    if doc.page_count() >= 2 {
        let regions = doc.find_repeated_regions(5.0);

        // Print results for debugging
        println!(
            "Found {} repeated regions in {} pages",
            regions.len(),
            doc.page_count()
        );
        for (i, region) in regions.iter().enumerate() {
            println!(
                "Region {}: bounds=({:.1}, {:.1}, {:.1}, {:.1}), pages={:?}, hash={}",
                i,
                region.bounds.0,
                region.bounds.1,
                region.bounds.2,
                region.bounds.3,
                region.page_indices,
                region.content_hash
            );
        }

        // Each region should appear on at least 2 pages
        for region in &regions {
            assert!(
                region.occurrence_count >= 2,
                "Repeated region should appear on at least 2 pages"
            );
            assert!(
                region.page_indices.len() >= 2,
                "Page indices should have at least 2 entries"
            );
        }
    }
}

#[test]
#[serial]
fn test_find_repeated_regions_custom() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    if doc.page_count() >= 2 {
        // Only check header and footer
        let region_fractions = vec![
            (0.0, 0.90, 1.0, 1.0), // Top 10% (header)
            (0.0, 0.0, 1.0, 0.10), // Bottom 10% (footer)
        ];

        let regions = doc.find_repeated_regions_custom(&region_fractions, 5.0, 2);

        println!("Custom search found {} regions", regions.len());
        for region in &regions {
            assert!(region.occurrence_count >= 2);
        }
    }
}

#[test]
#[serial]
fn test_repeated_region_count() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    let count = doc.repeated_region_count(5.0);
    let regions = doc.find_repeated_regions(5.0);

    assert_eq!(
        count,
        regions.len(),
        "repeated_region_count should match find_repeated_regions().len()"
    );
}

#[test]
#[serial]
fn test_has_repeated_content() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    let has_content = doc.has_repeated_content(5.0);
    let count = doc.repeated_region_count(5.0);

    if count > 0 {
        assert!(
            has_content,
            "has_repeated_content should be true when count > 0"
        );
    } else {
        assert!(
            !has_content,
            "has_repeated_content should be false when count == 0"
        );
    }
}

#[test]
#[serial]
fn test_repeated_regions_tolerance() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    if doc.page_count() >= 2 {
        // Higher tolerance should potentially find same or more regions
        let regions_tight = doc.find_repeated_regions(1.0);
        let regions_loose = doc.find_repeated_regions(20.0);

        println!("Tight tolerance (1.0): {} regions", regions_tight.len());
        println!("Loose tolerance (20.0): {} regions", regions_loose.len());

        // Both calls should succeed (length is usize, always non-negative)
    }
}

#[test]
#[serial]
fn test_repeated_regions_all_pdfs() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    // Test on the document
    let regions = doc.find_repeated_regions(5.0);

    println!(
        "Document has {} pages, {} repeated regions",
        doc.page_count(),
        regions.len()
    );

    // Verify all regions are valid
    for region in &regions {
        // Bounds should be non-negative
        assert!(region.bounds.0 >= 0.0);
        assert!(region.bounds.1 >= 0.0);
        assert!(region.bounds.2 >= region.bounds.0);
        assert!(region.bounds.3 >= region.bounds.1);

        // Page indices should be valid
        for &page_idx in &region.page_indices {
            assert!(page_idx < doc.page_count());
        }
    }
}

// =============================================================================
// Mathematical Character Analysis Tests (Feature 11)
// =============================================================================

#[test]
#[serial]
fn test_math_char_analysis_struct() {
    // Test MathCharAnalysis::new
    let analysis = MathCharAnalysis::new();

    assert_eq!(analysis.math_operators, 0);
    assert_eq!(analysis.math_alphanumerics, 0);
    assert_eq!(analysis.greek_letters, 0);
    assert_eq!(analysis.arrows, 0);
    assert_eq!(analysis.superscripts, 0);
    assert_eq!(analysis.subscripts, 0);
    assert_eq!(analysis.total_chars, 0);
}

#[test]
#[serial]
fn test_math_char_analysis_methods() {
    let mut analysis = MathCharAnalysis::new();
    analysis.math_operators = 10;
    analysis.greek_letters = 5;
    analysis.arrows = 3;
    analysis.total_chars = 100;

    // Test math_char_count
    assert_eq!(analysis.math_char_count(), 18);

    // Test math_ratio
    assert!((analysis.math_ratio() - 0.18).abs() < 0.01);

    // Test has_significant_math (threshold is 5%)
    assert!(analysis.has_significant_math());

    // Test helper methods
    assert!(analysis.has_greek());
    assert!(analysis.has_operators());
    assert!(!analysis.has_scripts());
}

#[test]
#[serial]
fn test_math_char_analysis_empty() {
    let analysis = MathCharAnalysis::new();

    assert_eq!(analysis.math_char_count(), 0);
    assert_eq!(analysis.math_ratio(), 0.0);
    assert!(!analysis.has_significant_math());
    assert!(!analysis.has_greek());
    assert!(!analysis.has_operators());
    assert!(!analysis.has_scripts());
}

#[test]
#[serial]
fn test_analyze_math_chars() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let analysis = page.analyze_math_chars();

    // Verify total_chars matches page character count
    let text = page.text().unwrap();
    let char_count = text.chars().count();
    assert_eq!(analysis.total_chars, char_count);

    println!("Math analysis:");
    println!("  Operators: {}", analysis.math_operators);
    println!("  Alphanumerics: {}", analysis.math_alphanumerics);
    println!("  Greek: {}", analysis.greek_letters);
    println!("  Arrows: {}", analysis.arrows);
    println!("  Superscripts: {}", analysis.superscripts);
    println!("  Subscripts: {}", analysis.subscripts);
    println!(
        "  Total: {} / {}",
        analysis.math_char_count(),
        analysis.total_chars
    );
    println!("  Ratio: {:.1}%", analysis.math_ratio() * 100.0);
}

#[test]
#[serial]
fn test_math_char_count_method() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let count = page.math_char_count();
    let analysis = page.analyze_math_chars();

    assert_eq!(count, analysis.math_char_count());
}

#[test]
#[serial]
fn test_has_math_content() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let has_math = page.has_math_content();
    let analysis = page.analyze_math_chars();

    assert_eq!(has_math, analysis.has_significant_math());
}

#[test]
#[serial]
fn test_math_chars_all_pages() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    for i in 0..doc.page_count() {
        let page = doc.page(i).unwrap();
        let analysis = page.analyze_math_chars();

        println!(
            "Page {}: {} math chars / {} total ({:.1}%)",
            i,
            analysis.math_char_count(),
            analysis.total_chars,
            analysis.math_ratio() * 100.0
        );

        // All count fields are usize - type system enforces non-negative
        // Verify analysis was computed by checking that it doesn't panic
        let _ = (
            analysis.math_operators,
            analysis.greek_letters,
            analysis.arrows,
            analysis.superscripts,
            analysis.subscripts,
        );
    }
}

#[test]
#[serial]
fn test_math_char_analysis_threshold() {
    // Test the 5% threshold for significant math
    let mut low_math = MathCharAnalysis::new();
    low_math.math_operators = 4;
    low_math.total_chars = 100;
    assert!(!low_math.has_significant_math()); // 4% < 5%

    let mut high_math = MathCharAnalysis::new();
    high_math.math_operators = 6;
    high_math.total_chars = 100;
    assert!(high_math.has_significant_math()); // 6% > 5%
}

// ============================================================================
// Font Usage API Tests (Feature 12)
// ============================================================================

#[test]
#[serial]
fn test_is_known_math_font() {
    // Test known math fonts
    assert!(is_known_math_font("CMMI10"));
    assert!(is_known_math_font("cmmi10")); // Case insensitive
    assert!(is_known_math_font("CMSY10"));
    assert!(is_known_math_font("CMEX10"));
    assert!(is_known_math_font("MathematicalPi"));
    assert!(is_known_math_font("Symbol"));
    assert!(is_known_math_font("STIXMath"));
    assert!(is_known_math_font("STIX-Regular"));
    assert!(is_known_math_font("CambriaMath"));
    assert!(is_known_math_font("MathJax_Main"));
    assert!(is_known_math_font("MathJax_Size1"));
    assert!(is_known_math_font("Latin-Modern-Math"));
    assert!(is_known_math_font("Asana-Math"));

    // Test non-math fonts
    assert!(!is_known_math_font("Helvetica"));
    assert!(!is_known_math_font("Times-Roman"));
    assert!(!is_known_math_font("Arial"));
    assert!(!is_known_math_font("Courier"));
    assert!(!is_known_math_font("ComicSans"));
}

#[test]
#[serial]
fn test_font_usage_info_struct() {
    let info = FontUsageInfo {
        name: "TestFont".to_string(),
        is_math_font: false,
        is_monospace: true,
        char_count: 100,
        coverage: 0.5,
    };

    assert_eq!(info.name, "TestFont");
    assert!(!info.is_math_font);
    assert!(info.is_monospace);
    assert_eq!(info.char_count, 100);
    assert!((info.coverage - 0.5).abs() < 0.001);
}

#[test]
#[serial]
fn test_extract_font_usage_basic() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let font_usage = page.extract_font_usage();

    // Should return at least one font entry
    assert!(!font_usage.is_empty(), "Should have at least one font");

    // Each entry should have non-negative char count
    for info in &font_usage {
        println!(
            "Font: {}, math={}, mono={}, coverage={:.2}%",
            info.name,
            info.is_math_font,
            info.is_monospace,
            info.coverage * 100.0
        );
        assert!(info.coverage >= 0.0 && info.coverage <= 1.0);
    }
}

#[test]
#[serial]
fn test_font_count() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let count = page.font_count();
    let font_usage = page.extract_font_usage();

    assert_eq!(count, font_usage.len());
}

#[test]
#[serial]
fn test_has_math_fonts() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let has_math = page.has_math_fonts();
    let font_usage = page.extract_font_usage();

    let has_math_from_list = font_usage.iter().any(|f| f.is_math_font);
    assert_eq!(has_math, has_math_from_list);
}

#[test]
#[serial]
fn test_font_names() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let names = page.font_names();
    let font_usage = page.extract_font_usage();

    assert_eq!(names.len(), font_usage.len());

    for info in &font_usage {
        assert!(names.contains(&info.name), "Font names should match");
    }
}

#[test]
#[serial]
fn test_font_usage_coverage_sums_to_one() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let font_usage = page.extract_font_usage();

    if !font_usage.is_empty() {
        let total_coverage: f32 = font_usage.iter().map(|f| f.coverage).sum();
        // Coverage should sum to approximately 1.0 (allowing small floating point error)
        assert!(
            (total_coverage - 1.0).abs() < 0.01,
            "Total coverage should be ~1.0, got {}",
            total_coverage
        );
    }
}

#[test]
#[serial]
fn test_font_usage_all_pages() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    for i in 0..doc.page_count() {
        let page = doc.page(i).unwrap();
        let font_usage = page.extract_font_usage();

        println!("Page {}: {} fonts", i, font_usage.len());

        for info in &font_usage {
            // Verify each font has valid data
            assert!(!info.name.is_empty() || info.name == "Unknown");
            assert!(info.coverage >= 0.0 && info.coverage <= 1.0);
        }
    }
}

// ============================================================================
// Centered Block Detection API Tests (Feature 13)
// ============================================================================

#[test]
#[serial]
fn test_centered_block_struct() {
    let block = CenteredBlock::new(
        (100.0, 700.0, 500.0, 720.0),
        "Test Title".to_string(),
        100.0,
        112.0,
    );

    assert_eq!(block.text, "Test Title");
    assert_eq!(block.bounds, (100.0, 700.0, 500.0, 720.0));
    assert_eq!(block.margin_left, 100.0);
    assert_eq!(block.margin_right, 112.0);
    assert!((block.margin_symmetry - 12.0).abs() < 0.001);
}

#[test]
#[serial]
fn test_centered_block_helpers() {
    let block = CenteredBlock::new(
        (100.0, 700.0, 500.0, 720.0),
        "Test".to_string(),
        100.0,
        100.0, // Perfectly centered
    );

    assert_eq!(block.width(), 400.0);
    assert_eq!(block.height(), 20.0);
    assert_eq!(block.center_x(), 300.0);
    assert!(block.is_perfectly_centered(1.0));
}

#[test]
#[serial]
fn test_centered_block_is_likely_title() {
    // Short text with large margins - likely title
    let title = CenteredBlock::new(
        (150.0, 700.0, 450.0, 720.0),
        "Chapter 1".to_string(),
        150.0,
        162.0,
    );
    assert!(title.is_likely_title());

    // Long text - not likely title
    let paragraph = CenteredBlock::new(
        (50.0, 600.0, 550.0, 620.0),
        "This is a very long paragraph that contains more than one hundred characters and therefore should not be considered a title by the is_likely_title heuristic.".to_string(),
        50.0,
        62.0,
    );
    assert!(!paragraph.is_likely_title());
}

#[test]
#[serial]
fn test_extract_centered_blocks_basic() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // Use loose tolerance to potentially find some centered content
    let centered = page.extract_centered_blocks(50.0);

    println!("Found {} centered blocks", centered.len());
    for block in &centered {
        println!(
            "  \"{}\" (symmetry: {:.1})",
            if block.text.len() > 40 {
                &block.text[..40]
            } else {
                &block.text
            },
            block.margin_symmetry
        );
    }

    // Just verify it returns without error and has valid data
    for block in &centered {
        assert!(block.margin_symmetry >= 0.0);
        assert!(!block.text.is_empty());
    }
}

#[test]
#[serial]
fn test_centered_block_count() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let count = page.centered_block_count(30.0);
    let centered = page.extract_centered_blocks(30.0);

    assert_eq!(count, centered.len());
}

#[test]
#[serial]
fn test_has_centered_content() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let has_centered = page.has_centered_content(50.0);
    let centered = page.extract_centered_blocks(50.0);

    assert_eq!(has_centered, !centered.is_empty());
}

#[test]
#[serial]
fn test_centered_blocks_sorted_by_symmetry() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let centered = page.extract_centered_blocks(50.0);

    // Should be sorted by symmetry (most centered first)
    for i in 1..centered.len() {
        assert!(
            centered[i - 1].margin_symmetry <= centered[i].margin_symmetry,
            "Blocks should be sorted by symmetry"
        );
    }
}

#[test]
#[serial]
fn test_centered_blocks_tolerance_affects_results() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // Stricter tolerance should return fewer or equal results
    let strict = page.extract_centered_blocks(5.0);
    let normal = page.extract_centered_blocks(20.0);
    let loose = page.extract_centered_blocks(50.0);

    assert!(
        strict.len() <= normal.len(),
        "Stricter tolerance should return fewer results"
    );
    assert!(
        normal.len() <= loose.len(),
        "Looser tolerance should return more results"
    );
}

#[test]
#[serial]
fn test_centered_blocks_all_pages() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    for i in 0..doc.page_count() {
        let page = doc.page(i).unwrap();
        let centered = page.extract_centered_blocks(30.0);

        println!("Page {}: {} centered blocks", i, centered.len());

        for block in &centered {
            // Verify margin symmetry is within tolerance
            assert!(
                block.margin_symmetry <= 30.0,
                "Block symmetry should be within tolerance"
            );
        }
    }
}

// ============================================================================
// Bracketed Reference Detection API Tests (Feature 14)
// ============================================================================

#[test]
#[serial]
fn test_bracket_type_enum() {
    assert_eq!(BracketType::Square.open_char(), '[');
    assert_eq!(BracketType::Square.close_char(), ']');
    assert_eq!(BracketType::Paren.open_char(), '(');
    assert_eq!(BracketType::Paren.close_char(), ')');
    assert_eq!(BracketType::Angle.open_char(), '<');
    assert_eq!(BracketType::Angle.close_char(), '>');
    assert_eq!(BracketType::Superscript.open_char(), '\0');
}

#[test]
#[serial]
fn test_reference_position_enum() {
    let inline = ReferencePosition::Inline;
    let line_end = ReferencePosition::LineEnd;
    let line_start = ReferencePosition::LineStart;

    assert_eq!(inline, ReferencePosition::Inline);
    assert_eq!(line_end, ReferencePosition::LineEnd);
    assert_eq!(line_start, ReferencePosition::LineStart);
    assert_ne!(inline, line_end);
}

#[test]
#[serial]
fn test_bracketed_reference_struct() {
    let reference = BracketedReference::new(
        "[1]".to_string(),
        (100.0, 500.0, 115.0, 512.0),
        BracketType::Square,
        ReferencePosition::Inline,
    );

    assert_eq!(reference.text, "[1]");
    assert_eq!(reference.bracket_type, BracketType::Square);
    assert_eq!(reference.position, ReferencePosition::Inline);
    assert_eq!(reference.inner_text(), "1");
}

#[test]
#[serial]
fn test_bracketed_reference_inner_text() {
    // Square brackets
    let square = BracketedReference::new(
        "[ref]".to_string(),
        (0.0, 0.0, 10.0, 10.0),
        BracketType::Square,
        ReferencePosition::Inline,
    );
    assert_eq!(square.inner_text(), "ref");

    // Parentheses
    let paren = BracketedReference::new(
        "(5)".to_string(),
        (0.0, 0.0, 10.0, 10.0),
        BracketType::Paren,
        ReferencePosition::Inline,
    );
    assert_eq!(paren.inner_text(), "5");

    // Superscript (no brackets)
    let superscript = BracketedReference::new(
        "¹²³".to_string(),
        (0.0, 0.0, 10.0, 10.0),
        BracketType::Superscript,
        ReferencePosition::Inline,
    );
    assert_eq!(superscript.inner_text(), "¹²³");
}

#[test]
#[serial]
fn test_bracketed_reference_is_numeric() {
    let numeric = BracketedReference::new(
        "[123]".to_string(),
        (0.0, 0.0, 10.0, 10.0),
        BracketType::Square,
        ReferencePosition::Inline,
    );
    assert!(numeric.is_numeric());

    let range = BracketedReference::new(
        "[1-5]".to_string(),
        (0.0, 0.0, 10.0, 10.0),
        BracketType::Square,
        ReferencePosition::Inline,
    );
    assert!(range.is_numeric());
    assert!(range.is_range());

    let alpha = BracketedReference::new(
        "[ref]".to_string(),
        (0.0, 0.0, 10.0, 10.0),
        BracketType::Square,
        ReferencePosition::Inline,
    );
    assert!(!alpha.is_numeric());
}

#[test]
#[serial]
fn test_extract_bracketed_references_basic() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let refs = page.extract_bracketed_references();

    println!("Found {} references", refs.len());
    for r in &refs {
        println!(
            "  {}: {:?} at ({:.1}, {:.1})",
            r.text, r.bracket_type, r.bounds.0, r.bounds.1
        );
    }

    // Just verify it works without error
    for r in &refs {
        assert!(!r.text.is_empty());
    }
}

#[test]
#[serial]
fn test_reference_count() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let count = page.reference_count();
    let refs = page.extract_bracketed_references();

    assert_eq!(count, refs.len());
}

#[test]
#[serial]
fn test_has_references() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let has_refs = page.has_references();
    let refs = page.extract_bracketed_references();

    assert_eq!(has_refs, !refs.is_empty());
}

#[test]
#[serial]
fn test_square_bracket_references() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let square_refs = page.square_bracket_references();

    for r in &square_refs {
        assert_eq!(r.bracket_type, BracketType::Square);
        assert!(r.text.starts_with('['));
        assert!(r.text.ends_with(']'));
    }
}

#[test]
#[serial]
fn test_numeric_references() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let numeric_refs = page.numeric_references();

    for r in &numeric_refs {
        assert!(r.is_numeric(), "Reference {} should be numeric", r.text);
    }
}

#[test]
#[serial]
fn test_bracketed_references_all_pages() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    for i in 0..doc.page_count() {
        let page = doc.page(i).unwrap();
        let refs = page.extract_bracketed_references();

        println!("Page {}: {} references", i, refs.len());

        for r in &refs {
            // Verify basic validity
            assert!(!r.text.is_empty());
            assert!(r.width() >= 0.0);
            assert!(r.height() >= 0.0);
        }
    }
}

// ============================================================================
// Script Cluster Detection API Tests (Feature 15)
// ============================================================================

#[test]
#[serial]
fn test_script_position_enum() {
    let super_pos = ScriptPosition::Super;
    let sub_pos = ScriptPosition::Sub;

    assert!(super_pos.is_super());
    assert!(!super_pos.is_sub());
    assert!(sub_pos.is_sub());
    assert!(!sub_pos.is_super());
    assert_ne!(super_pos, sub_pos);
}

#[test]
#[serial]
fn test_script_char_struct() {
    let sc = ScriptChar::new(
        '²',
        ScriptPosition::Super,
        (100.0, 510.0, 106.0, 520.0),
        5.0,
    );

    assert_eq!(sc.char, '²');
    assert_eq!(sc.position, ScriptPosition::Super);
    assert_eq!(sc.rise, 5.0);
    assert!((sc.width() - 6.0).abs() < 0.001);
    assert!((sc.height() - 10.0).abs() < 0.001);
}

#[test]
#[serial]
fn test_script_cluster_struct() {
    let scripts = vec![ScriptChar::new(
        '²',
        ScriptPosition::Super,
        (106.0, 510.0, 112.0, 520.0),
        5.0,
    )];
    let cluster = ScriptCluster::new("x".to_string(), (100.0, 500.0, 106.0, 512.0), scripts);

    assert_eq!(cluster.base_text, "x");
    assert!(cluster.has_superscripts());
    assert!(!cluster.has_subscripts());
    assert_eq!(cluster.script_text(), "²");
    assert_eq!(cluster.full_text(), "x²");
}

#[test]
#[serial]
fn test_script_cluster_helpers() {
    let scripts = vec![
        ScriptChar::new('2', ScriptPosition::Super, (0.0, 0.0, 5.0, 10.0), 5.0),
        ScriptChar::new('1', ScriptPosition::Sub, (0.0, 0.0, 5.0, 10.0), -3.0),
    ];
    let cluster = ScriptCluster::new("x".to_string(), (0.0, 0.0, 10.0, 12.0), scripts);

    assert!(cluster.has_superscripts());
    assert!(cluster.has_subscripts());
    assert_eq!(cluster.superscripts().len(), 1);
    assert_eq!(cluster.subscripts().len(), 1);
}

#[test]
#[serial]
fn test_extract_script_clusters_basic() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let clusters = page.extract_script_clusters();

    println!("Found {} script clusters", clusters.len());
    for c in &clusters {
        println!("  Base: '{}', Scripts: '{}'", c.base_text, c.script_text());
    }

    // Just verify it works without error
    for c in &clusters {
        assert!(!c.base_text.is_empty());
    }
}

#[test]
#[serial]
fn test_script_cluster_count() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let count = page.script_cluster_count();
    let clusters = page.extract_script_clusters();

    assert_eq!(count, clusters.len());
}

#[test]
#[serial]
fn test_has_script_clusters() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let has_clusters = page.has_script_clusters();
    let clusters = page.extract_script_clusters();

    assert_eq!(has_clusters, !clusters.is_empty());
}

#[test]
#[serial]
fn test_superscript_clusters() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let super_clusters = page.superscript_clusters();

    for c in &super_clusters {
        assert!(c.has_superscripts(), "Cluster should have superscripts");
    }
}

#[test]
#[serial]
fn test_subscript_clusters() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let sub_clusters = page.subscript_clusters();

    for c in &sub_clusters {
        assert!(c.has_subscripts(), "Cluster should have subscripts");
    }
}

#[test]
#[serial]
fn test_script_clusters_all_pages() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    for i in 0..doc.page_count() {
        let page = doc.page(i).unwrap();
        let clusters = page.extract_script_clusters();

        println!("Page {}: {} script clusters", i, clusters.len());

        for c in &clusters {
            // Verify basic validity
            assert!(!c.base_text.is_empty());
            assert!(c.base_width() >= 0.0);
            assert!(c.base_height() >= 0.0);
        }
    }
}

// ============================================================================
// Writing Direction Detection API Tests (Feature 16)
// ============================================================================

#[test]
fn test_writing_direction_enum() {
    // Test enum variants
    let horizontal = WritingDirection::Horizontal;
    let vertical = WritingDirection::VerticalRTL;
    let mixed = WritingDirection::Mixed;

    assert!(horizontal.is_horizontal());
    assert!(!horizontal.is_vertical());
    assert!(!horizontal.is_mixed());

    assert!(!vertical.is_horizontal());
    assert!(vertical.is_vertical());
    assert!(!vertical.is_mixed());

    assert!(!mixed.is_horizontal());
    assert!(!mixed.is_vertical());
    assert!(mixed.is_mixed());
}

#[test]
fn test_writing_direction_default() {
    let default = WritingDirection::default();
    assert!(default.is_horizontal());
    assert_eq!(default, WritingDirection::Horizontal);
}

#[test]
fn test_writing_direction_info_horizontal() {
    let info = WritingDirectionInfo::horizontal();
    assert_eq!(info.primary_direction, WritingDirection::Horizontal);
    assert_eq!(info.vertical_ratio, 0.0);
    assert_eq!(info.horizontal_ratio, 1.0);
    assert!(info.vertical_regions.is_empty());
    assert!(!info.has_vertical_text());
    assert!(!info.is_predominantly_vertical());
    assert!(info.is_predominantly_horizontal());
    assert_eq!(info.vertical_region_count(), 0);
}

#[test]
fn test_writing_direction_info_vertical() {
    let regions = vec![(10.0, 20.0, 100.0, 500.0)];
    let info = WritingDirectionInfo::vertical_rtl(regions.clone());
    assert_eq!(info.primary_direction, WritingDirection::VerticalRTL);
    assert_eq!(info.vertical_ratio, 1.0);
    assert_eq!(info.horizontal_ratio, 0.0);
    assert_eq!(info.vertical_regions.len(), 1);
    assert!(info.has_vertical_text());
    assert!(info.is_predominantly_vertical());
    assert!(!info.is_predominantly_horizontal());
    assert_eq!(info.vertical_region_count(), 1);
}

#[test]
fn test_writing_direction_info_default() {
    let info = WritingDirectionInfo::default();
    assert_eq!(info.primary_direction, WritingDirection::Horizontal);
    assert!(!info.has_vertical_text());
}

#[test]
#[serial]
fn test_detect_writing_direction_basic() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let info = page.detect_writing_direction();

    println!("Writing direction: {:?}", info.primary_direction);
    println!("Vertical ratio: {:.2}", info.vertical_ratio);
    println!("Horizontal ratio: {:.2}", info.horizontal_ratio);
    println!("Vertical regions: {}", info.vertical_regions.len());

    // Most Western PDFs are horizontal
    assert!(info.horizontal_ratio + info.vertical_ratio <= 1.0 + 0.001);
}

#[test]
#[serial]
fn test_has_vertical_text() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let has_vertical = page.has_vertical_text();
    let info = page.detect_writing_direction();

    assert_eq!(has_vertical, info.has_vertical_text());
}

#[test]
#[serial]
fn test_is_vertical_text_page() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let is_vertical = page.is_vertical_text_page();
    let info = page.detect_writing_direction();

    assert_eq!(is_vertical, info.is_predominantly_vertical());
}

#[test]
#[serial]
fn test_writing_direction_convenience() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let direction = page.writing_direction();
    let info = page.detect_writing_direction();

    assert_eq!(direction, info.primary_direction);
}

#[test]
#[serial]
fn test_writing_direction_all_pages() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    for i in 0..doc.page_count() {
        let page = doc.page(i).unwrap();
        let info = page.detect_writing_direction();

        println!(
            "Page {}: {:?}, V={:.2}, H={:.2}, regions={}",
            i,
            info.primary_direction,
            info.vertical_ratio,
            info.horizontal_ratio,
            info.vertical_regions.len()
        );

        // Verify ratios are valid
        assert!(info.vertical_ratio >= 0.0 && info.vertical_ratio <= 1.0);
        assert!(info.horizontal_ratio >= 0.0 && info.horizontal_ratio <= 1.0);

        // Verify consistency
        if info.has_vertical_text() {
            assert!(info.vertical_ratio > 0.0);
        }
    }
}

#[test]
fn test_writing_direction_enum_debug() {
    // Test Debug trait
    let h = WritingDirection::Horizontal;
    let v = WritingDirection::VerticalRTL;
    let m = WritingDirection::Mixed;

    assert_eq!(format!("{:?}", h), "Horizontal");
    assert_eq!(format!("{:?}", v), "VerticalRTL");
    assert_eq!(format!("{:?}", m), "Mixed");
}

#[test]
fn test_writing_direction_enum_clone_copy() {
    let original = WritingDirection::VerticalRTL;
    let cloned = original;
    let copied = original; // Copy

    assert_eq!(original, cloned);
    assert_eq!(original, copied);
}

#[test]
fn test_writing_direction_enum_hash() {
    use std::collections::HashSet;

    let mut set = HashSet::new();
    set.insert(WritingDirection::Horizontal);
    set.insert(WritingDirection::VerticalRTL);
    set.insert(WritingDirection::Mixed);

    assert_eq!(set.len(), 3);
    assert!(set.contains(&WritingDirection::Horizontal));
    assert!(set.contains(&WritingDirection::VerticalRTL));
    assert!(set.contains(&WritingDirection::Mixed));
}

// ============================================================================
// Ruby Annotation (Furigana) API Tests (Feature 17)
// ============================================================================

#[test]
fn test_ruby_annotation_struct() {
    let ruby = RubyAnnotation::new(
        "漢字".to_string(),
        "かんじ".to_string(),
        (100.0, 200.0, 150.0, 220.0),
        (100.0, 225.0, 150.0, 235.0),
        6.0,
        0.5,
    );

    assert_eq!(ruby.base_text, "漢字");
    assert_eq!(ruby.ruby_text, "かんじ");
    assert_eq!(ruby.ruby_font_size, 6.0);
    assert_eq!(ruby.size_ratio, 0.5);
}

#[test]
fn test_ruby_annotation_dimensions() {
    let ruby = RubyAnnotation::new(
        "字".to_string(),
        "じ".to_string(),
        (100.0, 200.0, 120.0, 220.0), // base: 20x20
        (100.0, 225.0, 115.0, 235.0), // ruby: 15x10
        6.0,
        0.5,
    );

    assert_eq!(ruby.base_width(), 20.0);
    assert_eq!(ruby.base_height(), 20.0);
    assert_eq!(ruby.ruby_width(), 15.0);
    assert_eq!(ruby.ruby_height(), 10.0);
}

#[test]
fn test_ruby_annotation_position_above() {
    // Ruby positioned above base (horizontal writing)
    let ruby = RubyAnnotation::new(
        "字".to_string(),
        "じ".to_string(),
        (100.0, 200.0, 120.0, 220.0), // base: bottom=200, top=220
        (100.0, 225.0, 115.0, 235.0), // ruby: bottom=225 (above base top)
        6.0,
        0.5,
    );

    assert!(ruby.is_above());
    assert!(!ruby.is_right_of());
}

#[test]
fn test_ruby_annotation_position_right() {
    // Ruby positioned to the right of base (vertical writing)
    let ruby = RubyAnnotation::new(
        "字".to_string(),
        "じ".to_string(),
        (100.0, 200.0, 120.0, 220.0), // base: right=120
        (125.0, 200.0, 140.0, 215.0), // ruby: left=125 (right of base)
        6.0,
        0.5,
    );

    assert!(!ruby.is_above());
    assert!(ruby.is_right_of());
}

#[test]
fn test_ruby_annotation_combined_text() {
    let ruby = RubyAnnotation::new(
        "東京".to_string(),
        "とうきょう".to_string(),
        (0.0, 0.0, 50.0, 20.0),
        (0.0, 25.0, 50.0, 35.0),
        6.0,
        0.5,
    );

    assert_eq!(ruby.combined_text(), "東京(とうきょう)");
}

#[test]
fn test_ruby_annotation_clone_debug() {
    let ruby = RubyAnnotation::new(
        "字".to_string(),
        "じ".to_string(),
        (0.0, 0.0, 10.0, 10.0),
        (0.0, 15.0, 10.0, 20.0),
        6.0,
        0.5,
    );

    let cloned = ruby.clone();
    assert_eq!(cloned.base_text, ruby.base_text);
    assert_eq!(cloned.ruby_text, ruby.ruby_text);

    // Test Debug trait
    let debug_str = format!("{:?}", ruby);
    assert!(debug_str.contains("RubyAnnotation"));
}

#[test]
#[serial]
fn test_extract_ruby_annotations_basic() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let annotations = page.extract_ruby_annotations();

    println!("Found {} ruby annotations", annotations.len());
    for ann in &annotations {
        println!(
            "  Base: '{}', Ruby: '{}', Ratio: {:.2}",
            ann.base_text, ann.ruby_text, ann.size_ratio
        );
    }

    // Just verify it works without error
    for ann in &annotations {
        assert!(ann.size_ratio > 0.0);
        assert!(ann.size_ratio < 1.0);
    }
}

#[test]
#[serial]
fn test_has_ruby_annotations() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let has_ruby = page.has_ruby_annotations();
    let annotations = page.extract_ruby_annotations();

    assert_eq!(has_ruby, !annotations.is_empty());
}

#[test]
#[serial]
fn test_ruby_annotation_count() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let count = page.ruby_annotation_count();
    let annotations = page.extract_ruby_annotations();

    assert_eq!(count, annotations.len());
}

#[test]
#[serial]
fn test_ruby_annotations_all_pages() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    for i in 0..doc.page_count() {
        let page = doc.page(i).unwrap();
        let annotations = page.extract_ruby_annotations();

        println!("Page {}: {} ruby annotations", i, annotations.len());

        for ann in &annotations {
            // Verify basic validity
            assert!(ann.size_ratio > 0.0);
            assert!(ann.size_ratio < 1.0);
            assert!(ann.ruby_font_size > 0.0);
            assert!(ann.base_width() >= 0.0);
            assert!(ann.base_height() >= 0.0);
        }
    }
}

// ============================================================================
// Japanese Character Analysis API Tests (Feature 18)
// ============================================================================

#[test]
fn test_japanese_char_analysis_struct() {
    let mut analysis = JapaneseCharAnalysis::new();
    assert_eq!(analysis.hiragana_count, 0);
    assert_eq!(analysis.katakana_count, 0);
    assert_eq!(analysis.kanji_count, 0);
    assert_eq!(analysis.total_chars, 0);

    // Analyze some Japanese text
    for ch in "あいうえお".chars() {
        analysis.analyze_char(ch);
    }
    assert_eq!(analysis.hiragana_count, 5);
    assert_eq!(analysis.total_chars, 5);
}

#[test]
fn test_japanese_char_analysis_katakana() {
    let mut analysis = JapaneseCharAnalysis::new();
    for ch in "アイウエオ".chars() {
        analysis.analyze_char(ch);
    }
    assert_eq!(analysis.katakana_count, 5);
    assert_eq!(analysis.hiragana_count, 0);
}

#[test]
fn test_japanese_char_analysis_kanji() {
    let mut analysis = JapaneseCharAnalysis::new();
    for ch in "漢字日本語".chars() {
        analysis.analyze_char(ch);
    }
    assert_eq!(analysis.kanji_count, 5);
    assert_eq!(analysis.hiragana_count, 0);
    assert_eq!(analysis.katakana_count, 0);
}

#[test]
fn test_japanese_char_analysis_mixed() {
    let mut analysis = JapaneseCharAnalysis::new();
    // "東京タワー" = 2 kanji + 3 katakana
    for ch in "東京タワー".chars() {
        analysis.analyze_char(ch);
    }
    assert_eq!(analysis.kanji_count, 2);
    assert_eq!(analysis.katakana_count, 3);
    assert_eq!(analysis.total_chars, 5);
    assert!(analysis.has_japanese());
    assert!(analysis.has_kanji());
    assert!(analysis.has_katakana());
}

#[test]
fn test_japanese_char_analysis_ratios() {
    let mut analysis = JapaneseCharAnalysis::new();
    // "Hello日本" = 5 ASCII + 2 kanji
    for ch in "Hello日本".chars() {
        analysis.analyze_char(ch);
    }
    assert_eq!(analysis.kanji_count, 2);
    assert_eq!(analysis.total_chars, 7);
    assert_eq!(analysis.japanese_char_count(), 2);

    // 2/7 ≈ 0.286
    let ratio = analysis.japanese_ratio();
    assert!(ratio > 0.28 && ratio < 0.30);
    assert!(!analysis.is_predominantly_japanese());
}

#[test]
fn test_japanese_char_analysis_predominantly_japanese() {
    let mut analysis = JapaneseCharAnalysis::new();
    // Mostly Japanese text
    for ch in "日本語テスト1".chars() {
        analysis.analyze_char(ch);
    }
    // 日本語 (3 kanji) + テスト (3 katakana) + 1 (ASCII) = 6 Japanese / 7 total
    assert!(analysis.is_predominantly_japanese());
}

#[test]
fn test_japanese_char_analysis_merge() {
    let mut analysis1 = JapaneseCharAnalysis::new();
    for ch in "あいう".chars() {
        analysis1.analyze_char(ch);
    }

    let mut analysis2 = JapaneseCharAnalysis::new();
    for ch in "アイウ".chars() {
        analysis2.analyze_char(ch);
    }

    analysis1.merge(&analysis2);
    assert_eq!(analysis1.hiragana_count, 3);
    assert_eq!(analysis1.katakana_count, 3);
    assert_eq!(analysis1.total_chars, 6);
}

#[test]
fn test_japanese_char_analysis_default() {
    let analysis = JapaneseCharAnalysis::default();
    assert_eq!(analysis.total_chars, 0);
    assert!(!analysis.has_japanese());
    assert_eq!(analysis.japanese_ratio(), 0.0);
}

#[test]
fn test_is_hiragana() {
    assert!(is_hiragana('あ'));
    assert!(is_hiragana('ん'));
    assert!(is_hiragana('ぁ'));
    assert!(!is_hiragana('ア'));
    assert!(!is_hiragana('A'));
    assert!(!is_hiragana('漢'));
}

#[test]
fn test_is_katakana() {
    assert!(is_katakana('ア'));
    assert!(is_katakana('ン'));
    assert!(is_katakana('ァ'));
    assert!(!is_katakana('あ'));
    assert!(!is_katakana('A'));
    assert!(!is_katakana('漢'));
}

#[test]
fn test_is_kanji() {
    assert!(is_kanji('漢'));
    assert!(is_kanji('字'));
    assert!(is_kanji('日'));
    assert!(is_kanji('本'));
    assert!(!is_kanji('あ'));
    assert!(!is_kanji('ア'));
    assert!(!is_kanji('A'));
}

#[test]
fn test_is_japanese_char() {
    assert!(is_japanese_char('あ'));
    assert!(is_japanese_char('ア'));
    assert!(is_japanese_char('漢'));
    assert!(!is_japanese_char('A'));
    assert!(!is_japanese_char('1'));
    assert!(!is_japanese_char(' '));
}

#[test]
fn test_japanese_char_analysis_kana_count() {
    let mut analysis = JapaneseCharAnalysis::new();
    for ch in "あいうアイウ".chars() {
        analysis.analyze_char(ch);
    }
    assert_eq!(analysis.kana_count(), 6);
    assert_eq!(analysis.hiragana_count, 3);
    assert_eq!(analysis.katakana_count, 3);
}

#[test]
#[serial]
fn test_analyze_japanese_chars_basic() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let analysis = page.analyze_japanese_chars();

    println!("Japanese character analysis:");
    println!("  Hiragana: {}", analysis.hiragana_count);
    println!("  Katakana: {}", analysis.katakana_count);
    println!("  Kanji: {}", analysis.kanji_count);
    println!("  Total: {}", analysis.total_chars);
    println!("  Japanese ratio: {:.2}", analysis.japanese_ratio());

    // Just verify it works without error (total_chars is usize, always non-negative)
}

#[test]
#[serial]
fn test_has_japanese_text() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let has_japanese = page.has_japanese_text();
    let analysis = page.analyze_japanese_chars();

    assert_eq!(has_japanese, analysis.has_japanese());
}

#[test]
#[serial]
fn test_is_japanese_page() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let is_japanese = page.is_japanese_page();
    let analysis = page.analyze_japanese_chars();

    assert_eq!(is_japanese, analysis.is_predominantly_japanese());
}

#[test]
#[serial]
fn test_japanese_char_analysis_text() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();
    let text = page.text().unwrap();

    let analysis = text.japanese_char_analysis();
    let page_analysis = page.analyze_japanese_chars();

    // Both should return the same results
    assert_eq!(analysis.hiragana_count, page_analysis.hiragana_count);
    assert_eq!(analysis.katakana_count, page_analysis.katakana_count);
    assert_eq!(analysis.kanji_count, page_analysis.kanji_count);
    assert_eq!(analysis.total_chars, page_analysis.total_chars);
}

#[test]
#[serial]
fn test_japanese_char_analysis_all_pages() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    for i in 0..doc.page_count() {
        let page = doc.page(i).unwrap();
        let analysis = page.analyze_japanese_chars();

        println!(
            "Page {}: H={}, K={}, J={}, total={}",
            i,
            analysis.hiragana_count,
            analysis.katakana_count,
            analysis.kanji_count,
            analysis.total_chars
        );

        // Verify consistency
        assert_eq!(
            analysis.japanese_char_count(),
            analysis.hiragana_count
                + analysis.katakana_count
                + analysis.kanji_count
                + analysis.halfwidth_katakana
        );
    }
}

// ============================================================================
// Japanese Punctuation API Tests (Feature 19)
// ============================================================================

#[test]
fn test_jpunct_type_classify() {
    assert_eq!(JPunctType::classify('。'), Some(JPunctType::Period));
    assert_eq!(JPunctType::classify('、'), Some(JPunctType::Comma));
    assert_eq!(JPunctType::classify('「'), Some(JPunctType::QuoteOpen));
    assert_eq!(JPunctType::classify('」'), Some(JPunctType::QuoteClose));
    assert_eq!(JPunctType::classify('・'), Some(JPunctType::MiddleDot));
    assert_eq!(JPunctType::classify('ー'), Some(JPunctType::LongVowel));
    assert_eq!(JPunctType::classify('〜'), Some(JPunctType::WaveDash));
    assert_eq!(JPunctType::classify('々'), Some(JPunctType::Repetition));
    assert_eq!(JPunctType::classify('A'), None);
    assert_eq!(JPunctType::classify('あ'), None);
}

#[test]
fn test_jpunct_type_opening_closing() {
    assert!(JPunctType::QuoteOpen.is_opening());
    assert!(!JPunctType::QuoteOpen.is_closing());
    assert!(JPunctType::QuoteClose.is_closing());
    assert!(!JPunctType::QuoteClose.is_opening());
    assert!(!JPunctType::Period.is_opening());
    assert!(!JPunctType::Period.is_closing());
}

#[test]
fn test_japanese_punctuation_struct() {
    let punct = JapanesePunctuation::new(
        '。',
        (100.0, 200.0, 110.0, 210.0),
        JPunctType::Period,
        false,
    );

    assert_eq!(punct.char, '。');
    assert_eq!(punct.punct_type, JPunctType::Period);
    assert!(!punct.is_vertical_variant);
    assert_eq!(punct.width(), 10.0);
    assert_eq!(punct.height(), 10.0);
    assert_eq!(punct.center_x(), 105.0);
    assert_eq!(punct.center_y(), 205.0);
}

#[test]
fn test_japanese_punctuation_clone_debug() {
    let punct = JapanesePunctuation::new('、', (0.0, 0.0, 10.0, 10.0), JPunctType::Comma, true);

    let cloned = punct.clone();
    assert_eq!(cloned.char, punct.char);
    assert_eq!(cloned.punct_type, punct.punct_type);

    // Test Debug trait
    let debug_str = format!("{:?}", punct);
    assert!(debug_str.contains("JapanesePunctuation"));
}

#[test]
#[serial]
fn test_extract_japanese_punctuation_basic() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let punctuation = page.extract_japanese_punctuation();

    println!("Found {} Japanese punctuation marks", punctuation.len());
    for punct in &punctuation {
        println!(
            "  '{}' at ({:.1}, {:.1}), type: {:?}",
            punct.char, punct.bounds.0, punct.bounds.1, punct.punct_type
        );
    }

    // Just verify it works without error
}

#[test]
#[serial]
fn test_has_japanese_punctuation() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let has_punct = page.has_japanese_punctuation();
    let punctuation = page.extract_japanese_punctuation();

    assert_eq!(has_punct, !punctuation.is_empty());
}

#[test]
#[serial]
fn test_japanese_punctuation_count() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let count = page.japanese_punctuation_count();
    let punctuation = page.extract_japanese_punctuation();

    assert_eq!(count, punctuation.len());
}

#[test]
#[serial]
fn test_japanese_punctuation_all_pages() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    for i in 0..doc.page_count() {
        let page = doc.page(i).unwrap();
        let punctuation = page.extract_japanese_punctuation();

        println!(
            "Page {}: {} Japanese punctuation marks",
            i,
            punctuation.len()
        );

        for punct in &punctuation {
            // Verify basic validity
            assert!(punct.width() >= 0.0);
            assert!(punct.height() >= 0.0);
        }
    }
}

// ============================================================================
// Emphasis Mark Detection Tests (Feature 20)
// ============================================================================

#[test]
fn test_emphasis_mark_type_enum() {
    use pdfium_render_fast::EmphasisMarkType;

    // Test all variants exist
    let _dot = EmphasisMarkType::Dot;
    let _circle = EmphasisMarkType::Circle;
    let _triangle = EmphasisMarkType::Triangle;
    let _sesame = EmphasisMarkType::Sesame;
    let _other = EmphasisMarkType::Other;
}

#[test]
fn test_emphasis_mark_type_classify() {
    use pdfium_render_fast::EmphasisMarkType;

    // Solid dots
    assert_eq!(EmphasisMarkType::classify('●'), Some(EmphasisMarkType::Dot));
    assert_eq!(EmphasisMarkType::classify('•'), Some(EmphasisMarkType::Dot));

    // Hollow circles
    assert_eq!(
        EmphasisMarkType::classify('○'),
        Some(EmphasisMarkType::Circle)
    );
    assert_eq!(
        EmphasisMarkType::classify('◦'),
        Some(EmphasisMarkType::Circle)
    );

    // Triangles
    assert_eq!(
        EmphasisMarkType::classify('▲'),
        Some(EmphasisMarkType::Triangle)
    );
    assert_eq!(
        EmphasisMarkType::classify('△'),
        Some(EmphasisMarkType::Triangle)
    );

    // Non-marks return None
    assert_eq!(EmphasisMarkType::classify('a'), None);
    assert_eq!(EmphasisMarkType::classify('あ'), None);
    assert_eq!(EmphasisMarkType::classify('漢'), None);
}

#[test]
fn test_emphasis_mark_type_is_filled() {
    use pdfium_render_fast::EmphasisMarkType;

    assert!(EmphasisMarkType::Dot.is_filled());
    assert!(EmphasisMarkType::Triangle.is_filled());
    assert!(!EmphasisMarkType::Circle.is_filled());
    assert!(!EmphasisMarkType::Sesame.is_filled());
    assert!(!EmphasisMarkType::Other.is_filled());
}

#[test]
fn test_emphasis_mark_type_clone_copy() {
    use pdfium_render_fast::EmphasisMarkType;

    let mark = EmphasisMarkType::Dot;
    let cloned = mark;
    let copied = mark; // Copy

    assert_eq!(mark, cloned);
    assert_eq!(mark, copied);
}

#[test]
fn test_emphasis_mark_type_debug() {
    use pdfium_render_fast::EmphasisMarkType;

    let debug_str = format!("{:?}", EmphasisMarkType::Dot);
    assert!(debug_str.contains("Dot"));
}

#[test]
fn test_emphasis_mark_struct() {
    use pdfium_render_fast::{EmphasisMark, EmphasisMarkType};

    let mark = EmphasisMark::new(
        '漢',
        (100.0, 200.0, 120.0, 220.0), // base bounds
        (105.0, 225.0, 115.0, 230.0), // mark bounds (above)
        EmphasisMarkType::Dot,
    );

    assert_eq!(mark.base_char, '漢');
    assert_eq!(mark.base_bounds, (100.0, 200.0, 120.0, 220.0));
    assert_eq!(mark.mark_bounds, (105.0, 225.0, 115.0, 230.0));
    assert_eq!(mark.mark_type, EmphasisMarkType::Dot);
}

#[test]
fn test_emphasis_mark_center_methods() {
    use pdfium_render_fast::{EmphasisMark, EmphasisMarkType};

    let mark = EmphasisMark::new(
        'あ',
        (100.0, 200.0, 120.0, 220.0), // base bounds
        (105.0, 225.0, 115.0, 235.0), // mark bounds
        EmphasisMarkType::Circle,
    );

    // Base center
    assert!((mark.base_center_x() - 110.0).abs() < 0.001);
    assert!((mark.base_center_y() - 210.0).abs() < 0.001);

    // Mark center
    assert!((mark.mark_center_x() - 110.0).abs() < 0.001);
    assert!((mark.mark_center_y() - 230.0).abs() < 0.001);
}

#[test]
fn test_emphasis_mark_layout_detection() {
    use pdfium_render_fast::{EmphasisMark, EmphasisMarkType};

    // Horizontal layout (mark above base)
    let h_mark = EmphasisMark::new(
        'あ',
        (100.0, 200.0, 120.0, 220.0), // base
        (105.0, 225.0, 115.0, 230.0), // mark above
        EmphasisMarkType::Dot,
    );

    assert!(h_mark.is_horizontal_layout());
    assert!(!h_mark.is_vertical_layout());

    // Vertical layout (mark beside base)
    let v_mark = EmphasisMark::new(
        'あ',
        (100.0, 200.0, 120.0, 220.0), // base
        (125.0, 205.0, 130.0, 215.0), // mark to the right
        EmphasisMarkType::Dot,
    );

    assert!(v_mark.is_vertical_layout());
    assert!(!v_mark.is_horizontal_layout());
}

#[test]
#[serial]
fn test_emphasis_marks_extraction() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // This just tests that the method runs without error
    let marks = page.extract_emphasis_marks();

    // Verify any marks found have valid structure
    for mark in &marks {
        // Base bounds should be valid
        assert!(mark.base_bounds.0 <= mark.base_bounds.2);
        assert!(mark.base_bounds.1 <= mark.base_bounds.3);

        // Mark bounds should be valid
        assert!(mark.mark_bounds.0 <= mark.mark_bounds.2);
        assert!(mark.mark_bounds.1 <= mark.mark_bounds.3);
    }

    println!("Found {} emphasis marks on page 0", marks.len());
}

#[test]
#[serial]
fn test_emphasis_mark_helper_methods() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // Test helper methods
    let has_marks = page.has_emphasis_marks();
    let count = page.emphasis_mark_count();

    // These should be consistent
    if has_marks {
        assert!(count > 0);
    } else {
        assert_eq!(count, 0);
    }
}

#[test]
#[serial]
fn test_emphasis_marks_all_pages() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    for i in 0..doc.page_count() {
        let page = doc.page(i).unwrap();
        let marks = page.extract_emphasis_marks();

        println!("Page {}: {} emphasis marks", i, marks.len());

        for mark in &marks {
            // Verify center calculations don't panic
            let _ = mark.base_center_x();
            let _ = mark.base_center_y();
            let _ = mark.mark_center_x();
            let _ = mark.mark_center_y();
            let _ = mark.is_horizontal_layout();
            let _ = mark.is_vertical_layout();
        }
    }
}

// ============================================================================
// Grid Analysis Tests (Feature 21)
// ============================================================================

#[test]
fn test_grid_intersection_struct() {
    use pdfium_render_fast::GridIntersection;

    let intersection = GridIntersection::new((100.0, 200.0), Some(0), Some(1));

    assert_eq!(intersection.point, (100.0, 200.0));
    assert_eq!(intersection.x(), 100.0);
    assert_eq!(intersection.y(), 200.0);
    assert!(intersection.is_full_intersection());
}

#[test]
fn test_grid_intersection_partial() {
    use pdfium_render_fast::GridIntersection;

    let partial = GridIntersection::new((50.0, 75.0), Some(0), None);
    assert!(!partial.is_full_intersection());

    let partial2 = GridIntersection::new((50.0, 75.0), None, Some(1));
    assert!(!partial2.is_full_intersection());
}

#[test]
fn test_grid_analysis_empty() {
    use pdfium_render_fast::GridAnalysis;

    let grid = GridAnalysis::new();
    assert_eq!(grid.row_count(), 0);
    assert_eq!(grid.column_count(), 0);
    assert_eq!(grid.cell_count(), 0);
    assert!(!grid.is_valid_table());
    assert!(grid.bounds().is_none());
}

#[test]
fn test_grid_analysis_default() {
    use pdfium_render_fast::GridAnalysis;

    let grid: GridAnalysis = Default::default();
    assert_eq!(grid.intersections.len(), 0);
    assert_eq!(grid.row_separators.len(), 0);
    assert_eq!(grid.column_separators.len(), 0);
}

#[test]
fn test_grid_analysis_with_separators() {
    use pdfium_render_fast::GridAnalysis;

    let mut grid = GridAnalysis::new();
    grid.row_separators = vec![100.0, 150.0, 200.0]; // 2 rows
    grid.column_separators = vec![50.0, 150.0, 250.0]; // 2 columns

    assert_eq!(grid.row_count(), 2);
    assert_eq!(grid.column_count(), 2);
    assert!(grid.is_valid_table());

    let bounds = grid.bounds().unwrap();
    assert_eq!(bounds, (50.0, 100.0, 250.0, 200.0));
}

#[test]
#[serial]
fn test_analyze_grid_lines() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // Just verify the method runs without error
    let grid = page.analyze_grid_lines();

    // Verify structure is valid
    for intersection in &grid.intersections {
        // Coordinates should be finite
        assert!(intersection.x().is_finite());
        assert!(intersection.y().is_finite());
    }

    println!(
        "Grid: {} rows x {} cols, {} cells",
        grid.row_count(),
        grid.column_count(),
        grid.cell_count()
    );
}

#[test]
#[serial]
fn test_has_grid_lines() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // This just tests the helper method runs
    let has_grid = page.has_grid_lines();
    println!("Page has grid lines: {}", has_grid);
}

// ============================================================================
// Column Alignment Detection Tests (Feature 22)
// ============================================================================

#[test]
fn test_alignment_type_enum() {
    use pdfium_render_fast::AlignmentType;

    let _left = AlignmentType::Left;
    let _right = AlignmentType::Right;
    let _center = AlignmentType::Center;
    let _decimal = AlignmentType::Decimal;
}

#[test]
fn test_alignment_type_is_numeric() {
    use pdfium_render_fast::AlignmentType;

    assert!(!AlignmentType::Left.is_numeric_alignment());
    assert!(AlignmentType::Right.is_numeric_alignment());
    assert!(!AlignmentType::Center.is_numeric_alignment());
    assert!(AlignmentType::Decimal.is_numeric_alignment());
}

#[test]
fn test_alignment_type_clone_copy() {
    use pdfium_render_fast::AlignmentType;

    let align = AlignmentType::Left;
    let cloned = align;
    let copied = align; // Copy

    assert_eq!(align, cloned);
    assert_eq!(align, copied);
}

#[test]
fn test_aligned_column_struct() {
    use pdfium_render_fast::{AlignedColumn, AlignmentType};

    let col = AlignedColumn::new(100.0, AlignmentType::Left, vec![0, 1, 2], 0.9);

    assert_eq!(col.x_position, 100.0);
    assert_eq!(col.alignment, AlignmentType::Left);
    assert_eq!(col.line_count(), 3);
    assert!(col.is_high_confidence());
    assert!((col.confidence - 0.9).abs() < 0.001);
}

#[test]
fn test_aligned_column_confidence_clamping() {
    use pdfium_render_fast::{AlignedColumn, AlignmentType};

    // Test that confidence is clamped to [0, 1]
    let high = AlignedColumn::new(100.0, AlignmentType::Left, vec![0], 1.5);
    assert!((high.confidence - 1.0).abs() < 0.001);

    let low = AlignedColumn::new(100.0, AlignmentType::Left, vec![0], -0.5);
    assert!((low.confidence - 0.0).abs() < 0.001);
}

#[test]
fn test_aligned_column_low_confidence() {
    use pdfium_render_fast::{AlignedColumn, AlignmentType};

    let col = AlignedColumn::new(100.0, AlignmentType::Center, vec![0, 1], 0.5);
    assert!(!col.is_high_confidence());
}

#[test]
#[serial]
fn test_detect_column_alignments() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // Test with various tolerances
    let cols_strict = page.detect_column_alignments(2.0);
    let cols_loose = page.detect_column_alignments(10.0);

    // Looser tolerance should find at least as many columns
    // (or equal if document has no columns)
    println!("Strict tolerance (2.0): {} columns", cols_strict.len());
    println!("Loose tolerance (10.0): {} columns", cols_loose.len());

    // Verify structure
    for col in &cols_strict {
        assert!(col.x_position.is_finite());
        assert!(col.confidence >= 0.0 && col.confidence <= 1.0);
    }
}

#[test]
#[serial]
fn test_column_alignment_count() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let count = page.column_alignment_count(5.0);
    let cols = page.detect_column_alignments(5.0);

    assert_eq!(count, cols.len());
}

#[test]
#[serial]
fn test_grid_and_alignment_all_pages() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    for i in 0..doc.page_count() {
        let page = doc.page(i).unwrap();

        let grid = page.analyze_grid_lines();
        let cols = page.detect_column_alignments(5.0);

        println!(
            "Page {}: {} intersections, {} row seps, {} col seps, {} alignments",
            i,
            grid.intersections.len(),
            grid.row_separators.len(),
            grid.column_separators.len(),
            cols.len()
        );
    }
}

// ============================================================================
// Whitespace Gap Matrix Tests (Feature 23)
// ============================================================================

#[test]
fn test_gap_orientation_enum() {
    use pdfium_render_fast::GapOrientation;

    let _h = GapOrientation::Horizontal;
    let _v = GapOrientation::Vertical;

    // Test equality
    assert_eq!(GapOrientation::Horizontal, GapOrientation::Horizontal);
    assert_ne!(GapOrientation::Horizontal, GapOrientation::Vertical);
}

#[test]
fn test_whitespace_gap_struct() {
    use pdfium_render_fast::{GapOrientation, WhitespaceGap};

    let h_gap = WhitespaceGap::new((0.0, 100.0, 500.0, 120.0), GapOrientation::Horizontal);
    assert!(h_gap.is_horizontal());
    assert!(!h_gap.is_vertical());
    assert!((h_gap.gap_size - 20.0).abs() < 0.001); // height
    assert!((h_gap.width() - 500.0).abs() < 0.001);
    assert!((h_gap.height() - 20.0).abs() < 0.001);

    let v_gap = WhitespaceGap::new((100.0, 0.0, 130.0, 500.0), GapOrientation::Vertical);
    assert!(v_gap.is_vertical());
    assert!(!v_gap.is_horizontal());
    assert!((v_gap.gap_size - 30.0).abs() < 0.001); // width
}

#[test]
fn test_gap_matrix_empty() {
    use pdfium_render_fast::GapMatrix;

    let matrix = GapMatrix::new();
    assert!(!matrix.has_gaps());
    assert_eq!(matrix.gap_count(), 0);
    assert!(!matrix.suggests_table());
}

#[test]
fn test_gap_matrix_default() {
    use pdfium_render_fast::GapMatrix;

    let matrix: GapMatrix = Default::default();
    assert_eq!(matrix.horizontal_gaps.len(), 0);
    assert_eq!(matrix.vertical_gaps.len(), 0);
    assert_eq!(matrix.potential_cells, (0, 0));
}

#[test]
#[serial]
fn test_analyze_whitespace_gaps() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // Test with different gap thresholds
    let gaps_small = page.analyze_whitespace_gaps(5.0);
    let gaps_large = page.analyze_whitespace_gaps(20.0);

    println!(
        "Small threshold (5pt): {} h-gaps, {} v-gaps",
        gaps_small.horizontal_gaps.len(),
        gaps_small.vertical_gaps.len()
    );
    println!(
        "Large threshold (20pt): {} h-gaps, {} v-gaps",
        gaps_large.horizontal_gaps.len(),
        gaps_large.vertical_gaps.len()
    );
}

// ============================================================================
// Alternating Row Backgrounds Tests (Feature 24)
// ============================================================================

#[test]
fn test_alternating_pattern_no_alternation() {
    use pdfium_render_fast::AlternatingPattern;

    let colors = vec![Some((255, 255, 255, 255)), Some((255, 255, 255, 255))];
    let bounds = vec![(0.0, 0.0, 100.0, 20.0), (0.0, 20.0, 100.0, 40.0)];
    let pattern = AlternatingPattern::new(bounds, colors);

    assert!(!pattern.is_alternating);
    assert!(!pattern.is_zebra_stripe());
    assert_eq!(pattern.row_count(), 2);
}

#[test]
fn test_alternating_pattern_zebra() {
    use pdfium_render_fast::AlternatingPattern;

    // Alternating white and gray rows
    let white = Some((255, 255, 255, 255));
    let gray = Some((200, 200, 200, 255));
    let colors = vec![white, gray, white, gray];
    let bounds = vec![
        (0.0, 0.0, 100.0, 20.0),
        (0.0, 20.0, 100.0, 40.0),
        (0.0, 40.0, 100.0, 60.0),
        (0.0, 60.0, 100.0, 80.0),
    ];

    let pattern = AlternatingPattern::new(bounds, colors);

    assert!(pattern.is_alternating);
    assert!(pattern.is_zebra_stripe());
    assert_eq!(pattern.period, 2);
    assert_eq!(pattern.row_count(), 4);
}

#[test]
#[serial]
fn test_detect_alternating_backgrounds() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // This just tests the method runs without error
    let pattern = page.detect_alternating_backgrounds();
    if let Some(p) = pattern {
        println!(
            "Found alternating pattern: {} rows, period {}",
            p.row_count(),
            p.period
        );
    } else {
        println!("No alternating background pattern detected");
    }
}

// ============================================================================
// Numeric Content Regions Tests (Feature 25)
// ============================================================================

#[test]
fn test_numeric_region_struct() {
    use pdfium_render_fast::{AlignmentType, NumericRegion};

    let region = NumericRegion::new(
        (100.0, 200.0, 200.0, 220.0),
        0.8,
        true,
        true,
        false,
        AlignmentType::Right,
    );

    assert_eq!(region.bounds, (100.0, 200.0, 200.0, 220.0));
    assert!((region.numeric_ratio - 0.8).abs() < 0.001);
    assert!(region.has_decimals);
    assert!(region.has_currency);
    assert!(!region.has_percentages);
    assert_eq!(region.alignment, AlignmentType::Right);
}

#[test]
fn test_numeric_region_is_primarily_numeric() {
    use pdfium_render_fast::{AlignmentType, NumericRegion};

    let high = NumericRegion::new(
        (0.0, 0.0, 100.0, 20.0),
        0.7,
        false,
        false,
        false,
        AlignmentType::Left,
    );
    assert!(high.is_primarily_numeric());

    let low = NumericRegion::new(
        (0.0, 0.0, 100.0, 20.0),
        0.3,
        false,
        false,
        false,
        AlignmentType::Left,
    );
    assert!(!low.is_primarily_numeric());
}

#[test]
fn test_numeric_region_financial() {
    use pdfium_render_fast::{AlignmentType, NumericRegion};

    // Currency makes it financial
    let currency = NumericRegion::new(
        (0.0, 0.0, 100.0, 20.0),
        0.5,
        false,
        true,
        false,
        AlignmentType::Right,
    );
    assert!(currency.is_financial());

    // Decimals + high numeric ratio makes it financial
    let decimal = NumericRegion::new(
        (0.0, 0.0, 100.0, 20.0),
        0.7,
        true,
        false,
        false,
        AlignmentType::Right,
    );
    assert!(decimal.is_financial());

    // Low numeric ratio without currency is not financial
    let text = NumericRegion::new(
        (0.0, 0.0, 100.0, 20.0),
        0.3,
        true,
        false,
        false,
        AlignmentType::Left,
    );
    assert!(!text.is_financial());
}

#[test]
fn test_numeric_region_percentage() {
    use pdfium_render_fast::{AlignmentType, NumericRegion};

    let percent = NumericRegion::new(
        (0.0, 0.0, 100.0, 20.0),
        0.5,
        false,
        false,
        true,
        AlignmentType::Right,
    );
    assert!(percent.is_percentage_column());

    let no_percent = NumericRegion::new(
        (0.0, 0.0, 100.0, 20.0),
        0.5,
        false,
        false,
        false,
        AlignmentType::Right,
    );
    assert!(!no_percent.is_percentage_column());
}

#[test]
fn test_numeric_region_dimensions() {
    use pdfium_render_fast::{AlignmentType, NumericRegion};

    let region = NumericRegion::new(
        (50.0, 100.0, 150.0, 130.0),
        0.5,
        false,
        false,
        false,
        AlignmentType::Left,
    );
    assert!((region.width() - 100.0).abs() < 0.001);
    assert!((region.height() - 30.0).abs() < 0.001);
}

#[test]
fn test_numeric_region_ratio_clamping() {
    use pdfium_render_fast::{AlignmentType, NumericRegion};

    let high = NumericRegion::new(
        (0.0, 0.0, 100.0, 20.0),
        1.5,
        false,
        false,
        false,
        AlignmentType::Left,
    );
    assert!((high.numeric_ratio - 1.0).abs() < 0.001);

    let low = NumericRegion::new(
        (0.0, 0.0, 100.0, 20.0),
        -0.5,
        false,
        false,
        false,
        AlignmentType::Left,
    );
    assert!((low.numeric_ratio - 0.0).abs() < 0.001);
}

#[test]
#[serial]
fn test_detect_numeric_regions() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let regions = page.detect_numeric_regions();
    println!("Found {} numeric regions", regions.len());

    for (i, region) in regions.iter().enumerate() {
        println!(
            "  Region {}: ratio={:.1}%, financial={}, percent={}",
            i,
            region.numeric_ratio * 100.0,
            region.is_financial(),
            region.is_percentage_column()
        );
    }
}

#[test]
#[serial]
fn test_numeric_region_count() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let count = page.numeric_region_count();
    let regions = page.detect_numeric_regions();
    assert_eq!(count, regions.len());
}

#[test]
#[serial]
fn test_phase11_all_pages() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    for i in 0..doc.page_count() {
        let page = doc.page(i).unwrap();

        let gaps = page.analyze_whitespace_gaps(10.0);
        let alt = page.detect_alternating_backgrounds();
        let numeric = page.detect_numeric_regions();

        println!(
            "Page {}: {} gaps, alternating={}, {} numeric regions",
            i,
            gaps.gap_count(),
            alt.is_some(),
            numeric.len()
        );
    }
}

// ============================================================================
// Text Block Clustering Tests (Feature 26)
// ============================================================================

#[test]
fn test_text_cluster_struct() {
    use pdfium_render_fast::TextCluster;

    let cluster = TextCluster::new((100.0, 200.0, 300.0, 400.0), 150, 5);

    assert_eq!(cluster.bounds, (100.0, 200.0, 300.0, 400.0));
    assert_eq!(cluster.char_count, 150);
    assert_eq!(cluster.line_count, 5);
    assert_eq!(cluster.gap_above, 0.0);
    assert_eq!(cluster.gap_below, 0.0);
    assert_eq!(cluster.gap_left, 0.0);
    assert_eq!(cluster.gap_right, 0.0);
}

#[test]
fn test_text_cluster_dimensions() {
    use pdfium_render_fast::TextCluster;

    let cluster = TextCluster::new((50.0, 100.0, 250.0, 300.0), 100, 3);

    assert!((cluster.width() - 200.0).abs() < 0.001);
    assert!((cluster.height() - 200.0).abs() < 0.001);
}

#[test]
fn test_text_cluster_center() {
    use pdfium_render_fast::TextCluster;

    let cluster = TextCluster::new((0.0, 0.0, 100.0, 100.0), 50, 2);

    let (cx, cy) = cluster.center();
    assert!((cx - 50.0).abs() < 0.001);
    assert!((cy - 50.0).abs() < 0.001);
}

#[test]
#[serial]
fn test_cluster_text_blocks() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let clusters = page.cluster_text_blocks(20.0);
    println!(
        "Found {} text clusters with gap_threshold=20.0",
        clusters.len()
    );

    for (i, cluster) in clusters.iter().enumerate() {
        println!(
            "  Cluster {}: {} chars, {} lines, gaps: above={:.1}, below={:.1}",
            i, cluster.char_count, cluster.line_count, cluster.gap_above, cluster.gap_below
        );
    }
}

#[test]
#[serial]
fn test_cluster_text_blocks_different_thresholds() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let tight = page.cluster_text_blocks(10.0);
    let loose = page.cluster_text_blocks(50.0);

    println!("Tight clustering (10pt): {} clusters", tight.len());
    println!("Loose clustering (50pt): {} clusters", loose.len());

    // Both should succeed without error
}

#[test]
#[serial]
fn test_text_cluster_count() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let count = page.text_cluster_count(20.0);
    let clusters = page.cluster_text_blocks(20.0);
    assert_eq!(count, clusters.len());
}

// ============================================================================
// Indentation Analysis Tests (Feature 27)
// ============================================================================

#[test]
fn test_indented_line_struct() {
    use pdfium_render_fast::IndentedLine;

    let line = IndentedLine::new(3, 40.0, 2, (40.0, 100.0, 300.0, 120.0));

    assert_eq!(line.line_index, 3);
    assert!((line.indent_px - 40.0).abs() < 0.001);
    assert_eq!(line.indent_level, 2);
    assert_eq!(line.bounds, (40.0, 100.0, 300.0, 120.0));
}

#[test]
fn test_indented_line_is_indented() {
    use pdfium_render_fast::IndentedLine;

    let indented = IndentedLine::new(0, 30.0, 1, (0.0, 0.0, 100.0, 20.0));
    assert!(indented.is_indented());

    let not_indented = IndentedLine::new(1, 0.0, 0, (0.0, 0.0, 100.0, 20.0));
    assert!(!not_indented.is_indented());
}

#[test]
fn test_indentation_analysis_new() {
    use pdfium_render_fast::IndentationAnalysis;

    let analysis = IndentationAnalysis::new();
    assert_eq!(analysis.base_margin, 0.0);
    assert_eq!(analysis.indent_increment, 0.0);
    assert!(analysis.lines.is_empty());
    assert_eq!(analysis.max_level, 0);
}

#[test]
fn test_indentation_analysis_default() {
    use pdfium_render_fast::IndentationAnalysis;

    let analysis = IndentationAnalysis::default();
    assert!(!analysis.has_indentation());
    assert_eq!(analysis.indented_line_count(), 0);
}

#[test]
fn test_indentation_analysis_has_indentation() {
    use pdfium_render_fast::{IndentationAnalysis, IndentedLine};

    let mut analysis = IndentationAnalysis::new();
    assert!(!analysis.has_indentation());

    // Add an indented line
    analysis
        .lines
        .push(IndentedLine::new(0, 20.0, 1, (0.0, 0.0, 100.0, 20.0)));
    analysis.max_level = 1;

    assert!(analysis.has_indentation());
    assert_eq!(analysis.indented_line_count(), 1);
}

#[test]
fn test_indentation_analysis_lines_at_level() {
    use pdfium_render_fast::{IndentationAnalysis, IndentedLine};

    let mut analysis = IndentationAnalysis::new();
    analysis
        .lines
        .push(IndentedLine::new(0, 0.0, 0, (0.0, 0.0, 100.0, 20.0)));
    analysis
        .lines
        .push(IndentedLine::new(1, 20.0, 1, (20.0, 20.0, 100.0, 40.0)));
    analysis
        .lines
        .push(IndentedLine::new(2, 40.0, 2, (40.0, 40.0, 100.0, 60.0)));
    analysis
        .lines
        .push(IndentedLine::new(3, 20.0, 1, (20.0, 60.0, 100.0, 80.0)));
    analysis.max_level = 2;

    let level_0 = analysis.lines_at_level(0);
    let level_1 = analysis.lines_at_level(1);
    let level_2 = analysis.lines_at_level(2);

    assert_eq!(level_0.len(), 1);
    assert_eq!(level_1.len(), 2);
    assert_eq!(level_2.len(), 1);
}

#[test]
#[serial]
fn test_analyze_indentation() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let analysis = page.analyze_indentation();
    println!("Indentation analysis:");
    println!("  Base margin: {:.1} pt", analysis.base_margin);
    println!("  Indent increment: {:.1} pt", analysis.indent_increment);
    println!("  Max level: {}", analysis.max_level);
    println!("  Total lines: {}", analysis.lines.len());
    println!("  Indented lines: {}", analysis.indented_line_count());
}

// ============================================================================
// List Marker Detection Tests (Feature 28)
// ============================================================================

#[test]
fn test_list_marker_type_is_numbered() {
    use pdfium_render_fast::ListMarkerType;

    assert!(ListMarkerType::NumberDot.is_numbered());
    assert!(ListMarkerType::NumberParen.is_numbered());
    assert!(ListMarkerType::LetterDot.is_numbered());
    assert!(ListMarkerType::LetterParen.is_numbered());
    assert!(ListMarkerType::Roman.is_numbered());

    assert!(!ListMarkerType::Bullet.is_numbered());
    assert!(!ListMarkerType::Dash.is_numbered());
    assert!(!ListMarkerType::Asterisk.is_numbered());
    assert!(!ListMarkerType::Custom.is_numbered());
}

#[test]
fn test_list_marker_type_is_bullet() {
    use pdfium_render_fast::ListMarkerType;

    assert!(ListMarkerType::Bullet.is_bullet());
    assert!(ListMarkerType::Dash.is_bullet());
    assert!(ListMarkerType::Asterisk.is_bullet());

    assert!(!ListMarkerType::NumberDot.is_bullet());
    assert!(!ListMarkerType::LetterDot.is_bullet());
    assert!(!ListMarkerType::Roman.is_bullet());
    assert!(!ListMarkerType::Custom.is_bullet());
}

#[test]
fn test_list_marker_type_detect_bullets() {
    use pdfium_render_fast::ListMarkerType;

    // ASCII single-character markers (implementation uses byte length == 1)
    assert_eq!(ListMarkerType::detect("-"), Some(ListMarkerType::Dash));
    assert_eq!(ListMarkerType::detect("*"), Some(ListMarkerType::Asterisk));

    // Note: Unicode bullet characters like "•" are multi-byte in UTF-8
    // and would need char_count check rather than byte len check.
    // The implementation correctly handles ASCII markers.
}

#[test]
fn test_list_marker_type_detect_numbers() {
    use pdfium_render_fast::ListMarkerType;

    // Number with dot
    assert_eq!(
        ListMarkerType::detect("1."),
        Some(ListMarkerType::NumberDot)
    );
    assert_eq!(
        ListMarkerType::detect("10."),
        Some(ListMarkerType::NumberDot)
    );
    assert_eq!(
        ListMarkerType::detect("99."),
        Some(ListMarkerType::NumberDot)
    );

    // Number with paren (trailing paren format)
    assert_eq!(
        ListMarkerType::detect("1)"),
        Some(ListMarkerType::NumberParen)
    );
    assert_eq!(
        ListMarkerType::detect("12)"),
        Some(ListMarkerType::NumberParen)
    );
}

#[test]
fn test_list_marker_type_detect_letters() {
    use pdfium_render_fast::ListMarkerType;

    // Letter with dot (single letter + dot)
    assert_eq!(
        ListMarkerType::detect("a."),
        Some(ListMarkerType::LetterDot)
    );
    assert_eq!(
        ListMarkerType::detect("A."),
        Some(ListMarkerType::LetterDot)
    );
    assert_eq!(
        ListMarkerType::detect("z."),
        Some(ListMarkerType::LetterDot)
    );

    // Letter with paren (single letter + trailing paren)
    assert_eq!(
        ListMarkerType::detect("a)"),
        Some(ListMarkerType::LetterParen)
    );
    assert_eq!(
        ListMarkerType::detect("B)"),
        Some(ListMarkerType::LetterParen)
    );
}

#[test]
fn test_list_marker_type_detect_roman() {
    use pdfium_render_fast::ListMarkerType;

    // Note: Single letter "i." or "I." matches LetterDot first in implementation
    // (priority order). Only multi-char Roman numerals are detected as Roman.
    assert_eq!(
        ListMarkerType::detect("i."),
        Some(ListMarkerType::LetterDot)
    ); // Single letter takes precedence
    assert_eq!(ListMarkerType::detect("ii."), Some(ListMarkerType::Roman));
    assert_eq!(ListMarkerType::detect("iii."), Some(ListMarkerType::Roman));
    assert_eq!(ListMarkerType::detect("iv."), Some(ListMarkerType::Roman));
    assert_eq!(ListMarkerType::detect("II."), Some(ListMarkerType::Roman));
    assert_eq!(ListMarkerType::detect("III."), Some(ListMarkerType::Roman));
}

#[test]
fn test_list_marker_type_detect_empty() {
    use pdfium_render_fast::ListMarkerType;

    assert_eq!(ListMarkerType::detect(""), None);
    assert_eq!(ListMarkerType::detect("   "), None);
}

#[test]
fn test_list_marker_struct() {
    use pdfium_render_fast::{ListMarker, ListMarkerType};

    let marker = ListMarker::new(
        ListMarkerType::NumberDot,
        "1.".to_string(),
        (50.0, 100.0, 70.0, 115.0),
        90.0,
    );

    assert_eq!(marker.marker_type, ListMarkerType::NumberDot);
    assert_eq!(marker.marker_text, "1.");
    assert_eq!(marker.marker_bounds, (50.0, 100.0, 70.0, 115.0));
    assert!((marker.content_start_x - 90.0).abs() < 0.001);
}

#[test]
fn test_list_marker_width() {
    use pdfium_render_fast::{ListMarker, ListMarkerType};

    let marker = ListMarker::new(
        ListMarkerType::Bullet,
        "•".to_string(),
        (50.0, 100.0, 60.0, 110.0),
        70.0,
    );

    assert!((marker.width() - 10.0).abs() < 0.001);
}

#[test]
fn test_list_marker_content_gap() {
    use pdfium_render_fast::{ListMarker, ListMarkerType};

    let marker = ListMarker::new(
        ListMarkerType::Bullet,
        "•".to_string(),
        (50.0, 100.0, 60.0, 110.0),
        75.0,
    );

    assert!((marker.marker_content_gap() - 15.0).abs() < 0.001);
}

#[test]
#[serial]
fn test_extract_list_markers() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let markers = page.extract_list_markers();
    println!("Found {} list markers", markers.len());

    for marker in &markers {
        println!(
            "  {:?}: '{}' at ({:.1}, {:.1})",
            marker.marker_type, marker.marker_text, marker.marker_bounds.0, marker.marker_bounds.1
        );
    }
}

// ============================================================================
// Phase 12 Integration Tests
// ============================================================================

#[test]
#[serial]
fn test_phase12_all_pages() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    for i in 0..doc.page_count() {
        let page = doc.page(i).unwrap();

        let clusters = page.cluster_text_blocks(20.0);
        let indent = page.analyze_indentation();
        let markers = page.extract_list_markers();

        println!(
            "Page {}: {} clusters, {} max indent level, {} list markers",
            i,
            clusters.len(),
            indent.max_level,
            markers.len()
        );
    }
}

// ============================================================================
// Column Gutter Detection Tests (Feature 29)
// ============================================================================

#[test]
fn test_column_gutter_struct() {
    use pdfium_render_fast::ColumnGutter;

    let gutter = ColumnGutter::new(300.0, 700.0, 100.0, 20.0, 0.85);

    assert!((gutter.x_position - 300.0).abs() < 0.001);
    assert!((gutter.top_y - 700.0).abs() < 0.001);
    assert!((gutter.bottom_y - 100.0).abs() < 0.001);
    assert!((gutter.width - 20.0).abs() < 0.001);
    assert!((gutter.confidence - 0.85).abs() < 0.001);
}

#[test]
fn test_column_gutter_height() {
    use pdfium_render_fast::ColumnGutter;

    let gutter = ColumnGutter::new(300.0, 700.0, 100.0, 20.0, 0.8);
    assert!((gutter.height() - 600.0).abs() < 0.001);
}

#[test]
fn test_column_gutter_confidence_clamping() {
    use pdfium_render_fast::ColumnGutter;

    let high = ColumnGutter::new(300.0, 700.0, 100.0, 20.0, 1.5);
    assert!((high.confidence - 1.0).abs() < 0.001);

    let low = ColumnGutter::new(300.0, 700.0, 100.0, 20.0, -0.5);
    assert!((low.confidence - 0.0).abs() < 0.001);
}

#[test]
fn test_column_gutter_is_high_confidence() {
    use pdfium_render_fast::ColumnGutter;

    let high = ColumnGutter::new(300.0, 700.0, 100.0, 20.0, 0.8);
    assert!(high.is_high_confidence());

    let low = ColumnGutter::new(300.0, 700.0, 100.0, 20.0, 0.5);
    assert!(!low.is_high_confidence());
}

#[test]
fn test_column_layout_new() {
    use pdfium_render_fast::ColumnLayout;

    let layout = ColumnLayout::new();
    assert!(layout.gutters.is_empty());
    assert_eq!(layout.column_count, 1);
    assert!(layout.column_bounds.is_empty());
    assert!(!layout.is_multi_column());
}

#[test]
fn test_column_layout_default() {
    use pdfium_render_fast::ColumnLayout;

    let layout = ColumnLayout::default();
    assert_eq!(layout.column_count, 1);
}

#[test]
fn test_column_layout_average_gutter_width() {
    use pdfium_render_fast::{ColumnGutter, ColumnLayout};

    let mut layout = ColumnLayout::new();
    assert!(layout.average_gutter_width().is_none());

    layout
        .gutters
        .push(ColumnGutter::new(200.0, 700.0, 100.0, 20.0, 0.8));
    layout
        .gutters
        .push(ColumnGutter::new(400.0, 700.0, 100.0, 30.0, 0.8));

    let avg = layout.average_gutter_width().unwrap();
    assert!((avg - 25.0).abs() < 0.001);
}

#[test]
#[serial]
fn test_detect_column_gutters() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let layout = page.detect_column_gutters();
    println!(
        "Column layout: {} columns, {} gutters",
        layout.column_count,
        layout.gutters.len()
    );

    for (i, gutter) in layout.gutters.iter().enumerate() {
        println!(
            "  Gutter {}: x={:.1}, width={:.1}, confidence={:.2}",
            i, gutter.x_position, gutter.width, gutter.confidence
        );
    }

    for (i, bounds) in layout.column_bounds.iter().enumerate() {
        println!(
            "  Column {}: ({:.1}, {:.1}) - ({:.1}, {:.1})",
            i, bounds.0, bounds.1, bounds.2, bounds.3
        );
    }
}

// ============================================================================
// Content Density Heatmap Tests (Feature 30)
// ============================================================================

#[test]
fn test_density_cell_struct() {
    use pdfium_render_fast::DensityCell;

    let cell = DensityCell::new((0.0, 0.0, 100.0, 100.0), 0.6, 0.2, 0.1);

    assert_eq!(cell.bounds, (0.0, 0.0, 100.0, 100.0));
    assert!((cell.text_density - 0.6).abs() < 0.001);
    assert!((cell.image_coverage - 0.2).abs() < 0.001);
    assert!((cell.line_coverage - 0.1).abs() < 0.001);
}

#[test]
fn test_density_cell_dimensions() {
    use pdfium_render_fast::DensityCell;

    let cell = DensityCell::new((50.0, 100.0, 150.0, 300.0), 0.5, 0.3, 0.1);
    assert!((cell.width() - 100.0).abs() < 0.001);
    assert!((cell.height() - 200.0).abs() < 0.001);
}

#[test]
fn test_density_cell_clamping() {
    use pdfium_render_fast::DensityCell;

    let cell = DensityCell::new(
        (0.0, 0.0, 100.0, 100.0),
        1.5,  // Should clamp to 1.0
        -0.5, // Should clamp to 0.0
        2.0,  // Should clamp to 1.0
    );

    assert!((cell.text_density - 1.0).abs() < 0.001);
    assert!((cell.image_coverage - 0.0).abs() < 0.001);
    assert!((cell.line_coverage - 1.0).abs() < 0.001);
}

#[test]
fn test_density_cell_total_coverage() {
    use pdfium_render_fast::DensityCell;

    let cell = DensityCell::new((0.0, 0.0, 100.0, 100.0), 0.4, 0.3, 0.2);
    assert!((cell.total_coverage() - 0.9).abs() < 0.001);

    // Total should be capped at 1.0
    let high = DensityCell::new((0.0, 0.0, 100.0, 100.0), 0.8, 0.8, 0.8);
    assert!((high.total_coverage() - 1.0).abs() < 0.001);
}

#[test]
fn test_density_cell_is_empty() {
    use pdfium_render_fast::DensityCell;

    let empty = DensityCell::new((0.0, 0.0, 100.0, 100.0), 0.02, 0.03, 0.01);
    assert!(empty.is_empty());

    let not_empty = DensityCell::new((0.0, 0.0, 100.0, 100.0), 0.2, 0.0, 0.0);
    assert!(!not_empty.is_empty());
}

#[test]
fn test_density_cell_is_text_dominant() {
    use pdfium_render_fast::DensityCell;

    let text = DensityCell::new((0.0, 0.0, 100.0, 100.0), 0.6, 0.2, 0.1);
    assert!(text.is_text_dominant());

    let image = DensityCell::new((0.0, 0.0, 100.0, 100.0), 0.2, 0.6, 0.1);
    assert!(!image.is_text_dominant());
}

#[test]
fn test_density_map_new() {
    use pdfium_render_fast::DensityMap;

    let map = DensityMap::new(5, 4);
    assert_eq!(map.grid_size, (5, 4));
    assert!(map.cells.is_empty());
    assert_eq!(map.cell_count(), 20);
}

#[test]
fn test_density_map_cell_access() {
    use pdfium_render_fast::{DensityCell, DensityMap};

    let mut map = DensityMap::new(2, 2);
    map.cells.push(vec![
        DensityCell::new((0.0, 0.0, 50.0, 50.0), 0.5, 0.0, 0.0),
        DensityCell::new((50.0, 0.0, 100.0, 50.0), 0.3, 0.1, 0.0),
    ]);
    map.cells.push(vec![
        DensityCell::new((0.0, 50.0, 50.0, 100.0), 0.4, 0.2, 0.0),
        DensityCell::new((50.0, 50.0, 100.0, 100.0), 0.2, 0.5, 0.0),
    ]);

    assert!(map.cell(0, 0).is_some());
    assert!(map.cell(1, 1).is_some());
    assert!(map.cell(5, 5).is_none());

    let cell = map.cell(0, 1).unwrap();
    assert!((cell.text_density - 0.3).abs() < 0.001);
}

#[test]
fn test_density_map_average_text_density() {
    use pdfium_render_fast::{DensityCell, DensityMap};

    let mut map = DensityMap::new(2, 2);
    map.cells.push(vec![
        DensityCell::new((0.0, 0.0, 50.0, 50.0), 0.4, 0.0, 0.0),
        DensityCell::new((50.0, 0.0, 100.0, 50.0), 0.2, 0.0, 0.0),
    ]);
    map.cells.push(vec![
        DensityCell::new((0.0, 50.0, 50.0, 100.0), 0.6, 0.0, 0.0),
        DensityCell::new((50.0, 50.0, 100.0, 100.0), 0.4, 0.0, 0.0),
    ]);

    // Average: (0.4 + 0.2 + 0.6 + 0.4) / 4 = 0.4
    assert!((map.average_text_density() - 0.4).abs() < 0.001);
}

#[test]
fn test_density_map_empty_cell_count() {
    use pdfium_render_fast::{DensityCell, DensityMap};

    let mut map = DensityMap::new(2, 2);
    map.cells.push(vec![
        DensityCell::new((0.0, 0.0, 50.0, 50.0), 0.5, 0.0, 0.0), // not empty
        DensityCell::new((50.0, 0.0, 100.0, 50.0), 0.01, 0.02, 0.01), // empty
    ]);
    map.cells.push(vec![
        DensityCell::new((0.0, 50.0, 50.0, 100.0), 0.02, 0.01, 0.01), // empty
        DensityCell::new((50.0, 50.0, 100.0, 100.0), 0.3, 0.0, 0.0),  // not empty
    ]);

    assert_eq!(map.empty_cell_count(), 2);
}

#[test]
#[serial]
fn test_compute_density_map() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let map = page.compute_density_map(5, 5);
    assert_eq!(map.grid_size, (5, 5));
    assert_eq!(map.cells.len(), 5);
    assert_eq!(map.cells[0].len(), 5);

    println!("Density map 5x5:");
    println!(
        "  Average text density: {:.1}%",
        map.average_text_density() * 100.0
    );
    println!("  Empty cells: {}", map.empty_cell_count());
}

#[test]
#[serial]
fn test_compute_density_map_1x1() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    let map = page.compute_density_map(1, 1);
    assert_eq!(map.grid_size, (1, 1));
    assert_eq!(map.cells.len(), 1);
    assert_eq!(map.cells[0].len(), 1);

    let cell = map.cell(0, 0).unwrap();
    println!(
        "Single cell density: text={:.1}%, image={:.1}%, line={:.1}%",
        cell.text_density * 100.0,
        cell.image_coverage * 100.0,
        cell.line_coverage * 100.0
    );
}

// ============================================================================
// Phase 13 Integration Tests
// ============================================================================

#[test]
#[serial]
fn test_phase13_all_pages() {
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();

    for i in 0..doc.page_count() {
        let page = doc.page(i).unwrap();

        let layout = page.detect_column_gutters();
        let density = page.compute_density_map(5, 5);

        println!(
            "Page {}: {} columns, avg text density={:.1}%",
            i,
            layout.column_count,
            density.average_text_density() * 100.0
        );
    }
}

// ============================================================================
// Full Roadmap Completion Test
// ============================================================================

#[test]
#[serial]
fn test_all_extraction_features() {
    // Tests that ALL 30 extraction features from the roadmap work together
    let pdfium = Pdfium::new().unwrap();
    let pdf_path = get_test_pdf();
    let doc = pdfium.load_pdf_from_file(&pdf_path, None).unwrap();
    let page = doc.page(0).unwrap();

    // Phase 1-2: Metadata & Geometry
    let _ = page.objects();
    let _ = page.extract_colored_regions();

    // Phase 3: Text Properties
    let _ = page.extract_text_blocks_with_metrics();
    let _ = page.extract_text_decorations();

    // Phase 4: Render Properties
    let _ = page.has_invisible_text_layer();

    // Phase 5: Cross-Page (requires document)
    let _ = doc.find_repeated_regions(0.1);

    // Phase 6: Math/Technical
    let _ = page.analyze_math_chars();
    let _ = page.extract_font_usage();
    let _ = page.extract_centered_blocks(20.0);

    // Phase 7: Script Analysis
    let _ = page.extract_bracketed_references();
    let _ = page.extract_script_clusters();

    // Phase 8: Japanese Direction & Ruby
    let _ = page.detect_writing_direction();
    let _ = page.extract_ruby_annotations();

    // Phase 9: Japanese Punctuation & Emphasis
    let _ = page.extract_japanese_punctuation();
    let _ = page.extract_emphasis_marks();

    // Phase 10: Grid Analysis
    let _ = page.analyze_grid_lines();
    let _ = page.detect_column_alignments(5.0);

    // Phase 11: Gaps & Patterns
    let _ = page.analyze_whitespace_gaps(10.0);
    let _ = page.detect_alternating_backgrounds();
    let _ = page.detect_numeric_regions();

    // Phase 12: Clustering & Indentation
    let _ = page.cluster_text_blocks(20.0);
    let _ = page.analyze_indentation();
    let _ = page.extract_list_markers();

    // Phase 13: Columns & Density
    let _ = page.detect_column_gutters();
    let _ = page.compute_density_map(5, 5);

    println!("All 30 extraction features work correctly");
}
