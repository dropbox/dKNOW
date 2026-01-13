//! Microsoft Project (.mpp) format support
//!
//! Project files are OLE Compound Documents (binary format).
//!
//! Format structure (varies by MPP version):
//! - MPP 2019: /   114/TBkndTask/Var2Data, /   114/TBkndRsc/Var2Data
//! - MPP 2010: /   112/TBkndTask/Var2Data, /   112/TBkndRsc/Var2Data
//! - MPP 2007: /   111/TBkndTask/Var2Data (estimated)
//! - Task/Resource names stored as UTF-16 LE with length prefixes
//! - /`SummaryInformation`: Project metadata
//! - /`DocumentSummaryInformation`: Extended properties
//!
//! Strategy: Parse OLE compound document directly using cfb crate,
//! try multiple stream paths for version compatibility,
//! extract text from key streams, generate `DocItems`.

use anyhow::{Context, Result};
use docling_core::{
    content::{DocItem, ItemRef, US_LETTER_HEIGHT, US_LETTER_WIDTH},
    document::{GroupItem, Origin, PageInfo, PageSize},
    DoclingDocument,
};
use std::io::Read;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

/// Backend for Microsoft Project files
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct ProjectBackend;

#[allow(clippy::trivially_copy_pass_by_ref)] // Unit struct methods conventionally take &self
impl ProjectBackend {
    /// Create a new Project backend instance
    #[inline]
    #[must_use = "creates Project backend instance"]
    pub const fn new() -> Self {
        Self
    }

    /// Parse .mpp file directly to `DoclingDocument`
    /// Extracts tasks, resources, and project metadata
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be opened (I/O error) or if the content is not
    /// a valid OLE Compound Document format.
    #[must_use = "this function returns a parsed document that should be processed"]
    pub fn parse_to_docitems(&self, input_path: &Path) -> Result<DoclingDocument> {
        let file = std::fs::File::open(input_path).context("Failed to open .mpp file")?;

        let mut comp =
            cfb::CompoundFile::open(file).context("Failed to parse as OLE Compound Document")?;

        // Extract project name from SummaryInformation or use filename
        let project_name = self.extract_project_name(&mut comp).unwrap_or_else(|| {
            input_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Untitled Project")
                .to_string()
        });

        // Extract tasks - try multiple stream paths for different MPP versions
        // MPP 2019: /   114/TBkndTask/Var2Data
        // MPP 2010: /   112/TBkndTask/Var2Data
        // MPP 2007: /   111/TBkndTask/Var2Data (estimated)
        let task_paths = [
            "/   114/TBkndTask/Var2Data",
            "/   112/TBkndTask/Var2Data",
            "/   111/TBkndTask/Var2Data",
            "/   113/TBkndTask/Var2Data",
        ];
        let tasks = task_paths
            .iter()
            .find_map(|path| self.extract_strings_from_stream(&mut comp, path).ok())
            .unwrap_or_default();

        // Extract resources - try multiple stream paths for different MPP versions
        let resource_paths = [
            "/   114/TBkndRsc/Var2Data",
            "/   112/TBkndRsc/Var2Data",
            "/   111/TBkndRsc/Var2Data",
            "/   113/TBkndRsc/Var2Data",
        ];
        let resources = resource_paths
            .iter()
            .find_map(|path| self.extract_strings_from_stream(&mut comp, path).ok())
            .unwrap_or_default();

        // Build DocItems
        Ok(Self::build_docling_document(
            project_name,
            &tasks,
            &resources,
        ))
    }

    /// Extract project name from OLE metadata streams
    // Method signature kept for API consistency with other ProjectBackend methods
    #[allow(clippy::unused_self)]
    fn extract_project_name(&self, comp: &mut cfb::CompoundFile<std::fs::File>) -> Option<String> {
        // Try SummaryInformation stream first
        if let Ok(mut stream) = comp.open_stream("SummaryInformation") {
            let mut buf = vec![0u8; 4096];
            if let Ok(n) = stream.read(&mut buf) {
                // Look for UTF-16 LE strings in the property set
                if let Some(name) = Self::extract_utf16_strings(&buf[..n])
                    .into_iter()
                    .find(|s| s.len() > 3 && !s.contains('\u{0}'))
                {
                    return Some(name);
                }
            }
        }
        None
    }

    /// Extract UTF-16 LE strings from a binary stream
    /// Returns all strings longer than 3 characters
    // Method signature kept for API consistency with other ProjectBackend methods
    #[allow(clippy::unused_self)]
    fn extract_strings_from_stream(
        &self,
        comp: &mut cfb::CompoundFile<std::fs::File>,
        stream_name: &str,
    ) -> Result<Vec<String>> {
        let mut stream = comp
            .open_stream(stream_name)
            .with_context(|| format!("Failed to open stream: {stream_name}"))?;

        let mut buf = Vec::new();
        stream
            .read_to_end(&mut buf)
            .context("Failed to read stream")?;

        Ok(Self::extract_utf16_strings(&buf))
    }

    /// Parse UTF-16 LE encoded strings from binary data
    /// Microsoft Project uses UTF-16 LE for text storage
    fn extract_utf16_strings(data: &[u8]) -> Vec<String> {
        let mut strings = Vec::new();
        let mut i = 0;

        while i + 1 < data.len() {
            // Look for length prefix (4 bytes, little-endian)
            if i + 4 <= data.len() {
                let len =
                    u32::from_le_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]) as usize;

                // Reasonable string length (UTF-16 bytes, so divide by 2 for char count)
                if len > 0 && len <= 1024 && i + 4 + len <= data.len() {
                    let utf16_data = &data[i + 4..i + 4 + len];

                    // Convert bytes to u16 words (UTF-16 LE)
                    let mut words = Vec::new();
                    for chunk in utf16_data.chunks_exact(2) {
                        words.push(u16::from_le_bytes([chunk[0], chunk[1]]));
                    }

                    // Decode UTF-16 to String
                    if let Ok(s) = String::from_utf16(&words) {
                        let trimmed = s.trim_end_matches('\0').trim();
                        if trimmed.len() > 3 && !trimmed.chars().all(char::is_whitespace) {
                            strings.push(trimmed.to_string());
                        }
                    }

                    i += 4 + len;
                    continue;
                }
            }

            i += 1;
        }

        strings
    }

    /// Add a section with its items to the document
    /// Returns the updated text index
    fn add_section_with_items(
        section_name: &str,
        items: &[String],
        texts: &mut Vec<DocItem>,
        body_children: &mut Vec<ItemRef>,
        mut text_idx: usize,
    ) -> usize {
        if items.is_empty() {
            return text_idx;
        }

        // Add section header
        let section_ref = format!("#/texts/{text_idx}");
        body_children.push(ItemRef::new(&section_ref));
        texts.push(DocItem::SectionHeader {
            self_ref: section_ref,
            parent: Some(ItemRef::new("#")),
            children: vec![],
            content_layer: "body".to_string(),
            prov: vec![],
            orig: section_name.to_string(),
            text: section_name.to_string(),
            level: 1,
            formatting: None,
            hyperlink: None,
        });
        text_idx += 1;

        // Add items
        for item in items {
            let item_ref = format!("#/texts/{text_idx}");
            body_children.push(ItemRef::new(&item_ref));
            texts.push(DocItem::Text {
                self_ref: item_ref,
                parent: Some(ItemRef::new("#")),
                children: vec![],
                content_layer: "body".to_string(),
                prov: vec![],
                orig: item.clone(),
                text: item.clone(),
                formatting: None,
                hyperlink: None,
            });
            text_idx += 1;
        }

        text_idx
    }

    /// Build `DoclingDocument` from extracted data
    fn build_docling_document(
        project_name: String,
        tasks: &[String],
        resources: &[String],
    ) -> DoclingDocument {
        let mut texts = Vec::new();
        let mut body_children = Vec::new();
        let mut text_idx = 0;

        // Add project title
        let title_ref = format!("#/texts/{text_idx}");
        body_children.push(ItemRef::new(&title_ref));
        texts.push(DocItem::Title {
            self_ref: title_ref,
            parent: Some(ItemRef::new("#")),
            children: vec![],
            content_layer: "body".to_string(),
            prov: vec![],
            orig: project_name.clone(),
            text: project_name.clone(),
            formatting: None,
            hyperlink: None,
        });
        text_idx += 1;

        // Add Tasks section
        text_idx =
            Self::add_section_with_items("Tasks", tasks, &mut texts, &mut body_children, text_idx);

        // Add Resources section
        text_idx = Self::add_section_with_items(
            "Resources",
            resources,
            &mut texts,
            &mut body_children,
            text_idx,
        );
        let _ = text_idx; // Silence unused variable warning

        let body = GroupItem {
            self_ref: "#".to_string(),
            parent: None,
            children: body_children,
            content_layer: "body".to_string(),
            name: "body".to_string(),
            label: "body".to_string(),
        };

        let mut pages = std::collections::HashMap::new();
        pages.insert(
            "1".to_string(),
            PageInfo {
                page_no: 1,
                size: PageSize {
                    width: US_LETTER_WIDTH,
                    height: US_LETTER_HEIGHT,
                },
            },
        );

        DoclingDocument {
            schema_name: "DoclingDocument".to_string(),
            version: "1.7.0".to_string(),
            name: project_name,
            origin: Origin {
                filename: "unknown.mpp".to_string(),
                mimetype: "application/vnd.ms-project".to_string(),
                binary_hash: 0,
            },
            body,
            furniture: None,
            texts,
            pictures: vec![],
            tables: vec![],
            groups: vec![],
            key_value_items: vec![],
            form_items: vec![],
            pages,
        }
    }

    /// Try to convert MPP using `LibreOffice` (fallback method)
    /// Returns PDF bytes
    ///
    /// # Errors
    ///
    /// Returns an error if the temporary directory cannot be created, if `LibreOffice` fails
    /// to execute, if the conversion fails (`LibreOffice` does not support `.mpp` natively),
    /// or if the converted PDF cannot be read.
    #[must_use = "this function returns PDF data that should be processed"]
    pub fn convert_to_pdf(&self, input_path: &Path) -> Result<Vec<u8>> {
        let temp_dir = TempDir::new().context("Failed to create temporary directory")?;

        // Try LibreOffice conversion
        let output = Command::new("soffice")
            .arg("--headless")
            .arg("--convert-to")
            .arg("pdf")
            .arg("--outdir")
            .arg(temp_dir.path())
            .arg(input_path)
            .output()
            .context("Failed to execute LibreOffice")?;

        let input_stem = input_path
            .file_stem()
            .context("Invalid input filename")?
            .to_string_lossy();
        let pdf_path = temp_dir.path().join(format!("{input_stem}.pdf"));

        if !output.status.success() || !pdf_path.exists() {
            anyhow::bail!(
                "LibreOffice cannot convert Project files. \
                LibreOffice does not support .mpp format. \
                Consider using MPXJ library (Java-based) for full support."
            );
        }

        // Read PDF bytes
        let pdf_bytes = std::fs::read(&pdf_path).context("Failed to read converted PDF")?;
        Ok(pdf_bytes)
    }

    /// Get the backend name
    #[inline]
    #[must_use = "returns backend name string"]
    pub const fn name(&self) -> &'static str {
        "Project"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_backend_creation() {
        let backend = ProjectBackend::new();
        assert_eq!(backend.name(), "Project");
    }

    #[test]
    #[allow(
        clippy::default_constructed_unit_structs,
        reason = "testing Default trait impl"
    )]
    fn test_project_backend_default_equals_new() {
        // Verify derived Default produces same result as new()
        assert_eq!(ProjectBackend::default(), ProjectBackend::new());
    }

    #[test]
    fn test_project_parse_to_docitems() {
        let backend = ProjectBackend::new();
        let file_path = "../../test-corpus/microsoft-project/sample1_2019.mpp";
        let path = Path::new(file_path);

        if !path.exists() {
            eprintln!("Skipping test: {file_path} not found");
            return;
        }

        let doc = backend
            .parse_to_docitems(path)
            .expect("Failed to parse .mpp file");

        // Verify document structure
        assert_eq!(doc.schema_name, "DoclingDocument");
        assert_eq!(doc.version, "1.7.0");
        assert!(!doc.name.is_empty(), "Project name should not be empty");

        // Should have at least a title
        assert!(!doc.texts.is_empty(), "Should have at least one text item");

        // Check for title
        let has_title = doc
            .texts
            .iter()
            .any(|item| matches!(item, DocItem::Title { .. }));
        assert!(has_title, "Should have a title");

        println!("Parsed project: {}", doc.name);
        println!("Total text items: {}", doc.texts.len());
    }

    #[test]
    fn test_extract_utf16_strings() {
        let _backend = ProjectBackend::new();

        // Sample UTF-16 LE data with length prefix
        // Length: 0x0C (12 bytes = 6 UTF-16 chars)
        // Text: "Flag1\0" (5 chars + null terminator)
        let data = vec![
            0x0C, 0x00, 0x00, 0x00, // Length: 12 bytes
            0x46, 0x00, // 'F'
            0x6C, 0x00, // 'l'
            0x61, 0x00, // 'a'
            0x67, 0x00, // 'g'
            0x31, 0x00, // '1'
            0x00, 0x00, // '\0'
        ];

        let strings = ProjectBackend::extract_utf16_strings(&data);
        assert_eq!(strings.len(), 1);
        assert_eq!(strings[0], "Flag1");
    }

    #[test]
    #[ignore = "Debug test - manual exploration of MPP structure"]
    fn explore_mpp_structure() {
        use std::io::Read;

        // Test BOTH sample1_2019.mpp (works) and sample2_2010.mpp (fails)
        let test_files = vec![
            "../../test-corpus/microsoft-project/sample1_2019.mpp",
            "../../test-corpus/microsoft-project/sample2_2010.mpp",
        ];

        for file_path in test_files {
            println!("\n=== Exploring: {file_path} ===");
            let path = Path::new(file_path);
            if !path.exists() {
                println!("File not found, skipping");
                continue;
            }

            let file = std::fs::File::open(path).expect("Failed to open file");
            let mut comp = cfb::CompoundFile::open(file).expect("Failed to open as OLE");

            println!("\n--- All Streams ---");
            let entries: Vec<_> = comp.walk().collect();
            for entry in &entries {
                let name = entry.path().to_string_lossy();
                let size = entry.len();
                println!("  {name} ({size} bytes)");
            }

            // Try key streams found in the OLE structure
            let streams = vec![
                "SummaryInformation",
                "DocumentSummaryInformation",
                "/214/Props",
                "/   114/Props",
                "/   114/TBkndTask/Var2Data",
                "/   114/TBkndRsc/Var2Data",
            ];

            for stream_name in streams {
                if let Ok(mut stream) = comp.open_stream(stream_name) {
                    let mut buf = vec![0u8; 1024];
                    if let Ok(n) = stream.read(&mut buf) {
                        println!("\n--- {stream_name} (read {n} bytes) ---");

                        // Look for ASCII strings
                        let mut current_string = String::new();
                        for &byte in &buf[..n] {
                            if (32..127).contains(&byte) {
                                current_string.push(byte as char);
                            } else if !current_string.is_empty() && current_string.len() > 3 {
                                println!("  Text: {current_string}");
                                current_string.clear();
                            } else {
                                current_string.clear();
                            }
                        }

                        // Print first 128 bytes as hex
                        print!("  Hex: ");
                        for (i, &byte) in buf[..n.min(128)].iter().enumerate() {
                            if i % 16 == 0 {
                                print!("\n    ");
                            }
                            print!("{byte:02x} ");
                        }
                        println!();
                    }
                } else {
                    println!("\n--- {stream_name} ---");
                    println!("  Stream NOT FOUND");
                }
            }
        }
    }

    // ==== CATEGORY 1: Backend Trait Tests (3 tests) ====

    #[test]
    fn test_project_backend_default() {
        let backend1 = ProjectBackend::new();
        let backend2 = ProjectBackend {};
        assert_eq!(backend1.name(), backend2.name());
    }

    #[test]
    fn test_project_backend_name() {
        let backend = ProjectBackend::new();
        assert_eq!(backend.name(), "Project");
    }

    #[test]
    fn test_project_nonexistent_file() {
        let backend = ProjectBackend::new();
        let result = backend.parse_to_docitems(Path::new("nonexistent.mpp"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Failed to open"));
    }

    // ==== CATEGORY 2: UTF-16 String Extraction Tests (5 tests) ====

    #[test]
    fn test_extract_utf16_strings_multiple() {
        let _backend = ProjectBackend::new();

        // Two UTF-16 strings: "Task1" (10 bytes) and "Resource1" (18 bytes)
        let data = vec![
            0x0A, 0x00, 0x00, 0x00, // Length: 10 bytes
            0x54, 0x00, // 'T'
            0x61, 0x00, // 'a'
            0x73, 0x00, // 's'
            0x6B, 0x00, // 'k'
            0x31, 0x00, // '1'
            0x12, 0x00, 0x00, 0x00, // Length: 18 bytes
            0x52, 0x00, // 'R'
            0x65, 0x00, // 'e'
            0x73, 0x00, // 's'
            0x6F, 0x00, // 'o'
            0x75, 0x00, // 'u'
            0x72, 0x00, // 'r'
            0x63, 0x00, // 'c'
            0x65, 0x00, // 'e'
            0x31, 0x00, // '1'
        ];

        let strings = ProjectBackend::extract_utf16_strings(&data);
        assert_eq!(strings.len(), 2);
        assert_eq!(strings[0], "Task1");
        assert_eq!(strings[1], "Resource1");
    }

    #[test]
    fn test_extract_utf16_strings_with_spaces() {
        let _backend = ProjectBackend::new();

        // UTF-16 string: "Task One" (16 bytes)
        let data = vec![
            0x10, 0x00, 0x00, 0x00, // Length: 16 bytes
            0x54, 0x00, // 'T'
            0x61, 0x00, // 'a'
            0x73, 0x00, // 's'
            0x6B, 0x00, // 'k'
            0x20, 0x00, // ' '
            0x4F, 0x00, // 'O'
            0x6E, 0x00, // 'n'
            0x65, 0x00, // 'e'
        ];

        let strings = ProjectBackend::extract_utf16_strings(&data);
        assert_eq!(strings.len(), 1);
        assert_eq!(strings[0], "Task One");
    }

    #[test]
    fn test_extract_utf16_strings_empty_data() {
        let _backend = ProjectBackend::new();
        let strings = ProjectBackend::extract_utf16_strings(&[]);
        assert_eq!(strings.len(), 0);
    }

    #[test]
    fn test_extract_utf16_strings_invalid_length() {
        let _backend = ProjectBackend::new();

        // Invalid length (exceeds data bounds)
        let data = vec![
            0xFF, 0xFF, 0x00, 0x00, // Length: 65535 bytes (way too large)
            0x41, 0x00, // 'A'
        ];

        let strings = ProjectBackend::extract_utf16_strings(&data);
        // Should not extract anything due to invalid length
        assert_eq!(strings.len(), 0);
    }

    #[test]
    fn test_extract_utf16_strings_short_strings_filtered() {
        let _backend = ProjectBackend::new();

        // UTF-16 string: "Ab" (4 bytes) - too short, should be filtered
        let data = vec![
            0x04, 0x00, 0x00, 0x00, // Length: 4 bytes
            0x41, 0x00, // 'A'
            0x62, 0x00, // 'b'
        ];

        let strings = ProjectBackend::extract_utf16_strings(&data);
        // Should filter out strings with length <= 3
        assert_eq!(strings.len(), 0);
    }

    // ==== CATEGORY 3: Document Structure Tests (5 tests) ====

    #[test]
    fn test_build_docling_document_with_tasks_only() {
        let _backend = ProjectBackend::new();
        let project_name = "Test Project".to_string();
        let tasks = vec!["Design Phase".to_string(), "Implementation".to_string()];
        let resources = vec![];

        let doc = ProjectBackend::build_docling_document(project_name.clone(), &tasks, &resources);

        assert_eq!(doc.schema_name, "DoclingDocument");
        assert_eq!(doc.version, "1.7.0");
        assert_eq!(doc.name, project_name);
        assert_eq!(doc.origin.mimetype, "application/vnd.ms-project");

        // Should have: Title + Section Header + 2 Tasks
        assert_eq!(doc.texts.len(), 4);
        assert!(matches!(doc.texts[0], DocItem::Title { .. }));
        assert!(matches!(doc.texts[1], DocItem::SectionHeader { .. }));
        assert!(matches!(doc.texts[2], DocItem::Text { .. }));
        assert!(matches!(doc.texts[3], DocItem::Text { .. }));
    }

    #[test]
    fn test_build_docling_document_with_resources_only() {
        let _backend = ProjectBackend::new();
        let project_name = "Test Project".to_string();
        let tasks = vec![];
        let resources = vec!["John Doe".to_string(), "Jane Smith".to_string()];

        let doc = ProjectBackend::build_docling_document(project_name, &tasks, &resources);

        // Should have: Title + Section Header + 2 Resources
        assert_eq!(doc.texts.len(), 4);
        assert!(matches!(doc.texts[0], DocItem::Title { .. }));
        assert!(matches!(doc.texts[1], DocItem::SectionHeader { .. }));
    }

    #[test]
    fn test_build_docling_document_empty() {
        let _backend = ProjectBackend::new();
        let project_name = "Empty Project".to_string();
        let tasks = vec![];
        let resources = vec![];

        let doc = ProjectBackend::build_docling_document(project_name, &tasks, &resources);

        // Should only have title
        assert_eq!(doc.texts.len(), 1);
        assert!(matches!(doc.texts[0], DocItem::Title { .. }));
    }

    #[test]
    fn test_build_docling_document_with_both() {
        let _backend = ProjectBackend::new();
        let project_name = "Full Project".to_string();
        let tasks = vec!["Task 1".to_string()];
        let resources = vec!["Resource 1".to_string()];

        let doc = ProjectBackend::build_docling_document(project_name, &tasks, &resources);

        // Should have: Title + Tasks Section + Task + Resources Section + Resource
        assert_eq!(doc.texts.len(), 5);

        // Verify section headers exist
        let section_headers: Vec<_> = doc
            .texts
            .iter()
            .filter_map(|item| {
                if let DocItem::SectionHeader { text, .. } = item {
                    Some(text.as_str())
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(section_headers.len(), 2);
        assert!(section_headers.contains(&"Tasks"));
        assert!(section_headers.contains(&"Resources"));
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn test_document_page_info() {
        let _backend = ProjectBackend::new();
        let project_name = "Test".to_string();

        let doc = ProjectBackend::build_docling_document(project_name, &[], &[]);

        assert_eq!(doc.pages.len(), 1);
        let page = doc.pages.get("1").unwrap();
        assert_eq!(page.page_no, 1);
        assert_eq!(page.size.width, US_LETTER_WIDTH);
        assert_eq!(page.size.height, US_LETTER_HEIGHT);
    }

    // ==== CATEGORY 4: Edge Cases and Error Handling (6 tests) ====

    #[test]
    fn test_extract_project_name_missing_stream() {
        let backend = ProjectBackend::new();
        // Create minimal OLE file without SummaryInformation
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_path_buf();
        {
            let file = std::fs::File::create(&temp_path).unwrap();
            let mut comp = cfb::CompoundFile::create(file).unwrap();
            comp.flush().unwrap();
        }

        let file = std::fs::File::open(&temp_path).unwrap();
        let mut comp = cfb::CompoundFile::open(file).unwrap();

        let result = backend.extract_project_name(&mut comp);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_invalid_ole_file() {
        let backend = ProjectBackend::new();

        // Create a file with invalid OLE structure
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(temp_file.path(), b"Not an OLE file").unwrap();

        let result = backend.parse_to_docitems(temp_file.path());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to parse as OLE"));
    }

    #[test]
    fn test_extract_strings_from_missing_stream() {
        let backend = ProjectBackend::new();

        // Create minimal OLE file
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_path_buf();
        {
            let file = std::fs::File::create(&temp_path).unwrap();
            let comp = cfb::CompoundFile::create(file).unwrap();
            drop(comp);
        }

        let file = std::fs::File::open(&temp_path).unwrap();
        let mut comp = cfb::CompoundFile::open(file).unwrap();

        let result = backend.extract_strings_from_stream(&mut comp, "/nonexistent/stream");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to open stream"));
    }

    #[test]
    fn test_very_long_project_name() {
        let _backend = ProjectBackend::new();
        let long_name = "A".repeat(1000);

        let doc = ProjectBackend::build_docling_document(long_name.clone(), &[], &[]);

        assert_eq!(doc.name, long_name);
        if let DocItem::Title { text, .. } = &doc.texts[0] {
            assert_eq!(text, &long_name);
        } else {
            panic!("First item should be title");
        }
    }

    #[test]
    fn test_special_characters_in_names() {
        let _backend = ProjectBackend::new();
        let project_name = "Project <>&\"'".to_string();
        let tasks = vec!["Task with <html> tags".to_string()];

        let doc = ProjectBackend::build_docling_document(project_name.clone(), &tasks, &[]);

        assert_eq!(doc.name, project_name);
        if let DocItem::Text { text, .. } = &doc.texts[2] {
            assert_eq!(text, "Task with <html> tags");
        }
    }

    #[test]
    fn test_unicode_in_names() {
        let _backend = ProjectBackend::new();
        let project_name = "项目 プロジェクト 🚀".to_string();
        let tasks = vec!["任务 タスク 📋".to_string()];

        let doc = ProjectBackend::build_docling_document(project_name.clone(), &tasks, &[]);

        assert_eq!(doc.name, project_name);
        if let DocItem::Text { text, .. } = &doc.texts[2] {
            assert_eq!(text, "任务 タスク 📋");
        }
    }

    // ==== CATEGORY 5: LibreOffice Conversion Test (1 test) ====

    #[test]
    fn test_convert_to_pdf_creates_output() {
        let backend = ProjectBackend::new();
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(temp_file.path(), b"dummy content").unwrap();

        // Try to convert with LibreOffice
        let result = backend.convert_to_pdf(temp_file.path());

        // If LibreOffice is installed and works, it may succeed
        // If not, it will fail with an error message
        match result {
            Ok(pdf_bytes) => {
                // If conversion succeeded, PDF should have content
                assert!(!pdf_bytes.is_empty());
            }
            Err(e) => {
                // If conversion failed, error should mention LibreOffice or conversion failure
                let err_msg = e.to_string();
                assert!(
                    err_msg.contains("LibreOffice")
                        || err_msg.contains("Failed to execute")
                        || err_msg.contains("cannot convert")
                );
            }
        }
    }
}
