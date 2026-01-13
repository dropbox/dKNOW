//! # docling-gps
//!
//! GPS and geographic format support for docling-rs.
//!
//! This crate provides parsing support for GPS and geographic data formats,
//! extracting tracks, waypoints, routes, and placemarks for document processing.
//!
//! ## Supported Formats
//!
//! | Format | Extension | Description |
//! |--------|-----------|-------------|
//! | GPX | `.gpx` | GPS Exchange Format (XML-based GPS data) |
//! | KML | `.kml` | Keyhole Markup Language (Google Earth) |
//! | KMZ | `.kmz` | Compressed KML (ZIP archive) |
//!
//! ## Quick Start
//!
//! ### Parse a GPX File
//!
//! ```no_run
//! use docling_gps::parse_gpx;
//!
//! let gpx = parse_gpx("hiking_trail.gpx")?;
//!
//! // Metadata
//! println!("Name: {}", gpx.name.unwrap_or_default());
//! println!("Description: {:?}", gpx.description);
//! println!("Author: {:?}", gpx.author);
//!
//! // Track statistics
//! println!("Tracks: {}", gpx.tracks.len());
//! println!("Waypoints: {}", gpx.waypoints.len());
//! println!("Routes: {}", gpx.routes.len());
//!
//! // Access tracks
//! for track in &gpx.tracks {
//!     println!("Track: {}", track.name.as_deref().unwrap_or("Unnamed"));
//!     println!("  Segments: {}", track.segments.len());
//!     println!("  Points: {}", track.total_points);
//! }
//!
//! // Access waypoints
//! for wp in &gpx.waypoints {
//!     println!("Waypoint: {} at ({:.6}, {:.6})",
//!         wp.name.as_deref().unwrap_or("Unnamed"),
//!         wp.point.latitude, wp.point.longitude);
//! }
//! # Ok::<(), docling_gps::GpsError>(())
//! ```
//!
//! ### Parse a KML/KMZ File
//!
//! ```no_run
//! use docling_gps::parse_kml;
//!
//! // Works for both .kml and .kmz files
//! let kml = parse_kml("landmarks.kml")?;
//!
//! // Document info
//! println!("Document: {}", kml.name.unwrap_or_default());
//! println!("Description: {:?}", kml.description);
//!
//! // Placemarks
//! for placemark in &kml.placemarks {
//!     if let (Some(name), Some(lat), Some(lon)) =
//!         (&placemark.name, placemark.latitude, placemark.longitude)
//!     {
//!         println!("{}: {:.6}°, {:.6}°", name, lat, lon);
//!     }
//! }
//!
//! // Folders (hierarchical organization)
//! for folder in &kml.folders {
//!     println!("Folder: {}", folder.name.as_deref().unwrap_or("Unnamed"));
//!     println!("  Placemarks: {}", folder.placemark_count);
//! }
//! # Ok::<(), docling_gps::GpsError>(())
//! ```
//!
//! ## GPX Structure
//!
//! ### `GpxInfo`
//!
//! | Field | Type | Description |
//! |-------|------|-------------|
//! | `name` | `Option<String>` | Document name |
//! | `description` | `Option<String>` | Document description |
//! | `author` | `Option<String>` | Author name |
//! | `tracks` | `Vec<GpxTrack>` | GPS tracks |
//! | `routes` | `Vec<GpxRoute>` | Planned routes |
//! | `waypoints` | `Vec<GpxWaypoint>` | Individual waypoints |
//!
//! ### `GpxTrack`
//!
//! | Field | Type | Description |
//! |-------|------|-------------|
//! | `name` | `Option<String>` | Track name |
//! | `description` | `Option<String>` | Track description |
//! | `segments` | `Vec<GpxTrackSegment>` | Track segments |
//! | `total_points` | `usize` | Total point count |
//!
//! ### `GpxPoint`
//!
//! | Field | Type | Description |
//! |-------|------|-------------|
//! | `latitude` | `f64` | Latitude (degrees) |
//! | `longitude` | `f64` | Longitude (degrees) |
//! | `elevation` | `Option<f64>` | Elevation (meters) |
//! | `time` | `Option<String>` | Timestamp (ISO 8601) |
//!
//! ## KML Structure
//!
//! ### `KmlInfo`
//!
//! | Field | Type | Description |
//! |-------|------|-------------|
//! | `name` | `Option<String>` | Document name |
//! | `description` | `Option<String>` | Document description |
//! | `placemarks` | `Vec<KmlPlacemark>` | Location markers |
//! | `folders` | `Vec<KmlFolder>` | Organizational folders |
//! | `resources` | `Vec<EmbeddedResource>` | Embedded files (KMZ) |
//!
//! ### `KmlPlacemark`
//!
//! | Field | Type | Description |
//! |-------|------|-------------|
//! | `name` | `Option<String>` | Placemark name |
//! | `description` | `Option<String>` | Placemark description |
//! | `latitude` | `Option<f64>` | Latitude (degrees) |
//! | `longitude` | `Option<f64>` | Longitude (degrees) |
//! | `altitude` | `Option<f64>` | Altitude (meters) |
//! | `coordinates` | `Vec<KmlCoordinate>` | Polygon/path coordinates |
//!
//! ## Format Details
//!
//! ### GPX (GPS Exchange Format)
//!
//! GPX is an open standard for GPS data:
//! - XML-based, human-readable format
//! - Used by GPS devices (Garmin, etc.)
//! - Contains tracks (recorded paths), routes (planned paths), and waypoints
//! - Includes timestamps and elevation data
//!
//! ### KML (Keyhole Markup Language)
//!
//! KML is Google Earth's format:
//! - XML-based with geographic features
//! - Supports placemarks, paths, polygons
//! - Rich styling (icons, colors, labels)
//! - Hierarchical folder organization
//!
//! ### KMZ (Compressed KML)
//!
//! KMZ is a ZIP archive containing:
//! - A root `doc.kml` file
//! - Embedded images and icons
//! - Additional data files
//!
//! ## Use Cases
//!
//! - **Activity tracking**: Extract hiking/cycling/running tracks
//! - **Location documentation**: Document geographic locations
//! - **Route planning**: Parse planned routes
//! - **Map data extraction**: Extract coordinates for mapping
//! - **Travel logs**: Process GPS-recorded journeys
//!
//! ## Error Handling
//!
//! ```no_run
//! use docling_gps::{parse_gpx, GpsError};
//!
//! match parse_gpx("track.gpx") {
//!     Ok(gpx) => println!("Parsed {} tracks", gpx.tracks.len()),
//!     Err(GpsError::Io(e)) => println!("File error: {}", e),
//!     Err(GpsError::GpxParse(e)) => println!("Parse error: {}", e),
//!     Err(e) => println!("Error: {}", e),
//! }
//! ```

pub mod error;
pub mod gpx;
pub mod kml;

pub use error::{GpsError, Result};
pub use gpx::{parse_gpx, GpxInfo, GpxPoint, GpxRoute, GpxTrack, GpxTrackSegment, GpxWaypoint};
pub use kml::{parse_kml, EmbeddedResource, KmlCoordinate, KmlFolder, KmlInfo, KmlPlacemark};
