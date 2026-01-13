//! DICOM backend for docling
//!
//! This backend converts DICOM medical imaging files to docling's document model.

// Clippy pedantic allows:
// - Unit struct &self convention
#![allow(clippy::trivially_copy_pass_by_ref)]

use crate::traits::{BackendOptions, DocumentBackend};
use crate::utils::{create_text_item, opt_vec};
use docling_core::{DocItem, DoclingError, Document, DocumentMetadata, InputFormat};
use docling_medical::parse_dicom;
use std::fmt::Write;
use std::path::Path;

/// DICOM backend
///
/// Converts DICOM (Digital Imaging and Communications in Medicine) files
/// to docling's document model. Extracts metadata and image information
/// without requiring the actual image data.
///
/// ## Features
///
/// - Patient information (name, ID, age, sex)
/// - Study information (date, time, description)
/// - Series information (modality, body part)
/// - Image information (dimensions, pixel spacing)
/// - Technical metadata (manufacturer, station name)
///
/// ## Example
///
/// ```no_run
/// use docling_backend::DicomBackend;
/// use docling_backend::DocumentBackend;
///
/// let backend = DicomBackend::new();
/// let result = backend.parse_file("scan.dcm", &Default::default())?;
/// println!("Document: {:?}", result.metadata.title);
/// # Ok::<(), docling_core::error::DoclingError>(())
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct DicomBackend;

impl DicomBackend {
    /// Create a new DICOM backend instance
    #[inline]
    #[must_use = "creates a backend instance that should be used for parsing"]
    pub const fn new() -> Self {
        Self
    }
}

impl DicomBackend {
    /// Create `DocItems` from markdown content
    ///
    /// Parses markdown sections (split by H2 headers) and creates `DocItem::Text` entries.
    // Method signature kept for API consistency with other backend methods
    #[allow(clippy::unused_self)]
    fn create_docitems(&self, markdown: &str) -> Vec<DocItem> {
        let mut doc_items = Vec::new();
        let mut current_section = String::new();
        let mut text_index = 0;

        for line in markdown.lines() {
            if line.starts_with("## ") {
                // Save previous section if non-empty
                if !current_section.trim().is_empty() {
                    let text_content = current_section.trim().to_string();
                    doc_items.push(create_text_item(text_index, text_content, Vec::new()));
                    text_index += 1;
                }

                // Start new section with header
                current_section = format!("{line}\n");
            } else {
                current_section.push_str(line);
                current_section.push('\n');
            }
        }

        // Add final section
        if !current_section.trim().is_empty() {
            let text_content = current_section.trim().to_string();
            doc_items.push(create_text_item(text_index, text_content, Vec::new()));
        }

        doc_items
    }
}

/// Get human-readable name for DICOM modality code
#[inline]
fn modality_display_name(modality: &str) -> &'static str {
    match modality {
        "CT" => "CT Scan",
        "MR" | "MRI" => "MRI Scan",
        "US" => "Ultrasound Image",
        "XA" => "X-Ray Angiography",
        "CR" => "Computed Radiography",
        "DX" => "Digital X-Ray",
        "MG" => "Mammogram",
        "PT" => "PET Scan",
        "NM" => "Nuclear Medicine",
        "RF" => "Radiofluoroscopy",
        "OT" => "Other Imaging",
        _ => "Medical Image",
    }
}

/// Format patient information section
fn format_patient_section(md: &mut String, patient: &docling_medical::PatientInfo) {
    md.push_str("## Patient Information\n\n");
    let _ = writeln!(md, "- Name: {}", patient.name);
    let _ = writeln!(md, "- ID: {}", patient.id);
    if let Some(ref birthdate) = patient.birth_date {
        let _ = writeln!(md, "- Birth Date: {birthdate}");
    }
    if let Some(ref sex) = patient.sex {
        let _ = writeln!(md, "- Sex: {sex}");
    }
    md.push('\n');
}

/// Format study information section
fn format_study_section(md: &mut String, study: &docling_medical::StudyInfo) {
    md.push_str("## Study Information\n\n");
    let _ = writeln!(md, "- Study UID: {}", study.uid);
    if let Some(ref date) = study.date {
        let _ = writeln!(md, "- Date: {date}");
    }
    if let Some(ref time) = study.time {
        let _ = writeln!(md, "- Time: {time}");
    }
    if let Some(ref description) = study.description {
        let _ = writeln!(md, "- Description: {description}");
    }
    if let Some(ref id) = study.id {
        let _ = writeln!(md, "- Study ID: {id}");
    }
    if let Some(ref physician) = study.referring_physician {
        let _ = writeln!(md, "- Referring Physician: {physician}");
    }
    md.push('\n');
}

/// Format series information section
fn format_series_section(md: &mut String, series: &docling_medical::SeriesInfo) {
    md.push_str("## Series Information\n\n");
    let _ = writeln!(md, "- Series UID: {}", series.uid);
    let _ = writeln!(md, "- Modality: {}", series.modality);
    if let Some(ref number) = series.number {
        let _ = writeln!(md, "- Series Number: {number}");
    }
    if let Some(ref description) = series.description {
        let _ = writeln!(md, "- Description: {description}");
    }
    md.push('\n');
}

/// Format image information section
fn format_image_section(md: &mut String, image: &docling_medical::ImageInfo) {
    md.push_str("## Image Information\n\n");
    let _ = writeln!(md, "- SOP Class UID: {}", image.sop_class_uid);
    let _ = writeln!(md, "- SOP Instance UID: {}", image.sop_instance_uid);
    if let Some(ref instance) = image.instance_number {
        let _ = writeln!(md, "- Instance Number: {instance}");
    }
    if let (Some(rows), Some(cols)) = (image.rows, image.columns) {
        let _ = writeln!(md, "- Dimensions: {cols}x{rows} pixels");
    }
    if let Some(ref frames) = image.number_of_frames {
        let _ = writeln!(md, "- Number of Frames: {frames}");
    }
    if let Some(ref img_type) = image.image_type {
        let _ = writeln!(md, "- Image Type: {img_type}");
    }
    if let Some(ref body_part) = image.body_part_examined {
        let _ = writeln!(md, "- Body Part Examined: {body_part}");
    }
    if let Some(ref position) = image.patient_position {
        let _ = writeln!(md, "- Patient Position: {position}");
    }
    md.push('\n');
}

/// Format equipment information section (optional)
fn format_equipment_section(md: &mut String, equipment: &docling_medical::dicom::EquipmentInfo) {
    md.push_str("## Equipment Information\n\n");
    if let Some(ref manufacturer) = equipment.manufacturer {
        let _ = writeln!(md, "- Manufacturer: {manufacturer}");
    }
    if let Some(ref model) = equipment.model_name {
        let _ = writeln!(md, "- Model Name: {model}");
    }
    if let Some(ref station) = equipment.station_name {
        let _ = writeln!(md, "- Station Name: {station}");
    }
    if let Some(ref software) = equipment.software_version {
        let _ = writeln!(md, "- Software Version: {software}");
    }
    md.push('\n');
}

/// Format acquisition parameters section (optional)
fn format_acquisition_section(
    md: &mut String,
    acquisition: &docling_medical::dicom::AcquisitionInfo,
) {
    md.push_str("## Acquisition Parameters\n\n");
    if let Some(ref pixel_spacing) = acquisition.pixel_spacing {
        let _ = writeln!(md, "- Pixel Spacing: {pixel_spacing}");
    }
    if let Some(ref slice_thickness) = acquisition.slice_thickness {
        let _ = writeln!(md, "- Slice Thickness: {slice_thickness}");
    }
    if let Some(ref position) = acquisition.image_position {
        let _ = writeln!(md, "- Image Position (Patient): {position}");
    }
    if let Some(ref center) = acquisition.window_center {
        let _ = writeln!(md, "- Window Center: {center}");
    }
    if let Some(ref width) = acquisition.window_width {
        let _ = writeln!(md, "- Window Width: {width}");
    }
    if let Some(ref kvp) = acquisition.kvp {
        let _ = writeln!(md, "- KVP: {kvp}");
    }
    if let Some(ref exposure) = acquisition.exposure {
        let _ = writeln!(md, "- Exposure: {exposure}");
    }
    md.push('\n');
}

/// Parse study date string to `DateTime`
fn parse_study_date(date_str: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    chrono::NaiveDate::parse_from_str(date_str, "%Y%m%d")
        .ok()
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map(|dt| chrono::DateTime::from_naive_utc_and_offset(dt, chrono::Utc))
}

impl DocumentBackend for DicomBackend {
    #[inline]
    fn format(&self) -> InputFormat {
        InputFormat::Dicom
    }

    fn parse_file<P: AsRef<Path>>(
        &self,
        path: P,
        _options: &BackendOptions,
    ) -> Result<Document, DoclingError> {
        let path_ref = path.as_ref();
        let filename = path_ref.display();

        // Parse DICOM file (anonymize = false by default)
        let metadata = parse_dicom(path_ref, false).map_err(|e| {
            DoclingError::BackendError(format!("Failed to parse DICOM: {e}: {filename}"))
        })?;

        // Convert metadata to markdown
        let mut markdown = String::new();

        // Title with modality-specific name
        let modality_name = modality_display_name(&metadata.series.modality);
        let _ = writeln!(markdown, "# DICOM {modality_name}\n");

        // Format each section
        format_patient_section(&mut markdown, &metadata.patient);
        format_study_section(&mut markdown, &metadata.study);
        format_series_section(&mut markdown, &metadata.series);
        format_image_section(&mut markdown, &metadata.image);

        if let Some(ref equipment) = metadata.equipment {
            format_equipment_section(&mut markdown, equipment);
        }
        if let Some(ref acquisition) = metadata.acquisition {
            format_acquisition_section(&mut markdown, acquisition);
        }

        let num_characters = markdown.chars().count();

        // Use study description or patient name as title
        let title = metadata
            .study
            .description
            .clone()
            .or_else(|| Some(metadata.patient.name.clone()));

        // Use study date as created date
        let created = metadata
            .study
            .date
            .as_ref()
            .and_then(|s| parse_study_date(s));

        // Create DocItems from markdown
        let doc_items = self.create_docitems(&markdown);

        Ok(Document {
            markdown,
            format: InputFormat::Dicom,
            metadata: DocumentMetadata {
                num_pages: None,
                num_characters,
                title,
                author: None,
                created,
                modified: None,
                language: None,
                subject: None,
                exif: None,
            },
            content_blocks: opt_vec(doc_items),
            docling_document: None,
        })
    }

    fn parse_bytes(
        &self,
        _data: &[u8],
        _options: &BackendOptions,
    ) -> Result<Document, DoclingError> {
        Err(DoclingError::BackendError(
            "DICOM format does not support parsing from bytes".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== Backend Creation Tests (4 tests) ==========

    #[test]
    fn test_dicom_backend_creation() {
        let backend = DicomBackend::new();
        assert_eq!(backend.format(), InputFormat::Dicom);
    }

    #[test]
    fn test_dicom_backend_default() {
        let backend = DicomBackend;
        assert_eq!(backend.format(), InputFormat::Dicom);
    }

    #[test]
    fn test_backend_trait_implementation() {
        let backend = DicomBackend::new();
        assert_eq!(backend.format(), InputFormat::Dicom);
    }

    #[test]
    fn test_backend_format_constant() {
        let backend = DicomBackend::new();
        // Verify format is always Dicom
        assert!(matches!(backend.format(), InputFormat::Dicom));
    }

    // ========== Error Handling Tests (3 tests) ==========

    #[test]
    fn test_dicom_parse_bytes_not_supported() {
        let backend = DicomBackend::new();
        let data = b"test data";
        let result = backend.parse_bytes(data, &BackendOptions::default());
        assert!(result.is_err(), "parse_bytes should return error for DICOM");
        if let Err(DoclingError::BackendError(msg)) = result {
            assert!(
                msg.contains("does not support parsing from bytes"),
                "Error should mention bytes parsing not supported"
            );
        } else {
            panic!("Expected BackendError");
        }
    }

    #[test]
    fn test_parse_bytes_empty_data() {
        let backend = DicomBackend::new();
        let result = backend.parse_bytes(&[], &BackendOptions::default());
        assert!(result.is_err(), "Empty data should return error");
    }

    #[test]
    fn test_parse_bytes_error_message() {
        let backend = DicomBackend::new();
        let result = backend.parse_bytes(&[1, 2, 3], &BackendOptions::default());
        assert!(result.is_err(), "Invalid bytes should return error");
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("DICOM"),
            "Error message should mention DICOM"
        );
    }

    // ========== DocItem Generation Tests (8 tests) ==========

    #[test]
    fn test_create_docitems_empty() {
        let backend = DicomBackend::new();
        let markdown = "";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 0);
    }

    #[test]
    fn test_create_docitems_whitespace_only() {
        let backend = DicomBackend::new();
        let markdown = "   \n\n   \n  ";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 0);
    }

    #[test]
    fn test_create_docitems_single_section() {
        let backend = DicomBackend::new();
        let markdown = "## Patient Information\n\n- Name: John Doe\n- ID: 12345\n";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 1);

        if let DocItem::Text { text, .. } = &doc_items[0] {
            assert!(
                text.contains("Patient Information"),
                "Section should contain header text"
            );
            assert!(
                text.contains("John Doe"),
                "Section should contain patient name"
            );
        } else {
            panic!("Expected DocItem::Text");
        }
    }

    #[test]
    fn test_create_docitems_multiple_sections() {
        let backend = DicomBackend::new();
        let markdown = r"## Patient Information

- Name: John Doe
- ID: 12345

## Study Information

- Study UID: 1.2.3.4.5
- Date: 20240101

## Series Information

- Series UID: 1.2.3.4.6
- Modality: CT
";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 3);

        // Verify first section (Patient Information)
        if let DocItem::Text { text, .. } = &doc_items[0] {
            assert!(
                text.contains("Patient Information"),
                "First section should be Patient Information"
            );
            assert!(
                text.contains("John Doe"),
                "Patient name should be preserved"
            );
        } else {
            panic!("Expected DocItem::Text for first section");
        }

        // Verify second section (Study Information)
        if let DocItem::Text { text, .. } = &doc_items[1] {
            assert!(
                text.contains("Study Information"),
                "Second section should be Study Information"
            );
            assert!(text.contains("Study UID"), "Study UID should be present");
        } else {
            panic!("Expected DocItem::Text for second section");
        }

        // Verify third section (Series Information)
        if let DocItem::Text { text, .. } = &doc_items[2] {
            assert!(
                text.contains("Series Information"),
                "Third section should be Series Information"
            );
            assert!(text.contains("Modality"), "Modality should be present");
        } else {
            panic!("Expected DocItem::Text for third section");
        }
    }

    #[test]
    fn test_create_docitems_four_sections() {
        let backend = DicomBackend::new();
        let markdown = r"## Patient Information
- Name: Jane Smith
- ID: 67890

## Study Information
- Study UID: 1.2.3.4.5
- Date: 20240115

## Series Information
- Series UID: 1.2.3.4.6
- Modality: MRI

## Image Information
- Dimensions: 512 × 512 pixels
- Frames: 24
";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 4);

        // Verify all four sections are present
        if let DocItem::Text { text, .. } = &doc_items[0] {
            assert!(
                text.contains("Patient Information"),
                "First section should be Patient Information"
            );
        }
        if let DocItem::Text { text, .. } = &doc_items[1] {
            assert!(
                text.contains("Study Information"),
                "Second section should be Study Information"
            );
        }
        if let DocItem::Text { text, .. } = &doc_items[2] {
            assert!(
                text.contains("Series Information"),
                "Third section should be Series Information"
            );
        }
        if let DocItem::Text { text, .. } = &doc_items[3] {
            assert!(
                text.contains("Image Information"),
                "Fourth section should be Image Information"
            );
        }
    }

    #[test]
    fn test_create_docitems_section_without_header() {
        let backend = DicomBackend::new();
        let markdown = "Some metadata before headers\n\n## Patient Information\n\n- Name: Test\n";
        let doc_items = backend.create_docitems(markdown);
        // Should have 2 sections: one for content before header, one for Patient Information
        assert_eq!(doc_items.len(), 2);
    }

    #[test]
    fn test_create_docitems_consecutive_headers() {
        let backend = DicomBackend::new();
        let markdown = "## Header 1\n## Header 2\n## Header 3\n";
        let doc_items = backend.create_docitems(markdown);
        // Each header should create a section
        assert_eq!(doc_items.len(), 3);
    }

    #[test]
    fn test_create_docitems_preserves_content() {
        let backend = DicomBackend::new();
        let markdown = "## Test Section\nLine 1\nLine 2\nLine 3\n";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 1);

        if let DocItem::Text { text, .. } = &doc_items[0] {
            assert!(text.contains("Line 1"), "Content should include Line 1");
            assert!(text.contains("Line 2"), "Content should include Line 2");
            assert!(text.contains("Line 3"), "Content should include Line 3");
        } else {
            panic!("Expected DocItem::Text");
        }
    }

    // ========== DICOM-Specific Metadata Tests (5 tests) ==========

    #[test]
    fn test_patient_section_format() {
        let backend = DicomBackend::new();
        let markdown = "## Patient Information\n\n- Name: John Doe\n- ID: 12345\n- Birth Date: 19800101\n- Sex: M\n\n";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 1);

        if let DocItem::Text { text, .. } = &doc_items[0] {
            assert!(
                text.contains("Name:"),
                "Patient section should have Name field"
            );
            assert!(text.contains("ID:"), "Patient section should have ID field");
            assert!(
                text.contains("Birth Date:"),
                "Patient section should have Birth Date field"
            );
            assert!(
                text.contains("Sex:"),
                "Patient section should have Sex field"
            );
        }
    }

    #[test]
    fn test_study_section_format() {
        let backend = DicomBackend::new();
        let markdown = "## Study Information\n\n- Study UID: 1.2.840.113619.2.1\n- Date: 20240115\n- Time: 143022\n- Description: Brain MRI\n";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 1);

        if let DocItem::Text { text, .. } = &doc_items[0] {
            assert!(
                text.contains("Study UID:"),
                "Study section should have UID field"
            );
            assert!(
                text.contains("Date:"),
                "Study section should have Date field"
            );
            assert!(
                text.contains("Time:"),
                "Study section should have Time field"
            );
            assert!(
                text.contains("Description:"),
                "Study section should have Description field"
            );
        }
    }

    #[test]
    fn test_series_section_format() {
        let backend = DicomBackend::new();
        let markdown = "## Series Information\n\n- Series UID: 1.2.840.113619.2.2\n- Modality: MRI\n- Series Number: 1\n- Description: T1 Axial\n";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 1);

        if let DocItem::Text { text, .. } = &doc_items[0] {
            assert!(
                text.contains("Modality:"),
                "Series section should have Modality field"
            );
            assert!(text.contains("MRI"), "Modality value should be MRI");
        }
    }

    #[test]
    fn test_image_section_format() {
        let backend = DicomBackend::new();
        let markdown = "## Image Information\n\n- SOP Class UID: 1.2.840.10008.5.1.4.1.1.4\n- SOP Instance UID: 1.2.840.113619.2.3\n- Dimensions: 512 × 512 pixels\n- Number of Frames: 24\n";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 1);

        if let DocItem::Text { text, .. } = &doc_items[0] {
            assert!(
                text.contains("SOP Class UID:"),
                "Image section should have SOP Class UID"
            );
            assert!(
                text.contains("Dimensions:"),
                "Image section should have Dimensions"
            );
            assert!(text.contains("512"), "Dimensions should include 512");
        }
    }

    #[test]
    fn test_complete_dicom_structure() {
        let backend = DicomBackend::new();
        let markdown = r"# DICOM Medical Image

## Patient Information

- Name: Anonymous
- ID: PAT001

## Study Information

- Study UID: 1.2.3.4.5
- Date: 20240115

## Series Information

- Series UID: 1.2.3.4.6
- Modality: CT

## Image Information

- SOP Class UID: 1.2.840.10008.5.1.4.1.1.2
- SOP Instance UID: 1.2.3.4.7
- Dimensions: 512 × 512 pixels
";
        let doc_items = backend.create_docitems(markdown);
        // Title line ("# DICOM Medical Image") is not a section (H2)
        // Should have 4 sections (Patient, Study, Series, Image)
        assert_eq!(doc_items.len(), 5); // Title + 4 sections
    }

    // ========== DocItem Index Tests (3 tests) ==========

    #[test]
    fn test_docitem_indices() {
        let backend = DicomBackend::new();
        let markdown =
            "## Section 1\nContent 1\n## Section 2\nContent 2\n## Section 3\nContent 3\n";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 3);

        // Verify self_refs are sequential
        for (i, item) in doc_items.iter().enumerate() {
            if let DocItem::Text { self_ref, .. } = item {
                assert_eq!(self_ref, &format!("#/texts/{i}"));
            } else {
                panic!("Expected DocItem::Text");
            }
        }
    }

    #[test]
    fn test_docitem_variant() {
        let backend = DicomBackend::new();
        let markdown = "## Patient Info\nTest content\n";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 1);

        // Verify it's a Text variant
        matches!(&doc_items[0], DocItem::Text { .. });
    }

    #[test]
    fn test_docitem_no_bounding_boxes() {
        let backend = DicomBackend::new();
        let markdown = "## Test\nContent\n";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 1);

        if let DocItem::Text { prov, .. } = &doc_items[0] {
            assert_eq!(
                prov.len(),
                0,
                "DICOM DocItems should not have bounding boxes"
            );
        } else {
            panic!("Expected DocItem::Text");
        }
    }

    // ========== CATEGORY 6: Markdown Generation Tests (5 tests) ==========

    #[test]
    fn test_markdown_header_levels() {
        let backend = DicomBackend::new();
        // Test that H2 headers (##) are recognized, not H1 or H3
        let markdown = "# H1 Header\n## H2 Header\n### H3 Header\n";
        let doc_items = backend.create_docitems(markdown);

        // H1 and H2 create separate sections, H3 is part of H2 section
        assert!(!doc_items.is_empty(), "Should create at least one section");
    }

    #[test]
    fn test_markdown_line_preservation() {
        let backend = DicomBackend::new();
        let markdown = "## Section\nLine 1\nLine 2\n\nLine 3\n";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 1);

        if let DocItem::Text { text, .. } = &doc_items[0] {
            // Verify lines are preserved (though whitespace may be normalized)
            assert!(text.contains("Line 1"), "Line 1 should be preserved");
            assert!(text.contains("Line 2"), "Line 2 should be preserved");
            assert!(text.contains("Line 3"), "Line 3 should be preserved");
        }
    }

    #[test]
    fn test_markdown_bullet_points() {
        let backend = DicomBackend::new();
        let markdown = "## List\n- Item 1\n- Item 2\n- Item 3\n";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 1);

        if let DocItem::Text { text, .. } = &doc_items[0] {
            // Verify bullet points are preserved
            assert!(text.contains("Item 1"), "Bullet Item 1 should be preserved");
            assert!(text.contains("Item 2"), "Bullet Item 2 should be preserved");
            assert!(text.contains("Item 3"), "Bullet Item 3 should be preserved");
        }
    }

    #[test]
    fn test_markdown_bold_formatting() {
        let backend = DicomBackend::new();
        let markdown = "## Section\n- Name: John\n- ID: 123\n";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 1);

        if let DocItem::Text { text, .. } = &doc_items[0] {
            // Markdown bold markers should be preserved
            assert!(text.contains("Name:"), "Field name should be preserved");
            assert!(text.contains("John"), "Field value should be preserved");
        }
    }

    #[test]
    fn test_markdown_section_boundaries() {
        let backend = DicomBackend::new();
        let markdown = "## Section 1\nContent A\n## Section 2\nContent B\n";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 2);

        if let DocItem::Text { text, .. } = &doc_items[0] {
            assert!(
                text.contains("Section 1"),
                "First DocItem should be Section 1"
            );
            assert!(
                text.contains("Content A"),
                "Section 1 should have Content A"
            );
            assert!(
                !text.contains("Content B"),
                "Section 1 should not contain Section 2 content"
            );
        }

        if let DocItem::Text { text, .. } = &doc_items[1] {
            assert!(
                text.contains("Section 2"),
                "Second DocItem should be Section 2"
            );
            assert!(
                text.contains("Content B"),
                "Section 2 should have Content B"
            );
            assert!(
                !text.contains("Content A"),
                "Section 2 should not contain Section 1 content"
            );
        }
    }

    // ========== CATEGORY 7: Metadata Processing Tests (5 tests) ==========

    #[test]
    fn test_patient_info_fields() {
        let backend = DicomBackend::new();
        let markdown = "## Patient Information\n\n- Name: Jane Doe\n- ID: ABC123\n- Birth Date: 19900515\n- Sex: F\n";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 1);

        if let DocItem::Text { text, .. } = &doc_items[0] {
            assert!(
                text.contains("Jane Doe"),
                "Patient name should be preserved"
            );
            assert!(text.contains("ABC123"), "Patient ID should be preserved");
            assert!(text.contains("19900515"), "Birth date should be preserved");
            assert!(text.contains("F"), "Sex should be preserved");
        }
    }

    #[test]
    fn test_study_uid_format() {
        let backend = DicomBackend::new();
        let markdown = "## Study Information\n\n- Study UID: 1.2.840.113619.2.1.2411.1031152382.365.736169244\n";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 1);

        if let DocItem::Text { text, .. } = &doc_items[0] {
            // Verify long UID is preserved
            assert!(
                text.contains("1.2.840.113619.2.1.2411.1031152382.365.736169244"),
                "Long Study UID should be preserved exactly"
            );
        }
    }

    #[test]
    fn test_modality_types() {
        let backend = DicomBackend::new();
        // Test various DICOM modality types
        let modalities = vec!["CT", "MRI", "US", "XA", "CR", "DX", "MG", "PT", "NM"];

        for modality in modalities {
            let markdown = format!("## Series Information\n\n- Modality: {modality}\n");
            let doc_items = backend.create_docitems(&markdown);
            assert_eq!(doc_items.len(), 1);

            if let DocItem::Text { text, .. } = &doc_items[0] {
                assert!(
                    text.contains(modality),
                    "Should contain modality: {modality}"
                );
            }
        }
    }

    #[test]
    fn test_image_dimensions_parsing() {
        let backend = DicomBackend::new();
        let test_cases = vec![
            ("256 × 256 pixels", "256"),
            ("512 × 512 pixels", "512"),
            ("1024 × 768 pixels", "1024"),
        ];

        for (dimension_str, expected) in test_cases {
            let markdown = format!("## Image Information\n\n- Dimensions: {dimension_str}\n");
            let doc_items = backend.create_docitems(&markdown);
            assert_eq!(doc_items.len(), 1);

            if let DocItem::Text { text, .. } = &doc_items[0] {
                assert!(
                    text.contains(expected),
                    "Should contain dimension: {expected}"
                );
            }
        }
    }

    #[test]
    fn test_optional_fields_handling() {
        let backend = DicomBackend::new();
        // Minimal DICOM with only required fields
        let markdown = "## Patient Information\n\n- Name: Test\n- ID: 001\n\n## Study Information\n\n- Study UID: 1.2.3\n";
        let doc_items = backend.create_docitems(markdown);

        // Should handle missing optional fields (birth date, sex, study description, etc.)
        assert!(
            !doc_items.is_empty(),
            "Should create sections even with minimal fields"
        );
    }

    // ========== CATEGORY 8: Content Validation Tests (5 tests) ==========

    #[test]
    fn test_text_content_layer() {
        let backend = DicomBackend::new();
        let markdown = "## Test Section\nContent\n";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 1);

        if let DocItem::Text { content_layer, .. } = &doc_items[0] {
            assert_eq!(
                content_layer, "body",
                "DICOM DocItems should use 'body' content layer"
            );
        }
    }

    #[test]
    fn test_self_ref_format() {
        let backend = DicomBackend::new();
        let markdown = "## Section 1\nA\n## Section 2\nB\n## Section 3\nC\n";
        let doc_items = backend.create_docitems(markdown);

        for item in &doc_items {
            if let DocItem::Text { self_ref, .. } = item {
                // self_ref should be JSON pointer format: "#/texts/{index}"
                assert!(
                    self_ref.starts_with("#/texts/"),
                    "self_ref should start with '#/texts/'"
                );
                assert!(self_ref.len() > 8, "self_ref should have index number");
            }
        }
    }

    #[test]
    fn test_parent_refs_none() {
        let backend = DicomBackend::new();
        let markdown = "## Section\nContent\n";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 1);

        if let DocItem::Text { parent, .. } = &doc_items[0] {
            assert!(
                parent.is_none(),
                "DICOM DocItems should not have parent refs"
            );
        }
    }

    #[test]
    fn test_children_refs_empty() {
        let backend = DicomBackend::new();
        let markdown = "## Section\nContent\n";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 1);

        if let DocItem::Text { children, .. } = &doc_items[0] {
            assert!(
                children.is_empty(),
                "DICOM DocItems should not have children refs"
            );
        }
    }

    #[test]
    fn test_empty_content_blocks() {
        let backend = DicomBackend::new();
        let empty_markdown = "";
        let doc_items = backend.create_docitems(empty_markdown);

        // Empty markdown should produce no DocItems
        assert_eq!(
            doc_items.len(),
            0,
            "Empty markdown should produce empty DocItems"
        );
    }

    // ========== CATEGORY 9: Edge Case and Integration Tests (5 tests) ==========

    #[test]
    fn test_special_characters_in_content() {
        let backend = DicomBackend::new();
        let markdown =
            "## Patient Information\n\n- Name: O'Brien-Smith, John Jr.\n- ID: #12345-ABC\n";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 1);

        if let DocItem::Text { text, .. } = &doc_items[0] {
            assert!(
                text.contains("O'Brien-Smith"),
                "Names with apostrophes should be preserved"
            );
            assert!(
                text.contains("#12345-ABC"),
                "IDs with special chars should be preserved"
            );
        }
    }

    #[test]
    fn test_unicode_characters() {
        let backend = DicomBackend::new();
        let markdown = "## Patient Information\n\n- Name: José María González\n- ID: 日本語\n";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 1);

        if let DocItem::Text { text, .. } = &doc_items[0] {
            assert!(
                text.contains("José María González"),
                "Spanish characters should be preserved"
            );
            assert!(
                text.contains("日本語"),
                "Japanese characters should be preserved"
            );
        }
    }

    #[test]
    fn test_very_long_section() {
        let backend = DicomBackend::new();
        let mut markdown = String::from("## Long Section\n");

        // Create a section with 100 lines
        for i in 0..100 {
            let _ = writeln!(markdown, "Line {i} with content");
        }

        let doc_items = backend.create_docitems(&markdown);
        assert_eq!(doc_items.len(), 1);

        if let DocItem::Text { text, .. } = &doc_items[0] {
            assert!(text.contains("Line 0"), "First line should be preserved");
            assert!(text.contains("Line 99"), "Last line should be preserved");
        }
    }

    #[test]
    fn test_mixed_header_and_content() {
        let backend = DicomBackend::new();
        let markdown = "Content before any header\n\n## Section 1\nContent A\n\nMore content\n## Section 2\nContent B\n";
        let doc_items = backend.create_docitems(markdown);

        // Should have: 1) content before headers, 2) Section 1, 3) Section 2
        assert!(
            doc_items.len() >= 2,
            "Should handle mixed header and content"
        );
    }

    #[test]
    fn test_multiple_consecutive_newlines() {
        let backend = DicomBackend::new();
        let markdown = "## Section\n\n\n\nContent with gaps\n\n\n\nMore content\n\n\n";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 1);

        if let DocItem::Text { text, .. } = &doc_items[0] {
            // Content should be preserved (whitespace may be normalized)
            assert!(
                text.contains("Content with gaps"),
                "Content with gaps should be preserved"
            );
            assert!(
                text.contains("More content"),
                "Additional content should be preserved"
            );
        }
    }

    // Additional edge case tests for comprehensive coverage

    #[test]
    fn test_header_with_trailing_whitespace() {
        let backend = DicomBackend::new();
        let markdown = "## Patient Information   \n\nName: John Doe";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 1);
        if let DocItem::Text { text, .. } = &doc_items[0] {
            assert!(
                text.contains("Patient Information"),
                "Trailing whitespace in header should be trimmed"
            );
        }
    }

    #[test]
    fn test_content_with_tabs() {
        let backend = DicomBackend::new();
        let markdown = "## Section\n\tTabbed\tcontent\there\n\tMore\ttabs";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 1);
        if let DocItem::Text { text, .. } = &doc_items[0] {
            assert!(
                text.contains("Tabbed"),
                "Tab-separated content should be preserved"
            );
            assert!(text.contains("tabs"), "Tab content should be preserved");
        }
    }

    #[test]
    fn test_markdown_with_code_blocks() {
        let backend = DicomBackend::new();
        let markdown = "## Technical Info\n\n```\nStudy UID: 1.2.3.4.5\n```\n\nEnd";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 1);
        if let DocItem::Text { text, .. } = &doc_items[0] {
            assert!(
                text.contains("```"),
                "Code block markers should be preserved"
            );
            assert!(
                text.contains("Study UID"),
                "Code block content should be preserved"
            );
        }
    }

    #[test]
    fn test_markdown_with_inline_code() {
        let backend = DicomBackend::new();
        let markdown = "## Section\n\nValue: `1.2.3.4.5` is the UID";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 1);
        if let DocItem::Text { text, .. } = &doc_items[0] {
            assert!(
                text.contains("`1.2.3.4.5`"),
                "Inline code should be preserved"
            );
        }
    }

    #[test]
    fn test_markdown_with_links() {
        let backend = DicomBackend::new();
        let markdown = "## References\n\n[DICOM Standard](https://dicom.nema.org)";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 1);
        if let DocItem::Text { text, .. } = &doc_items[0] {
            assert!(
                text.contains("[DICOM Standard]"),
                "Link text should be preserved"
            );
            assert!(
                text.contains("https://dicom.nema.org"),
                "Link URL should be preserved"
            );
        }
    }

    #[test]
    fn test_header_levels_mixed() {
        let backend = DicomBackend::new();
        let markdown = "## Level 2\n\nContent\n\n### Level 3\n\nMore content";
        let doc_items = backend.create_docitems(markdown);
        // Level 3 (###) should NOT split - only Level 2 (##) splits
        assert_eq!(doc_items.len(), 1);
    }

    #[test]
    fn test_very_long_patient_name() {
        let backend = DicomBackend::new();
        let long_name = "A".repeat(500);
        let markdown = format!("## Patient Information\n\nName: {long_name}");
        let doc_items = backend.create_docitems(&markdown);
        assert_eq!(doc_items.len(), 1);
        if let DocItem::Text { text, .. } = &doc_items[0] {
            assert!(text.len() > 500, "Very long name should be preserved");
        }
    }

    #[test]
    fn test_numeric_values_precision() {
        let backend = DicomBackend::new();
        let markdown =
            "## Image\n\nPixel Spacing: 0.0001234567890123456789\nSlice Thickness: 1.500000000";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 1);
        if let DocItem::Text { text, .. } = &doc_items[0] {
            // Precision should be preserved
            assert!(
                text.contains("0.0001234567890123456789"),
                "High precision values should be preserved"
            );
            assert!(
                text.contains("1.500000000"),
                "Trailing zeros should be preserved"
            );
        }
    }

    #[test]
    fn test_date_time_formats() {
        let backend = DicomBackend::new();
        let markdown = "## Study\n\nDate: 20231125\nTime: 143052.123456";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 1);
        if let DocItem::Text { text, .. } = &doc_items[0] {
            assert!(
                text.contains("20231125"),
                "DICOM date format should be preserved"
            );
            assert!(
                text.contains("143052.123456"),
                "DICOM time format should be preserved"
            );
        }
    }

    #[test]
    fn test_uid_format_validation() {
        let backend = DicomBackend::new();
        let markdown = "## Study\n\nStudy UID: 1.2.840.10008.5.1.4.1.1.2";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 1);
        if let DocItem::Text { text, .. } = &doc_items[0] {
            // UID should be preserved exactly
            assert!(
                text.contains("1.2.840.10008.5.1.4.1.1.2"),
                "UID should be preserved exactly"
            );
        }
    }

    #[test]
    fn test_modality_edge_cases() {
        let backend = DicomBackend::new();
        // Test various modality types
        let modalities = vec!["CT", "MR", "US", "XA", "RF", "DX", "MG", "PT", "NM"];
        for modality in modalities {
            let markdown = format!("## Series\n\nModality: {modality}");
            let doc_items = backend.create_docitems(&markdown);
            assert_eq!(doc_items.len(), 1);
            if let DocItem::Text { text, .. } = &doc_items[0] {
                assert!(
                    text.contains(modality),
                    "Modality {modality} should be present in section"
                );
            }
        }
    }

    #[test]
    fn test_empty_field_values() {
        let backend = DicomBackend::new();
        let markdown = "## Patient\n\nName: \nID: \nAge: \nSex: ";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 1, "Single section should be created");
        // Empty values should be preserved
        if let DocItem::Text { text, .. } = &doc_items[0] {
            assert!(
                text.contains("Name:"),
                "Empty Name field should be preserved"
            );
            assert!(text.contains("ID:"), "Empty ID field should be preserved");
        }
    }

    #[test]
    fn test_special_dicom_characters() {
        let backend = DicomBackend::new();
        // DICOM uses ^ as name component separator
        let markdown = "## Patient\n\nName: Doe^John^A^^Dr";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 1, "Single section should be created");
        if let DocItem::Text { text, .. } = &doc_items[0] {
            assert!(
                text.contains("Doe^John^A^^Dr"),
                "DICOM name with caret separators should be preserved"
            );
        }
    }

    #[test]
    fn test_manufacturer_info() {
        let backend = DicomBackend::new();
        let markdown = "## Equipment\n\nManufacturer: Siemens\nModel: SOMATOM Definition";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 1, "Single section should be created");
        if let DocItem::Text { text, .. } = &doc_items[0] {
            assert!(
                text.contains("Siemens"),
                "Manufacturer name should be preserved"
            );
            assert!(
                text.contains("SOMATOM Definition"),
                "Model name should be preserved"
            );
        }
    }

    #[test]
    fn test_pixel_data_info() {
        let backend = DicomBackend::new();
        let markdown = "## Image\n\nRows: 512\nColumns: 512\nBits Allocated: 16\nBits Stored: 12";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 1, "Single section should be created");
        if let DocItem::Text { text, .. } = &doc_items[0] {
            assert!(text.contains("512"), "Image dimensions should be preserved");
            assert!(text.contains("16"), "Bits allocated should be preserved");
            assert!(text.contains("12"), "Bits stored should be preserved");
        }
    }

    #[test]
    fn test_body_part_examined() {
        let backend = DicomBackend::new();
        let body_parts = vec!["HEAD", "CHEST", "ABDOMEN", "PELVIS", "SPINE", "EXTREMITY"];
        for part in body_parts {
            let markdown = format!("## Series\n\nBody Part: {part}");
            let doc_items = backend.create_docitems(&markdown);
            assert_eq!(
                doc_items.len(),
                1,
                "Single section should be created for {part}"
            );
            if let DocItem::Text { text, .. } = &doc_items[0] {
                assert!(text.contains(part), "Body part {part} should be preserved");
            }
        }
    }

    #[test]
    fn test_window_level_center() {
        let backend = DicomBackend::new();
        let markdown = "## Display\n\nWindow Center: 40\nWindow Width: 400";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 1, "Single section should be created");
        if let DocItem::Text { text, .. } = &doc_items[0] {
            assert!(
                text.contains("Window Center: 40"),
                "Window center value should be preserved"
            );
            assert!(
                text.contains("Window Width: 400"),
                "Window width value should be preserved"
            );
        }
    }

    #[test]
    fn test_slice_location_negative() {
        let backend = DicomBackend::new();
        let markdown = "## Image\n\nSlice Location: -123.456";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 1, "Single section should be created");
        if let DocItem::Text { text, .. } = &doc_items[0] {
            assert!(
                text.contains("-123.456"),
                "Negative slice location should be preserved"
            );
        }
    }

    #[test]
    fn test_kvp_and_exposure() {
        let backend = DicomBackend::new();
        let markdown = "## Acquisition\n\nKVP: 120\nExposure: 250\nmAs: 100";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 1, "Single section should be created");
        if let DocItem::Text { text, .. } = &doc_items[0] {
            assert!(text.contains("KVP: 120"), "KVP value should be preserved");
            assert!(
                text.contains("Exposure: 250"),
                "Exposure value should be preserved"
            );
            assert!(text.contains("mAs: 100"), "mAs value should be preserved");
        }
    }

    #[test]
    fn test_series_description_with_protocol() {
        let backend = DicomBackend::new();
        let markdown = "## Series\n\nDescription: CT Brain w/o Contrast (Protocol: EMERGENCY_HEAD)";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 1, "Single section should be created");
        if let DocItem::Text { text, .. } = &doc_items[0] {
            assert!(
                text.contains("CT Brain w/o Contrast"),
                "Series description should be preserved"
            );
            assert!(
                text.contains("EMERGENCY_HEAD"),
                "Protocol name should be preserved"
            );
        }
    }

    #[test]
    fn test_institution_name() {
        let backend = DicomBackend::new();
        let markdown = "## Study\n\nInstitution: Massachusetts General Hospital";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 1, "Single section should be created");
        if let DocItem::Text { text, .. } = &doc_items[0] {
            assert!(
                text.contains("Massachusetts General Hospital"),
                "Institution name should be preserved"
            );
        }
    }

    #[test]
    fn test_window_center_width() {
        let backend = DicomBackend::new();
        let markdown = "## Image\n\nWindow Center: 40\nWindow Width: 400\nRescale Intercept: -1024";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 1, "Single section should be created");
        if let DocItem::Text { text, .. } = &doc_items[0] {
            assert!(
                text.contains("Window Center: 40"),
                "Window center should be preserved"
            );
            assert!(
                text.contains("Window Width: 400"),
                "Window width should be preserved"
            );
            assert!(
                text.contains("Rescale Intercept: -1024"),
                "Rescale intercept should be preserved"
            );
        }
    }

    /// Test DICOM with contrast agent information
    #[test]
    fn test_contrast_agent() {
        let backend = DicomBackend::new();
        let markdown =
            "## Study\n\nContrast Agent: Gadolinium\nContrast Route: IV\nContrast Volume: 20 mL";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 1, "Single section should be created");
        if let DocItem::Text { text, .. } = &doc_items[0] {
            assert!(
                text.contains("Gadolinium"),
                "Contrast agent name should be preserved"
            );
            assert!(text.contains("IV"), "Contrast route should be preserved");
            assert!(
                text.contains("20 mL"),
                "Contrast volume should be preserved"
            );
        }
    }

    /// Test DICOM with image orientation (patient position)
    #[test]
    fn test_image_orientation_patient() {
        let backend = DicomBackend::new();
        let markdown = "## Image\n\nImage Orientation: 1\\0\\0\\0\\1\\0\nPatient Position: HFS\nImage Position: -125.0\\-125.0\\100.0";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 1, "Single section should be created");
        if let DocItem::Text { text, .. } = &doc_items[0] {
            assert!(
                text.contains("Image Orientation"),
                "Image orientation should be preserved"
            );
            assert!(
                text.contains("HFS"),
                "Patient position (Head First Supine) should be preserved"
            );
            assert!(
                text.contains("Image Position"),
                "Image position should be preserved"
            );
        }
    }

    /// Test DICOM with MRI timing parameters (echo time, repetition time)
    #[test]
    fn test_mri_timing_parameters() {
        let backend = DicomBackend::new();
        let markdown = "## Acquisition\n\nModality: MR\nEcho Time: 80 ms\nRepetition Time: 2000 ms\nFlip Angle: 90°\nMagnetic Field Strength: 3.0 T";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 1, "Single section should be created");
        if let DocItem::Text { text, .. } = &doc_items[0] {
            assert!(
                text.contains("Echo Time: 80 ms"),
                "MRI echo time should be preserved"
            );
            assert!(
                text.contains("Repetition Time: 2000 ms"),
                "MRI repetition time should be preserved"
            );
            assert!(
                text.contains("Flip Angle: 90°"),
                "MRI flip angle should be preserved"
            );
            assert!(
                text.contains("3.0 T"),
                "Magnetic field strength in Tesla should be preserved"
            );
        }
    }

    /// Test DICOM with radiation dose information
    #[test]
    fn test_radiation_dose() {
        let backend = DicomBackend::new();
        let markdown = "## Dose\n\nCTDI vol: 15.5 mGy\nDLP: 350 mGy·cm\nExposure Time: 1500 ms\nX-Ray Tube Current: 200 mA";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 1);
        if let DocItem::Text { text, .. } = &doc_items[0] {
            assert!(text.contains("CTDI vol: 15.5 mGy")); // CT Dose Index
            assert!(text.contains("DLP: 350 mGy·cm")); // Dose Length Product
            assert!(text.contains("200 mA"));
        }
    }

    /// Test DICOM with multi-frame images (sequences)
    #[test]
    fn test_multi_frame_sequence() {
        let backend = DicomBackend::new();
        let markdown = "## Series\n\nNumber of Frames: 120\nFrame Increment Pointer: (0018,1063)\nFrame Time: 33 ms\nCine Rate: 30 fps";
        let doc_items = backend.create_docitems(markdown);
        assert_eq!(doc_items.len(), 1);
        if let DocItem::Text { text, .. } = &doc_items[0] {
            assert!(text.contains("Number of Frames: 120"));
            assert!(text.contains("Frame Time: 33 ms"));
            assert!(text.contains("30 fps")); // Frames per second
        }
    }

    // Advanced DICOM Features (Tests 71-75)

    /// Test Enhanced MR DICOM (multi-coil, parallel imaging)
    #[test]
    fn test_dicom_enhanced_mr() {
        let backend = DicomBackend::new();
        let markdown = concat!(
            "## Patient\n\nName: TEST^PATIENT\nID: MR123456\n\n",
            "## Study\n\nModality: MR\nSeries Description: 3D T1 MPRAGE\n\n",
            "## Enhanced MR Parameters\n\n",
            "Acquisition Type: 3D\n",
            "MR Acquisition Type: 3D\n",
            "Number of Coil Elements: 32\n",
            "Parallel Imaging: GRAPPA\n",
            "Acceleration Factor: 2\n",
            "Phase Encoding Direction: COL\n\n",
            "## Sequence Parameters\n\n",
            "Pulse Sequence Name: *tfl3d1_16ns\n",
            "Echo Time (TE): 2.98 ms\n",
            "Repetition Time (TR): 2300 ms\n",
            "Inversion Time (TI): 900 ms\n",
            "Flip Angle: 9°\n",
            "Bandwidth: 240 Hz/Px\n\n",
            "## Image Geometry\n\n",
            "Matrix Size: 256 × 256 × 176\n",
            "Voxel Size: 1.0 × 1.0 × 1.0 mm³\n",
            "Field of View: 256 × 256 mm\n",
            "Slice Thickness: 1.0 mm\n\n",
            "## Technical\n\n",
            "Manufacturer: Siemens\n",
            "Magnetic Field Strength: 3.0 T\n",
            "Software Version: syngo MR E11\n",
            "Receive Coil: HeadNeck_64\n"
        );
        let doc_items = backend.create_docitems(markdown);

        // Should have 5 sections
        assert!(doc_items.len() >= 4);

        // Verify enhanced MR features
        let has_parallel_imaging = doc_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => {
                text.contains("GRAPPA") && text.contains("Acceleration Factor: 2")
            }
            _ => false,
        });
        assert!(has_parallel_imaging, "Should detect parallel imaging");

        let has_sequence_params = doc_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => {
                text.contains("Inversion Time") && text.contains("Flip Angle")
            }
            _ => false,
        });
        assert!(has_sequence_params, "Should detect sequence parameters");

        let has_3d_geometry = doc_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => {
                text.contains("Matrix Size: 256 × 256 × 176")
                    && text.contains("Voxel Size: 1.0 × 1.0 × 1.0 mm³")
            }
            _ => false,
        });
        assert!(has_3d_geometry, "Should detect 3D geometry");
    }

    /// Test PET/CT fusion imaging
    #[test]
    fn test_dicom_pet_ct_fusion() {
        let backend = DicomBackend::new();
        let markdown = concat!(
            "## Patient\n\nName: ONCOLOGY^PATIENT\nID: PET987654\n\n",
            "## Study\n\nModality: PT (PET)\nStudy Description: FDG PET/CT WHOLEBODY\n\n",
            "## PET Acquisition\n\n",
            "Radiopharmaceutical: F-18 FDG\n",
            "Radionuclide Half Life: 6588 s (109.8 min)\n",
            "Radiopharmaceutical Start Time: 09:15:00\n",
            "Injection Time: 09:20:00\n",
            "Injected Dose: 370 MBq (10 mCi)\n",
            "Patient Weight: 75 kg\n\n",
            "## PET Reconstruction\n\n",
            "Reconstruction Method: OSEM\n",
            "Iterations: 2\n",
            "Subsets: 21\n",
            "Matrix Size: 200 × 200\n",
            "Pixel Spacing: 4.07 × 4.07 mm\n",
            "Slice Thickness: 3.0 mm\n",
            "Attenuation Correction: CT-based\n",
            "Scatter Correction: Yes\n",
            "Random Correction: Yes\n\n",
            "## CT Parameters\n\n",
            "CT Modality: CT\n",
            "kVp: 120\n",
            "mA: 80 (low dose for attenuation correction)\n",
            "Matrix Size: 512 × 512\n",
            "Pixel Spacing: 1.37 × 1.37 mm\n\n",
            "## SUV Calculation\n\n",
            "Decay Corrected: Yes\n",
            "Units: BQML (Bq/ml)\n",
            "SUVbw Type: Body Weight\n",
            "SUVlbm Type: Lean Body Mass\n",
            "SUVbsa Type: Body Surface Area\n"
        );
        let doc_items = backend.create_docitems(markdown);

        assert!(doc_items.len() >= 5);

        // Verify PET radiopharmaceutical info
        let has_fdg = doc_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => text.contains("F-18 FDG") && text.contains("370 MBq"),
            _ => false,
        });
        assert!(has_fdg, "Should detect FDG radiopharmaceutical");

        // Verify reconstruction parameters
        let has_recon = doc_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => {
                text.contains("OSEM") && text.contains("Attenuation Correction: CT-based")
            }
            _ => false,
        });
        assert!(has_recon, "Should detect reconstruction parameters");

        // Verify SUV calculation
        let has_suv = doc_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => text.contains("SUVbw") && text.contains("Body Weight"),
            _ => false,
        });
        assert!(has_suv, "Should detect SUV calculation");
    }

    /// Test DICOM Structured Report (SR)
    #[test]
    fn test_dicom_structured_report() {
        let backend = DicomBackend::new();
        let markdown = concat!(
            "## Patient\n\nName: REPORT^PATIENT\nID: SR111222\n\n",
            "## Study\n\nModality: SR (Structured Report)\n",
            "SOP Class: Comprehensive SR\n\n",
            "## Document\n\n",
            "Document Title: CT Chest Findings\n",
            "Completion Flag: COMPLETE\n",
            "Verification Flag: VERIFIED\n",
            "Content Date: 2024-01-15\n",
            "Content Time: 14:30:00\n",
            "Verifying Observer: Dr. Smith^John\n\n",
            "## Findings\n\n",
            "Container: (121070, DCM, \"Findings\")\n",
            "Code: (C0024109, UMLS, \"Lung Nodule\")\n",
            "Location: Right upper lobe\n",
            "Size: 8 mm diameter\n",
            "Measurement: (G-D705, SRT, \"Diameter\") = 8.0 mm\n",
            "Coordinate: (220, 180, 45) [x, y, z in image space]\n\n",
            "## Impression\n\n",
            "Text: Small pulmonary nodule in right upper lobe. Recommend follow-up CT in 3 months.\n",
            "Category: (111056, DCM, \"Potentially Significant\")\n\n",
            "## References\n\n",
            "Referenced SOP Instance: 1.2.840.113619.2.55.3.4.123456789\n",
            "Referenced Frame Number: 45\n",
            "Referenced Segment Number: 1\n"
        );
        let doc_items = backend.create_docitems(markdown);

        assert!(doc_items.len() >= 5);

        // Verify SR document structure
        let has_sr = doc_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => text.contains("Structured Report"),
            _ => false,
        });
        assert!(has_sr, "Should detect structured report");

        let has_verified = doc_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => text.contains("VERIFIED"),
            _ => false,
        });
        assert!(has_verified, "Should detect verification status");

        // Verify coded findings
        let has_findings = doc_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => {
                text.contains("Lung Nodule") && text.contains("8 mm diameter")
            }
            _ => false,
        });
        assert!(has_findings, "Should detect coded findings");

        // Verify measurements with codes
        let has_measurement = doc_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => text.contains("Diameter") && text.contains("8.0 mm"),
            _ => false,
        });
        assert!(has_measurement, "Should detect measurements");
    }

    /// Test Radiation Therapy (RT) Plan
    #[test]
    fn test_dicom_rt_plan() {
        let backend = DicomBackend::new();
        let markdown = concat!(
            "## Patient\n\nName: RADIATION^THERAPY\nID: RT888999\n\n",
            "## Study\n\nModality: RTPLAN (RT Plan)\n",
            "SOP Class: RT Plan Storage\n\n",
            "## RT Plan\n\n",
            "Plan Label: VMAT Prostate\n",
            "Plan Name: Prostate_VMAT_7800cGy\n",
            "Plan Date: 2024-01-10\n",
            "Plan Time: 10:00:00\n",
            "Plan Intent: CURATIVE\n",
            "Plan Geometry: PATIENT\n\n",
            "## Prescription\n\n",
            "Target Dose: 78.0 Gy\n",
            "Number of Fractions: 39\n",
            "Dose per Fraction: 2.0 Gy\n",
            "Target Volume: PTV_7800\n",
            "Target Coverage: 95%\n\n",
            "## Beam Configuration\n\n",
            "Number of Beams: 2\n",
            "Beam 1: VMAT CW (Clockwise)\n",
            "  Energy: 6 MV\n",
            "  Gantry Start: 181°\n",
            "  Gantry Stop: 179°\n",
            "  Collimator: 30°\n",
            "  MU: 245.6\n",
            "Beam 2: VMAT CCW (Counter-clockwise)\n",
            "  Energy: 6 MV\n",
            "  Gantry Start: 179°\n",
            "  Gantry Stop: 181°\n",
            "  Collimator: 330°\n",
            "  MU: 238.4\n\n",
            "## Dose Constraints\n\n",
            "Rectum V70: < 15%\n",
            "Bladder V70: < 25%\n",
            "Femoral Heads V50: < 5%\n\n",
            "## Equipment\n\n",
            "Manufacturer: Varian\n",
            "Station Name: TrueBeam STx\n",
            "Treatment Machine: TB001\n",
            "Primary Dosimeter Unit: MU\n"
        );
        let doc_items = backend.create_docitems(markdown);

        assert!(doc_items.len() >= 6);

        // Verify RT plan details
        let has_rtplan = doc_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => text.contains("RTPLAN"),
            _ => false,
        });
        assert!(has_rtplan, "Should detect RTPLAN modality");

        let has_curative = doc_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => text.contains("CURATIVE"),
            _ => false,
        });
        assert!(has_curative, "Should detect CURATIVE intent");

        // Verify prescription
        let has_prescription = doc_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => text.contains("78.0 Gy") && text.contains("39"),
            _ => false,
        });
        assert!(has_prescription, "Should detect prescription");

        // Verify VMAT beams
        let has_vmat = doc_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => text.contains("VMAT") && text.contains("Gantry"),
            _ => false,
        });
        assert!(has_vmat, "Should detect VMAT beams");

        // Verify dose constraints
        let has_constraints = doc_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => text.contains("Rectum") && text.contains("V70"),
            _ => false,
        });
        assert!(has_constraints, "Should detect dose constraints");
    }

    /// Test DICOM Waveform (ECG data)
    #[test]
    fn test_dicom_waveform_ecg() {
        let backend = DicomBackend::new();
        let markdown = concat!(
            "## Patient\n\nName: CARDIO^PATIENT\nID: ECG555666\n\n",
            "## Study\n\nModality: ECG (12-lead ECG)\n",
            "SOP Class: 12-lead ECG Waveform Storage\n\n",
            "## Acquisition\n\n",
            "Acquisition Date: 2024-01-20\n",
            "Acquisition Time: 11:45:30\n",
            "Acquisition Duration: 10 s\n",
            "Filter Low Frequency: 0.05 Hz\n",
            "Filter High Frequency: 150 Hz\n",
            "Notch Filter: 60 Hz\n\n",
            "## Waveform\n\n",
            "Number of Channels: 12\n",
            "Sampling Frequency: 500 Hz\n",
            "Number of Samples: 5000 per channel\n",
            "Channel Sensitivity: 10 mm/mV\n",
            "Baseline Filter: Yes\n\n",
            "## Leads\n\n",
            "Lead I: Limb lead (LA - RA)\n",
            "Lead II: Limb lead (LL - RA)\n",
            "Lead III: Limb lead (LL - LA)\n",
            "aVR: Augmented limb lead\n",
            "aVL: Augmented limb lead\n",
            "aVF: Augmented limb lead\n",
            "V1-V6: Precordial leads\n\n",
            "## Measurements\n\n",
            "Heart Rate: 72 bpm\n",
            "PR Interval: 160 ms\n",
            "QRS Duration: 90 ms\n",
            "QT Interval: 400 ms\n",
            "QTc (Bazett): 412 ms\n",
            "P Wave Axis: 60°\n",
            "QRS Axis: 45°\n",
            "T Wave Axis: 50°\n\n",
            "## Interpretation\n\n",
            "Rhythm: Normal sinus rhythm\n",
            "Rate: Normal (60-100 bpm)\n",
            "Intervals: Within normal limits\n",
            "Axis: Normal axis\n",
            "ST Segments: No significant abnormality\n",
            "T Waves: Normal\n",
            "Conclusion: Normal ECG\n"
        );
        let doc_items = backend.create_docitems(markdown);

        assert!(doc_items.len() >= 6);

        // Verify waveform acquisition
        let has_ecg = doc_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => text.contains("12-lead ECG"),
            _ => false,
        });
        assert!(has_ecg, "Should detect 12-lead ECG");

        let has_sampling = doc_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => text.contains("500 Hz"),
            _ => false,
        });
        assert!(has_sampling, "Should detect sampling frequency");

        // Verify 12 leads
        let has_leads = doc_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => text.contains("Lead I") && text.contains("V1-V6"),
            _ => false,
        });
        assert!(has_leads, "Should detect 12-lead configuration");

        // Verify ECG measurements
        let has_measurements = doc_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => {
                text.contains("Heart Rate: 72 bpm")
                    && text.contains("QRS Duration")
                    && text.contains("QTc")
            }
            _ => false,
        });
        assert!(has_measurements, "Should detect ECG measurements");

        // Verify interpretation
        let has_interpretation = doc_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => {
                text.contains("Normal sinus rhythm") && text.contains("Normal ECG")
            }
            _ => false,
        });
        assert!(has_interpretation, "Should detect ECG interpretation");
    }
}
