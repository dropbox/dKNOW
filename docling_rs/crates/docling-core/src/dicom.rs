//! DICOM format backend for docling-core
//!
//! Processes DICOM (Digital Imaging and Communications in Medicine) files into markdown documents.
//! Extracts metadata only - pixel data is not extracted or converted.

use std::fmt::Write;
use std::path::Path;

use crate::error::{DoclingError, Result};

/// Process a DICOM file into markdown
///
/// Anonymizes patient information by default for privacy compliance.
///
/// # Arguments
///
/// * `path` - Path to the DICOM file
///
/// # Returns
///
/// Returns markdown document with DICOM metadata (patient, study, series, image info).
///
/// # Errors
///
/// Returns an error if the file cannot be read or if DICOM parsing fails.
///
/// # Examples
///
/// ```no_run
/// use docling_core::dicom::process_dicom;
///
/// let markdown = process_dicom("scan.dcm")?;
/// println!("{}", markdown);
/// # Ok::<(), docling_core::error::DoclingError>(())
/// ```
#[must_use = "this function returns the extracted markdown content"]
pub fn process_dicom<P: AsRef<Path>>(path: P) -> Result<String> {
    process_dicom_with_options(path, true)
}

/// Process a DICOM file with privacy options
///
/// # Arguments
///
/// * `path` - Path to DICOM file
/// * `anonymize` - Whether to anonymize patient information (default: true for privacy)
///
/// # Returns
///
/// Returns markdown document with DICOM metadata
#[allow(
    clippy::cast_precision_loss,
    reason = "file_size as f64 is for human-readable display, precision loss is acceptable"
)]
fn process_dicom_with_options<P: AsRef<Path>>(path: P, anonymize: bool) -> Result<String> {
    // Parse DICOM file
    let metadata = docling_medical::parse_dicom(&path, anonymize)
        .map_err(|e| DoclingError::ConversionError(format!("Failed to parse DICOM file: {e}")))?;

    // Build markdown output
    let mut markdown = String::new();

    markdown.push_str("# DICOM Medical Image\n\n");

    // Patient Information
    markdown.push_str("## Patient Information\n\n");
    let _ = writeln!(markdown, "- **Name**: {}", metadata.patient.name);
    let _ = writeln!(markdown, "- **ID**: {}", metadata.patient.id);
    if let Some(birth_date) = &metadata.patient.birth_date {
        let formatted_date = docling_medical::format_dicom_date(birth_date);
        let _ = writeln!(markdown, "- **Birth Date**: {formatted_date}");
    }
    if let Some(sex) = &metadata.patient.sex {
        let _ = writeln!(markdown, "- **Sex**: {sex}");
    }
    markdown.push('\n');

    // Study Information
    markdown.push_str("## Study Information\n\n");
    if let Some(date) = &metadata.study.date {
        let formatted_date = docling_medical::format_dicom_date(date);
        if let Some(time) = &metadata.study.time {
            let formatted_time = docling_medical::format_dicom_time(time);
            let _ = writeln!(
                markdown,
                "- **Study Date**: {formatted_date} {formatted_time}"
            );
        } else {
            let _ = writeln!(markdown, "- **Study Date**: {formatted_date}");
        }
    }
    if let Some(description) = &metadata.study.description {
        let _ = writeln!(markdown, "- **Study Description**: {description}");
    }
    if let Some(physician) = &metadata.study.referring_physician {
        let _ = writeln!(markdown, "- **Referring Physician**: {physician}");
    }
    if let Some(id) = &metadata.study.id {
        let _ = writeln!(markdown, "- **Study ID**: {id}");
    }
    let _ = writeln!(markdown, "- **Study UID**: {}", metadata.study.uid);
    markdown.push('\n');

    // Series Information
    markdown.push_str("## Series Information\n\n");
    let _ = writeln!(markdown, "- **Modality**: {}", metadata.series.modality);
    if let Some(number) = &metadata.series.number {
        let _ = writeln!(markdown, "- **Series Number**: {number}");
    }
    if let Some(description) = &metadata.series.description {
        let _ = writeln!(markdown, "- **Series Description**: {description}");
    }
    let _ = writeln!(markdown, "- **Series UID**: {}", metadata.series.uid);
    markdown.push('\n');

    // Image Information
    markdown.push_str("## Image Information\n\n");
    if let Some(instance_number) = &metadata.image.instance_number {
        let _ = writeln!(markdown, "- **Instance Number**: {instance_number}");
    }
    if let (Some(rows), Some(cols)) = (metadata.image.rows, metadata.image.columns) {
        let _ = writeln!(markdown, "- **Image Dimensions**: {rows} × {cols} pixels");
    }
    if let Some(frames) = &metadata.image.number_of_frames {
        let _ = writeln!(markdown, "- **Number of Frames**: {frames}");
    }
    if let Some(image_type) = &metadata.image.image_type {
        let _ = writeln!(markdown, "- **Image Type**: {image_type}");
    }
    markdown.push('\n');

    // Technical Details
    markdown.push_str("## Technical Details\n\n");
    let _ = writeln!(
        markdown,
        "- **SOP Class UID**: {}",
        metadata.image.sop_class_uid
    );
    let _ = writeln!(
        markdown,
        "- **SOP Instance UID**: {}",
        metadata.image.sop_instance_uid
    );
    let _ = writeln!(
        markdown,
        "- **File Size**: {} bytes ({:.2} KB)",
        metadata.file_size,
        metadata.file_size as f64 / 1024.0
    );

    Ok(markdown)
}
