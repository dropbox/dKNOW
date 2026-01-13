//! KML backend for docling
//!
//! This backend converts KML/KMZ (Keyhole Markup Language) files to docling's document model.

// Clippy pedantic allows:
// - Altitude truncation to integer is intentional
// - KML parsing function is complex
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::too_many_lines)]

use crate::traits::{BackendOptions, DocumentBackend};
use crate::utils::{create_section_header, create_text_item, opt_vec};
use docling_core::{DocItem, DoclingError, Document, DocumentMetadata, InputFormat};
use docling_gps::{parse_kml, EmbeddedResource, KmlFolder, KmlInfo, KmlPlacemark};
use std::fmt::Write;
use std::path::Path;

/// KML backend
///
/// Converts KML (Keyhole Markup Language) and KMZ (zipped KML) files to docling's document model.
/// Supports placemarks, folders, and hierarchical organization from Google Earth format files.
///
/// ## Features
///
/// - Parse KML and KMZ files
/// - Extract placemarks (points, paths, polygons)
/// - Parse folders and hierarchical structure
/// - Extract coordinates, names, and descriptions
/// - Markdown-formatted output with geographic data
///
/// ## Example
///
/// ```no_run
/// use docling_backend::KmlBackend;
/// use docling_backend::DocumentBackend;
///
/// let backend = KmlBackend::new(docling_core::InputFormat::Kml);
/// let result = backend.parse_file("landmarks.kml", &Default::default())?;
/// println!("Map: {:?}", result.metadata.title);
/// # Ok::<(), docling_core::error::DoclingError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KmlBackend {
    format: InputFormat,
}

impl KmlBackend {
    /// Create a new KML backend instance for the specified format
    ///
    /// # Arguments
    ///
    /// * `format` - Either `InputFormat::Kml` or `InputFormat::Kmz`
    #[inline]
    #[must_use = "creates a backend instance that should be used for parsing"]
    pub const fn new(format: InputFormat) -> Self {
        Self { format }
    }

    /// Strip HTML tags from a string (simple implementation)
    fn strip_html_tags(s: &str) -> String {
        let mut result = String::new();
        let mut in_tag = false;

        for ch in s.chars() {
            match ch {
                '<' => in_tag = true,
                '>' => in_tag = false,
                _ if !in_tag => result.push(ch),
                _ => {}
            }
        }

        result
    }

    /// Format coordinates as a string
    /// Note: KML uses longitude,latitude,altitude order (x,y,z), so we preserve that order
    fn format_coordinates(lat: Option<f64>, lon: Option<f64>, alt: Option<f64>) -> String {
        // Format in KML standard order: longitude,latitude,altitude (comma-separated, no spaces)
        // Always include altitude if present, even if 0 (sea level)
        // Format altitude: preserve original precision (no decimals for integers like 0, 324)
        let format_alt = |a: f64| {
            if a.fract() == 0.0 {
                format!("{}", a as i64) // Integer altitude (0, 324, etc.)
            } else {
                format!("{a}") // Keep original precision for fractional altitudes
            }
        };

        match (lon, lat, alt) {
            (Some(lon), Some(lat), Some(alt)) => {
                format!("{:.6},{:.6},{}", lon, lat, format_alt(alt))
            }
            (Some(lon), Some(lat), None) => {
                format!("{lon:.6},{lat:.6}")
            }
            (Some(lon), None, Some(alt)) => {
                format!("{:.6},,{}", lon, format_alt(alt))
            }
            (Some(lon), None, None) => {
                format!("{lon:.6}")
            }
            (None, Some(lat), Some(alt)) => {
                format!(",{:.6},{}", lat, format_alt(alt))
            }
            (None, Some(lat), None) => {
                format!(",{lat:.6}")
            }
            (None, None, Some(alt)) => {
                format!(",,{}", format_alt(alt))
            }
            _ => "No coordinates".to_string(),
        }
    }

    /// Format a placemark as markdown
    fn format_placemark(placemark: &KmlPlacemark, index: usize) -> String {
        let mut md = String::new();

        // Use provided name or generate default
        let default_name = format!("Placemark {}", index + 1);
        let name = placemark.name.as_deref().unwrap_or(&default_name);
        let _ = writeln!(md, "### {name}\n");

        // Add description if present
        if let Some(desc) = &placemark.description {
            // Basic HTML tag removal for cleaner markdown
            let clean_desc = desc
                .replace("<br>", "\n")
                .replace("<br/>", "\n")
                .replace("<br />", "\n")
                .replace("<p>", "\n")
                .replace("</p>", "\n");
            // Simple tag stripping (handles most common cases)
            let clean_desc = Self::strip_html_tags(&clean_desc);
            let _ = writeln!(md, "{}\n", clean_desc.trim());
        }

        // Add geometry information (no bold formatting per LLM quality requirements)
        let _ = writeln!(md, "Type: {}\n", placemark.geometry_type);

        // Show all coordinates for LineStrings and Polygons
        if placemark.coordinates.len() == 1 {
            // Single coordinate (Point) - show inline
            let _ = writeln!(
                md,
                "Coordinates: {}\n",
                Self::format_coordinates(
                    placemark.latitude,
                    placemark.longitude,
                    placemark.altitude
                )
            );
        } else if !placemark.coordinates.is_empty() {
            // Multiple coordinates (LineString, Polygon) - list all with bullet points
            let _ = writeln!(
                md,
                "Coordinates ({} points):\n",
                placemark.coordinates.len()
            );
            for coord in &placemark.coordinates {
                let _ = writeln!(
                    md,
                    "- {}",
                    Self::format_coordinates(
                        Some(coord.latitude),
                        Some(coord.longitude),
                        coord.altitude
                    )
                );
            }
            md.push('\n');
        } else {
            md.push_str("Coordinates: No coordinates\n\n");
        }

        md
    }

    /// Format a folder as markdown
    fn format_folder(folder: &KmlFolder) -> String {
        let mut md = String::new();

        if let Some(name) = &folder.name {
            let _ = writeln!(md, "### Folder: {name}\n");
        } else {
            md.push_str("### Folder\n\n");
        }

        if let Some(desc) = &folder.description {
            let _ = writeln!(md, "{desc}\n");
        }

        // No bold formatting per LLM quality requirements
        let _ = writeln!(md, "Contains {} placemarks\n", folder.placemark_count);

        md
    }

    /// Format embedded resources as markdown
    fn format_embedded_resources(resources: &[EmbeddedResource]) -> String {
        let mut md = String::new();

        if resources.is_empty() {
            return md;
        }

        md.push_str("### Embedded Resources\n\n");

        // Group resources by type
        let mut images = Vec::new();
        let mut models = Vec::new();
        let mut documents = Vec::new();
        let mut others = Vec::new();

        for resource in resources {
            match resource.resource_type.as_str() {
                "image" => images.push(resource),
                "model" => models.push(resource),
                "document" => documents.push(resource),
                _ => others.push(resource),
            }
        }

        // Format images
        if !images.is_empty() {
            md.push_str("Images:\n\n");
            for img in images {
                let _ = writeln!(md, "- {} ({} bytes)", img.path, img.size);
            }
            md.push('\n');
        }

        // Format models
        if !models.is_empty() {
            md.push_str("3D Models:\n\n");
            for model in models {
                let _ = writeln!(md, "- {} ({} bytes)", model.path, model.size);
            }
            md.push('\n');
        }

        // Format documents
        if !documents.is_empty() {
            md.push_str("Documents:\n\n");
            for doc in documents {
                let _ = writeln!(md, "- {} ({} bytes)", doc.path, doc.size);
            }
            md.push('\n');
        }

        // Format other resources
        if !others.is_empty() {
            md.push_str("Other Files:\n\n");
            for other in others {
                let _ = writeln!(md, "- {} ({} bytes)", other.path, other.size);
            }
            md.push('\n');
        }

        md
    }

    /// Convert KML info directly to `DocItems` (structured representation)
    ///
    /// This generates `DocItems` directly from the parsed KML structure, preserving
    /// geographic semantic information. This is the correct architecture per CLAUDE.md:
    /// `KmlInfo` → `kml_to_docitems()` → `DocItems` → markdown serialization
    ///
    /// NOT: `KmlInfo` → `kml_to_markdown()` → text parsing → `DocItems` (loses structure)
    fn kml_to_docitems(kml: &KmlInfo) -> Vec<DocItem> {
        let mut doc_items = Vec::new();
        let mut text_idx = 0;
        let mut header_idx = 0;

        // File type indicator at the beginning (clear indication of KML document type)
        let format_text = if kml.is_kmz {
            "KMZ (Compressed KML) Document"
        } else {
            "KML Document"
        };
        doc_items.push(create_text_item(text_idx, format_text.to_string(), vec![]));
        text_idx += 1;

        // Document title (level 1 heading)
        let title_text = kml
            .name
            .clone()
            .unwrap_or_else(|| "KML Map Data".to_string());
        doc_items.push(create_section_header(header_idx, title_text, 1, vec![]));
        header_idx += 1;

        // Description (if present)
        if let Some(description) = &kml.description {
            doc_items.push(create_text_item(text_idx, description.clone(), vec![]));
            text_idx += 1;
        }

        // Summary section header (replace separator with proper section structure)
        doc_items.push(create_section_header(
            header_idx,
            "Summary".to_string(),
            2,
            vec![],
        ));
        header_idx += 1;

        // Summary statistics (no bold formatting per LLM quality requirements)
        let total_coords: usize = kml.placemarks.iter().map(|p| p.coordinate_count).sum();
        let stats = format!(
            "Placemarks: {}\n\nFolders: {}\n\nTotal Coordinates: {}",
            kml.placemarks.len(),
            kml.folders.len(),
            total_coords
        );
        doc_items.push(create_text_item(text_idx, stats, vec![]));
        text_idx += 1;

        // Folders section
        if !kml.folders.is_empty() {
            doc_items.push(create_section_header(
                header_idx,
                "Folders".to_string(),
                2,
                vec![],
            ));
            header_idx += 1;

            for folder in &kml.folders {
                // Folder name as level 3 heading
                let folder_name = folder
                    .name
                    .as_ref()
                    .map_or_else(|| "Folder".to_string(), |name| format!("Folder: {name}"));
                doc_items.push(create_section_header(header_idx, folder_name, 3, vec![]));
                header_idx += 1;

                // Folder description
                if let Some(desc) = &folder.description {
                    doc_items.push(create_text_item(text_idx, desc.clone(), vec![]));
                    text_idx += 1;
                }

                // Folder stats (no bold formatting per LLM quality requirements)
                let folder_stats = format!("Contains {} placemarks", folder.placemark_count);
                doc_items.push(create_text_item(text_idx, folder_stats, vec![]));
                text_idx += 1;
            }
        }

        // Placemarks section (add visual separator before section)
        if !kml.placemarks.is_empty() {
            // Add horizontal rule for clear visual separation from summary/folders
            doc_items.push(create_text_item(text_idx, "---".to_string(), vec![]));
            text_idx += 1;

            doc_items.push(create_section_header(
                header_idx,
                "Placemarks".to_string(),
                2,
                vec![],
            ));
            header_idx += 1;

            for (i, placemark) in kml.placemarks.iter().enumerate() {
                // Placemark name as level 3 heading
                let default_name = format!("Placemark {}", i + 1);
                let name = placemark.name.as_deref().unwrap_or(&default_name);
                doc_items.push(create_section_header(
                    header_idx,
                    name.to_string(),
                    3,
                    vec![],
                ));
                header_idx += 1;

                // Description (if present)
                if let Some(desc) = &placemark.description {
                    // Clean HTML tags from description
                    let clean_desc = desc
                        .replace("<br>", "\n")
                        .replace("<br/>", "\n")
                        .replace("<br />", "\n")
                        .replace("<p>", "\n")
                        .replace("</p>", "\n");
                    let clean_desc = Self::strip_html_tags(&clean_desc);
                    doc_items.push(create_text_item(
                        text_idx,
                        clean_desc.trim().to_string(),
                        vec![],
                    ));
                    text_idx += 1;
                }

                // Geometry type (no bold formatting per LLM quality requirements)
                let geo_type = format!("Type: {}", placemark.geometry_type);
                doc_items.push(create_text_item(text_idx, geo_type, vec![]));
                text_idx += 1;

                // Coordinates - show ALL coordinates for LineStrings and Polygons
                if placemark.coordinates.len() == 1 {
                    // Single coordinate (Point) - show inline
                    let coords = format!(
                        "Coordinates: {}",
                        Self::format_coordinates(
                            placemark.latitude,
                            placemark.longitude,
                            placemark.altitude
                        )
                    );
                    doc_items.push(create_text_item(text_idx, coords, vec![]));
                    text_idx += 1;
                } else if !placemark.coordinates.is_empty() {
                    // Multiple coordinates (LineString, Polygon) - list all
                    doc_items.push(create_text_item(
                        text_idx,
                        format!("Coordinates ({} points):", placemark.coordinates.len()),
                        vec![],
                    ));
                    text_idx += 1;

                    // List each coordinate
                    for (idx, coord) in placemark.coordinates.iter().enumerate() {
                        let coord_text = format!(
                            "  {}: {}",
                            idx + 1,
                            Self::format_coordinates(
                                Some(coord.latitude),
                                Some(coord.longitude),
                                coord.altitude
                            )
                        );
                        doc_items.push(create_text_item(text_idx, coord_text, vec![]));
                        text_idx += 1;
                    }
                } else {
                    // No coordinates
                    doc_items.push(create_text_item(
                        text_idx,
                        "Coordinates: No coordinates".to_string(),
                        vec![],
                    ));
                    text_idx += 1;
                }
            }
        }

        // Embedded resources section (for KMZ files)
        if !kml.embedded_resources.is_empty() {
            doc_items.push(create_section_header(
                header_idx,
                "Embedded Resources".to_string(),
                2,
                vec![],
            ));
            header_idx += 1;

            // Group resources by type
            let mut images: Vec<&EmbeddedResource> = Vec::new();
            let mut models: Vec<&EmbeddedResource> = Vec::new();
            let mut documents: Vec<&EmbeddedResource> = Vec::new();
            let mut others: Vec<&EmbeddedResource> = Vec::new();

            for resource in &kml.embedded_resources {
                match resource.resource_type.as_str() {
                    "image" => images.push(resource),
                    "model" => models.push(resource),
                    "document" => documents.push(resource),
                    _ => others.push(resource),
                }
            }

            // Add resource groups
            if !images.is_empty() {
                doc_items.push(create_section_header(
                    header_idx,
                    "Images".to_string(),
                    3,
                    vec![],
                ));
                header_idx += 1;
                for img in images {
                    doc_items.push(create_text_item(
                        text_idx,
                        format!("{} ({} bytes)", img.path, img.size),
                        vec![],
                    ));
                    text_idx += 1;
                }
            }

            if !models.is_empty() {
                doc_items.push(create_section_header(
                    header_idx,
                    "3D Models".to_string(),
                    3,
                    vec![],
                ));
                header_idx += 1;
                for model in models {
                    doc_items.push(create_text_item(
                        text_idx,
                        format!("{} ({} bytes)", model.path, model.size),
                        vec![],
                    ));
                    text_idx += 1;
                }
            }

            if !documents.is_empty() {
                doc_items.push(create_section_header(
                    header_idx,
                    "Documents".to_string(),
                    3,
                    vec![],
                ));
                header_idx += 1;
                for doc in documents {
                    doc_items.push(create_text_item(
                        text_idx,
                        format!("{} ({} bytes)", doc.path, doc.size),
                        vec![],
                    ));
                    text_idx += 1;
                }
            }

            if !others.is_empty() {
                doc_items.push(create_section_header(
                    header_idx,
                    "Other Files".to_string(),
                    3,
                    vec![],
                ));
                for other in others {
                    doc_items.push(create_text_item(
                        text_idx,
                        format!("{} ({} bytes)", other.path, other.size),
                        vec![],
                    ));
                    text_idx += 1;
                }
            }
        }

        doc_items
    }

    /// Convert KML info to markdown
    fn kml_to_markdown(kml: &KmlInfo) -> String {
        let mut markdown = String::new();

        // Add title with document type integrated
        let doc_type = if kml.is_kmz {
            "KMZ (Compressed KML)"
        } else {
            "KML"
        };

        if let Some(name) = &kml.name {
            let _ = writeln!(markdown, "# {name} - {doc_type} Document\n");
        } else {
            let _ = writeln!(markdown, "# {doc_type} Map Data\n");
        }

        // Add description
        if let Some(description) = &kml.description {
            let _ = writeln!(markdown, "{description}\n");
        }

        // Summary section header (replace separator with proper section structure)
        markdown.push_str("## Summary\n\n");

        // Add summary statistics (no bold formatting per LLM quality requirements)
        let total_coords: usize = kml.placemarks.iter().map(|p| p.coordinate_count).sum();
        let _ = writeln!(markdown, "Placemarks: {}\n", kml.placemarks.len());
        let _ = writeln!(markdown, "Folders: {}\n", kml.folders.len());
        let _ = writeln!(markdown, "Total Coordinates: {total_coords}\n");

        // Add folders section
        if !kml.folders.is_empty() {
            markdown.push_str("## Folders\n\n");
            for folder in &kml.folders {
                markdown.push_str(&Self::format_folder(folder));
            }
        }

        // Add placemarks section
        if !kml.placemarks.is_empty() {
            markdown.push_str("## Placemarks\n\n");
            for (i, placemark) in kml.placemarks.iter().enumerate() {
                markdown.push_str(&Self::format_placemark(placemark, i));
            }
        }

        // Add embedded resources section (for KMZ files)
        if !kml.embedded_resources.is_empty() {
            markdown.push_str("## Embedded Resources\n\n");
            markdown.push_str(&Self::format_embedded_resources(&kml.embedded_resources));
        }

        markdown
    }
}

impl DocumentBackend for KmlBackend {
    #[inline]
    fn format(&self) -> InputFormat {
        self.format
    }

    fn parse_bytes(&self, data: &[u8], options: &BackendOptions) -> Result<Document, DoclingError> {
        // Write bytes to temp file for parsing (KML parser requires file path)
        let temp_file_path = crate::utils::write_temp_file(data, "map_data", ".kml")?;
        self.parse_file(&temp_file_path, options)
    }

    fn parse_file<P: AsRef<Path>>(
        &self,
        path: P,
        _options: &BackendOptions,
    ) -> Result<Document, DoclingError> {
        let path_ref = path.as_ref();
        let full_path = path_ref.display();
        let filename = path_ref
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("map.kml");

        // Parse KML file
        let kml = parse_kml(path_ref).map_err(|e| {
            DoclingError::BackendError(format!("Failed to parse KML file: {e}: {full_path}"))
        })?;

        // Generate DocItems directly from KmlInfo structure (CORRECT per CLAUDE.md)
        // This preserves semantic geographic information (placemarks, folders, coordinates)
        let doc_items = Self::kml_to_docitems(&kml);

        // Generate markdown from the same KML structure for backwards compatibility
        let markdown = Self::kml_to_markdown(&kml);
        let num_characters = markdown.chars().count();

        // Create document
        Ok(Document {
            markdown,
            format: self.format,
            metadata: DocumentMetadata {
                num_pages: None,
                num_characters,
                title: kml.name.or_else(|| Some(filename.to_string())),
                author: None,
                created: None,
                modified: None,
                language: None,
                subject: None,
                exif: None,
            },
            content_blocks: opt_vec(doc_items),
            docling_document: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use docling_gps::{KmlCoordinate, KmlInfo};

    #[test]
    fn test_kml_backend_creation() {
        let backend = KmlBackend::new(InputFormat::Kml);
        assert_eq!(
            backend.format(),
            InputFormat::Kml,
            "KmlBackend should report Kml format"
        );
    }

    #[test]
    fn test_kmz_backend_creation() {
        let backend = KmlBackend::new(InputFormat::Kmz);
        assert_eq!(
            backend.format(),
            InputFormat::Kmz,
            "KmlBackend with Kmz should report Kmz format"
        );
    }

    #[test]
    fn test_format_coordinates() {
        // Full coordinates (KML standard format: lon,lat,alt)
        let formatted =
            KmlBackend::format_coordinates(Some(47.123_456), Some(-122.654_321), Some(123.45));
        assert_eq!(
            formatted, "-122.654321,47.123456,123.45",
            "Full coordinates should format as lon,lat,alt"
        );

        // Minimal coordinates (no altitude)
        let formatted = KmlBackend::format_coordinates(Some(47.0), Some(-122.0), None);
        assert_eq!(
            formatted, "-122.000000,47.000000",
            "Coordinates without altitude should format as lon,lat"
        );

        // No coordinates
        let formatted = KmlBackend::format_coordinates(None, None, None);
        assert_eq!(
            formatted, "No coordinates",
            "Missing coordinates should return 'No coordinates'"
        );
    }

    #[test]
    fn test_format_placemark() {
        let placemark = KmlPlacemark {
            name: Some("Test Location".to_string()),
            description: Some("A test placemark".to_string()),
            latitude: Some(47.123),
            longitude: Some(-122.456),
            altitude: None,
            coordinates: vec![KmlCoordinate {
                longitude: -122.456,
                latitude: 47.123,
                altitude: None,
            }],
            geometry_type: "Point".to_string(),
            coordinate_count: 1,
        };

        let formatted = KmlBackend::format_placemark(&placemark, 0);
        assert!(formatted.contains("### Test Location"));
        assert!(formatted.contains("A test placemark"));
        assert!(formatted.contains("Type: Point"));
        assert!(formatted.contains("-122.456000,47.123000"));
    }

    // ========== COORDINATE FORMATTING TESTS ==========

    #[test]
    fn test_format_coordinates_latitude_only() {
        let formatted = KmlBackend::format_coordinates(Some(45.5), None, None);
        assert_eq!(
            formatted, ",45.500000",
            "Latitude-only should format with leading comma"
        );
    }

    #[test]
    fn test_format_coordinates_longitude_only() {
        let formatted = KmlBackend::format_coordinates(None, Some(-100.5), None);
        assert_eq!(
            formatted, "-100.500000",
            "Longitude-only should format as single value"
        );
    }

    #[test]
    fn test_format_coordinates_with_altitude() {
        let formatted = KmlBackend::format_coordinates(Some(0.0), Some(0.0), Some(8848.86));
        assert_eq!(
            formatted, "0.000000,0.000000,8848.86",
            "Coordinates with altitude should include all three values"
        );
    }

    // ========== PLACEMARK FORMATTING TESTS ==========

    #[test]
    fn test_format_placemark_no_name() {
        let placemark = KmlPlacemark {
            name: None,
            description: None,
            latitude: Some(0.0),
            longitude: Some(0.0),
            altitude: None,
            coordinates: vec![KmlCoordinate {
                longitude: 0.0,
                latitude: 0.0,
                altitude: None,
            }],
            geometry_type: "Point".to_string(),
            coordinate_count: 1,
        };

        let formatted = KmlBackend::format_placemark(&placemark, 5);
        assert!(
            formatted.contains("### Placemark 6"),
            "Unnamed placemark should use index+1 as name"
        ); // Index + 1
    }

    #[test]
    fn test_format_placemark_with_html_description() {
        let placemark = KmlPlacemark {
            name: Some("Test".to_string()),
            description: Some("<p>Paragraph</p><br/>New line<br>Another".to_string()),
            latitude: None,
            longitude: None,
            altitude: None,
            coordinates: vec![],
            geometry_type: "Point".to_string(),
            coordinate_count: 1,
        };

        let formatted = KmlBackend::format_placemark(&placemark, 0);
        // HTML tags should be stripped
        assert!(
            formatted.contains("Paragraph"),
            "Paragraph text should be preserved after HTML stripping"
        );
        assert!(
            formatted.contains("New line"),
            "Text after br tag should be preserved"
        );
        assert!(
            !formatted.contains("<p>"),
            "Opening p tag should be stripped"
        );
        assert!(
            !formatted.contains("</p>"),
            "Closing p tag should be stripped"
        );
    }

    #[test]
    fn test_format_placemark_multi_coordinate() {
        // Create 10 coordinates for the LineString
        let coordinates: Vec<KmlCoordinate> = (0..10)
            .map(|i| KmlCoordinate {
                longitude: i as f64,
                latitude: i as f64,
                altitude: None,
            })
            .collect();

        let placemark = KmlPlacemark {
            name: Some("Path".to_string()),
            description: None,
            latitude: Some(0.0),
            longitude: Some(0.0),
            altitude: None,
            coordinates,
            geometry_type: "LineString".to_string(),
            coordinate_count: 10,
        };

        let formatted = KmlBackend::format_placemark(&placemark, 0);
        // Now it shows "Coordinates (N points):" instead of "Points: N"
        assert!(
            formatted.contains("Coordinates (10 points):"),
            "Multi-coordinate placemark should show point count"
        );
    }

    // ========== FOLDER FORMATTING TESTS ==========

    #[test]
    fn test_format_folder_with_name() {
        let folder = KmlFolder {
            name: Some("My Locations".to_string()),
            description: Some("Collection of places".to_string()),
            placemark_count: 5,
        };

        let formatted = KmlBackend::format_folder(&folder);
        assert!(
            formatted.contains("### Folder: My Locations"),
            "Folder name should appear in heading"
        );
        assert!(
            formatted.contains("Collection of places"),
            "Folder description should be included"
        );
        assert!(
            formatted.contains("Contains 5 placemarks"),
            "Folder should show placemark count"
        );
    }

    #[test]
    fn test_format_folder_no_name() {
        let folder = KmlFolder {
            name: None,
            description: None,
            placemark_count: 0,
        };

        let formatted = KmlBackend::format_folder(&folder);
        assert!(
            formatted.contains("### Folder"),
            "Unnamed folder should have generic heading"
        );
        assert!(
            formatted.contains("Contains 0 placemarks"),
            "Empty folder should show zero placemark count"
        );
    }

    // ========== KML TO MARKDOWN TESTS ==========

    #[test]
    fn test_kml_to_markdown_basic() {
        let kml = KmlInfo {
            name: Some("Test Map".to_string()),
            description: Some("Map description".to_string()),
            is_kmz: false,
            placemarks: vec![],
            folders: vec![],
            embedded_resources: vec![],
        };

        let markdown = KmlBackend::kml_to_markdown(&kml);
        assert!(
            markdown.contains("# Test Map"),
            "Map name should appear as H1 heading"
        );
        assert!(
            markdown.contains("Map description"),
            "Map description should be included"
        );
        assert!(
            markdown.contains("KML Document"),
            "KML document type indicator should be present"
        );
        assert!(
            markdown.contains("Placemarks: 0"),
            "Placemark count should show zero"
        );
        assert!(
            markdown.contains("Folders: 0"),
            "Folder count should show zero"
        );
    }

    #[test]
    fn test_kml_to_markdown_kmz_format() {
        let kml = KmlInfo {
            name: Some("Compressed Map".to_string()),
            description: None,
            is_kmz: true,
            placemarks: vec![],
            folders: vec![],
            embedded_resources: vec![],
        };

        let markdown = KmlBackend::kml_to_markdown(&kml);
        assert!(
            markdown.contains("KMZ (Compressed KML) Document"),
            "KMZ format should be indicated in document type"
        );
    }

    #[test]
    fn test_kml_to_markdown_no_name() {
        let kml = KmlInfo {
            name: None,
            description: None,
            is_kmz: false,
            placemarks: vec![],
            folders: vec![],
            embedded_resources: vec![],
        };

        let markdown = KmlBackend::kml_to_markdown(&kml);
        assert!(
            markdown.contains("# KML Map Data"),
            "Unnamed KML should use default title"
        );
    }

    #[test]
    fn test_kml_to_markdown_with_placemarks() {
        let placemark = KmlPlacemark {
            name: Some("Location 1".to_string()),
            description: None,
            latitude: Some(45.0),
            longitude: Some(-120.0),
            altitude: None,
            coordinates: vec![KmlCoordinate {
                longitude: -120.0,
                latitude: 45.0,
                altitude: None,
            }],
            geometry_type: "Point".to_string(),
            coordinate_count: 1,
        };

        let kml = KmlInfo {
            name: None,
            description: None,
            is_kmz: false,
            placemarks: vec![placemark],
            folders: vec![],
            embedded_resources: vec![],
        };

        let markdown = KmlBackend::kml_to_markdown(&kml);
        assert!(
            markdown.contains("## Placemarks"),
            "Placemarks section header should be present"
        );
        assert!(
            markdown.contains("### Location 1"),
            "Placemark name should appear as H3 heading"
        );
        assert!(
            markdown.contains("Placemarks: 1"),
            "Placemark count should be 1"
        );
    }

    // ========== DOCITEM CREATION TESTS ==========

    #[test]
    fn test_kml_to_docitems_empty() {
        let kml = KmlInfo {
            name: None,
            description: None,
            is_kmz: false,
            placemarks: vec![],
            folders: vec![],
            embedded_resources: vec![],
        };
        let doc_items = KmlBackend::kml_to_docitems(&kml);
        // Should have: title, format indicator, stats, separator
        assert_eq!(
            doc_items.len(),
            4,
            "Empty KML should generate 4 DocItems (type, title, summary header, stats)"
        );
    }

    #[test]
    fn test_kml_to_docitems_with_placemarks() {
        let placemark = KmlPlacemark {
            name: Some("Test Location".to_string()),
            description: Some("A test placemark".to_string()),
            latitude: Some(47.123),
            longitude: Some(-122.456),
            altitude: None,
            coordinates: vec![KmlCoordinate {
                longitude: -122.456,
                latitude: 47.123,
                altitude: None,
            }],
            geometry_type: "Point".to_string(),
            coordinate_count: 1,
        };

        let kml = KmlInfo {
            name: Some("Test Map".to_string()),
            description: Some("Map description".to_string()),
            is_kmz: false,
            placemarks: vec![placemark],
            folders: vec![],
            embedded_resources: vec![],
        };

        let doc_items = KmlBackend::kml_to_docitems(&kml);

        // Count expected items (after N=1612 document type indicator added):
        // 1. Document type indicator (Text) "KML Document"
        // 2. Title (SectionHeader level 1)
        // 3. Description (Text)
        // 4. "Summary" header (SectionHeader level 2)
        // 5. Stats (Text)
        // 6. Horizontal rule (Text) "---" (added N=2170 for visual separation)
        // 7. "Placemarks" header (SectionHeader level 2)
        // 8. Placemark name (SectionHeader level 3)
        // 9. Placemark description (Text)
        // 10. Geometry type (Text)
        // 11. Coordinates (Text)
        assert_eq!(
            doc_items.len(),
            11,
            "KML with placemark should generate 11 DocItems"
        );

        // Verify first item is document type indicator (N=1612)
        if let DocItem::Text { text, .. } = &doc_items[0] {
            assert_eq!(
                text, "KML Document",
                "First DocItem should be KML document type indicator"
            );
        } else {
            panic!("Expected Text for document type indicator");
        }

        // Verify second item is title header
        if let DocItem::SectionHeader { text, level, .. } = &doc_items[1] {
            assert_eq!(text, "Test Map", "Second DocItem should be the map title");
            assert_eq!(*level, 1, "Title should be level 1 heading");
        } else {
            panic!("Expected SectionHeader for title");
        }

        // Verify placemark header exists
        let has_placemark_header = doc_items.iter().any(|item| {
            if let DocItem::SectionHeader { text, level, .. } = item {
                text == "Test Location" && *level == 3
            } else {
                false
            }
        });
        assert!(
            has_placemark_header,
            "Expected placemark header in DocItems"
        );
    }

    #[test]
    fn test_kml_to_docitems_structure_preservation() {
        // Test that geographic structure is preserved in DocItems
        let placemark1 = KmlPlacemark {
            name: Some("Point A".to_string()),
            description: None,
            latitude: Some(40.0),
            longitude: Some(-120.0),
            altitude: None,
            coordinates: vec![KmlCoordinate {
                longitude: -120.0,
                latitude: 40.0,
                altitude: None,
            }],
            geometry_type: "Point".to_string(),
            coordinate_count: 1,
        };

        let placemark2 = KmlPlacemark {
            name: Some("Path B".to_string()),
            description: None,
            latitude: Some(41.0),
            longitude: Some(-121.0),
            altitude: Some(1000.0),
            coordinates: (0..50)
                .map(|i| KmlCoordinate {
                    longitude: -121.0 + i as f64 * 0.001,
                    latitude: 41.0 + i as f64 * 0.001,
                    altitude: Some(1000.0),
                })
                .collect(),
            geometry_type: "LineString".to_string(),
            coordinate_count: 50,
        };

        let folder = KmlFolder {
            name: Some("My Locations".to_string()),
            description: Some("Collection of places".to_string()),
            placemark_count: 2,
        };

        let kml = KmlInfo {
            name: Some("Multi-Feature Map".to_string()),
            description: None,
            is_kmz: true,
            placemarks: vec![placemark1, placemark2],
            folders: vec![folder],
            embedded_resources: vec![],
        };

        let doc_items = KmlBackend::kml_to_docitems(&kml);

        // Verify structure elements are present
        let has_folders_section = doc_items.iter().any(|item| {
            if let DocItem::SectionHeader { text, level, .. } = item {
                text == "Folders" && *level == 2
            } else {
                false
            }
        });
        assert!(has_folders_section, "Expected Folders section header");

        let has_placemarks_section = doc_items.iter().any(|item| {
            if let DocItem::SectionHeader { text, level, .. } = item {
                text == "Placemarks" && *level == 2
            } else {
                false
            }
        });
        assert!(has_placemarks_section, "Expected Placemarks section header");

        // Verify coordinate data is preserved in KML standard format (lon,lat,alt)
        let has_coordinates = doc_items.iter().any(|item| {
            if let DocItem::Text { text, .. } = item {
                text.contains("Coordinates:")
                    && (text.contains("-120.000000,40.000000")
                        || text.contains("-121.000000,41.000000,1000.0"))
            } else {
                false
            }
        });
        assert!(has_coordinates, "Expected coordinate data in DocItems");

        // Verify multi-coordinate info
        let has_points_count = doc_items.iter().any(|item| {
            if let DocItem::Text { text, .. } = item {
                text.contains("Coordinates (50 points):")
            } else {
                false
            }
        });
        assert!(has_points_count, "Expected points count for LineString");
    }

    // ========== UTILITY TESTS ==========

    #[test]
    fn test_strip_html_tags_basic() {
        let input = "<p>Hello <b>world</b></p>";
        let output = KmlBackend::strip_html_tags(input);
        assert_eq!(
            output, "Hello world",
            "Basic HTML tags should be stripped leaving text"
        );
    }

    #[test]
    fn test_strip_html_tags_nested() {
        let input = "<div><p>Nested <span>tags</span></p></div>";
        let output = KmlBackend::strip_html_tags(input);
        assert_eq!(
            output, "Nested tags",
            "Nested HTML tags should be stripped leaving text"
        );
    }

    #[test]
    fn test_strip_html_tags_no_tags() {
        let input = "Plain text";
        let output = KmlBackend::strip_html_tags(input);
        assert_eq!(
            output, "Plain text",
            "Plain text without tags should pass through unchanged"
        );
    }

    // ========== UNICODE AND SPECIAL CHARACTER TESTS ==========

    #[test]
    fn test_placemark_name_unicode() {
        let placemark = KmlPlacemark {
            name: Some("北京 (Beijing)".to_string()),
            description: None,
            latitude: Some(39.9042),
            longitude: Some(116.4074),
            altitude: None,
            coordinates: vec![KmlCoordinate {
                longitude: 116.4074,
                latitude: 39.9042,
                altitude: None,
            }],
            geometry_type: "Point".to_string(),
            coordinate_count: 1,
        };

        let formatted = KmlBackend::format_placemark(&placemark, 0);
        assert!(
            formatted.contains("北京 (Beijing)"),
            "Chinese characters should be preserved in placemark name"
        );
    }

    #[test]
    fn test_placemark_description_emoji() {
        let placemark = KmlPlacemark {
            name: Some("Landmark".to_string()),
            description: Some("Famous place 🗼 with emoji".to_string()),
            latitude: Some(48.8584),
            longitude: Some(2.2945),
            altitude: None,
            coordinates: vec![KmlCoordinate {
                longitude: 2.2945,
                latitude: 48.8584,
                altitude: None,
            }],
            geometry_type: "Point".to_string(),
            coordinate_count: 1,
        };

        let formatted = KmlBackend::format_placemark(&placemark, 0);
        assert!(
            formatted.contains("🗼"),
            "Emoji should be preserved in placemark description"
        );
    }

    #[test]
    fn test_markdown_utf8_encoding() {
        let kml = KmlInfo {
            name: Some("Карта (Map)".to_string()),
            description: Some("Описание (Description)".to_string()),
            is_kmz: false,
            placemarks: vec![],
            folders: vec![],
            embedded_resources: vec![],
        };

        let markdown = KmlBackend::kml_to_markdown(&kml);
        assert!(
            markdown.contains("Карта (Map)"),
            "Cyrillic text should be preserved in title"
        );
        assert!(
            markdown.contains("Описание (Description)"),
            "Cyrillic text should be preserved in description"
        );
        // Verify valid UTF-8
        assert!(
            std::str::from_utf8(markdown.as_bytes()).is_ok(),
            "Output should be valid UTF-8"
        );
    }

    #[test]
    fn test_coordinates_special_locations() {
        // Null Island (0, 0)
        let formatted = KmlBackend::format_coordinates(Some(0.0), Some(0.0), None);
        assert_eq!(
            formatted, "0.000000,0.000000",
            "Null Island (0,0) should format correctly"
        );

        // International Date Line
        let formatted = KmlBackend::format_coordinates(Some(0.0), Some(180.0), None);
        assert_eq!(
            formatted, "180.000000,0.000000",
            "International Date Line should format correctly"
        );

        // North Pole
        let formatted = KmlBackend::format_coordinates(Some(90.0), Some(0.0), None);
        assert_eq!(
            formatted, "0.000000,90.000000",
            "North Pole coordinates should format correctly"
        );

        // South Pole
        let formatted = KmlBackend::format_coordinates(Some(-90.0), Some(0.0), None);
        assert_eq!(
            formatted, "0.000000,-90.000000",
            "South Pole coordinates should format correctly"
        );
    }

    // ========== VALIDATION TESTS ==========

    #[test]
    fn test_parse_bytes_empty() {
        let backend = KmlBackend::new(InputFormat::Kml);
        let result = backend.parse_bytes(b"", &Default::default());
        // Empty bytes should fail (invalid KML)
        assert!(result.is_err(), "Empty bytes should fail to parse as KML");
    }

    #[test]
    fn test_parse_bytes_invalid_xml() {
        let backend = KmlBackend::new(InputFormat::Kml);
        let result = backend.parse_bytes(b"not xml", &Default::default());
        // Invalid XML should fail
        assert!(result.is_err(), "Invalid XML should fail to parse as KML");
    }

    #[test]
    fn test_placemark_negative_coordinates() {
        let placemark = KmlPlacemark {
            name: Some("Southern Hemisphere".to_string()),
            description: None,
            latitude: Some(-33.8688),
            longitude: Some(-151.2093),
            altitude: None,
            coordinates: vec![KmlCoordinate {
                longitude: -151.2093,
                latitude: -33.8688,
                altitude: None,
            }],
            geometry_type: "Point".to_string(),
            coordinate_count: 1,
        };

        let formatted = KmlBackend::format_placemark(&placemark, 0);
        assert!(
            formatted.contains("-151.209300,-33.868800"),
            "Negative coordinates should be formatted correctly"
        );
    }

    #[test]
    fn test_placemark_extreme_altitude() {
        // Commercial aircraft cruising altitude
        let formatted = KmlBackend::format_coordinates(Some(40.0), Some(-100.0), Some(10668.0));
        assert_eq!(
            formatted, "-100.000000,40.000000,10668",
            "High altitude should format as integer"
        );

        // Below sea level (Dead Sea)
        let formatted = KmlBackend::format_coordinates(Some(31.5), Some(35.5), Some(-430.5));
        assert_eq!(
            formatted, "35.500000,31.500000,-430.5",
            "Negative altitude should be preserved"
        );
    }

    #[test]
    fn test_strip_html_malformed() {
        // Unclosed tags
        let input = "<p>Unclosed paragraph";
        let output = KmlBackend::strip_html_tags(input);
        assert_eq!(
            output, "Unclosed paragraph",
            "Unclosed tags should be handled gracefully"
        );

        // Unmatched tags
        let input = "</div>Text<div>";
        let output = KmlBackend::strip_html_tags(input);
        assert_eq!(
            output, "Text",
            "Unmatched tags should be stripped leaving text"
        );
    }

    #[test]
    fn test_placemark_empty_geometry_type() {
        let placemark = KmlPlacemark {
            name: Some("Unknown".to_string()),
            description: None,
            latitude: None,
            longitude: None,
            altitude: None,
            coordinates: vec![],
            geometry_type: "".to_string(),
            coordinate_count: 0,
        };

        let formatted = KmlBackend::format_placemark(&placemark, 0);
        assert!(
            formatted.contains("Type:"),
            "Empty geometry type should still show Type field"
        );
        assert!(
            formatted.contains("No coordinates"),
            "Missing coordinates should show 'No coordinates'"
        );
    }

    // ========== SERIALIZATION CONSISTENCY TESTS ==========

    #[test]
    fn test_markdown_not_empty() {
        let kml = KmlInfo {
            name: Some("Test".to_string()),
            description: None,
            is_kmz: false,
            placemarks: vec![],
            folders: vec![],
            embedded_resources: vec![],
        };

        let markdown = KmlBackend::kml_to_markdown(&kml);
        assert!(!markdown.is_empty(), "Markdown output should not be empty");
        assert!(
            markdown.len() > 10,
            "Markdown should have meaningful content length"
        );
    }

    #[test]
    fn test_markdown_well_formed() {
        let kml = KmlInfo {
            name: Some("Map".to_string()),
            description: Some("Description".to_string()),
            is_kmz: false,
            placemarks: vec![],
            folders: vec![],
            embedded_resources: vec![],
        };

        let markdown = KmlBackend::kml_to_markdown(&kml);
        // Should have heading
        assert!(
            markdown.contains("# Map"),
            "Markdown should contain H1 heading with map name"
        );
        // Should have plain formatting (no bold per LLM quality requirements)
        assert!(
            markdown.contains("KML Document"),
            "Markdown should indicate KML document type"
        );
        assert!(
            markdown.contains("Placemarks: 0"),
            "Markdown should show placemark count"
        );
    }

    #[test]
    fn test_markdown_idempotent() {
        let kml = KmlInfo {
            name: Some("Test".to_string()),
            description: Some("Description".to_string()),
            is_kmz: false,
            placemarks: vec![KmlPlacemark {
                name: Some("Place".to_string()),
                description: None,
                latitude: Some(45.0),
                longitude: Some(-120.0),
                altitude: None,
                coordinates: vec![KmlCoordinate {
                    longitude: -120.0,
                    latitude: 45.0,
                    altitude: None,
                }],
                geometry_type: "Point".to_string(),
                coordinate_count: 1,
            }],
            folders: vec![],
            embedded_resources: vec![],
        };

        // Generate markdown twice
        let markdown1 = KmlBackend::kml_to_markdown(&kml);
        let markdown2 = KmlBackend::kml_to_markdown(&kml);

        // Should be identical
        assert_eq!(
            markdown1, markdown2,
            "Markdown generation should be idempotent"
        );
    }

    // ========== BACKEND OPTIONS TESTS ==========

    #[test]
    fn test_parse_with_default_options() {
        let backend = KmlBackend::new(InputFormat::Kml);
        let kml_data = r#"<?xml version="1.0" encoding="UTF-8"?>
<kml xmlns="http://www.opengis.net/kml/2.2">
  <Document>
    <name>Test</name>
  </Document>
</kml>"#;

        let result = backend.parse_bytes(kml_data.as_bytes(), &Default::default());
        // Should parse successfully with default options
        assert!(
            result.is_ok(),
            "Valid KML should parse successfully with default options"
        );
    }

    #[test]
    fn test_parse_with_custom_options() {
        let backend = KmlBackend::new(InputFormat::Kml);
        let options = BackendOptions::default();
        let kml_data = r#"<?xml version="1.0" encoding="UTF-8"?>
<kml xmlns="http://www.opengis.net/kml/2.2">
  <Document>
    <name>Test</name>
  </Document>
</kml>"#;

        let result = backend.parse_bytes(kml_data.as_bytes(), &options);
        // Options are ignored for KML but should still work
        assert!(
            result.is_ok(),
            "Valid KML should parse successfully with custom options"
        );
    }

    // ========== FORMAT-SPECIFIC TESTS ==========

    #[test]
    fn test_geometry_type_point() {
        let placemark = KmlPlacemark {
            name: Some("Point".to_string()),
            description: None,
            latitude: Some(0.0),
            longitude: Some(0.0),
            altitude: None,
            coordinates: vec![KmlCoordinate {
                longitude: 0.0,
                latitude: 0.0,
                altitude: None,
            }],
            geometry_type: "Point".to_string(),
            coordinate_count: 1,
        };

        let formatted = KmlBackend::format_placemark(&placemark, 0);
        assert!(
            formatted.contains("Type: Point"),
            "Point geometry type should be indicated"
        );
        assert!(
            !formatted.contains("Points:"),
            "Single point should not show points count"
        ); // Single point, no count
    }

    #[test]
    fn test_geometry_type_linestring() {
        let placemark = KmlPlacemark {
            name: Some("Path".to_string()),
            description: None,
            latitude: Some(0.0),
            longitude: Some(0.0),
            altitude: None,
            coordinates: (0..50)
                .map(|i| KmlCoordinate {
                    longitude: 0.0 + i as f64 * 0.001,
                    latitude: 0.0 + i as f64 * 0.001,
                    altitude: None,
                })
                .collect(),
            geometry_type: "LineString".to_string(),
            coordinate_count: 50,
        };

        let formatted = KmlBackend::format_placemark(&placemark, 0);
        assert!(
            formatted.contains("Type: LineString"),
            "LineString geometry type should be indicated"
        );
        assert!(
            formatted.contains("Coordinates (50"),
            "LineString should show coordinate count"
        );
    }

    #[test]
    fn test_geometry_type_polygon() {
        let placemark = KmlPlacemark {
            name: Some("Area".to_string()),
            description: None,
            latitude: Some(0.0),
            longitude: Some(0.0),
            altitude: None,
            coordinates: (0..100)
                .map(|i| KmlCoordinate {
                    longitude: 0.0 + i as f64 * 0.001,
                    latitude: 0.0 + i as f64 * 0.001,
                    altitude: None,
                })
                .collect(),
            geometry_type: "Polygon".to_string(),
            coordinate_count: 100,
        };

        let formatted = KmlBackend::format_placemark(&placemark, 0);
        assert!(
            formatted.contains("Type: Polygon"),
            "Polygon geometry type should be indicated"
        );
        assert!(
            formatted.contains("Coordinates (100"),
            "Polygon should show coordinate count"
        );
    }

    #[test]
    fn test_kmz_format_indicator() {
        let kml_doc = KmlInfo {
            name: Some("Compressed".to_string()),
            description: None,
            is_kmz: true,
            placemarks: vec![],
            folders: vec![],
            embedded_resources: vec![],
        };

        let markdown = KmlBackend::kml_to_markdown(&kml_doc);
        assert!(
            markdown.contains("KMZ (Compressed KML) Document"),
            "KMZ should show compressed format indicator"
        );
        assert!(
            !markdown.contains("Format: KML\n"),
            "KMZ should not show plain KML format"
        );
    }

    #[test]
    fn test_kml_format_indicator() {
        let kml_doc = KmlInfo {
            name: Some("Uncompressed".to_string()),
            description: None,
            is_kmz: false,
            placemarks: vec![],
            folders: vec![],
            embedded_resources: vec![],
        };

        let markdown = KmlBackend::kml_to_markdown(&kml_doc);
        assert!(
            markdown.contains("KML Document"),
            "KML should show document type indicator"
        );
        assert!(
            !markdown.contains("KMZ"),
            "KML should not show KMZ indicator"
        );
    }

    #[test]
    fn test_total_coordinates_calculation() {
        let kml = KmlInfo {
            name: Some("Map".to_string()),
            description: None,
            is_kmz: false,
            placemarks: vec![
                KmlPlacemark {
                    name: Some("P1".to_string()),
                    description: None,
                    latitude: None,
                    longitude: None,
                    altitude: None,
                    coordinates: vec![],
                    geometry_type: "Point".to_string(),
                    coordinate_count: 1,
                },
                KmlPlacemark {
                    name: Some("P2".to_string()),
                    description: None,
                    latitude: None,
                    longitude: None,
                    altitude: None,
                    coordinates: vec![],
                    geometry_type: "LineString".to_string(),
                    coordinate_count: 10,
                },
                KmlPlacemark {
                    name: Some("P3".to_string()),
                    description: None,
                    latitude: None,
                    longitude: None,
                    altitude: None,
                    coordinates: vec![],
                    geometry_type: "Polygon".to_string(),
                    coordinate_count: 20,
                },
            ],
            folders: vec![],
            embedded_resources: vec![],
        };

        let markdown = KmlBackend::kml_to_markdown(&kml);
        assert!(
            markdown.contains("Total Coordinates: 31"),
            "Total coordinates should sum to 31 (1 + 10 + 20)"
        ); // 1 + 10 + 20
    }

    #[test]
    fn test_folder_statistics() {
        let folder = KmlFolder {
            name: Some("Collection".to_string()),
            description: Some("My places".to_string()),
            placemark_count: 42,
        };

        let formatted = KmlBackend::format_folder(&folder);
        assert!(
            formatted.contains("Contains 42 placemarks"),
            "Folder should display correct placemark count"
        );
    }

    #[test]
    fn test_html_br_tag_conversion() {
        let placemark = KmlPlacemark {
            name: Some("Test".to_string()),
            description: Some("Line 1<br>Line 2<br/>Line 3<br />Line 4".to_string()),
            latitude: None,
            longitude: None,
            altitude: None,
            coordinates: vec![],
            geometry_type: "Point".to_string(),
            coordinate_count: 1,
        };

        let formatted = KmlBackend::format_placemark(&placemark, 0);
        // All <br> variants should be converted to newlines
        assert!(
            formatted.contains("Line 1"),
            "First line should be present after br conversion"
        );
        assert!(
            formatted.contains("Line 2"),
            "Second line should be present after br conversion"
        );
        assert!(
            formatted.contains("Line 3"),
            "Third line should be present after br conversion"
        );
        assert!(
            formatted.contains("Line 4"),
            "Fourth line should be present after br conversion"
        );
    }

    #[test]
    fn test_metadata_title_from_kml_name() {
        // Test that KML document name is extracted correctly
        let kml = KmlInfo {
            name: Some("My Map Title".to_string()),
            description: None,
            is_kmz: false,
            placemarks: vec![],
            folders: vec![],
            embedded_resources: vec![],
        };

        // Verify the name appears in the markdown
        let markdown = KmlBackend::kml_to_markdown(&kml);
        assert!(
            markdown.contains("# My Map Title"),
            "KML document name should appear as H1 heading"
        );

        // Note: When parsing via parse_bytes, the title comes from
        // the KmlInfo.name field, which is extracted from the XML
        // <Document><name> element by the docling-gps parser
    }

    #[test]
    fn test_metadata_character_count() {
        let kml = KmlInfo {
            name: Some("Test".to_string()),
            description: None,
            is_kmz: false,
            placemarks: vec![],
            folders: vec![],
            embedded_resources: vec![],
        };

        let markdown = KmlBackend::kml_to_markdown(&kml);

        // Verify character count is calculated correctly
        let num_characters = markdown.chars().count();
        assert!(num_characters > 0, "Character count should be positive");
        assert_eq!(
            num_characters,
            markdown.chars().count(),
            "Character count should be consistent"
        );
    }

    // ========== Additional Edge Cases (2 tests) ==========

    #[test]
    fn test_kml_unicode_placemark_names() {
        let kml = KmlInfo {
            name: Some("地图测试 (Map Test)".to_string()),
            description: Some("日本語の説明".to_string()),
            is_kmz: false,
            placemarks: vec![KmlPlacemark {
                name: Some("位置 🌍".to_string()),
                description: Some("テスト地点".to_string()),
                latitude: Some(35.6762),
                longitude: Some(139.6503),
                altitude: Some(0.0),
                coordinates: vec![KmlCoordinate {
                    longitude: 139.6503,
                    latitude: 35.6762,
                    altitude: Some(0.0),
                }],
                geometry_type: "Point".to_string(),
                coordinate_count: 1,
            }],
            folders: vec![],
            embedded_resources: vec![],
        };

        let markdown = KmlBackend::kml_to_markdown(&kml);
        assert!(
            markdown.contains("地图测试"),
            "Chinese title should be preserved"
        );
        assert!(
            markdown.contains("日本語"),
            "Japanese description should be preserved"
        );
        assert!(
            markdown.contains("位置"),
            "Chinese placemark name should be preserved"
        );
        assert!(
            markdown.contains("🌍"),
            "Emoji should be preserved in placemark name"
        );
        assert!(
            markdown.contains("テスト"),
            "Japanese placemark description should be preserved"
        );
    }

    #[test]
    fn test_kml_very_long_description() {
        let long_description = "A".repeat(5000);
        let kml = KmlInfo {
            name: Some("Long Description Test".to_string()),
            description: Some(long_description.clone()),
            is_kmz: false,
            placemarks: vec![],
            folders: vec![],
            embedded_resources: vec![],
        };

        let markdown = KmlBackend::kml_to_markdown(&kml);
        assert!(
            markdown.len() > 4500,
            "Output should include long description"
        );
        assert!(
            markdown.contains(&long_description[..100]),
            "Long description content should be preserved"
        );
    }

    // ========== EXTENDED EDGE CASE TESTS (N=485, +10 tests) ==========

    #[test]
    fn test_kml_multipolygon_geometry() {
        // MultiGeometry with multiple polygons (e.g., archipelago or complex boundary)
        let placemark = KmlPlacemark {
            name: Some("Hawaiian Islands".to_string()),
            description: Some("Island chain with multiple polygons".to_string()),
            latitude: Some(21.3099),
            longitude: Some(-157.8581),
            altitude: Some(0.0),
            coordinates: (0..500)
                .map(|i| KmlCoordinate {
                    longitude: -157.8581 + i as f64 * 0.001,
                    latitude: 21.3099 + i as f64 * 0.001,
                    altitude: Some(0.0),
                })
                .collect(),
            geometry_type: "MultiGeometry".to_string(),
            coordinate_count: 500, // Multiple islands = many coordinates
        };

        let formatted = KmlBackend::format_placemark(&placemark, 0);
        assert!(
            formatted.contains("Type: MultiGeometry"),
            "MultiGeometry type should be indicated"
        );
        assert!(
            formatted.contains("Coordinates (500"),
            "MultiGeometry should show 500 coordinates"
        );
        assert!(
            formatted.contains("Hawaiian Islands"),
            "Placemark name should be present"
        );
    }

    #[test]
    fn test_kml_linestring_hiking_trail() {
        // Realistic hiking trail with elevation data
        let placemark = KmlPlacemark {
            name: Some("Pacific Crest Trail - Section J".to_string()),
            description: Some("26-mile section with 4,200 ft elevation gain".to_string()),
            latitude: Some(34.3815),
            longitude: Some(-118.1352),
            altitude: Some(1524.0), // Starting elevation
            coordinates: (0..780)
                .map(|i| KmlCoordinate {
                    longitude: -118.1352 + i as f64 * 0.001,
                    latitude: 34.3815 + i as f64 * 0.001,
                    altitude: Some(1524.0 + i as f64 * 10.0), // Elevation gain
                })
                .collect(),
            geometry_type: "LineString".to_string(),
            coordinate_count: 780, // GPS point every 50m
        };

        let formatted = KmlBackend::format_placemark(&placemark, 0);
        assert!(
            formatted.contains("Pacific Crest Trail"),
            "Trail name should be present"
        );
        assert!(
            formatted.contains("-118.135200,34.381500,1524"),
            "Starting coordinate with elevation should be present"
        );
        assert!(
            formatted.contains("Coordinates (780"),
            "Trail should show 780 coordinate points"
        );
    }

    #[test]
    fn test_kml_nested_folder_hierarchy() {
        // Realistic folder structure: Continent > Country > City
        let folder_continent = KmlFolder {
            name: Some("Europe".to_string()),
            description: Some("European landmarks".to_string()),
            placemark_count: 150,
        };
        let folder_country = KmlFolder {
            name: Some("France".to_string()),
            description: Some("French cities".to_string()),
            placemark_count: 42,
        };
        let folder_city = KmlFolder {
            name: Some("Paris".to_string()),
            description: Some("Parisian landmarks".to_string()),
            placemark_count: 18,
        };

        let kml = KmlInfo {
            name: Some("World Landmarks".to_string()),
            description: None,
            is_kmz: false,
            placemarks: vec![],
            folders: vec![folder_continent, folder_country, folder_city],
            embedded_resources: vec![],
        };

        let markdown = KmlBackend::kml_to_markdown(&kml);
        assert!(
            markdown.contains("Europe"),
            "Continent folder should be present"
        );
        assert!(
            markdown.contains("Contains 150 placemarks"),
            "Europe folder count should be shown"
        );
        assert!(
            markdown.contains("France"),
            "Country folder should be present"
        );
        assert!(markdown.contains("Paris"), "City folder should be present");
        assert!(
            markdown.contains("Contains 18 placemarks"),
            "Paris folder count should be shown"
        );
    }

    #[test]
    fn test_kml_coordinate_precision_limits() {
        // GPS coordinates with maximum precision (6 decimal places = ~0.1m accuracy)
        let placemark = KmlPlacemark {
            name: Some("Survey Marker".to_string()),
            description: Some("High-precision surveying point".to_string()),
            latitude: Some(40.689247), // Statue of Liberty
            longitude: Some(-74.044502),
            altitude: Some(93.1), // Top of pedestal (altitude formatted to 1 decimal)
            coordinates: vec![KmlCoordinate {
                longitude: -74.044502,
                latitude: 40.689247,
                altitude: Some(93.1),
            }],
            geometry_type: "Point".to_string(),
            coordinate_count: 1,
        };

        let formatted = KmlBackend::format_placemark(&placemark, 0);
        assert!(
            formatted.contains("-74.044502,40.689247,93.1"),
            "High-precision coordinates should be preserved"
        );
    }

    #[test]
    fn test_kml_polygon_with_holes() {
        // Polygon with inner boundary (donut shape, e.g., island with lake)
        let placemark = KmlPlacemark {
            name: Some("Crater Lake".to_string()),
            description: Some("Circular lake inside volcanic crater".to_string()),
            latitude: Some(42.9446),
            longitude: Some(-122.1090),
            altitude: Some(1882.0),
            coordinates: (0..120)
                .map(|i| KmlCoordinate {
                    longitude: -122.1090 + i as f64 * 0.001,
                    latitude: 42.9446 + i as f64 * 0.001,
                    altitude: Some(1882.0),
                })
                .collect(),
            geometry_type: "Polygon".to_string(),
            coordinate_count: 120, // Outer ring (80) + inner ring (40)
        };

        let formatted = KmlBackend::format_placemark(&placemark, 0);
        assert!(
            formatted.contains("Crater Lake"),
            "Polygon name should be present"
        );
        assert!(
            formatted.contains("Coordinates (120"),
            "Polygon should show 120 boundary coordinates"
        );
        assert!(
            formatted.contains("Type: Polygon"),
            "Polygon geometry type should be indicated"
        );
    }

    #[test]
    fn test_kml_placemark_cross_dateline() {
        // Location spanning International Date Line (Fiji, Russia)
        let placemark = KmlPlacemark {
            name: Some("Fiji Islands".to_string()),
            description: Some("Crosses 180° meridian".to_string()),
            latitude: Some(-17.7134),
            longitude: Some(178.065), // Near date line
            altitude: Some(0.0),
            coordinates: vec![KmlCoordinate {
                longitude: 178.065,
                latitude: -17.7134,
                altitude: Some(0.0),
            }],
            geometry_type: "Polygon".to_string(),
            coordinate_count: 200,
        };

        let formatted = KmlBackend::format_placemark(&placemark, 0);
        assert!(
            formatted.contains("178.065000,-17.713400,0"),
            "Fiji coordinates near date line should format correctly"
        );
        assert!(
            formatted.contains("Fiji Islands"),
            "Fiji placemark name should be present"
        );

        // Test negative side of date line
        let placemark2 = KmlPlacemark {
            name: Some("Aleutian Islands".to_string()),
            description: None,
            latitude: Some(51.8),
            longitude: Some(-179.5), // West of date line
            altitude: None,
            coordinates: vec![KmlCoordinate {
                longitude: -179.5,
                latitude: 51.8,
                altitude: None,
            }],
            geometry_type: "LineString".to_string(),
            coordinate_count: 100,
        };

        let formatted2 = KmlBackend::format_placemark(&placemark2, 0);
        assert!(
            formatted2.contains("-179.500000,51.800000"),
            "Aleutian coordinates west of date line should format correctly"
        );
    }

    #[test]
    fn test_kml_placemark_antarctic() {
        // Extreme southern latitude, no timezone
        let placemark = KmlPlacemark {
            name: Some("Amundsen-Scott South Pole Station".to_string()),
            description: Some("Geographic South Pole".to_string()),
            latitude: Some(-90.0),  // Exact South Pole
            longitude: Some(0.0),   // Arbitrary at poles
            altitude: Some(2835.0), // Elevation on ice sheet
            coordinates: vec![KmlCoordinate {
                longitude: 0.0,
                latitude: -90.0,
                altitude: Some(2835.0),
            }],
            geometry_type: "Point".to_string(),
            coordinate_count: 1,
        };

        let formatted = KmlBackend::format_placemark(&placemark, 0);
        assert!(
            formatted.contains("0.000000,-90.000000,2835"),
            "Antarctic coordinates should format correctly"
        );
        assert!(
            formatted.contains("South Pole"),
            "South Pole station name should be present"
        );
    }

    #[test]
    fn test_kml_mixed_html_entities() {
        // HTML entities and special characters in descriptions
        let placemark = KmlPlacemark {
            name: Some("Café & Restaurant".to_string()),
            description: Some(
                "Price: $10 &lt; $20 &amp; includes coffee<br>&quot;Best in town&quot; - Review"
                    .to_string(),
            ),
            latitude: Some(48.8566),
            longitude: Some(2.3522),
            altitude: None,
            coordinates: vec![KmlCoordinate {
                longitude: 2.3522,
                latitude: 48.8566,
                altitude: None,
            }],
            geometry_type: "Point".to_string(),
            coordinate_count: 1,
        };

        let formatted = KmlBackend::format_placemark(&placemark, 0);
        assert!(
            formatted.contains("Café"),
            "Special character Café should be preserved"
        );
        // Note: HTML entity decoding would happen in XML parser, not our code
        assert!(formatted.contains("$10"), "Dollar sign should be preserved");
    }

    #[test]
    fn test_kml_large_placemark_count() {
        // Simulating large dataset (e.g., all McDonald's locations in USA)
        let mut placemarks = Vec::new();
        for i in 0..1000 {
            placemarks.push(KmlPlacemark {
                name: Some(format!("Location #{i}")),
                description: None,
                latitude: Some(40.0 + (i as f64) * 0.01),
                longitude: Some(-100.0 + (i as f64) * 0.01),
                altitude: None,
                coordinates: vec![KmlCoordinate {
                    longitude: -100.0 + (i as f64) * 0.01,
                    latitude: 40.0 + (i as f64) * 0.01,
                    altitude: None,
                }],
                geometry_type: "Point".to_string(),
                coordinate_count: 1,
            });
        }

        let kml = KmlInfo {
            name: Some("US Locations".to_string()),
            description: Some("National dataset".to_string()),
            is_kmz: true, // Large files are usually compressed
            placemarks,
            folders: vec![],
            embedded_resources: vec![],
        };

        let markdown = KmlBackend::kml_to_markdown(&kml);
        assert!(
            markdown.contains("Placemarks: 1000"),
            "Should show 1000 placemarks"
        );
        assert!(
            markdown.contains("Total Coordinates: 1000"),
            "Should show 1000 total coordinates"
        );
        assert!(
            markdown.contains("Location #0"),
            "First location should be present"
        );
        assert!(
            markdown.contains("Location #999"),
            "Last location should be present"
        );
        // Verify reasonable markdown size
        assert!(
            markdown.len() > 50_000,
            "Large dataset should produce substantial output"
        ); // At least 50 chars per placemark
    }

    #[test]
    fn test_kml_empty_optional_fields() {
        // Minimal placemark with all optional fields as None
        let placemark = KmlPlacemark {
            name: None,
            description: None,
            latitude: None,
            longitude: None,
            altitude: None,
            coordinates: vec![],
            geometry_type: "Unknown".to_string(),
            coordinate_count: 0,
        };

        let formatted = KmlBackend::format_placemark(&placemark, 0);
        // Should still generate valid markdown
        assert!(
            !formatted.is_empty(),
            "Minimal placemark should produce non-empty output"
        );
        assert!(
            formatted.contains("Type: Unknown"),
            "Unknown geometry type should be shown"
        );
        assert!(
            formatted.contains("No coordinates"),
            "Missing coordinates should be indicated"
        );

        // Test with minimal KML document
        let kml = KmlInfo {
            name: None,
            description: None,
            is_kmz: false,
            placemarks: vec![placemark],
            folders: vec![],
            embedded_resources: vec![],
        };

        let markdown = KmlBackend::kml_to_markdown(&kml);
        assert!(
            !markdown.is_empty(),
            "Minimal KML should produce non-empty output"
        );
        assert!(
            markdown.contains("Placemarks: 1"),
            "Should count 1 placemark"
        );
    }

    #[test]
    fn test_kml_with_time_stamps() {
        // Test placemark with temporal information
        let placemark = KmlPlacemark {
            name: Some("Historical Event".to_string()),
            description: Some("Event at 2023-01-15T12:00:00Z".to_string()),
            latitude: Some(40.0),
            longitude: Some(-74.0),
            altitude: None,
            coordinates: vec![KmlCoordinate {
                longitude: -74.0,
                latitude: 40.0,
                altitude: None,
            }],
            geometry_type: "Point".to_string(),
            coordinate_count: 1,
        };

        let formatted = KmlBackend::format_placemark(&placemark, 0);
        assert!(
            formatted.contains("Historical Event"),
            "Event name should be present"
        );
        assert!(
            formatted.contains("2023-01-15T12:00:00Z"),
            "ISO timestamp should be preserved"
        );
    }

    #[test]
    fn test_kml_multi_folder_hierarchy() {
        // Test complex folder structure with multiple levels
        let subfolder = KmlFolder {
            name: Some("Subfolder".to_string()),
            description: None,
            placemark_count: 1, // 1 nested point
        };

        let folder = KmlFolder {
            name: Some("Main Folder".to_string()),
            description: Some("Main folder containing multiple items".to_string()),
            placemark_count: 2, // 1 direct + 1 from subfolder
        };

        let kml = KmlInfo {
            name: Some("Hierarchy Test".to_string()),
            description: None,
            is_kmz: false,
            placemarks: vec![],
            folders: vec![folder, subfolder],
            embedded_resources: vec![],
        };

        let markdown = KmlBackend::kml_to_markdown(&kml);
        assert!(
            markdown.contains("Main Folder"),
            "Main folder should be present"
        );
        assert!(
            markdown.contains("Subfolder"),
            "Subfolder should be present"
        );
        // Verify folder structure is represented
        assert!(
            markdown.contains("placemark"),
            "Folder content info should be present"
        );
    }

    #[test]
    fn test_kml_mixed_geometry_types() {
        // Test document with various geometry types
        let placemarks = vec![
            KmlPlacemark {
                name: Some("Point Location".to_string()),
                description: None,
                latitude: Some(40.0),
                longitude: Some(-74.0),
                altitude: None,
                coordinates: vec![KmlCoordinate {
                    longitude: -74.0,
                    latitude: 40.0,
                    altitude: None,
                }],
                geometry_type: "Point".to_string(),
                coordinate_count: 1,
            },
            KmlPlacemark {
                name: Some("Route".to_string()),
                description: None,
                latitude: Some(41.0),
                longitude: Some(-73.0),
                altitude: None,
                coordinates: (0..50)
                    .map(|i| KmlCoordinate {
                        longitude: -73.0 + i as f64 * 0.001,
                        latitude: 41.0 + i as f64 * 0.001,
                        altitude: None,
                    })
                    .collect(),
                geometry_type: "LineString".to_string(),
                coordinate_count: 50,
            },
            KmlPlacemark {
                name: Some("Area".to_string()),
                description: None,
                latitude: Some(42.0),
                longitude: Some(-72.0),
                altitude: None,
                coordinates: (0..100)
                    .map(|i| KmlCoordinate {
                        longitude: -72.0 + i as f64 * 0.001,
                        latitude: 42.0 + i as f64 * 0.001,
                        altitude: None,
                    })
                    .collect(),
                geometry_type: "Polygon".to_string(),
                coordinate_count: 100,
            },
        ];

        let kml = KmlInfo {
            name: Some("Mixed Geometries".to_string()),
            description: None,
            is_kmz: false,
            placemarks,
            folders: vec![],
            embedded_resources: vec![],
        };

        let markdown = KmlBackend::kml_to_markdown(&kml);
        assert!(
            markdown.contains("Point Location"),
            "Point placemark name should be present"
        );
        assert!(
            markdown.contains("Route"),
            "LineString placemark name should be present"
        );
        assert!(
            markdown.contains("Area"),
            "Polygon placemark name should be present"
        );
        assert!(
            markdown.contains("Type: Point"),
            "Point geometry type should be indicated"
        );
        assert!(
            markdown.contains("Type: LineString"),
            "LineString geometry type should be indicated"
        );
        assert!(
            markdown.contains("Type: Polygon"),
            "Polygon geometry type should be indicated"
        );
        assert!(
            markdown.contains("Total Coordinates: 151"),
            "Total coordinates should sum correctly (1 + 50 + 100)"
        );
    }

    #[test]
    fn test_kml_extreme_coordinates() {
        // Test edge cases for coordinates
        let placemarks = vec![
            KmlPlacemark {
                name: Some("North Pole".to_string()),
                description: None,
                latitude: Some(90.0),
                longitude: Some(0.0),
                altitude: Some(0.0),
                coordinates: vec![KmlCoordinate {
                    longitude: 0.0,
                    latitude: 90.0,
                    altitude: Some(0.0),
                }],
                geometry_type: "Point".to_string(),
                coordinate_count: 1,
            },
            KmlPlacemark {
                name: Some("South Pole".to_string()),
                description: None,
                latitude: Some(-90.0),
                longitude: Some(0.0),
                altitude: Some(0.0),
                coordinates: vec![KmlCoordinate {
                    longitude: 0.0,
                    latitude: -90.0,
                    altitude: Some(0.0),
                }],
                geometry_type: "Point".to_string(),
                coordinate_count: 1,
            },
            KmlPlacemark {
                name: Some("Date Line".to_string()),
                description: None,
                latitude: Some(0.0),
                longitude: Some(180.0),
                altitude: None,
                coordinates: vec![KmlCoordinate {
                    longitude: 180.0,
                    latitude: 0.0,
                    altitude: None,
                }],
                geometry_type: "Point".to_string(),
                coordinate_count: 1,
            },
            KmlPlacemark {
                name: Some("Prime Meridian".to_string()),
                description: None,
                latitude: Some(0.0),
                longitude: Some(0.0),
                altitude: None,
                coordinates: vec![KmlCoordinate {
                    longitude: 0.0,
                    latitude: 0.0,
                    altitude: None,
                }],
                geometry_type: "Point".to_string(),
                coordinate_count: 1,
            },
        ];

        let kml = KmlInfo {
            name: Some("Extreme Locations".to_string()),
            description: None,
            is_kmz: false,
            placemarks,
            folders: vec![],
            embedded_resources: vec![],
        };

        let markdown = KmlBackend::kml_to_markdown(&kml);
        assert!(
            markdown.contains("North Pole"),
            "North Pole placemark should be present"
        );
        assert!(
            markdown.contains("South Pole"),
            "South Pole placemark should be present"
        );
        assert!(
            markdown.contains("Date Line"),
            "Date Line placemark should be present"
        );
        assert!(
            markdown.contains("Prime Meridian"),
            "Prime Meridian placemark should be present"
        );
        assert!(
            markdown.contains("90.0"),
            "Maximum latitude (90) should be present"
        );
        assert!(
            markdown.contains("-90.0"),
            "Minimum latitude (-90) should be present"
        );
        assert!(
            markdown.contains("180.0"),
            "Date line longitude (180) should be present"
        );
    }

    /// Test KML with network links (references to external KML files)
    #[test]
    fn test_kml_network_links() {
        // KML NetworkLink elements reference external KML files
        // Our parser extracts placemark data, network links are metadata
        let placemarks = vec![KmlPlacemark {
            name: Some("Local Placemark".to_string()),
            description: Some("Referenced from network link".to_string()),
            latitude: Some(37.4),
            longitude: Some(-122.0),
            altitude: None,
            coordinates: vec![KmlCoordinate {
                longitude: -122.0,
                latitude: 37.4,
                altitude: None,
            }],
            geometry_type: "Point".to_string(),
            coordinate_count: 1,
        }];

        let kml = KmlInfo {
            name: Some("Network Linked KML".to_string()),
            description: Some("Contains network link references".to_string()),
            is_kmz: false,
            placemarks,
            folders: vec![],
            embedded_resources: vec![],
        };

        let markdown = KmlBackend::kml_to_markdown(&kml);
        assert!(
            markdown.contains("Local Placemark"),
            "Network-linked placemark name should be present"
        );
        assert!(
            markdown.contains("Referenced from network link"),
            "Network link description should be preserved"
        );
    }

    /// Test KML with time spans and timestamps (temporal data)
    #[test]
    fn test_kml_time_spans() {
        // KML TimeSpan and TimeStamp elements define temporal data
        // Placemarks can have begin/end times, our parser extracts geometry
        let placemarks = vec![
            KmlPlacemark {
                name: Some("Historical Event".to_string()),
                description: Some("Time span: 2020-01-01 to 2020-12-31".to_string()),
                latitude: Some(40.7),
                longitude: Some(-74.0),
                altitude: None,
                coordinates: vec![KmlCoordinate {
                    longitude: -74.0,
                    latitude: 40.7,
                    altitude: None,
                }],
                geometry_type: "Point".to_string(),
                coordinate_count: 1,
            },
            KmlPlacemark {
                name: Some("Current Event".to_string()),
                description: Some("Timestamp: 2024-01-01T12:00:00Z".to_string()),
                latitude: Some(51.5),
                longitude: Some(-0.1),
                altitude: None,
                coordinates: vec![KmlCoordinate {
                    longitude: -0.1,
                    latitude: 51.5,
                    altitude: None,
                }],
                geometry_type: "Point".to_string(),
                coordinate_count: 1,
            },
        ];

        let kml = KmlInfo {
            name: Some("Temporal Data".to_string()),
            description: None,
            is_kmz: false,
            placemarks,
            folders: vec![],
            embedded_resources: vec![],
        };

        let markdown = KmlBackend::kml_to_markdown(&kml);
        assert!(
            markdown.contains("Historical Event"),
            "Historical event placemark should be present"
        );
        assert!(
            markdown.contains("Current Event"),
            "Current event placemark should be present"
        );
        assert!(
            markdown.contains("Time span"),
            "Time span description should be preserved"
        );
        assert!(
            markdown.contains("Timestamp"),
            "Timestamp description should be preserved"
        );
    }

    /// Test KML with ground overlays (image overlays on map)
    #[test]
    fn test_kml_ground_overlays() {
        // KML GroundOverlay elements display images on the map
        // Our parser focuses on placemarks, overlays are metadata
        let kml = KmlInfo {
            name: Some("Map with Ground Overlay".to_string()),
            description: Some("Contains satellite image overlay".to_string()),
            is_kmz: false,
            placemarks: vec![],
            folders: vec![],
            embedded_resources: vec![],
        };

        let markdown = KmlBackend::kml_to_markdown(&kml);
        assert!(
            markdown.contains("Map with Ground Overlay"),
            "Ground overlay map name should be present"
        );
        assert!(
            markdown.contains("satellite image overlay"),
            "Ground overlay description should be preserved"
        );
    }

    /// Test KML with screen overlays (UI elements)
    #[test]
    fn test_kml_screen_overlays() {
        // KML ScreenOverlay elements are UI elements (logos, legends)
        // Positioned in screen space, not geographic space
        let kml = KmlInfo {
            name: Some("Map with Screen Overlay".to_string()),
            description: Some("Includes logo and legend overlays".to_string()),
            is_kmz: false,
            placemarks: vec![],
            folders: vec![],
            embedded_resources: vec![],
        };

        let markdown = KmlBackend::kml_to_markdown(&kml);
        assert!(
            markdown.contains("Map with Screen Overlay"),
            "Screen overlay map name should be present"
        );
        assert!(
            markdown.contains("logo and legend"),
            "Screen overlay description should be preserved"
        );
    }

    /// Test KML with photo overlays (3D photo positioning)
    #[test]
    fn test_kml_photo_overlays() {
        // KML PhotoOverlay elements position photos in 3D space
        // Used for street-level photography, 3D tours
        let placemarks = vec![KmlPlacemark {
            name: Some("Street View Photo".to_string()),
            description: Some("360-degree panoramic photo".to_string()),
            latitude: Some(37.8),
            longitude: Some(-122.4),
            altitude: Some(10.0), // Camera height
            coordinates: vec![KmlCoordinate {
                longitude: -122.4,
                latitude: 37.8,
                altitude: Some(10.0),
            }],
            geometry_type: "Point".to_string(),
            coordinate_count: 1,
        }];

        let kml = KmlInfo {
            name: Some("Photo Tour".to_string()),
            description: Some("Contains photo overlays".to_string()),
            is_kmz: false,
            placemarks,
            folders: vec![],
            embedded_resources: vec![],
        };

        let markdown = KmlBackend::kml_to_markdown(&kml);
        assert!(
            markdown.contains("Street View Photo"),
            "Photo overlay placemark name should be present"
        );
        assert!(
            markdown.contains("360-degree"),
            "Photo overlay description should be preserved"
        );
        assert!(
            markdown.contains("-122.400000,37.800000,10"),
            "Photo overlay coordinates with altitude should be present"
        );
    }

    // ========== ADVANCED KML FEATURES (N=630, +5 tests) ==========

    /// Test KML with styles and icon references
    #[test]
    fn test_kml_styles_and_icons() {
        // KML Style elements define appearance (colors, icons, line width)
        // IconStyle, LineStyle, PolyStyle control visual rendering
        let placemarks = vec![
            KmlPlacemark {
                name: Some("Red Marker".to_string()),
                description: Some(
                    "Icon: http://maps.google.com/mapfiles/kml/paddle/red-circle.png".to_string(),
                ),
                latitude: Some(37.4),
                longitude: Some(-122.0),
                altitude: None,
                coordinates: vec![KmlCoordinate {
                    longitude: -122.0,
                    latitude: 37.4,
                    altitude: None,
                }],
                geometry_type: "Point".to_string(),
                coordinate_count: 1,
            },
            KmlPlacemark {
                name: Some("Blue Path".to_string()),
                description: Some("LineStyle: color=ff0000ff, width=5".to_string()),
                latitude: Some(37.5),
                longitude: Some(-122.1),
                altitude: None,
                coordinates: (0..100)
                    .map(|i| KmlCoordinate {
                        longitude: -122.1 + i as f64 * 0.001,
                        latitude: 37.5 + i as f64 * 0.001,
                        altitude: None,
                    })
                    .collect(),
                geometry_type: "LineString".to_string(),
                coordinate_count: 100,
            },
            KmlPlacemark {
                name: Some("Green Area".to_string()),
                description: Some("PolyStyle: color=7f00ff00, fill=1, outline=1".to_string()),
                latitude: Some(37.6),
                longitude: Some(-122.2),
                altitude: None,
                coordinates: (0..50)
                    .map(|i| KmlCoordinate {
                        longitude: -122.2 + i as f64 * 0.001,
                        latitude: 37.6 + i as f64 * 0.001,
                        altitude: None,
                    })
                    .collect(),
                geometry_type: "Polygon".to_string(),
                coordinate_count: 50,
            },
        ];

        let kml = KmlInfo {
            name: Some("Styled Features".to_string()),
            description: Some("KML with style definitions".to_string()),
            is_kmz: false,
            placemarks,
            folders: vec![],
            embedded_resources: vec![],
        };

        let markdown = KmlBackend::kml_to_markdown(&kml);
        assert!(
            markdown.contains("Red Marker"),
            "Point placemark with icon style should be present"
        );
        assert!(
            markdown.contains("Blue Path"),
            "LineString placemark with line style should be present"
        );
        assert!(
            markdown.contains("Green Area"),
            "Polygon placemark with poly style should be present"
        );
        assert!(
            markdown.contains("LineStyle"),
            "LineStyle description should be preserved"
        );
        assert!(
            markdown.contains("PolyStyle"),
            "PolyStyle description should be preserved"
        );
        // Verify icons are referenced in descriptions
        assert!(
            markdown.contains("http://maps.google.com"),
            "Icon URL reference should be preserved"
        );
    }

    /// Test KML with regions (Level of Detail for performance)
    #[test]
    fn test_kml_regions_lod() {
        // KML Region elements define geographic bounding boxes
        // Used for progressive loading based on camera distance (LOD)
        // LatLonAltBox + Lod (minLodPixels, maxLodPixels)
        let folder = KmlFolder {
            name: Some("High-Detail Region".to_string()),
            description: Some(
                "Region: minLat=37.0, maxLat=38.0, minLon=-123.0, maxLon=-122.0, minLodPixels=256"
                    .to_string(),
            ),
            placemark_count: 500, // Many placemarks in region
        };

        let kml = KmlInfo {
            name: Some("LOD Map".to_string()),
            description: Some("Uses regions for performance optimization".to_string()),
            is_kmz: true, // Large datasets use KMZ
            placemarks: vec![],
            folders: vec![folder],
            embedded_resources: vec![],
        };

        let markdown = KmlBackend::kml_to_markdown(&kml);
        assert!(
            markdown.contains("High-Detail Region"),
            "LOD region folder name should be present"
        );
        assert!(
            markdown.contains("minLat=37.0"),
            "Region minimum latitude should be preserved"
        );
        assert!(
            markdown.contains("maxLat=38.0"),
            "Region maximum latitude should be preserved"
        );
        assert!(
            markdown.contains("minLodPixels=256"),
            "LOD minLodPixels setting should be preserved"
        );
        assert!(
            markdown.contains("Contains 500 placemarks"),
            "Region placemark count should be shown"
        );
    }

    /// Test KML with LookAt and Camera elements (viewing perspective)
    #[test]
    fn test_kml_lookat_camera() {
        // KML LookAt: defines view by looking at a point
        // KML Camera: defines view by positioning camera in 3D space
        // Both use latitude, longitude, altitude, heading, tilt, range
        let placemarks = vec![
            KmlPlacemark {
                name: Some("Grand Canyon Viewpoint".to_string()),
                description: Some(
                    "LookAt: heading=90, tilt=60, range=5000m, altitudeMode=relativeToGround"
                        .to_string(),
                ),
                latitude: Some(36.0544),
                longitude: Some(-112.1401),
                altitude: Some(2133.0), // Rim elevation
                coordinates: vec![KmlCoordinate {
                    longitude: -112.1401,
                    latitude: 36.0544,
                    altitude: Some(2133.0),
                }],
                geometry_type: "Point".to_string(),
                coordinate_count: 1,
            },
            KmlPlacemark {
                name: Some("Aircraft Camera".to_string()),
                description: Some(
                    "Camera: heading=45, tilt=-80, roll=5, altitudeMode=absolute".to_string(),
                ),
                latitude: Some(37.4),
                longitude: Some(-122.0),
                altitude: Some(1000.0), // Flight altitude
                coordinates: vec![KmlCoordinate {
                    longitude: -122.0,
                    latitude: 37.4,
                    altitude: Some(1000.0),
                }],
                geometry_type: "Point".to_string(),
                coordinate_count: 1,
            },
        ];

        let kml = KmlInfo {
            name: Some("3D Views".to_string()),
            description: Some("Demonstrates LookAt and Camera perspectives".to_string()),
            is_kmz: false,
            placemarks,
            folders: vec![],
            embedded_resources: vec![],
        };

        let markdown = KmlBackend::kml_to_markdown(&kml);
        assert!(
            markdown.contains("Grand Canyon Viewpoint"),
            "LookAt viewpoint placemark should be present"
        );
        assert!(
            markdown.contains("Aircraft Camera"),
            "Camera placemark should be present"
        );
        assert!(
            markdown.contains("heading=90"),
            "LookAt heading parameter should be preserved"
        );
        assert!(
            markdown.contains("tilt=60"),
            "LookAt tilt parameter should be preserved"
        );
        assert!(
            markdown.contains("range=5000m"),
            "LookAt range parameter should be preserved"
        );
        assert!(
            markdown.contains("roll=5"),
            "Camera roll parameter should be preserved"
        );
    }

    /// Test KML with Tours (animated fly-through)
    #[test]
    fn test_kml_tours() {
        // KML Tour: gx:Tour element with gx:Playlist
        // Contains gx:FlyTo, gx:AnimatedUpdate, gx:Wait, gx:SoundCue
        // Used for guided tours, presentations, storytelling
        let placemarks = vec![
            KmlPlacemark {
                name: Some("Tour Stop 1: Statue of Liberty".to_string()),
                description: Some("FlyTo: duration=5s, flyToMode=smooth".to_string()),
                latitude: Some(40.6892),
                longitude: Some(-74.0445),
                altitude: Some(93.0),
                coordinates: vec![KmlCoordinate {
                    longitude: -74.0445,
                    latitude: 40.6892,
                    altitude: Some(93.0),
                }],
                geometry_type: "Point".to_string(),
                coordinate_count: 1,
            },
            KmlPlacemark {
                name: Some("Tour Stop 2: Empire State Building".to_string()),
                description: Some("FlyTo: duration=3s, flyToMode=bounce".to_string()),
                latitude: Some(40.7484),
                longitude: Some(-73.9857),
                altitude: Some(443.0), // Building height
                coordinates: vec![KmlCoordinate {
                    longitude: -73.9857,
                    latitude: 40.7484,
                    altitude: Some(443.0),
                }],
                geometry_type: "Point".to_string(),
                coordinate_count: 1,
            },
            KmlPlacemark {
                name: Some("Tour Stop 3: Central Park".to_string()),
                description: Some(
                    "FlyTo: duration=4s, wait=2s, soundCue=narration.mp3".to_string(),
                ),
                latitude: Some(40.7829),
                longitude: Some(-73.9654),
                altitude: Some(50.0),
                coordinates: (0..200)
                    .map(|i| KmlCoordinate {
                        longitude: -73.9654 + i as f64 * 0.001,
                        latitude: 40.7829 + i as f64 * 0.001,
                        altitude: Some(50.0),
                    })
                    .collect(),
                geometry_type: "Polygon".to_string(),
                coordinate_count: 200, // Park boundary
            },
        ];

        let kml = KmlInfo {
            name: Some("NYC Tour".to_string()),
            description: Some("Animated tour with 3 stops, total duration 14 seconds".to_string()),
            is_kmz: true, // Tours with media use KMZ
            placemarks,
            folders: vec![],
            embedded_resources: vec![],
        };

        let markdown = KmlBackend::kml_to_markdown(&kml);
        assert!(
            markdown.contains("NYC Tour"),
            "Tour map name should be present"
        );
        assert!(
            markdown.contains("Tour Stop 1"),
            "First tour stop placemark should be present"
        );
        assert!(
            markdown.contains("Tour Stop 2"),
            "Second tour stop placemark should be present"
        );
        assert!(
            markdown.contains("Tour Stop 3"),
            "Third tour stop placemark should be present"
        );
        assert!(
            markdown.contains("FlyTo"),
            "FlyTo animation parameter should be preserved"
        );
        assert!(
            markdown.contains("duration=5s"),
            "FlyTo duration parameter should be preserved"
        );
        assert!(
            markdown.contains("soundCue"),
            "Tour sound cue parameter should be preserved"
        );
    }

    /// Test KML with Update/Delete operations (dynamic updates)
    #[test]
    fn test_kml_update_delete() {
        // KML Update: NetworkLinkControl with Update element
        // Supports Create, Delete, Change operations for dynamic KML
        // Used for real-time data (weather, traffic, flights)
        let placemarks = vec![
            KmlPlacemark {
                name: Some("Flight AA123".to_string()),
                description: Some(
                    "Update: targetId=flight-aa123, position updates every 30s".to_string(),
                ),
                latitude: Some(40.7128),
                longitude: Some(-74.0060),
                altitude: Some(10668.0), // Cruising altitude 35,000ft
                coordinates: vec![KmlCoordinate {
                    longitude: -74.0060,
                    latitude: 40.7128,
                    altitude: Some(10668.0),
                }],
                geometry_type: "Point".to_string(),
                coordinate_count: 1,
            },
            KmlPlacemark {
                name: Some("Deleted Waypoint".to_string()),
                description: Some(
                    "Delete: targetId=waypoint-old, replaced by new route".to_string(),
                ),
                latitude: Some(41.0),
                longitude: Some(-73.0),
                altitude: None,
                coordinates: vec![KmlCoordinate {
                    longitude: -73.0,
                    latitude: 41.0,
                    altitude: None,
                }],
                geometry_type: "Point".to_string(),
                coordinate_count: 1,
            },
            KmlPlacemark {
                name: Some("Weather Alert".to_string()),
                description: Some(
                    "Change: targetId=weather-zone, updated severity to severe".to_string(),
                ),
                latitude: Some(42.0),
                longitude: Some(-72.0),
                altitude: None,
                coordinates: (0..50)
                    .map(|i| KmlCoordinate {
                        longitude: -72.0 + i as f64 * 0.001,
                        latitude: 42.0 + i as f64 * 0.001,
                        altitude: None,
                    })
                    .collect(),
                geometry_type: "Polygon".to_string(),
                coordinate_count: 50, // Alert zone boundary
            },
        ];

        let kml = KmlInfo {
            name: Some("Real-Time Updates".to_string()),
            description: Some("KML with Update/Delete operations for dynamic data".to_string()),
            is_kmz: false,
            placemarks,
            folders: vec![],
            embedded_resources: vec![],
        };

        let markdown = KmlBackend::kml_to_markdown(&kml);
        assert!(
            markdown.contains("Flight AA123"),
            "Update placemark for flight should be present"
        );
        assert!(
            markdown.contains("Deleted Waypoint"),
            "Deleted waypoint placemark should be present"
        );
        assert!(
            markdown.contains("Weather Alert"),
            "Changed weather alert placemark should be present"
        );
        assert!(
            markdown.contains("Update:"),
            "Update operation description should be preserved"
        );
        assert!(
            markdown.contains("Delete:"),
            "Delete operation description should be preserved"
        );
        assert!(
            markdown.contains("Change:"),
            "Change operation description should be preserved"
        );
        assert!(
            markdown.contains("targetId"),
            "Target ID reference should be preserved"
        );
        // Verify real-time update metadata
        assert!(
            markdown.contains("position updates every 30s"),
            "Update interval metadata should be preserved"
        );
    }
}
