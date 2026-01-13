//! KML/KMZ format backend for docling-core
//!
//! Processes KML (Keyhole Markup Language) and KMZ (compressed KML) files into markdown documents.

use std::fmt::Write;
use std::path::Path;

use crate::error::{DoclingError, Result};

/// Process a KML or KMZ file into markdown
///
/// # Arguments
///
/// * `path` - Path to the KML or KMZ file
///
/// # Returns
///
/// Returns markdown document with placemark, folder, and geographic information.
///
/// # Errors
///
/// Returns an error if the file cannot be read or if KML/KMZ parsing fails.
///
/// # Examples
///
/// ```no_run
/// use docling_core::kml::process_kml;
///
/// let markdown = process_kml("landmarks.kml")?;
/// println!("{}", markdown);
/// # Ok::<(), docling_core::error::DoclingError>(())
/// ```
#[must_use = "this function returns the extracted markdown content"]
pub fn process_kml<P: AsRef<Path>>(path: P) -> Result<String> {
    let path = path.as_ref();

    // Parse KML file to get geographic data
    let kml = docling_gps::parse_kml(path)
        .map_err(|e| DoclingError::ConversionError(format!("Failed to parse KML: {e}")))?;

    // Start building markdown output
    let mut markdown = String::new();

    // Add title
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("document.kml");

    let kml_name = kml.name.unwrap_or_else(|| filename.to_string());
    let format_name = if kml.is_kmz { "KMZ" } else { "KML" };
    let _ = writeln!(markdown, "# {format_name} Document: {kml_name}\n");

    // Add description if available
    if let Some(desc) = kml.description {
        let _ = writeln!(markdown, "{desc}\n");
    }

    // Add document metadata
    markdown.push_str("## Document Information\n\n");
    let _ = writeln!(
        markdown,
        "- **Format:** {format_name} (Keyhole Markup Language)"
    );
    let _ = writeln!(markdown, "- **Placemarks:** {}", kml.placemarks.len());
    let _ = writeln!(markdown, "- **Folders:** {}", kml.folders.len());
    markdown.push('\n');

    // Add folders section
    if !kml.folders.is_empty() {
        markdown.push_str("## Folders\n\n");
        for (i, folder) in kml.folders.iter().enumerate() {
            let default_name = format!("Folder {}", i + 1);
            let folder_name = folder.name.as_deref().unwrap_or(&default_name);
            let _ = writeln!(markdown, "### {}. {}\n", i + 1, folder_name);

            if let Some(desc) = &folder.description {
                let _ = writeln!(markdown, "{desc}\n");
            }

            let _ = writeln!(
                markdown,
                "- **Placemarks in folder:** {}\n",
                folder.placemark_count
            );
        }
    }

    // Add placemarks section
    if !kml.placemarks.is_empty() {
        markdown.push_str("## Placemarks\n\n");
        for (i, placemark) in kml.placemarks.iter().enumerate() {
            let default_name = format!("Placemark {}", i + 1);
            let pm_name = placemark.name.as_deref().unwrap_or(&default_name);
            let _ = writeln!(markdown, "### {}. {}\n", i + 1, pm_name);

            if let Some(desc) = &placemark.description {
                let _ = writeln!(markdown, "{desc}\n");
            }

            markdown.push_str("**Location:**\n\n");
            let _ = writeln!(markdown, "- **Geometry Type:** {}", placemark.geometry_type);

            if let (Some(lat), Some(lon)) = (placemark.latitude, placemark.longitude) {
                let _ = writeln!(markdown, "- **Coordinates:** {lat:.6}°, {lon:.6}°");
            }

            if let Some(alt) = placemark.altitude {
                let _ = writeln!(markdown, "- **Altitude:** {alt:.1}m");
            }

            if placemark.coordinate_count > 1 {
                let _ = writeln!(
                    markdown,
                    "- **Total Coordinates:** {}",
                    placemark.coordinate_count
                );
            }

            markdown.push('\n');
        }
    }

    // If KML is empty, add a note
    if kml.placemarks.is_empty() && kml.folders.is_empty() {
        markdown.push_str("*This KML file is empty or contains no parseable data.*\n\n");
    }

    Ok(markdown)
}
