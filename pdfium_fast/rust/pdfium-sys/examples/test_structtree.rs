//! Structure Tree Extraction API Example
//! N=37: Demonstrates full structure tree extraction for docling_rs integration
//!
//! This example shows how to:
//! 1. Check if a PDF is tagged (has structure tree)
//! 2. Load the structure tree for a page
//! 3. Traverse the tree recursively
//! 4. Extract element types (H1, P, Table, etc.)
//! 5. Extract alt text for figures
//! 6. Extract attributes and marked content IDs
//!
//! Usage:
//!   cargo run --example test_structtree <pdf_path>
//!   cargo run --example test_structtree <pdf_path> --verbose
//!   cargo run --example test_structtree <pdf_path> --json

use std::ffi::CString;
use std::path::Path;

/// Element types we care about for document understanding
#[derive(Debug, Clone, PartialEq)]
pub enum StructElementType {
    Document,
    Part,
    Sect,
    H1,
    H2,
    H3,
    H4,
    H5,
    H6,
    P,
    L,     // List
    LI,    // List item
    Lbl,   // List label
    LBody, // List body
    Table,
    TR, // Table row
    TH, // Table header
    TD, // Table data cell
    Figure,
    Span,
    Link,
    Other(String),
}

impl StructElementType {
    pub fn from_type_string(s: &str) -> Self {
        match s {
            "Document" => StructElementType::Document,
            "Part" => StructElementType::Part,
            "Sect" => StructElementType::Sect,
            "H1" => StructElementType::H1,
            "H2" => StructElementType::H2,
            "H3" => StructElementType::H3,
            "H4" => StructElementType::H4,
            "H5" => StructElementType::H5,
            "H6" => StructElementType::H6,
            "P" => StructElementType::P,
            "L" => StructElementType::L,
            "LI" => StructElementType::LI,
            "Lbl" => StructElementType::Lbl,
            "LBody" => StructElementType::LBody,
            "Table" => StructElementType::Table,
            "TR" => StructElementType::TR,
            "TH" => StructElementType::TH,
            "TD" => StructElementType::TD,
            "Figure" => StructElementType::Figure,
            "Span" => StructElementType::Span,
            "Link" => StructElementType::Link,
            other => StructElementType::Other(other.to_string()),
        }
    }
}

/// Extracted structure element info
#[derive(Debug, Clone)]
pub struct StructElement {
    pub element_type: StructElementType,
    pub type_raw: String,
    pub title: Option<String>,
    pub alt_text: Option<String>,
    pub actual_text: Option<String>,
    pub lang: Option<String>,
    pub marked_content_id: i32,
    pub marked_content_ids: Vec<i32>,
    pub children: Vec<StructElement>,
}

/// Get UTF-16LE string from PDFium API
unsafe fn get_utf16_string<F>(get_fn: F) -> Option<String>
where
    F: Fn(*mut std::ffi::c_void, u64) -> u64,
{
    // First call to get length
    let len = get_fn(std::ptr::null_mut(), 0);
    if len == 0 || len == u64::MAX {
        return None;
    }

    // Allocate buffer (len is in bytes, including null terminator)
    let mut buffer = vec![0u16; (len / 2) as usize];
    let actual_len = get_fn(buffer.as_mut_ptr() as *mut std::ffi::c_void, len);

    if actual_len == 0 {
        return None;
    }

    // Remove null terminator and convert
    let chars = (actual_len / 2) as usize;
    if chars > 0 {
        buffer.truncate(chars - 1); // Remove null terminator
        Some(String::from_utf16_lossy(&buffer))
    } else {
        None
    }
}

/// Extract element type string
unsafe fn get_element_type(elem: pdfium_sys::FPDF_STRUCTELEMENT) -> Option<String> {
    get_utf16_string(|buf, len| pdfium_sys::FPDF_StructElement_GetType(elem, buf, len))
}

/// Extract element title
unsafe fn get_element_title(elem: pdfium_sys::FPDF_STRUCTELEMENT) -> Option<String> {
    get_utf16_string(|buf, len| pdfium_sys::FPDF_StructElement_GetTitle(elem, buf, len))
}

/// Extract element alt text (for figures/images)
unsafe fn get_element_alt_text(elem: pdfium_sys::FPDF_STRUCTELEMENT) -> Option<String> {
    get_utf16_string(|buf, len| pdfium_sys::FPDF_StructElement_GetAltText(elem, buf, len))
}

/// Extract actual text (for elements with replaced content)
unsafe fn get_element_actual_text(elem: pdfium_sys::FPDF_STRUCTELEMENT) -> Option<String> {
    get_utf16_string(|buf, len| pdfium_sys::FPDF_StructElement_GetActualText(elem, buf, len))
}

/// Extract language attribute
unsafe fn get_element_lang(elem: pdfium_sys::FPDF_STRUCTELEMENT) -> Option<String> {
    get_utf16_string(|buf, len| pdfium_sys::FPDF_StructElement_GetLang(elem, buf, len))
}

/// Recursively extract structure tree element
unsafe fn extract_element(elem: pdfium_sys::FPDF_STRUCTELEMENT) -> StructElement {
    let type_raw = get_element_type(elem).unwrap_or_else(|| "Unknown".to_string());
    let element_type = StructElementType::from_type_string(&type_raw);

    // Get marked content IDs
    let mc_id = pdfium_sys::FPDF_StructElement_GetMarkedContentID(elem);
    let mc_count = pdfium_sys::FPDF_StructElement_GetMarkedContentIdCount(elem);
    let mut mc_ids = Vec::new();
    if mc_count > 0 {
        for i in 0..mc_count {
            let id = pdfium_sys::FPDF_StructElement_GetMarkedContentIdAtIndex(elem, i);
            if id >= 0 {
                mc_ids.push(id);
            }
        }
    }

    // Recursively extract children
    let child_count = pdfium_sys::FPDF_StructElement_CountChildren(elem);
    let mut children = Vec::new();
    if child_count > 0 {
        for i in 0..child_count {
            let child = pdfium_sys::FPDF_StructElement_GetChildAtIndex(elem, i);
            if !child.is_null() {
                children.push(extract_element(child));
            }
        }
    }

    StructElement {
        element_type,
        type_raw,
        title: get_element_title(elem),
        alt_text: get_element_alt_text(elem),
        actual_text: get_element_actual_text(elem),
        lang: get_element_lang(elem),
        marked_content_id: mc_id,
        marked_content_ids: mc_ids,
        children,
    }
}

/// Extract full structure tree for a page
///
/// # Safety
///
/// The caller must ensure that `page` is a valid FPDF_PAGE handle obtained
/// from a document that has not been closed.
pub unsafe fn extract_page_structure(page: pdfium_sys::FPDF_PAGE) -> Vec<StructElement> {
    let struct_tree = pdfium_sys::FPDF_StructTree_GetForPage(page);
    if struct_tree.is_null() {
        return Vec::new();
    }

    let mut elements = Vec::new();
    let root_count = pdfium_sys::FPDF_StructTree_CountChildren(struct_tree);

    if root_count > 0 {
        for i in 0..root_count {
            let elem = pdfium_sys::FPDF_StructTree_GetChildAtIndex(struct_tree, i);
            if !elem.is_null() {
                elements.push(extract_element(elem));
            }
        }
    }

    pdfium_sys::FPDF_StructTree_Close(struct_tree);
    elements
}

/// Count total elements in tree
fn count_elements(elements: &[StructElement]) -> usize {
    let mut count = elements.len();
    for elem in elements {
        count += count_elements(&elem.children);
    }
    count
}

/// Print tree with indentation
fn print_tree(elements: &[StructElement], indent: usize, verbose: bool) {
    for elem in elements {
        let indent_str = "  ".repeat(indent);

        let mut info_parts = Vec::new();

        if let Some(ref title) = elem.title {
            info_parts.push(format!("title=\"{}\"", title));
        }
        if let Some(ref alt) = elem.alt_text {
            info_parts.push(format!("alt=\"{}\"", alt));
        }
        if let Some(ref actual) = elem.actual_text {
            info_parts.push(format!("text=\"{}\"", actual));
        }
        if verbose {
            if elem.marked_content_id >= 0 {
                info_parts.push(format!("mcid={}", elem.marked_content_id));
            }
            if !elem.marked_content_ids.is_empty() {
                info_parts.push(format!("mcids={:?}", elem.marked_content_ids));
            }
        }

        let info = if info_parts.is_empty() {
            String::new()
        } else {
            format!(" ({})", info_parts.join(", "))
        };

        println!("{}<{}>{}", indent_str, elem.type_raw, info);
        print_tree(&elem.children, indent + 1, verbose);
    }
}

/// Collect type statistics
fn collect_type_stats(
    elements: &[StructElement],
    stats: &mut std::collections::HashMap<String, usize>,
) {
    for elem in elements {
        *stats.entry(elem.type_raw.clone()).or_insert(0) += 1;
        collect_type_stats(&elem.children, stats);
    }
}

/// Output as JSON
fn to_json(elements: &[StructElement]) -> String {
    fn elem_to_json(elem: &StructElement, indent: usize) -> String {
        let indent_str = "  ".repeat(indent);
        let mut parts = Vec::new();

        parts.push(format!("{}\"type\": \"{}\"", indent_str, elem.type_raw));

        if let Some(ref title) = elem.title {
            parts.push(format!(
                "{}\"title\": \"{}\"",
                indent_str,
                escape_json(title)
            ));
        }
        if let Some(ref alt) = elem.alt_text {
            parts.push(format!(
                "{}\"alt_text\": \"{}\"",
                indent_str,
                escape_json(alt)
            ));
        }
        if let Some(ref actual) = elem.actual_text {
            parts.push(format!(
                "{}\"actual_text\": \"{}\"",
                indent_str,
                escape_json(actual)
            ));
        }
        if elem.marked_content_id >= 0 {
            parts.push(format!(
                "{}\"marked_content_id\": {}",
                indent_str, elem.marked_content_id
            ));
        }

        if !elem.children.is_empty() {
            let children_json: Vec<String> = elem
                .children
                .iter()
                .map(|c| elem_to_json(c, indent + 1))
                .collect();
            parts.push(format!(
                "{}\"children\": [\n{}\n{}]",
                indent_str,
                children_json.join(",\n"),
                indent_str
            ));
        }

        format!(
            "{}{{\n{}\n{}}}",
            "  ".repeat(indent.saturating_sub(1)),
            parts.join(",\n"),
            "  ".repeat(indent.saturating_sub(1))
        )
    }

    fn escape_json(s: &str) -> String {
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t")
    }

    let elements_json: Vec<String> = elements.iter().map(|e| elem_to_json(e, 1)).collect();

    format!("[\n{}\n]", elements_json.join(",\n"))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <pdf_path> [--verbose] [--json]", args[0]);
        eprintln!("\nExamples:");
        eprintln!(
            "  {} document.pdf             # Basic structure tree output",
            args[0]
        );
        eprintln!(
            "  {} document.pdf --verbose   # Include marked content IDs",
            args[0]
        );
        eprintln!("  {} document.pdf --json      # Output as JSON", args[0]);
        std::process::exit(1);
    }

    let pdf_path = Path::new(&args[1]);
    let verbose = args.iter().any(|a| a == "--verbose" || a == "-v");
    let json_output = args.iter().any(|a| a == "--json" || a == "-j");

    if !pdf_path.exists() {
        eprintln!("Error: PDF not found: {}", pdf_path.display());
        std::process::exit(1);
    }

    unsafe {
        pdfium_sys::FPDF_InitLibrary();
    }

    let path_str = pdf_path.to_str().unwrap();
    let c_path = CString::new(path_str).unwrap();

    let doc = unsafe { pdfium_sys::FPDF_LoadDocument(c_path.as_ptr(), std::ptr::null()) };
    if doc.is_null() {
        eprintln!("Error: Failed to load PDF: {}", pdf_path.display());
        unsafe {
            pdfium_sys::FPDF_DestroyLibrary();
        }
        std::process::exit(1);
    }

    let num_pages = unsafe { pdfium_sys::FPDF_GetPageCount(doc) };
    let is_tagged = unsafe { pdfium_sys::FPDFCatalog_IsTagged(doc) };

    if !json_output {
        println!("PDF: {}", pdf_path.file_name().unwrap().to_str().unwrap());
        println!("Pages: {}", num_pages);
        println!("Tagged: {}\n", if is_tagged != 0 { "YES" } else { "NO" });
    }

    if is_tagged == 0 {
        if !json_output {
            println!("This PDF has no structure tree (not tagged).");
            println!("Structure-based extraction is not available.");
        } else {
            println!("{{\"tagged\": false, \"pages\": {}}}", num_pages);
        }
        unsafe {
            pdfium_sys::FPDF_CloseDocument(doc);
            pdfium_sys::FPDF_DestroyLibrary();
        }
        return;
    }

    let mut all_elements: Vec<(i32, Vec<StructElement>)> = Vec::new();
    let mut global_stats: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    let mut total_elements = 0;

    for page_idx in 0..num_pages {
        let page = unsafe { pdfium_sys::FPDF_LoadPage(doc, page_idx) };
        if page.is_null() {
            continue;
        }

        let elements = unsafe { extract_page_structure(page) };
        let element_count = count_elements(&elements);
        total_elements += element_count;

        collect_type_stats(&elements, &mut global_stats);

        if !json_output && !elements.is_empty() {
            println!("=== Page {} ({} elements) ===", page_idx + 1, element_count);
            print_tree(&elements, 0, verbose);
            println!();
        }

        all_elements.push((page_idx, elements));

        unsafe {
            pdfium_sys::FPDF_ClosePage(page);
        }
    }

    if json_output {
        println!("{{");
        println!("  \"tagged\": true,");
        println!("  \"pages\": {},", num_pages);
        println!("  \"total_elements\": {},", total_elements);
        println!("  \"type_stats\": {{");
        let stats_vec: Vec<_> = global_stats.iter().collect();
        for (i, (k, v)) in stats_vec.iter().enumerate() {
            let comma = if i < stats_vec.len() - 1 { "," } else { "" };
            println!("    \"{}\": {}{}", k, v, comma);
        }
        println!("  }},");
        println!("  \"pages_content\": [");
        for (i, (page_idx, elements)) in all_elements.iter().enumerate() {
            let comma = if i < all_elements.len() - 1 { "," } else { "" };
            println!(
                "    {{\"page\": {}, \"elements\": {}}}{}",
                page_idx + 1,
                to_json(elements),
                comma
            );
        }
        println!("  ]");
        println!("}}");
    } else {
        println!("=== Summary ===");
        println!(
            "Total pages with structure: {}",
            all_elements.iter().filter(|(_, e)| !e.is_empty()).count()
        );
        println!("Total structure elements: {}", total_elements);
        println!("\nElement type distribution:");
        let mut stats_vec: Vec<_> = global_stats.iter().collect();
        stats_vec.sort_by(|a, b| b.1.cmp(a.1));
        for (elem_type, count) in stats_vec {
            println!("  {}: {}", elem_type, count);
        }
    }

    unsafe {
        pdfium_sys::FPDF_CloseDocument(doc);
        pdfium_sys::FPDF_DestroyLibrary();
    }
}
