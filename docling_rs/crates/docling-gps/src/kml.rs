//! KML (Keyhole Markup Language) parsing support
//!
//! Parses KML and KMZ files (Google Earth format) extracting:
//! - Placemarks (points of interest with coordinates, names, descriptions)
//! - Paths (`LineStrings` with coordinates)
//! - Regions (Polygons with boundaries)
//! - Folders (hierarchical organization)
//! - Styles and metadata
//!
//! ## Example
//!
//! ```no_run
//! use docling_gps::parse_kml;
//!
//! let kml_info = parse_kml("landmarks.kml")?;
//! println!("Found {} placemarks", kml_info.placemarks.len());
//! # Ok::<(), docling_gps::GpsError>(())
//! ```

use crate::error::{GpsError, Result};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

/// A single coordinate (lon, lat, alt)
#[derive(Debug, Clone, Copy, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct KmlCoordinate {
    /// Longitude (degrees)
    pub longitude: f64,
    /// Latitude (degrees)
    pub latitude: f64,
    /// Altitude/elevation (meters, optional)
    pub altitude: Option<f64>,
}

/// A placemark (point of interest) in a KML file
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct KmlPlacemark {
    /// Name of the placemark
    pub name: Option<String>,
    /// Description (may contain HTML)
    pub description: Option<String>,
    /// Latitude (degrees) - for backward compatibility, stores first coordinate's latitude
    pub latitude: Option<f64>,
    /// Longitude (degrees) - for backward compatibility, stores first coordinate's longitude
    pub longitude: Option<f64>,
    /// Altitude/elevation (meters, optional) - for backward compatibility, stores first coordinate's altitude
    pub altitude: Option<f64>,
    /// All coordinates in the geometry (for `LineString`, Polygon, etc.)
    pub coordinates: Vec<KmlCoordinate>,
    /// Geometry type (Point, `LineString`, Polygon, etc.)
    pub geometry_type: String,
    /// Number of coordinates in the geometry
    pub coordinate_count: usize,
}

/// A folder in a KML file (for hierarchical organization)
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct KmlFolder {
    /// Folder name
    pub name: Option<String>,
    /// Folder description
    pub description: Option<String>,
    /// Number of placemarks in folder
    pub placemark_count: usize,
}

/// An embedded resource in a KMZ file
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct EmbeddedResource {
    /// File path within the KMZ archive
    pub path: String,
    /// File size in bytes
    pub size: u64,
    /// Resource type (image, icon, model, other)
    pub resource_type: String,
}

/// Parsed KML file information
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct KmlInfo {
    /// Document name
    pub name: Option<String>,
    /// Document description
    pub description: Option<String>,
    /// Placemarks (points, paths, regions)
    pub placemarks: Vec<KmlPlacemark>,
    /// Folders (hierarchical organization)
    pub folders: Vec<KmlFolder>,
    /// Whether this was a KMZ file (zipped KML)
    pub is_kmz: bool,
    /// Embedded resources in KMZ (images, icons, models, etc.)
    pub embedded_resources: Vec<EmbeddedResource>,
}

/// Parse a KML or KMZ file
///
/// # Arguments
///
/// * `path` - Path to the KML or KMZ file
///
/// # Returns
///
/// Returns [`KmlInfo`] containing all placemarks, folders, and metadata.
///
/// # Errors
///
/// Returns [`GpsError::KmlParse`] if the file cannot be parsed.
#[must_use = "this function returns parsed KML data that should be processed"]
pub fn parse_kml<P: AsRef<Path>>(path: P) -> Result<KmlInfo> {
    use std::io::Read;
    use std::str::FromStr;

    let path = path.as_ref();

    // Check if this is a KMZ file (zip archive)
    let is_kmz = path
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("kmz"));

    // Parse KML
    let file = File::open(path)
        .map_err(|e| GpsError::KmlParse(format!("Failed to open KML file: {e}")))?;

    let (kml_data, embedded_resources) = if is_kmz {
        // KMZ is a ZIP archive containing doc.kml and embedded resources
        parse_kmz(file)?
    } else {
        // Regular KML file - no embedded resources
        let mut reader = BufReader::new(file);
        let mut kml_content = String::new();
        reader
            .read_to_string(&mut kml_content)
            .map_err(|e| GpsError::KmlParse(format!("Failed to read KML file: {e}")))?;

        let kml_data = kml::Kml::from_str(&kml_content)
            .map_err(|e| GpsError::KmlParse(format!("Failed to parse KML: {e}")))?;
        (kml_data, Vec::new())
    };

    // Extract information from KML structure
    let mut info = KmlInfo {
        name: None,
        description: None,
        placemarks: Vec::new(),
        folders: Vec::new(),
        is_kmz,
        embedded_resources,
    };

    extract_kml_info(&kml_data, &mut info);

    Ok(info)
}

/// Parse a KMZ file (zipped KML) and extract embedded resources
fn parse_kmz(file: File) -> Result<(kml::Kml, Vec<EmbeddedResource>)> {
    use std::io::Read;
    use std::str::FromStr;

    let mut zip = zip::ZipArchive::new(file)
        .map_err(|e| GpsError::KmlParse(format!("Failed to open KMZ archive: {e}")))?;

    let mut embedded_resources = Vec::new();
    let mut kml_entry_index = None;

    // First pass: collect metadata about all files
    for i in 0..zip.len() {
        let file = zip
            .by_index(i)
            .map_err(|e| GpsError::KmlParse(format!("Failed to read archive entry: {e}")))?;

        let name = file.name().to_string();
        let name_lower = name.to_lowercase();
        let size = file.size();

        // Check if this is the KML file
        if kml_entry_index.is_none()
            && (name_lower == "doc.kml"
                || std::path::Path::new(&name)
                    .extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("kml")))
        {
            kml_entry_index = Some(i);
        } else if !name.ends_with('/') {
            // This is an embedded resource (not a directory, not the KML file)
            let resource_type = classify_resource(&name);
            embedded_resources.push(EmbeddedResource {
                path: name,
                size,
                resource_type,
            });
        }
    }

    // Second pass: read the KML content
    let kml_entry_index = kml_entry_index
        .ok_or_else(|| GpsError::KmlParse("No KML file found in KMZ archive".to_string()))?;

    let mut kml_file = zip
        .by_index(kml_entry_index)
        .map_err(|e| GpsError::KmlParse(format!("Failed to read KML from KMZ: {e}")))?;

    let mut kml_content = String::new();
    kml_file
        .read_to_string(&mut kml_content)
        .map_err(|e| GpsError::KmlParse(format!("Failed to read KML content: {e}")))?;

    let kml_data = kml::Kml::from_str(&kml_content)
        .map_err(|e| GpsError::KmlParse(format!("Failed to parse KML from KMZ: {e}")))?;

    Ok((kml_data, embedded_resources))
}

/// Classify a resource file by its extension
#[inline]
fn classify_resource(filename: &str) -> String {
    let extension = std::path::Path::new(filename)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_lowercase)
        .unwrap_or_default();

    match extension.as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "tif" | "tiff" => "image".to_string(),
        "dae" | "obj" | "gltf" | "glb" => "model".to_string(),
        "xml" | "txt" | "html" | "htm" => "document".to_string(),
        _ => "other".to_string(),
    }
}

/// Recursively extract coordinates from a Geometry, including nested `MultiGeometry`
fn extract_geometry_coordinates(geometry: &kml::types::Geometry) -> Vec<KmlCoordinate> {
    use kml::types::Geometry;

    match geometry {
        Geometry::Point(point) => {
            vec![KmlCoordinate {
                longitude: point.coord.x,
                latitude: point.coord.y,
                altitude: point.coord.z,
            }]
        }
        Geometry::LineString(linestring) => linestring
            .coords
            .iter()
            .map(|c| KmlCoordinate {
                longitude: c.x,
                latitude: c.y,
                altitude: c.z,
            })
            .collect(),
        Geometry::LinearRing(ring) => ring
            .coords
            .iter()
            .map(|c| KmlCoordinate {
                longitude: c.x,
                latitude: c.y,
                altitude: c.z,
            })
            .collect(),
        Geometry::Polygon(polygon) => polygon
            .outer
            .coords
            .iter()
            .map(|c| KmlCoordinate {
                longitude: c.x,
                latitude: c.y,
                altitude: c.z,
            })
            .collect(),
        Geometry::MultiGeometry(multi) => {
            // Recursively extract from all sub-geometries
            multi
                .geometries
                .iter()
                .flat_map(extract_geometry_coordinates)
                .collect()
        }
        _ => Vec::new(),
    }
}

/// Recursively extract information from KML structure
#[allow(clippy::too_many_lines)] // Complex KML traversal - keeping together for clarity
fn extract_kml_info(kml: &kml::Kml, info: &mut KmlInfo) {
    use kml::types::Geometry;

    match kml {
        kml::Kml::KmlDocument(doc) => {
            // Extract document name and description from attrs
            if info.name.is_none() {
                info.name = doc.attrs.get("name").cloned();
            }
            if info.description.is_none() {
                info.description = doc.attrs.get("description").cloned();
            }

            // Process all elements
            for element in &doc.elements {
                extract_kml_info(element, info);
            }
        }
        kml::Kml::Folder(folder) => {
            // Extract folder information
            let folder_info = KmlFolder {
                name: folder.attrs.get("name").cloned(),
                description: folder.attrs.get("description").cloned(),
                placemark_count: folder
                    .elements
                    .iter()
                    .filter(|e| matches!(e, kml::Kml::Placemark(_)))
                    .count(),
            };
            info.folders.push(folder_info);

            // Recursively process folder contents
            for element in &folder.elements {
                extract_kml_info(element, info);
            }
        }
        kml::Kml::Placemark(placemark) => {
            // Extract placemark information including ALL coordinates
            let (geometry_type, coordinates, lat, lon, alt) = match &placemark.geometry {
                Some(Geometry::Point(point)) => {
                    let lat = Some(point.coord.y);
                    let lon = Some(point.coord.x);
                    let alt = point.coord.z;
                    let coords = vec![KmlCoordinate {
                        longitude: point.coord.x,
                        latitude: point.coord.y,
                        altitude: point.coord.z,
                    }];
                    ("Point".to_string(), coords, lat, lon, alt)
                }
                Some(Geometry::LineString(linestring)) => {
                    let coords: Vec<KmlCoordinate> = linestring
                        .coords
                        .iter()
                        .map(|c| KmlCoordinate {
                            longitude: c.x,
                            latitude: c.y,
                            altitude: c.z,
                        })
                        .collect();
                    let first_coord = linestring.coords.first();
                    let lat = first_coord.map(|c| c.y);
                    let lon = first_coord.map(|c| c.x);
                    let alt = first_coord.and_then(|c| c.z);
                    ("LineString".to_string(), coords, lat, lon, alt)
                }
                Some(Geometry::Polygon(polygon)) => {
                    let coords: Vec<KmlCoordinate> = polygon
                        .outer
                        .coords
                        .iter()
                        .map(|c| KmlCoordinate {
                            longitude: c.x,
                            latitude: c.y,
                            altitude: c.z,
                        })
                        .collect();
                    let first_coord = polygon.outer.coords.first();
                    let lat = first_coord.map(|c| c.y);
                    let lon = first_coord.map(|c| c.x);
                    let alt = first_coord.and_then(|c| c.z);
                    ("Polygon".to_string(), coords, lat, lon, alt)
                }
                Some(Geometry::MultiGeometry(multi)) => {
                    // Extract coordinates from all sub-geometries recursively
                    let coords: Vec<KmlCoordinate> = multi
                        .geometries
                        .iter()
                        .flat_map(extract_geometry_coordinates)
                        .collect();
                    let first_coord = coords.first();
                    let lat = first_coord.map(|c| c.latitude);
                    let lon = first_coord.map(|c| c.longitude);
                    let alt = first_coord.and_then(|c| c.altitude);
                    ("MultiGeometry".to_string(), coords, lat, lon, alt)
                }
                Some(Geometry::LinearRing(ring)) => {
                    let coords: Vec<KmlCoordinate> = ring
                        .coords
                        .iter()
                        .map(|c| KmlCoordinate {
                            longitude: c.x,
                            latitude: c.y,
                            altitude: c.z,
                        })
                        .collect();
                    let first_coord = ring.coords.first();
                    let lat = first_coord.map(|c| c.y);
                    let lon = first_coord.map(|c| c.x);
                    let alt = first_coord.and_then(|c| c.z);
                    ("LinearRing".to_string(), coords, lat, lon, alt)
                }
                Some(_) => {
                    // Other geometry types not explicitly handled
                    ("Other".to_string(), Vec::new(), None, None, None)
                }
                None => ("Unknown".to_string(), Vec::new(), None, None, None),
            };

            let coordinate_count = coordinates.len();
            let placemark_info = KmlPlacemark {
                name: placemark.name.clone(),
                description: placemark.description.clone(),
                latitude: lat,
                longitude: lon,
                altitude: alt, // Keep all altitudes, including 0 (sea level)
                coordinates,
                geometry_type,
                coordinate_count,
            };
            info.placemarks.push(placemark_info);
        }
        kml::Kml::Document { attrs, elements } => {
            // Same as KmlDocument but different enum variant
            if info.name.is_none() {
                info.name = attrs.get("name").cloned();
            }
            if info.description.is_none() {
                info.description = attrs.get("description").cloned();
            }

            for element in elements {
                extract_kml_info(element, info);
            }
        }
        kml::Kml::Element(element) => {
            // Handle generic elements like <name>, <description> that appear as child elements
            match element.name.as_str() {
                "name" => {
                    if info.name.is_none() {
                        info.name.clone_from(&element.content);
                    }
                }
                "description" => {
                    if info.description.is_none() {
                        info.description.clone_from(&element.content);
                    }
                }
                _ => {
                    // Ignore other generic elements
                }
            }
        }
        // Ignore other KML elements (StyleMap, Style, etc.)
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kmz_embedded_resources() {
        // Test KMZ file with embedded resources
        let test_path = "test-corpus/gps/kml/simple_landmark.kmz";
        if std::path::Path::new(test_path).exists() {
            let result = parse_kml(test_path);
            assert!(result.is_ok(), "Failed to parse KMZ: {:?}", result.err());

            let kml_info = result.unwrap();
            assert!(kml_info.is_kmz, "Should be identified as KMZ file");

            // Print embedded resources for debugging
            println!(
                "Embedded resources found: {}",
                kml_info.embedded_resources.len()
            );
            for resource in &kml_info.embedded_resources {
                println!(
                    "  - {} ({} bytes, type: {})",
                    resource.path, resource.size, resource.resource_type
                );
            }
        }
    }

    #[test]
    fn test_classify_resource() {
        assert_eq!(classify_resource("icons/marker.png"), "image");
        assert_eq!(classify_resource("images/photo.jpg"), "image");
        assert_eq!(classify_resource("models/building.dae"), "model");
        assert_eq!(classify_resource("models/object.gltf"), "model");
        assert_eq!(classify_resource("data.xml"), "document");
        assert_eq!(classify_resource("readme.txt"), "document");
        assert_eq!(classify_resource("unknown.dat"), "other");
    }

    #[test]
    fn test_extract_geometry_coordinates() {
        use kml::types::{Coord, Geometry, LineString, MultiGeometry, Point};

        // Test Point extraction
        let point = Geometry::Point(Point {
            coord: Coord {
                x: -122.0,
                y: 37.0,
                z: Some(100.0),
            },
            altitude_mode: kml::types::AltitudeMode::ClampToGround,
            extrude: false,
            attrs: std::collections::HashMap::new(),
        });
        let coords = extract_geometry_coordinates(&point);
        assert_eq!(coords.len(), 1);
        assert!((coords[0].longitude - (-122.0)).abs() < 0.001);
        assert!((coords[0].latitude - 37.0).abs() < 0.001);
        assert_eq!(coords[0].altitude, Some(100.0));

        // Test LineString extraction
        let linestring = Geometry::LineString(LineString {
            coords: vec![
                Coord {
                    x: -122.0,
                    y: 37.0,
                    z: None,
                },
                Coord {
                    x: -121.0,
                    y: 38.0,
                    z: None,
                },
            ],
            altitude_mode: kml::types::AltitudeMode::ClampToGround,
            extrude: false,
            tessellate: false,
            attrs: std::collections::HashMap::new(),
        });
        let coords = extract_geometry_coordinates(&linestring);
        assert_eq!(coords.len(), 2);

        // Test MultiGeometry extraction (recursive)
        let multi = Geometry::MultiGeometry(MultiGeometry {
            geometries: vec![
                Geometry::Point(Point {
                    coord: Coord {
                        x: -122.0,
                        y: 37.0,
                        z: None,
                    },
                    altitude_mode: kml::types::AltitudeMode::ClampToGround,
                    extrude: false,
                    attrs: std::collections::HashMap::new(),
                }),
                Geometry::LineString(LineString {
                    coords: vec![
                        Coord {
                            x: -121.0,
                            y: 36.0,
                            z: None,
                        },
                        Coord {
                            x: -120.0,
                            y: 35.0,
                            z: None,
                        },
                    ],
                    altitude_mode: kml::types::AltitudeMode::ClampToGround,
                    extrude: false,
                    tessellate: false,
                    attrs: std::collections::HashMap::new(),
                }),
            ],
            attrs: std::collections::HashMap::new(),
        });
        let coords = extract_geometry_coordinates(&multi);
        assert_eq!(
            coords.len(),
            3,
            "MultiGeometry should extract 1 point + 2 linestring coords = 3"
        );
        // First coord is from the Point
        assert!((coords[0].longitude - (-122.0)).abs() < 0.001);
        // Last coords are from the LineString
        assert!((coords[1].longitude - (-121.0)).abs() < 0.001);
        assert!((coords[2].longitude - (-120.0)).abs() < 0.001);
    }

    #[test]
    fn test_nested_multigeometry() {
        use kml::types::{Coord, Geometry, MultiGeometry, Point};

        // Test deeply nested MultiGeometry
        let nested = Geometry::MultiGeometry(MultiGeometry {
            geometries: vec![
                Geometry::Point(Point {
                    coord: Coord {
                        x: 1.0,
                        y: 1.0,
                        z: None,
                    },
                    altitude_mode: kml::types::AltitudeMode::ClampToGround,
                    extrude: false,
                    attrs: std::collections::HashMap::new(),
                }),
                Geometry::MultiGeometry(MultiGeometry {
                    geometries: vec![
                        Geometry::Point(Point {
                            coord: Coord {
                                x: 2.0,
                                y: 2.0,
                                z: None,
                            },
                            altitude_mode: kml::types::AltitudeMode::ClampToGround,
                            extrude: false,
                            attrs: std::collections::HashMap::new(),
                        }),
                        Geometry::Point(Point {
                            coord: Coord {
                                x: 3.0,
                                y: 3.0,
                                z: None,
                            },
                            altitude_mode: kml::types::AltitudeMode::ClampToGround,
                            extrude: false,
                            attrs: std::collections::HashMap::new(),
                        }),
                    ],
                    attrs: std::collections::HashMap::new(),
                }),
            ],
            attrs: std::collections::HashMap::new(),
        });

        let coords = extract_geometry_coordinates(&nested);
        assert_eq!(
            coords.len(),
            3,
            "Nested MultiGeometry should extract all 3 points"
        );
        assert!((coords[0].longitude - 1.0).abs() < 0.001);
        assert!((coords[1].longitude - 2.0).abs() < 0.001);
        assert!((coords[2].longitude - 3.0).abs() < 0.001);
    }
}
