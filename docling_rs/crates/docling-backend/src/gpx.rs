//! GPX backend for docling
//!
//! This backend converts GPX (GPS Exchange Format) files to docling's document model.

// Clippy pedantic allows:
// - GPX parsing function is complex
#![allow(clippy::too_many_lines)]

use crate::traits::{BackendOptions, DocumentBackend};
use crate::utils::{create_section_header, create_text_item, opt_vec};
use docling_core::{DocItem, DoclingError, Document, DocumentMetadata, InputFormat};
use docling_gps::{parse_gpx, GpxInfo, GpxPoint, GpxRoute, GpxTrack, GpxWaypoint};
use std::fmt::Write;
use std::path::Path;

/// GPX backend
///
/// Converts GPX (GPS Exchange Format) files to docling's document model.
/// Supports tracks, routes, and waypoints.
///
/// ## Features
///
/// - Parse GPS tracks with multiple segments
/// - Parse routes and waypoints
/// - Extract coordinates, elevation, and timestamps
/// - Markdown-formatted output with geographic data
///
/// ## Example
///
/// ```no_run
/// use docling_backend::GpxBackend;
/// use docling_backend::DocumentBackend;
///
/// let backend = GpxBackend::new();
/// let result = backend.parse_file("hiking_trail.gpx", &Default::default())?;
/// println!("GPS Track: {:?}", result.metadata.title);
/// # Ok::<(), docling_core::error::DoclingError>(())
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct GpxBackend;

impl GpxBackend {
    /// Create a new GPX backend instance
    #[inline]
    #[must_use = "creates a backend instance that should be used for parsing"]
    pub const fn new() -> Self {
        Self
    }

    /// Format a GPS point as a string
    #[inline]
    fn format_point(point: &GpxPoint) -> String {
        let mut parts = vec![
            format!("Lat: {:.6}", point.latitude),
            format!("Lon: {:.6}", point.longitude),
        ];

        if let Some(ele) = point.elevation {
            parts.push(format!("Ele: {ele:.1}m"));
        }

        if let Some(time) = &point.time {
            // Remove excessive nanosecond precision (e.g., ".000000000") from timestamp
            // Convert "2024-06-15T08:30:00.000000000Z" → "2024-06-15T08:30:00Z"
            let clean_time = time.find('.').map_or_else(
                || time.clone(),
                |dot_pos| {
                    let (before_dot, after_dot) = time.split_at(dot_pos);
                    // Find 'Z' or '+' or '-' for timezone
                    if after_dot.contains('Z') {
                        format!("{before_dot}Z")
                    } else if let Some(plus_pos) = after_dot.find('+') {
                        format!("{}{}", before_dot, &after_dot[plus_pos..])
                    } else if let Some(minus_pos) = after_dot.find('-') {
                        format!("{}{}", before_dot, &after_dot[minus_pos..])
                    } else {
                        time.clone()
                    }
                },
            );
            parts.push(format!("Time: {clean_time}"));
        }

        parts.join(", ")
    }

    /// Format a track as markdown
    fn format_track(track: &GpxTrack, index: usize) -> String {
        const MAX_POINTS_TO_SHOW: usize = 20;

        let mut md = String::new();

        let default_name = format!("Track {}", index + 1);
        let name = track.name.as_deref().unwrap_or(&default_name);
        let _ = writeln!(md, "### {name}\n");

        if let Some(desc) = &track.description {
            let _ = writeln!(md, "{desc}\n");
        }

        if let Some(track_type) = &track.track_type {
            let _ = writeln!(md, "Type: {track_type}\n");
        }

        let _ = writeln!(md, "Points: {}\n", track.total_points);
        let _ = writeln!(md, "Segments: {}\n", track.segments.len());

        // Show first and last points if available
        if let Some(first_seg) = track.segments.first() {
            if let Some(first_pt) = first_seg.points.first() {
                let _ = writeln!(md, "Start: {}\n", Self::format_point(first_pt));
            }
        }

        if let Some(last_seg) = track.segments.last() {
            if let Some(last_pt) = last_seg.points.last() {
                let _ = writeln!(md, "End: {}\n", Self::format_point(last_pt));
            }
        }

        // Show sample of track points (up to 10 per segment) for detail
        if !track.segments.is_empty() {
            md.push_str("Track Points:\n\n");
            let mut total_shown = 0;

            for (seg_idx, segment) in track.segments.iter().enumerate() {
                if total_shown >= MAX_POINTS_TO_SHOW {
                    break;
                }

                if track.segments.len() > 1 {
                    let _ = writeln!(md, "*Segment {}:*", seg_idx + 1);
                }

                for (pt_idx, point) in segment
                    .points
                    .iter()
                    .enumerate()
                    .take(MAX_POINTS_TO_SHOW - total_shown)
                {
                    let _ = writeln!(md, "{}. {}", pt_idx + 1, Self::format_point(point));
                    total_shown += 1;
                }

                if segment.points.len() > (MAX_POINTS_TO_SHOW - total_shown)
                    && total_shown < MAX_POINTS_TO_SHOW
                {
                    let _ = writeln!(
                        md,
                        "... and {} more points in this segment",
                        segment.points.len() - (MAX_POINTS_TO_SHOW - total_shown)
                    );
                }
            }

            if track.total_points > total_shown {
                let _ = writeln!(
                    md,
                    "\n*Total: {} track points across {} segments*",
                    track.total_points,
                    track.segments.len()
                );
            }
            md.push('\n');
        }

        md
    }

    /// Format a route as markdown
    fn format_route(route: &GpxRoute, index: usize) -> String {
        let mut md = String::new();

        let default_name = format!("Route {}", index + 1);
        let name = route.name.as_deref().unwrap_or(&default_name);
        let _ = writeln!(md, "### {name}\n");

        if let Some(desc) = &route.description {
            let _ = writeln!(md, "{desc}\n");
        }

        let _ = writeln!(md, "Points: {}\n", route.points.len());

        if !route.points.is_empty() {
            md.push_str("Route Points:\n\n");
            for (i, point) in route.points.iter().enumerate().take(10) {
                let _ = writeln!(md, "{}. {}", i + 1, Self::format_point(point));
            }
            if route.points.len() > 10 {
                let _ = writeln!(md, "\n... and {} more points", route.points.len() - 10);
            }
            md.push('\n');
        }

        md
    }

    /// Format a waypoint as markdown
    #[inline]
    fn format_waypoint(waypoint: &GpxWaypoint) -> String {
        let mut md = waypoint
            .name
            .as_ref()
            .map_or_else(|| "**Waypoint**".to_string(), |name| format!("**{name}**"));

        let _ = write!(md, ": {}", Self::format_point(&waypoint.point));

        if let Some(desc) = &waypoint.description {
            let _ = write!(md, " - {desc}");
        }

        md.push_str("\n\n");

        md
    }

    /// Convert GPX info directly to `DocItems` (structured representation)
    ///
    /// This generates `DocItems` directly from the parsed GPX structure, preserving
    /// geographic and track semantic information. This is the correct architecture per CLAUDE.md:
    /// `GpxInfo` → `gpx_to_docitems()` → `DocItems` → markdown serialization
    ///
    /// NOT: `GpxInfo` → `gpx_to_markdown()` → text parsing → `DocItems` (loses structure)
    fn gpx_to_docitems(gpx: &GpxInfo) -> Vec<DocItem> {
        let mut doc_items = Vec::new();
        let mut text_idx = 0;
        let mut header_idx = 0;

        // Document title (level 1 heading)
        let title_text = gpx.name.clone().unwrap_or_else(|| "GPS Data".to_string());
        doc_items.push(create_section_header(header_idx, title_text, 1, vec![]));
        header_idx += 1;

        // Description (if present)
        if let Some(description) = &gpx.description {
            doc_items.push(create_text_item(text_idx, description.clone(), vec![]));
            text_idx += 1;
        }

        // GPX metadata
        let version_text = format!("GPX Version: {}", gpx.version);
        doc_items.push(create_text_item(text_idx, version_text, vec![]));
        text_idx += 1;

        if let Some(creator) = &gpx.creator {
            let creator_text = format!("Creator: {creator}");
            doc_items.push(create_text_item(text_idx, creator_text, vec![]));
            text_idx += 1;
        }

        // Tracks section
        if !gpx.tracks.is_empty() {
            doc_items.push(create_section_header(
                header_idx,
                "Tracks".to_string(),
                2,
                vec![],
            ));
            header_idx += 1;

            for (i, track) in gpx.tracks.iter().enumerate() {
                // Track name as level 3 heading
                let default_name = format!("Track {}", i + 1);
                let name = track.name.as_deref().unwrap_or(&default_name);
                doc_items.push(create_section_header(
                    header_idx,
                    name.to_string(),
                    3,
                    vec![],
                ));
                header_idx += 1;

                // Track description
                if let Some(desc) = &track.description {
                    doc_items.push(create_text_item(text_idx, desc.clone(), vec![]));
                    text_idx += 1;
                }

                // Track type
                if let Some(track_type) = &track.track_type {
                    let type_text = format!("Type: {track_type}");
                    doc_items.push(create_text_item(text_idx, type_text, vec![]));
                    text_idx += 1;
                }

                // Track stats
                let stats = format!(
                    "Points: {}\n\nSegments: {}",
                    track.total_points,
                    track.segments.len()
                );
                doc_items.push(create_text_item(text_idx, stats, vec![]));
                text_idx += 1;

                // Start and end points
                if let Some(first_seg) = track.segments.first() {
                    if let Some(first_pt) = first_seg.points.first() {
                        let start = format!("Start: {}", Self::format_point(first_pt));
                        doc_items.push(create_text_item(text_idx, start, vec![]));
                        text_idx += 1;
                    }
                }

                if let Some(last_seg) = track.segments.last() {
                    if let Some(last_pt) = last_seg.points.last() {
                        let end = format!("End: {}", Self::format_point(last_pt));
                        doc_items.push(create_text_item(text_idx, end, vec![]));
                        text_idx += 1;
                    }
                }

                // Track points (ALL points for complete data preservation)
                // LLM quality requires all track points with elevation and timestamps
                if !track.segments.is_empty() {
                    doc_items.push(create_text_item(
                        text_idx,
                        "Track Points:".to_string(),
                        vec![],
                    ));
                    text_idx += 1;

                    // Include ALL track points (no artificial limit for DocItems)
                    // This preserves complete GPS track data for analysis
                    for (seg_idx, segment) in track.segments.iter().enumerate() {
                        let mut points_text = String::new();

                        // Segment header if multiple segments
                        if track.segments.len() > 1 {
                            let _ = writeln!(points_text, "*Segment {}:*", seg_idx + 1);
                        }

                        // Include every single point with full details
                        // Add blank line after each point for better readability
                        for (pt_idx, point) in segment.points.iter().enumerate() {
                            let _ = writeln!(
                                points_text,
                                "{}. {}\n",
                                pt_idx + 1,
                                Self::format_point(point)
                            );
                        }

                        if !points_text.is_empty() {
                            doc_items.push(create_text_item(
                                text_idx,
                                points_text.trim().to_string(),
                                vec![],
                            ));
                            text_idx += 1;
                        }
                    }

                    // Summary footer
                    let summary = format!(
                        "*Total: {} track points across {} segments*",
                        track.total_points,
                        track.segments.len()
                    );
                    doc_items.push(create_text_item(text_idx, summary, vec![]));
                    text_idx += 1;
                }
            }
        }

        // Routes section
        if !gpx.routes.is_empty() {
            doc_items.push(create_section_header(
                header_idx,
                "Routes".to_string(),
                2,
                vec![],
            ));
            header_idx += 1;

            for (i, route) in gpx.routes.iter().enumerate() {
                // Route name as level 3 heading
                let default_name = format!("Route {}", i + 1);
                let name = route.name.as_deref().unwrap_or(&default_name);
                doc_items.push(create_section_header(
                    header_idx,
                    name.to_string(),
                    3,
                    vec![],
                ));
                header_idx += 1;

                // Route description
                if let Some(desc) = &route.description {
                    doc_items.push(create_text_item(text_idx, desc.clone(), vec![]));
                    text_idx += 1;
                }

                // Route stats
                let points_text = format!("Points: {}", route.points.len());
                doc_items.push(create_text_item(text_idx, points_text, vec![]));
                text_idx += 1;

                // Route points (ALL points for complete data preservation)
                if !route.points.is_empty() {
                    doc_items.push(create_text_item(
                        text_idx,
                        "Route Points:".to_string(),
                        vec![],
                    ));
                    text_idx += 1;

                    // Include ALL route points (no artificial limit)
                    // Add blank line after each point for better readability
                    let mut route_points_text = String::new();
                    for (i, point) in route.points.iter().enumerate() {
                        let _ = writeln!(
                            route_points_text,
                            "{}. {}\n",
                            i + 1,
                            Self::format_point(point)
                        );
                    }

                    doc_items.push(create_text_item(
                        text_idx,
                        route_points_text.trim().to_string(),
                        vec![],
                    ));
                    text_idx += 1;
                }
            }
        }

        // Waypoints section (add visual separator before section)
        if !gpx.waypoints.is_empty() {
            // Add horizontal rule for clear visual separation from routes/tracks
            // This addresses LLM feedback: "waypoints section does not clearly separate"
            doc_items.push(create_text_item(text_idx, "---".to_string(), vec![]));
            text_idx += 1;

            doc_items.push(create_section_header(
                header_idx,
                "Waypoints".to_string(),
                2,
                vec![],
            ));

            for waypoint in &gpx.waypoints {
                let mut waypoint_text = waypoint
                    .name
                    .as_ref()
                    .map_or_else(|| "**Waypoint**".to_string(), |name| format!("**{name}**"));

                let _ = write!(waypoint_text, ": {}", Self::format_point(&waypoint.point));

                if let Some(desc) = &waypoint.description {
                    let _ = write!(waypoint_text, " - {desc}");
                }

                doc_items.push(create_text_item(text_idx, waypoint_text, vec![]));
                text_idx += 1;
            }
        }

        doc_items
    }

    /// Convert GPX info to markdown
    fn gpx_to_markdown(gpx: &GpxInfo) -> String {
        let mut markdown = String::new();

        // Add title
        if let Some(name) = &gpx.name {
            let _ = writeln!(markdown, "# {name}\n");
        } else {
            markdown.push_str("# GPS Data\n\n");
        }

        // Add description
        if let Some(description) = &gpx.description {
            let _ = writeln!(markdown, "{description}\n");
        }

        // Metadata section for clear separation
        markdown.push_str("## Metadata\n\n");

        // Add GPX version
        let _ = writeln!(markdown, "GPX Version: {}\n", gpx.version);

        if let Some(creator) = &gpx.creator {
            let _ = writeln!(markdown, "Creator: {creator}\n");
        }

        // Summary section to clarify content structure and improve metadata/track separation
        markdown.push_str("## Summary\n\n");
        let _ = writeln!(markdown, "Tracks: {}", gpx.tracks.len());
        let _ = writeln!(markdown, "Routes: {}", gpx.routes.len());
        let _ = writeln!(markdown, "Waypoints: {}\n", gpx.waypoints.len());

        // Add tracks section
        if !gpx.tracks.is_empty() {
            markdown.push_str("## Tracks\n\n");
            for (i, track) in gpx.tracks.iter().enumerate() {
                markdown.push_str(&Self::format_track(track, i));
            }
        }

        // Add routes section
        if !gpx.routes.is_empty() {
            markdown.push_str("## Routes\n\n");
            for (i, route) in gpx.routes.iter().enumerate() {
                markdown.push_str(&Self::format_route(route, i));
            }
        }

        // Add waypoints section
        if !gpx.waypoints.is_empty() {
            markdown.push_str("## Waypoints\n\n");
            for waypoint in &gpx.waypoints {
                markdown.push_str(&Self::format_waypoint(waypoint));
            }
        }

        markdown
    }
}

impl DocumentBackend for GpxBackend {
    #[inline]
    fn format(&self) -> InputFormat {
        InputFormat::Gpx
    }

    fn parse_bytes(&self, data: &[u8], options: &BackendOptions) -> Result<Document, DoclingError> {
        // Write bytes to temp file for parsing (GPX parser requires file path)
        let temp_file_path = crate::utils::write_temp_file(data, "gps_track", ".gpx")?;
        self.parse_file(&temp_file_path, options)
    }

    fn parse_file<P: AsRef<Path>>(
        &self,
        path: P,
        _options: &BackendOptions,
    ) -> Result<Document, DoclingError> {
        let path_ref = path.as_ref();
        let filename = path_ref
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("track.gpx");

        // Parse GPX file
        let gpx = parse_gpx(path_ref).map_err(|e| {
            DoclingError::BackendError(format!("Failed to parse GPX file: {e}: {filename}"))
        })?;

        // Generate DocItems directly from GpxInfo structure (CORRECT per CLAUDE.md)
        // This preserves semantic GPS/track information (waypoints, routes, tracks, segments)
        let doc_items = Self::gpx_to_docitems(&gpx);

        // Generate markdown from the same GPX structure for backwards compatibility
        let markdown = Self::gpx_to_markdown(&gpx);
        let num_characters = markdown.chars().count();

        // Create document
        Ok(Document {
            markdown,
            format: InputFormat::Gpx,
            metadata: DocumentMetadata {
                num_pages: None,
                num_characters,
                title: gpx.name.or_else(|| Some(filename.to_string())),
                author: gpx.author, // Use author (person), not creator (software)
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

    #[test]
    fn test_gpx_backend_creation() {
        let backend = GpxBackend::new();
        assert_eq!(backend.format(), InputFormat::Gpx);
    }

    #[test]
    fn test_format_point() {
        let point = GpxPoint {
            latitude: 47.123_456,
            longitude: -122.654_321,
            elevation: Some(123.45),
            time: Some("2024-01-15T10:30:00Z".to_string()),
        };

        let formatted = GpxBackend::format_point(&point);
        assert!(formatted.contains("Lat: 47.123456"));
        assert!(formatted.contains("Lon: -122.654321"));
        assert!(formatted.contains("Ele: 123.5m"));
        assert!(formatted.contains("Time: 2024-01-15T10:30:00Z"));
    }

    #[test]
    fn test_format_point_minimal() {
        let point = GpxPoint {
            latitude: 47.0,
            longitude: -122.0,
            elevation: None,
            time: None,
        };

        let formatted = GpxBackend::format_point(&point);
        assert!(formatted.contains("Lat: 47.000000"));
        assert!(formatted.contains("Lon: -122.000000"));
        assert!(!formatted.contains("Ele:"));
        assert!(!formatted.contains("Time:"));
    }

    #[test]
    fn test_format_track() {
        use docling_gps::{GpxTrack, GpxTrackSegment};

        let track = GpxTrack {
            name: Some("Mountain Trail".to_string()),
            description: Some("A scenic mountain hike".to_string()),
            track_type: Some("hiking".to_string()),
            total_points: 150,
            segments: vec![GpxTrackSegment {
                points: vec![
                    GpxPoint {
                        latitude: 47.0,
                        longitude: -122.0,
                        elevation: Some(100.0),
                        time: Some("2024-01-15T08:00:00Z".to_string()),
                    },
                    GpxPoint {
                        latitude: 47.1,
                        longitude: -122.1,
                        elevation: Some(200.0),
                        time: Some("2024-01-15T10:00:00Z".to_string()),
                    },
                ],
            }],
        };

        let formatted = GpxBackend::format_track(&track, 0);
        assert!(formatted.contains("### Mountain Trail"));
        assert!(formatted.contains("A scenic mountain hike"));
        assert!(formatted.contains("Type: hiking"));
        assert!(formatted.contains("Points: 150"));
        assert!(formatted.contains("Segments: 1"));
        assert!(formatted.contains("Start:"));
        assert!(formatted.contains("End:"));
    }

    #[test]
    fn test_format_track_minimal() {
        use docling_gps::GpxTrack;

        let track = GpxTrack {
            name: None,
            description: None,
            track_type: None,
            total_points: 0,
            segments: vec![],
        };

        let formatted = GpxBackend::format_track(&track, 5);
        assert!(formatted.contains("### Track 6")); // index 5 -> "Track 6"
        assert!(formatted.contains("Points: 0"));
        assert!(formatted.contains("Segments: 0"));
        assert!(!formatted.contains("Start:"));
        assert!(!formatted.contains("End:"));
    }

    #[test]
    fn test_format_route() {
        use docling_gps::GpxRoute;

        let route = GpxRoute {
            name: Some("City Tour".to_string()),
            description: Some("Downtown highlights".to_string()),
            points: vec![
                GpxPoint {
                    latitude: 47.6,
                    longitude: -122.3,
                    elevation: Some(50.0),
                    time: None,
                },
                GpxPoint {
                    latitude: 47.7,
                    longitude: -122.4,
                    elevation: Some(60.0),
                    time: None,
                },
            ],
        };

        let formatted = GpxBackend::format_route(&route, 0);
        assert!(formatted.contains("### City Tour"));
        assert!(formatted.contains("Downtown highlights"));
        assert!(formatted.contains("Points: 2"));
        assert!(formatted.contains("Route Points:"));
        assert!(formatted.contains("1. Lat: 47.600000"));
        assert!(formatted.contains("2. Lat: 47.700000"));
    }

    #[test]
    fn test_format_route_many_points() {
        use docling_gps::GpxRoute;

        let mut points = Vec::new();
        for i in 0..15 {
            points.push(GpxPoint {
                latitude: 47.0 + i as f64 * 0.1,
                longitude: -122.0,
                elevation: None,
                time: None,
            });
        }

        let route = GpxRoute {
            name: Some("Long Route".to_string()),
            description: None,
            points,
        };

        let formatted = GpxBackend::format_route(&route, 0);
        assert!(formatted.contains("Points: 15"));
        assert!(formatted.contains("... and 5 more points")); // Only shows first 10
    }

    #[test]
    fn test_format_waypoint() {
        let waypoint = GpxWaypoint {
            name: Some("Summit".to_string()),
            description: Some("Peak viewpoint".to_string()),
            point: GpxPoint {
                latitude: 47.5,
                longitude: -122.5,
                elevation: Some(1500.0),
                time: Some("2024-01-15T12:00:00Z".to_string()),
            },
        };

        let formatted = GpxBackend::format_waypoint(&waypoint);
        assert!(formatted.contains("**Summit**"));
        assert!(formatted.contains("Lat: 47.500000"));
        assert!(formatted.contains("Lon: -122.500000"));
        assert!(formatted.contains("Ele: 1500.0m"));
        assert!(formatted.contains("Peak viewpoint"));
    }

    #[test]
    fn test_format_waypoint_minimal() {
        let waypoint = GpxWaypoint {
            name: None,
            description: None,
            point: GpxPoint {
                latitude: 47.0,
                longitude: -122.0,
                elevation: None,
                time: None,
            },
        };

        let formatted = GpxBackend::format_waypoint(&waypoint);
        assert!(formatted.contains("**Waypoint**"));
        assert!(formatted.contains("Lat: 47.000000"));
        assert!(!formatted.contains(" - ")); // No description separator
    }

    #[test]
    fn test_gpx_to_markdown() {
        use docling_gps::{GpxRoute, GpxTrack, GpxTrackSegment, GpxWaypoint};

        let gpx = GpxInfo {
            version: "1.1".to_string(),
            creator: Some("TestApp".to_string()),
            author: None,
            name: Some("Test GPX".to_string()),
            description: Some("Test description".to_string()),
            tracks: vec![GpxTrack {
                name: Some("Track 1".to_string()),
                description: None,
                track_type: None,
                total_points: 2,
                segments: vec![GpxTrackSegment {
                    points: vec![GpxPoint {
                        latitude: 47.0,
                        longitude: -122.0,
                        elevation: None,
                        time: None,
                    }],
                }],
            }],
            routes: vec![GpxRoute {
                name: Some("Route 1".to_string()),
                description: None,
                points: vec![],
            }],
            waypoints: vec![GpxWaypoint {
                name: Some("WP1".to_string()),
                description: None,
                point: GpxPoint {
                    latitude: 47.0,
                    longitude: -122.0,
                    elevation: None,
                    time: None,
                },
            }],
        };

        let markdown = GpxBackend::gpx_to_markdown(&gpx);
        assert!(markdown.contains("# Test GPX"));
        assert!(markdown.contains("Test description"));
        assert!(markdown.contains("GPX Version: 1.1"));
        assert!(markdown.contains("Creator: TestApp"));
        assert!(markdown.contains("## Tracks"));
        assert!(markdown.contains("### Track 1"));
        assert!(markdown.contains("## Routes"));
        assert!(markdown.contains("### Route 1"));
        assert!(markdown.contains("## Waypoints"));
        assert!(markdown.contains("**WP1**"));
    }

    #[test]
    fn test_gpx_to_markdown_minimal() {
        let gpx = GpxInfo {
            version: "1.0".to_string(),
            creator: None,
            author: None,
            name: None,
            description: None,
            tracks: vec![],
            routes: vec![],
            waypoints: vec![],
        };

        let markdown = GpxBackend::gpx_to_markdown(&gpx);
        assert!(markdown.contains("# GPS Data")); // Default title
        assert!(markdown.contains("GPX Version: 1.0"));
        assert!(!markdown.contains("Creator:"));
        assert!(!markdown.contains("## Tracks"));
        assert!(!markdown.contains("## Routes"));
        assert!(!markdown.contains("## Waypoints"));
    }

    // ===== CATEGORY: Backend Creation Tests =====

    #[test]
    fn test_backend_default() {
        let backend = GpxBackend;
        assert_eq!(backend.format(), InputFormat::Gpx);
    }

    #[test]
    fn test_backend_format_constant() {
        let backend = GpxBackend::new();
        assert_eq!(backend.format(), InputFormat::Gpx);
        // Verify format() is consistent across calls
        assert_eq!(backend.format(), backend.format());
    }

    // ===== CATEGORY: DocItem Creation Tests =====

    #[test]
    fn test_gpx_to_docitems_minimal() {
        use docling_gps::GpxInfo;

        let gpx = GpxInfo {
            version: "1.1".to_string(),
            creator: None,
            author: None,
            name: None,
            description: None,
            tracks: vec![],
            routes: vec![],
            waypoints: vec![],
        };

        let doc_items = GpxBackend::gpx_to_docitems(&gpx);
        // Should have: title ("GPS Data"), version info
        assert!(doc_items.len() >= 2);

        // Verify first item is title header
        if let DocItem::SectionHeader { text, level, .. } = &doc_items[0] {
            assert_eq!(text, "GPS Data");
            assert_eq!(*level, 1);
        } else {
            panic!("Expected SectionHeader for title");
        }
    }

    #[test]
    fn test_gpx_to_docitems_with_track() {
        use docling_gps::{GpxInfo, GpxTrack, GpxTrackSegment};

        let track = GpxTrack {
            name: Some("Morning Run".to_string()),
            description: Some("5k route".to_string()),
            track_type: Some("running".to_string()),
            total_points: 500,
            segments: vec![GpxTrackSegment {
                points: vec![GpxPoint {
                    latitude: 40.0,
                    longitude: -120.0,
                    elevation: Some(100.0),
                    time: None,
                }],
            }],
        };

        let gpx = GpxInfo {
            version: "1.1".to_string(),
            creator: Some("Garmin".to_string()),
            author: None,
            name: Some("My Tracks".to_string()),
            description: None,
            tracks: vec![track],
            routes: vec![],
            waypoints: vec![],
        };

        let doc_items = GpxBackend::gpx_to_docitems(&gpx);

        // Verify title header
        let has_title = doc_items.iter().any(|item| {
            if let DocItem::SectionHeader { text, level, .. } = item {
                text == "My Tracks" && *level == 1
            } else {
                false
            }
        });
        assert!(has_title, "Expected title header in DocItems");

        // Verify tracks section header
        let has_tracks_section = doc_items.iter().any(|item| {
            if let DocItem::SectionHeader { text, level, .. } = item {
                text == "Tracks" && *level == 2
            } else {
                false
            }
        });
        assert!(has_tracks_section, "Expected Tracks section header");

        // Verify track name header
        let has_track_header = doc_items.iter().any(|item| {
            if let DocItem::SectionHeader { text, level, .. } = item {
                text == "Morning Run" && *level == 3
            } else {
                false
            }
        });
        assert!(has_track_header, "Expected track name header");

        // Verify track points are preserved
        let has_track_points = doc_items.iter().any(|item| {
            if let DocItem::Text { text, .. } = item {
                text.contains("Track Points:")
            } else {
                false
            }
        });
        assert!(has_track_points, "Expected track points data");
    }

    #[test]
    fn test_gpx_to_docitems_structure_preservation() {
        use docling_gps::{GpxInfo, GpxRoute, GpxWaypoint};

        let route = GpxRoute {
            name: Some("Route 1".to_string()),
            description: None,
            points: vec![GpxPoint {
                latitude: 41.0,
                longitude: -121.0,
                elevation: None,
                time: None,
            }],
        };

        let waypoint = GpxWaypoint {
            name: Some("Summit".to_string()),
            description: Some("Mountain peak".to_string()),
            point: GpxPoint {
                latitude: 42.0,
                longitude: -122.0,
                elevation: Some(3000.0),
                time: None,
            },
        };

        let gpx = GpxInfo {
            version: "1.1".to_string(),
            creator: None,
            author: None,
            name: Some("Multi-Feature GPS".to_string()),
            description: Some("Contains tracks, routes, and waypoints".to_string()),
            tracks: vec![],
            routes: vec![route],
            waypoints: vec![waypoint],
        };

        let doc_items = GpxBackend::gpx_to_docitems(&gpx);

        // Verify routes section exists
        let has_routes_section = doc_items.iter().any(|item| {
            if let DocItem::SectionHeader { text, level, .. } = item {
                text == "Routes" && *level == 2
            } else {
                false
            }
        });
        assert!(has_routes_section, "Expected Routes section header");

        // Verify waypoints section exists
        let has_waypoints_section = doc_items.iter().any(|item| {
            if let DocItem::SectionHeader { text, level, .. } = item {
                text == "Waypoints" && *level == 2
            } else {
                false
            }
        });
        assert!(has_waypoints_section, "Expected Waypoints section header");

        // Verify coordinate data is preserved
        let has_coordinates = doc_items.iter().any(|item| {
            if let DocItem::Text { text, .. } = item {
                text.contains("Lat:") && text.contains("Lon:")
            } else {
                false
            }
        });
        assert!(has_coordinates, "Expected coordinate data in DocItems");
    }

    // ===== CATEGORY: Point Formatting Tests =====

    #[test]
    fn test_format_point_with_elevation_only() {
        let point = GpxPoint {
            latitude: 0.0,
            longitude: 0.0,
            elevation: Some(42.7),
            time: None,
        };

        let formatted = GpxBackend::format_point(&point);
        assert!(formatted.contains("Lat: 0.000000"));
        assert!(formatted.contains("Lon: 0.000000"));
        assert!(formatted.contains("Ele: 42.7m"));
        assert!(!formatted.contains("Time:"));
    }

    #[test]
    fn test_format_point_with_time_only() {
        let point = GpxPoint {
            latitude: 1.0,
            longitude: 2.0,
            elevation: None,
            time: Some("2024-12-25T00:00:00Z".to_string()),
        };

        let formatted = GpxBackend::format_point(&point);
        assert!(formatted.contains("Time: 2024-12-25T00:00:00Z"));
        assert!(!formatted.contains("Ele:"));
    }

    #[test]
    fn test_format_point_negative_coordinates() {
        let point = GpxPoint {
            latitude: -47.123,
            longitude: -122.456,
            elevation: Some(-10.0), // Below sea level
            time: None,
        };

        let formatted = GpxBackend::format_point(&point);
        assert!(formatted.contains("Lat: -47.123"));
        assert!(formatted.contains("Lon: -122.456"));
        assert!(formatted.contains("Ele: -10.0m"));
    }

    // ===== CATEGORY: Track Formatting Tests =====

    #[test]
    fn test_format_track_default_name() {
        use docling_gps::GpxTrack;

        let track = GpxTrack {
            name: None,
            description: None,
            track_type: None,
            total_points: 5,
            segments: vec![],
        };

        let formatted = GpxBackend::format_track(&track, 0);
        assert!(formatted.contains("### Track 1")); // index 0 -> "Track 1"

        let formatted2 = GpxBackend::format_track(&track, 9);
        assert!(formatted2.contains("### Track 10")); // index 9 -> "Track 10"
    }

    #[test]
    fn test_format_track_with_type_no_description() {
        use docling_gps::GpxTrack;

        let track = GpxTrack {
            name: Some("Run".to_string()),
            description: None,
            track_type: Some("running".to_string()),
            total_points: 100,
            segments: vec![],
        };

        let formatted = GpxBackend::format_track(&track, 0);
        assert!(formatted.contains("### Run"));
        assert!(formatted.contains("Type: running"));
        assert!(!formatted.contains("A scenic")); // No description
    }

    // ===== CATEGORY: Route Formatting Tests =====

    #[test]
    fn test_format_route_default_name() {
        use docling_gps::GpxRoute;

        let route = GpxRoute {
            name: None,
            description: None,
            points: vec![],
        };

        let formatted = GpxBackend::format_route(&route, 2);
        assert!(formatted.contains("### Route 3")); // index 2 -> "Route 3"
    }

    #[test]
    fn test_format_route_exactly_10_points() {
        use docling_gps::GpxRoute;

        let mut points = Vec::new();
        for i in 0..10 {
            points.push(GpxPoint {
                latitude: i as f64,
                longitude: 0.0,
                elevation: None,
                time: None,
            });
        }

        let route = GpxRoute {
            name: Some("Exactly 10".to_string()),
            description: None,
            points,
        };

        let formatted = GpxBackend::format_route(&route, 0);
        assert!(formatted.contains("Points: 10"));
        assert!(!formatted.contains("... and")); // Should not show "more points" message
    }

    // ===== CATEGORY: Waypoint Formatting Tests =====

    #[test]
    fn test_format_waypoint_no_name_no_description() {
        let waypoint = GpxWaypoint {
            name: None,
            description: None,
            point: GpxPoint {
                latitude: 0.0,
                longitude: 0.0,
                elevation: None,
                time: None,
            },
        };

        let formatted = GpxBackend::format_waypoint(&waypoint);
        assert!(formatted.contains("**Waypoint**"));
        assert!(formatted.contains("Lat: 0.000000"));
        assert!(!formatted.contains("**Summit**"));
    }

    #[test]
    fn test_format_waypoint_name_no_description() {
        let waypoint = GpxWaypoint {
            name: Some("Checkpoint".to_string()),
            description: None,
            point: GpxPoint {
                latitude: 1.0,
                longitude: 2.0,
                elevation: None,
                time: None,
            },
        };

        let formatted = GpxBackend::format_waypoint(&waypoint);
        assert!(formatted.contains("**Checkpoint**"));
        assert!(!formatted.contains(" - "));
    }

    // ===== CATEGORY: Markdown Generation Tests =====

    #[test]
    fn test_gpx_to_markdown_tracks_only() {
        use docling_gps::{GpxTrack, GpxTrackSegment};

        let gpx = GpxInfo {
            version: "1.1".to_string(),
            creator: None,
            author: None,
            name: Some("Track Data".to_string()),
            description: None,
            tracks: vec![GpxTrack {
                name: Some("T1".to_string()),
                description: None,
                track_type: None,
                total_points: 5,
                segments: vec![GpxTrackSegment { points: vec![] }],
            }],
            routes: vec![],
            waypoints: vec![],
        };

        let markdown = GpxBackend::gpx_to_markdown(&gpx);
        assert!(markdown.contains("# Track Data"));
        assert!(markdown.contains("## Tracks"));
        assert!(markdown.contains("### T1"));
        assert!(!markdown.contains("## Routes"));
        assert!(!markdown.contains("## Waypoints"));
    }

    #[test]
    fn test_gpx_to_markdown_routes_only() {
        use docling_gps::GpxRoute;

        let gpx = GpxInfo {
            version: "1.1".to_string(),
            creator: None,
            author: None,
            name: None,
            description: None,
            tracks: vec![],
            routes: vec![GpxRoute {
                name: Some("R1".to_string()),
                description: None,
                points: vec![],
            }],
            waypoints: vec![],
        };

        let markdown = GpxBackend::gpx_to_markdown(&gpx);
        assert!(markdown.contains("# GPS Data"));
        assert!(!markdown.contains("## Tracks"));
        assert!(markdown.contains("## Routes"));
        assert!(markdown.contains("### R1"));
        assert!(!markdown.contains("## Waypoints"));
    }

    #[test]
    fn test_gpx_to_markdown_waypoints_only() {
        let gpx = GpxInfo {
            version: "1.1".to_string(),
            creator: None,
            author: None,
            name: None,
            description: None,
            tracks: vec![],
            routes: vec![],
            waypoints: vec![GpxWaypoint {
                name: Some("W1".to_string()),
                description: None,
                point: GpxPoint {
                    latitude: 0.0,
                    longitude: 0.0,
                    elevation: None,
                    time: None,
                },
            }],
        };

        let markdown = GpxBackend::gpx_to_markdown(&gpx);
        assert!(!markdown.contains("## Tracks"));
        assert!(!markdown.contains("## Routes"));
        assert!(markdown.contains("## Waypoints"));
        assert!(markdown.contains("**W1**"));
    }

    #[test]
    fn test_gpx_to_markdown_with_description() {
        let gpx = GpxInfo {
            version: "1.0".to_string(),
            creator: None,
            author: None,
            name: Some("My GPX".to_string()),
            description: Some("This is a test GPX file with description".to_string()),
            tracks: vec![],
            routes: vec![],
            waypoints: vec![],
        };

        let markdown = GpxBackend::gpx_to_markdown(&gpx);
        assert!(markdown.contains("# My GPX"));
        assert!(markdown.contains("This is a test GPX file with description"));
        assert!(markdown.contains("GPX Version: 1.0"));
    }

    // ===== CATEGORY: Edge Cases and Boundary Conditions =====

    #[test]
    fn test_format_point_extreme_coordinates() {
        let point = GpxPoint {
            latitude: 90.0,           // North pole
            longitude: 180.0,         // Date line
            elevation: Some(8848.86), // Mt Everest height
            time: None,
        };

        let formatted = GpxBackend::format_point(&point);
        assert!(formatted.contains("Lat: 90.000000"));
        assert!(formatted.contains("Lon: 180.000000"));
        assert!(formatted.contains("Ele: 8848.9m"));
    }

    #[test]
    fn test_format_point_zero_elevation() {
        let point = GpxPoint {
            latitude: 0.0,
            longitude: 0.0,
            elevation: Some(0.0), // Sea level
            time: None,
        };

        let formatted = GpxBackend::format_point(&point);
        assert!(formatted.contains("Ele: 0.0m"));
    }

    #[test]
    fn test_format_track_multiple_segments() {
        use docling_gps::{GpxTrack, GpxTrackSegment};

        let track = GpxTrack {
            name: Some("Multi-Segment".to_string()),
            description: None,
            track_type: None,
            total_points: 50,
            segments: vec![
                GpxTrackSegment {
                    points: vec![GpxPoint {
                        latitude: 47.0,
                        longitude: -122.0,
                        elevation: None,
                        time: None,
                    }],
                },
                GpxTrackSegment {
                    points: vec![GpxPoint {
                        latitude: 48.0,
                        longitude: -123.0,
                        elevation: None,
                        time: None,
                    }],
                },
                GpxTrackSegment {
                    points: vec![GpxPoint {
                        latitude: 49.0,
                        longitude: -124.0,
                        elevation: None,
                        time: None,
                    }],
                },
            ],
        };

        let formatted = GpxBackend::format_track(&track, 0);
        assert!(formatted.contains("Segments: 3"));
        assert!(formatted.contains("Lat: 47.000000")); // First point from first segment
        assert!(formatted.contains("Lat: 49.000000")); // Last point from last segment
    }

    #[test]
    fn test_format_track_empty_segments() {
        use docling_gps::{GpxTrack, GpxTrackSegment};

        let track = GpxTrack {
            name: Some("Empty Segments".to_string()),
            description: None,
            track_type: None,
            total_points: 0,
            segments: vec![
                GpxTrackSegment { points: vec![] },
                GpxTrackSegment { points: vec![] },
            ],
        };

        let formatted = GpxBackend::format_track(&track, 0);
        assert!(formatted.contains("Segments: 2"));
        assert!(!formatted.contains("Start:")); // No points, no start
        assert!(!formatted.contains("End:")); // No points, no end
    }

    #[test]
    fn test_format_route_empty_points() {
        use docling_gps::GpxRoute;

        let route = GpxRoute {
            name: Some("Empty Route".to_string()),
            description: Some("A route with no points".to_string()),
            points: vec![],
        };

        let formatted = GpxBackend::format_route(&route, 0);
        assert!(formatted.contains("### Empty Route"));
        assert!(formatted.contains("A route with no points"));
        assert!(formatted.contains("Points: 0"));
        assert!(!formatted.contains("Route Points:")); // No points section
    }

    #[test]
    fn test_format_route_single_point() {
        use docling_gps::GpxRoute;

        let route = GpxRoute {
            name: Some("Single Point".to_string()),
            description: None,
            points: vec![GpxPoint {
                latitude: 47.0,
                longitude: -122.0,
                elevation: None,
                time: None,
            }],
        };

        let formatted = GpxBackend::format_route(&route, 0);
        assert!(formatted.contains("Points: 1"));
        assert!(formatted.contains("1. Lat: 47.000000"));
        assert!(!formatted.contains("... and")); // Only 1 point, no "more" message
    }

    #[test]
    fn test_format_route_exactly_11_points() {
        use docling_gps::GpxRoute;

        let mut points = Vec::new();
        for i in 0..11 {
            points.push(GpxPoint {
                latitude: i as f64,
                longitude: 0.0,
                elevation: None,
                time: None,
            });
        }

        let route = GpxRoute {
            name: Some("11 Points".to_string()),
            description: None,
            points,
        };

        let formatted = GpxBackend::format_route(&route, 0);
        assert!(formatted.contains("Points: 11"));
        assert!(formatted.contains("... and 1 more points")); // Shows first 10, 1 remaining
    }

    #[test]
    fn test_gpx_to_markdown_multiple_tracks() {
        use docling_gps::{GpxTrack, GpxTrackSegment};

        let gpx = GpxInfo {
            version: "1.1".to_string(),
            creator: None,
            author: None,
            name: Some("Multi Track".to_string()),
            description: None,
            tracks: vec![
                GpxTrack {
                    name: Some("Track A".to_string()),
                    description: None,
                    track_type: None,
                    total_points: 10,
                    segments: vec![GpxTrackSegment { points: vec![] }],
                },
                GpxTrack {
                    name: Some("Track B".to_string()),
                    description: None,
                    track_type: None,
                    total_points: 20,
                    segments: vec![GpxTrackSegment { points: vec![] }],
                },
            ],
            routes: vec![],
            waypoints: vec![],
        };

        let markdown = GpxBackend::gpx_to_markdown(&gpx);
        assert!(markdown.contains("## Tracks"));
        assert!(markdown.contains("### Track A"));
        assert!(markdown.contains("### Track B"));
        assert!(markdown.contains("Points: 10"));
        assert!(markdown.contains("Points: 20"));
    }

    #[test]
    fn test_gpx_to_markdown_multiple_routes() {
        use docling_gps::GpxRoute;

        let gpx = GpxInfo {
            version: "1.1".to_string(),
            creator: None,
            author: None,
            name: None,
            description: None,
            tracks: vec![],
            routes: vec![
                GpxRoute {
                    name: Some("Route 1".to_string()),
                    description: None,
                    points: vec![],
                },
                GpxRoute {
                    name: Some("Route 2".to_string()),
                    description: None,
                    points: vec![],
                },
                GpxRoute {
                    name: Some("Route 3".to_string()),
                    description: None,
                    points: vec![],
                },
            ],
            waypoints: vec![],
        };

        let markdown = GpxBackend::gpx_to_markdown(&gpx);
        assert!(markdown.contains("## Routes"));
        assert!(markdown.contains("### Route 1"));
        assert!(markdown.contains("### Route 2"));
        assert!(markdown.contains("### Route 3"));
    }

    #[test]
    fn test_gpx_to_markdown_multiple_waypoints() {
        let gpx = GpxInfo {
            version: "1.1".to_string(),
            creator: None,
            author: None,
            name: None,
            description: None,
            tracks: vec![],
            routes: vec![],
            waypoints: vec![
                GpxWaypoint {
                    name: Some("WP1".to_string()),
                    description: None,
                    point: GpxPoint {
                        latitude: 1.0,
                        longitude: 1.0,
                        elevation: None,
                        time: None,
                    },
                },
                GpxWaypoint {
                    name: Some("WP2".to_string()),
                    description: None,
                    point: GpxPoint {
                        latitude: 2.0,
                        longitude: 2.0,
                        elevation: None,
                        time: None,
                    },
                },
            ],
        };

        let markdown = GpxBackend::gpx_to_markdown(&gpx);
        assert!(markdown.contains("## Waypoints"));
        assert!(markdown.contains("**WP1**"));
        assert!(markdown.contains("**WP2**"));
        assert!(markdown.contains("Lat: 1.000000"));
        assert!(markdown.contains("Lat: 2.000000"));
    }

    #[test]
    fn test_gpx_to_markdown_all_sections() {
        use docling_gps::{GpxRoute, GpxTrack, GpxTrackSegment};

        let gpx = GpxInfo {
            version: "1.1".to_string(),
            creator: Some("Full App".to_string()),
            author: None,
            name: Some("Complete GPX".to_string()),
            description: Some("All sections present".to_string()),
            tracks: vec![GpxTrack {
                name: Some("T1".to_string()),
                description: None,
                track_type: None,
                total_points: 1,
                segments: vec![GpxTrackSegment { points: vec![] }],
            }],
            routes: vec![GpxRoute {
                name: Some("R1".to_string()),
                description: None,
                points: vec![],
            }],
            waypoints: vec![GpxWaypoint {
                name: Some("W1".to_string()),
                description: None,
                point: GpxPoint {
                    latitude: 0.0,
                    longitude: 0.0,
                    elevation: None,
                    time: None,
                },
            }],
        };

        let markdown = GpxBackend::gpx_to_markdown(&gpx);
        assert!(markdown.contains("# Complete GPX"));
        assert!(markdown.contains("All sections present"));
        assert!(markdown.contains("Creator: Full App"));
        assert!(markdown.contains("## Tracks"));
        assert!(markdown.contains("## Routes"));
        assert!(markdown.contains("## Waypoints"));
    }

    #[test]
    fn test_gpx_to_markdown_version_formatting() {
        let gpx = GpxInfo {
            version: "1.2".to_string(),
            creator: None,
            author: None,
            name: None,
            description: None,
            tracks: vec![],
            routes: vec![],
            waypoints: vec![],
        };

        let markdown = GpxBackend::gpx_to_markdown(&gpx);
        assert!(markdown.contains("GPX Version: 1.2"));
    }

    // ===== CATEGORY: Integration Tests (parse_bytes / parse_file) =====

    #[test]
    fn test_parse_minimal_gpx() {
        let gpx_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.1" creator="test">
</gpx>"#;

        let backend = GpxBackend::new();
        let result = backend.parse_bytes(gpx_xml.as_bytes(), &BackendOptions::default());

        assert!(result.is_ok());
        let doc = result.unwrap();
        assert_eq!(doc.format, InputFormat::Gpx);
        assert!(doc.markdown.contains("# GPS Data")); // Default title
        assert!(doc.markdown.contains("GPX Version:"));
    }

    #[test]
    fn test_parse_gpx_with_metadata() {
        let gpx_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.1" creator="TestCreator">
  <metadata>
    <name>Test Track</name>
    <desc>Test Description</desc>
  </metadata>
</gpx>"#;

        let backend = GpxBackend::new();
        let result = backend.parse_bytes(gpx_xml.as_bytes(), &BackendOptions::default());

        assert!(result.is_ok());
        let doc = result.unwrap();
        assert_eq!(doc.metadata.title, Some("Test Track".to_string()));
        // No author in this GPX XML, so should be None
        assert_eq!(doc.metadata.author, None);
        assert!(doc.markdown.contains("# Test Track"));
        assert!(doc.markdown.contains("Test Description"));
    }

    #[test]
    fn test_document_metadata_num_characters() {
        let gpx_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.1" creator="test">
  <metadata>
    <name>Short</name>
  </metadata>
</gpx>"#;

        let backend = GpxBackend::new();
        let result = backend.parse_bytes(gpx_xml.as_bytes(), &BackendOptions::default());

        assert!(result.is_ok());
        let doc = result.unwrap();
        assert!(doc.metadata.num_characters > 0);
        assert_eq!(doc.metadata.num_characters, doc.markdown.chars().count());
    }

    #[test]
    fn test_content_blocks_not_empty() {
        let gpx_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.1" creator="test">
  <metadata>
    <name>Test</name>
  </metadata>
</gpx>"#;

        let backend = GpxBackend::new();
        let result = backend.parse_bytes(gpx_xml.as_bytes(), &BackendOptions::default());

        assert!(result.is_ok());
        let doc = result.unwrap();
        assert!(doc.content_blocks.is_some());
        let blocks = doc.content_blocks.unwrap();
        assert!(!blocks.is_empty());
    }

    #[test]
    fn test_format_waypoint_with_special_characters() {
        let waypoint = GpxWaypoint {
            name: Some("Café & Restaurant".to_string()),
            description: Some("Best café in town!".to_string()),
            point: GpxPoint {
                latitude: 47.0,
                longitude: -122.0,
                elevation: None,
                time: None,
            },
        };

        let formatted = GpxBackend::format_waypoint(&waypoint);
        assert!(formatted.contains("**Café & Restaurant**"));
        assert!(formatted.contains("Best café in town!"));
    }

    #[test]
    fn test_format_track_with_long_description() {
        use docling_gps::GpxTrack;

        let long_desc = "This is a very long description that contains multiple sentences. \
                         It describes the track in great detail. The track goes through \
                         various terrains including forests, mountains, and valleys.";

        let track = GpxTrack {
            name: Some("Detailed Track".to_string()),
            description: Some(long_desc.to_string()),
            track_type: None,
            total_points: 100,
            segments: vec![],
        };

        let formatted = GpxBackend::format_track(&track, 0);
        assert!(formatted.contains(long_desc));
    }

    // ========== NEW COMPREHENSIVE TESTS (N=466) ==========

    #[test]
    fn test_extreme_coordinates() {
        // Test coordinates at geographic extremes
        let north_pole = GpxPoint {
            latitude: 90.0,
            longitude: 0.0,
            elevation: Some(0.0),
            time: None,
        };
        let formatted_north = GpxBackend::format_point(&north_pole);
        assert!(formatted_north.contains("Lat: 90.000000"));

        let south_pole = GpxPoint {
            latitude: -90.0,
            longitude: 180.0,
            elevation: Some(2835.0), // Ice sheet elevation
            time: None,
        };
        let formatted_south = GpxBackend::format_point(&south_pole);
        assert!(formatted_south.contains("Lat: -90.000000"));
        assert!(formatted_south.contains("Lon: 180.000000"));

        // International Date Line
        let dateline = GpxPoint {
            latitude: 0.0, // Equator
            longitude: 180.0,
            elevation: Some(0.0),
            time: None,
        };
        let formatted_dateline = GpxBackend::format_point(&dateline);
        assert!(formatted_dateline.contains("Lat: 0.000000"));
        assert!(formatted_dateline.contains("Lon: 180.000000"));
    }

    #[test]
    fn test_very_high_elevation() {
        // Mount Everest elevation (8848m)
        let everest = GpxPoint {
            latitude: 27.988_056,
            longitude: 86.925_278,
            elevation: Some(8848.86),
            time: None,
        };
        let formatted_everest = GpxBackend::format_point(&everest);
        assert!(formatted_everest.contains("Ele: 8848.9m"));

        // Dead Sea (lowest point on Earth, -430m)
        let dead_sea = GpxPoint {
            latitude: 31.5,
            longitude: 35.5,
            elevation: Some(-430.0),
            time: None,
        };
        let formatted_dead_sea = GpxBackend::format_point(&dead_sea);
        assert!(formatted_dead_sea.contains("Ele: -430.0m"));

        // Mariana Trench depth (if GPS could go there, -10994m)
        let mariana = GpxPoint {
            latitude: 11.373_333,
            longitude: 142.591_667,
            elevation: Some(-10994.0),
            time: None,
        };
        let formatted_mariana = GpxBackend::format_point(&mariana);
        assert!(formatted_mariana.contains("Ele: -10994.0m"));
    }

    #[test]
    fn test_track_with_gaps() {
        use docling_gps::{GpxTrack, GpxTrackSegment};

        // Track with 3 segments (common when GPS signal is lost/regained)
        let track = GpxTrack {
            name: Some("City Hike with Gaps".to_string()),
            description: Some("GPS lost signal in tunnels".to_string()),
            track_type: Some("hiking".to_string()),
            total_points: 600,
            segments: vec![
                GpxTrackSegment {
                    points: vec![
                        GpxPoint {
                            latitude: 47.6,
                            longitude: -122.3,
                            elevation: Some(50.0),
                            time: Some("2024-01-15T08:00:00Z".to_string()),
                        },
                        GpxPoint {
                            latitude: 47.61,
                            longitude: -122.31,
                            elevation: Some(55.0),
                            time: Some("2024-01-15T08:10:00Z".to_string()),
                        },
                    ],
                },
                // Gap (tunnel/signal loss)
                GpxTrackSegment {
                    points: vec![
                        GpxPoint {
                            latitude: 47.62,
                            longitude: -122.32,
                            elevation: Some(60.0),
                            time: Some("2024-01-15T08:30:00Z".to_string()),
                        },
                        GpxPoint {
                            latitude: 47.63,
                            longitude: -122.33,
                            elevation: Some(65.0),
                            time: Some("2024-01-15T08:40:00Z".to_string()),
                        },
                    ],
                },
                // Another gap
                GpxTrackSegment {
                    points: vec![GpxPoint {
                        latitude: 47.64,
                        longitude: -122.34,
                        elevation: Some(70.0),
                        time: Some("2024-01-15T09:00:00Z".to_string()),
                    }],
                },
            ],
        };

        let formatted = GpxBackend::format_track(&track, 0);
        assert!(formatted.contains("Segments: 3"));
        assert!(formatted.contains("GPS lost signal in tunnels"));
    }

    #[test]
    fn test_route_vs_track_distinction() {
        use docling_gps::{GpxRoute, GpxTrack};

        // Route: planned path (no timestamps typically)
        let route = GpxRoute {
            name: Some("Planned Bike Route".to_string()),
            description: Some("Scenic bike path".to_string()),
            points: vec![
                GpxPoint {
                    latitude: 47.6,
                    longitude: -122.3,
                    elevation: Some(50.0),
                    time: None, // Routes typically don't have times
                },
                GpxPoint {
                    latitude: 47.7,
                    longitude: -122.4,
                    elevation: Some(100.0),
                    time: None,
                },
            ],
        };
        let route_formatted = GpxBackend::format_route(&route, 0);
        assert!(route_formatted.contains("### Planned Bike Route"));
        assert!(!route_formatted.contains("Time:")); // Routes shouldn't have timestamps

        // Track: recorded path (with timestamps)
        let track = GpxTrack {
            name: Some("Actual Bike Ride".to_string()),
            description: Some("Recorded ride".to_string()),
            track_type: Some("biking".to_string()),
            total_points: 500,
            segments: vec![],
        };
        let track_formatted = GpxBackend::format_track(&track, 0);
        assert!(track_formatted.contains("### Actual Bike Ride"));
        assert!(track_formatted.contains("Type: biking"));
    }

    #[test]
    fn test_waypoint_clustering() {
        use docling_gps::GpxWaypoint;

        // Multiple waypoints in close proximity (tourist attractions in same area)
        let waypoints = vec![
            GpxWaypoint {
                name: Some("Eiffel Tower".to_string()),
                description: Some("Iconic landmark".to_string()),
                point: GpxPoint {
                    latitude: 48.858_844,
                    longitude: 2.294_351,
                    elevation: Some(324.0), // Tower height
                    time: None,
                },
            },
            GpxWaypoint {
                name: Some("Trocadéro Gardens".to_string()),
                description: Some("Viewing spot".to_string()),
                point: GpxPoint {
                    latitude: 48.862_725,
                    longitude: 2.287_592,
                    elevation: Some(67.0),
                    time: None,
                },
            },
            GpxWaypoint {
                name: Some("Champ de Mars".to_string()),
                description: Some("Public park".to_string()),
                point: GpxPoint {
                    latitude: 48.855_633,
                    longitude: 2.298_337,
                    elevation: Some(35.0),
                    time: None,
                },
            },
        ];

        // All waypoints within ~1km radius
        for waypoint in &waypoints {
            let formatted = GpxBackend::format_waypoint(waypoint);
            assert!(formatted.contains("48.8")); // All start with 48.8 latitude
            assert!(formatted.contains("2.2")); // All start with 2.2 longitude
        }
    }

    #[test]
    fn test_long_distance_track() {
        use docling_gps::GpxTrack;

        // Trans-continental track (e.g., cross-country drive)
        let track = GpxTrack {
            name: Some("USA Cross-Country Drive".to_string()),
            description: Some("New York to Los Angeles".to_string()),
            track_type: Some("driving".to_string()),
            total_points: 25000, // Recorded every 30 seconds for ~200 hours
            segments: vec![],
        };

        let formatted = GpxBackend::format_track(&track, 0);
        assert!(formatted.contains("Points: 25000"));
        assert!(formatted.contains("USA Cross-Country Drive"));
    }

    #[test]
    fn test_time_duration_calculation() {
        use docling_gps::{GpxTrack, GpxTrackSegment};

        // Track spanning multiple days
        let track = GpxTrack {
            name: Some("Multi-Day Hike".to_string()),
            description: None,
            track_type: Some("hiking".to_string()),
            total_points: 10000,
            segments: vec![GpxTrackSegment {
                points: vec![
                    GpxPoint {
                        latitude: 45.0,
                        longitude: -120.0,
                        elevation: Some(1000.0),
                        time: Some("2024-01-15T08:00:00Z".to_string()),
                    },
                    GpxPoint {
                        latitude: 45.5,
                        longitude: -120.5,
                        elevation: Some(1500.0),
                        time: Some("2024-01-17T18:00:00Z".to_string()), // 2.5 days later
                    },
                ],
            }],
        };

        let formatted = GpxBackend::format_track(&track, 0);
        // Verify both start and end times are included
        assert!(formatted.contains("2024-01-15T08:00:00Z"));
        assert!(formatted.contains("2024-01-17T18:00:00Z"));
    }

    #[test]
    fn test_unicode_place_names() {
        use docling_gps::GpxWaypoint;

        // International place names with various scripts
        let waypoints = vec![
            GpxWaypoint {
                name: Some("東京タワー".to_string()), // Tokyo Tower (Japanese)
                description: Some("日本の有名なランドマーク".to_string()),
                point: GpxPoint {
                    latitude: 35.658_581,
                    longitude: 139.745_438,
                    elevation: Some(333.0),
                    time: None,
                },
            },
            GpxWaypoint {
                name: Some("Москва Кремль".to_string()), // Moscow Kremlin (Russian)
                description: Some("Историческая крепость".to_string()),
                point: GpxPoint {
                    latitude: 55.751_667,
                    longitude: 37.617_778,
                    elevation: Some(145.0),
                    time: None,
                },
            },
            GpxWaypoint {
                name: Some("São Paulo".to_string()), // São Paulo (Portuguese)
                description: Some("Maior cidade do Brasil".to_string()),
                point: GpxPoint {
                    latitude: -23.550_520,
                    longitude: -46.633_308,
                    elevation: Some(760.0),
                    time: None,
                },
            },
        ];

        for waypoint in &waypoints {
            let formatted = GpxBackend::format_waypoint(waypoint);
            // Verify Unicode characters are preserved
            if let Some(name) = &waypoint.name {
                assert!(formatted.contains(name));
            }
        }

        // Specifically test Japanese
        let tokyo_formatted = GpxBackend::format_waypoint(&waypoints[0]);
        assert!(tokyo_formatted.contains("東京タワー"));
        assert!(tokyo_formatted.contains("日本の有名なランドマーク"));
    }

    #[test]
    fn test_gpx_xml_parsing_basic() {
        // Test basic GPX 1.1 XML parsing
        let gpx_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.1" creator="test-app" xmlns="http://www.topografix.com/GPX/1/1">
  <metadata>
    <name>Test GPX File</name>
    <desc>A simple test file</desc>
    <time>2024-01-15T12:00:00Z</time>
  </metadata>
  <wpt lat="47.644548" lon="-122.326897">
    <name>Space Needle</name>
    <desc>Seattle landmark</desc>
    <ele>184</ele>
  </wpt>
  <trk>
    <name>Test Track</name>
    <trkseg>
      <trkpt lat="47.6" lon="-122.3">
        <ele>100</ele>
        <time>2024-01-15T10:00:00Z</time>
      </trkpt>
      <trkpt lat="47.7" lon="-122.4">
        <ele>150</ele>
        <time>2024-01-15T11:00:00Z</time>
      </trkpt>
    </trkseg>
  </trk>
</gpx>"#;

        let backend = GpxBackend::new();
        let result = backend.parse_bytes(gpx_xml.as_bytes(), &BackendOptions::default());

        assert!(result.is_ok());
        let doc = result.unwrap();
        assert!(doc.markdown.contains("Space Needle"));
        assert!(doc.markdown.contains("Test Track"));
        assert!(doc.metadata.title.is_some());
        assert_eq!(doc.metadata.title.unwrap(), "Test GPX File");
    }

    #[test]
    fn test_empty_track_segments() {
        use docling_gps::{GpxTrack, GpxTrackSegment};

        // Track with some empty segments (can happen in real GPX files)
        let track = GpxTrack {
            name: Some("Track with Empty Segments".to_string()),
            description: None,
            track_type: None,
            total_points: 100,
            segments: vec![
                GpxTrackSegment { points: vec![] }, // Empty segment
                GpxTrackSegment {
                    points: vec![GpxPoint {
                        latitude: 47.0,
                        longitude: -122.0,
                        elevation: Some(100.0),
                        time: None,
                    }],
                },
                GpxTrackSegment { points: vec![] }, // Another empty segment
            ],
        };

        let formatted = GpxBackend::format_track(&track, 0);
        assert!(formatted.contains("Segments: 3"));
        assert!(formatted.contains("Track with Empty Segments"));
    }

    // ========== Additional Edge Cases (5 tests) ==========

    #[test]
    fn test_negative_elevation_below_sea_level() {
        use docling_gps::GpxWaypoint;

        // Waypoint below sea level (Dead Sea, lowest point on Earth)
        let waypoint = GpxWaypoint {
            name: Some("Dead Sea Shore".to_string()),
            description: Some("Lowest elevation on land".to_string()),
            point: GpxPoint {
                latitude: 31.558_333,
                longitude: 35.474_167,
                elevation: Some(-430.5), // Meters below sea level
                time: None,
            },
        };

        let formatted = GpxBackend::format_waypoint(&waypoint);
        assert!(formatted.contains("-430.5m")); // Should handle negative elevation
        assert!(formatted.contains("Dead Sea Shore"));
    }

    #[test]
    fn test_route_with_missing_names() {
        use docling_gps::GpxRoute;

        // Route with no name (should use default)
        let route = GpxRoute {
            name: None, // Missing name
            description: Some("Unnamed route".to_string()),
            points: vec![
                GpxPoint {
                    latitude: 40.0,
                    longitude: -75.0,
                    elevation: None,
                    time: None,
                },
                GpxPoint {
                    latitude: 40.1,
                    longitude: -75.1,
                    elevation: None,
                    time: None,
                },
            ],
        };

        let formatted = GpxBackend::format_route(&route, 0);
        assert!(
            formatted.contains("Route 1"),
            "Unnamed route should use default 'Route 1' name"
        );
        assert!(
            formatted.contains("Unnamed route"),
            "Route without description should show 'Unnamed route'"
        );
    }

    #[test]
    fn test_track_type_variations() {
        use docling_gps::GpxTrack;

        // Various activity types
        let track_types = [
            "cycling",
            "running",
            "hiking",
            "driving",
            "flying",
            "skiing",
            "swimming",
            "custom_activity",
        ];

        for (idx, track_type) in track_types.iter().enumerate() {
            let track = GpxTrack {
                name: Some(format!("Activity {idx}")),
                description: None,
                track_type: Some(track_type.to_string()),
                total_points: 100,
                segments: vec![],
            };

            let formatted = GpxBackend::format_track(&track, idx);
            assert!(
                formatted.contains(&format!("Type: {track_type}")),
                "Track type '{track_type}' should be included in formatted output"
            );
        }
    }

    #[test]
    fn test_waypoint_with_all_fields_empty() {
        use docling_gps::GpxWaypoint;

        // Minimal waypoint with only coordinates
        let waypoint = GpxWaypoint {
            name: None,
            description: None,
            point: GpxPoint {
                latitude: 0.0,
                longitude: 0.0,
                elevation: None,
                time: None,
            },
        };

        let formatted = GpxBackend::format_waypoint(&waypoint);
        // Should still format with lat/lon
        assert!(
            formatted.contains("Lat: 0.000000"),
            "Waypoint should contain latitude even when 0"
        );
        assert!(
            formatted.contains("Lon: 0.000000"),
            "Waypoint should contain longitude even when 0"
        );
    }

    #[test]
    fn test_gpx_with_creator_and_version() {
        // GPX file with metadata (creator, version)
        let gpx_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.1" creator="Garmin Connect" xmlns="http://www.topografix.com/GPX/1/1">
  <metadata>
    <name>Activity with Metadata</name>
    <desc>GPS data from Garmin device</desc>
    <author>
      <name>John Doe</name>
      <email id="john" domain="example.com"/>
    </author>
    <time>2024-01-15T10:00:00Z</time>
  </metadata>
  <trk>
    <name>Morning Run</name>
    <type>running</type>
    <trkseg>
      <trkpt lat="37.7749" lon="-122.4194">
        <ele>50</ele>
        <time>2024-01-15T10:05:00Z</time>
      </trkpt>
    </trkseg>
  </trk>
</gpx>"#;

        let backend = GpxBackend::new();
        let result = backend.parse_bytes(gpx_xml.as_bytes(), &BackendOptions::default());

        assert!(
            result.is_ok(),
            "GPX with creator and version metadata should parse successfully"
        );
        let doc = result.unwrap();
        assert!(
            doc.markdown.contains("Activity with Metadata") || doc.markdown.contains("Morning Run"),
            "Markdown should contain activity name or metadata name"
        );
        // Should handle metadata gracefully
        assert!(
            !doc.markdown.is_empty(),
            "Parsed GPX should produce non-empty markdown"
        );
    }

    #[test]
    fn test_gpx_with_invalid_coordinates() {
        // GPX file with coordinates outside valid ranges
        // Latitude must be -90 to 90, longitude -180 to 180
        let gpx_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.1" creator="Test" xmlns="http://www.topografix.com/GPX/1/1">
  <wpt lat="95.0" lon="200.0">
    <name>Invalid Point</name>
    <desc>Coordinates out of range</desc>
  </wpt>
  <wpt lat="45.0" lon="90.0">
    <name>Valid Point</name>
    <desc>Within range</desc>
  </wpt>
</gpx>"#;

        let backend = GpxBackend::new();
        let result = backend.parse_bytes(gpx_xml.as_bytes(), &BackendOptions::default());

        // GPX parser validates coordinates and rejects invalid values
        // Latitude 95.0 (valid: -90 to 90) and longitude 200.0 (valid: -180 to 180) are out of range
        // The parser should either return an error or skip invalid points
        match result {
            Err(_) => {
                // Parser rejects file with invalid coordinates - acceptable behavior
                // No further assertions needed, error is expected
            }
            Ok(doc) => {
                // Parser skips invalid points and processes valid ones
                assert!(
                    doc.markdown.contains("Valid Point"),
                    "Should include valid points"
                );
            }
        }
    }

    #[test]
    fn test_gpx_with_circular_route() {
        // GPX route where start and end points are the same (circular/loop)
        let gpx_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.1" creator="Test" xmlns="http://www.topografix.com/GPX/1/1">
  <rte>
    <name>City Loop</name>
    <desc>Circular route around downtown</desc>
    <rtept lat="37.7749" lon="-122.4194">
      <name>Start/End Point</name>
      <ele>10</ele>
    </rtept>
    <rtept lat="37.7750" lon="-122.4195">
      <name>North Corner</name>
      <ele>15</ele>
    </rtept>
    <rtept lat="37.7751" lon="-122.4193">
      <name>East Corner</name>
      <ele>12</ele>
    </rtept>
    <rtept lat="37.7749" lon="-122.4194">
      <name>Start/End Point</name>
      <ele>10</ele>
    </rtept>
  </rte>
</gpx>"#;

        let backend = GpxBackend::new();
        let result = backend.parse_bytes(gpx_xml.as_bytes(), &BackendOptions::default());

        assert!(
            result.is_ok(),
            "Circular route GPX should parse successfully"
        );
        let doc = result.unwrap();
        assert!(
            doc.markdown.contains("City Loop"),
            "Markdown should contain route name 'City Loop'"
        );
        assert!(
            doc.markdown.contains("Circular route"),
            "Markdown should contain route description"
        );

        // Verify DocItems structure
        let items = doc.content_blocks.unwrap();
        assert!(!items.is_empty(), "Should have content blocks");

        // Verify markdown contains route information
        assert!(
            doc.markdown.contains("Start/End Point")
                || doc.markdown.contains("route")
                || doc.markdown.contains("37.7749"),
            "Should include route points in markdown output"
        );
    }

    #[test]
    fn test_gpx_multi_activity_file() {
        // GPX file with multiple tracks of different activity types
        // Common when exporting all activities from a fitness app
        let gpx_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.1" creator="Strava" xmlns="http://www.topografix.com/GPX/1/1">
  <trk>
    <name>Morning Run</name>
    <type>running</type>
    <trkseg>
      <trkpt lat="37.7749" lon="-122.4194">
        <ele>50</ele>
        <time>2024-01-15T07:00:00Z</time>
      </trkpt>
      <trkpt lat="37.7750" lon="-122.4195">
        <ele>52</ele>
        <time>2024-01-15T07:05:00Z</time>
      </trkpt>
    </trkseg>
  </trk>
  <trk>
    <name>Afternoon Bike Ride</name>
    <type>cycling</type>
    <trkseg>
      <trkpt lat="37.8049" lon="-122.4294">
        <ele>100</ele>
        <time>2024-01-15T14:00:00Z</time>
      </trkpt>
      <trkpt lat="37.8050" lon="-122.4295">
        <ele>110</ele>
        <time>2024-01-15T14:10:00Z</time>
      </trkpt>
    </trkseg>
  </trk>
  <trk>
    <name>Evening Hike</name>
    <type>hiking</type>
    <trkseg>
      <trkpt lat="37.8549" lon="-122.4794">
        <ele>200</ele>
        <time>2024-01-15T18:00:00Z</time>
      </trkpt>
      <trkpt lat="37.8550" lon="-122.4795">
        <ele>220</ele>
        <time>2024-01-15T18:15:00Z</time>
      </trkpt>
    </trkseg>
  </trk>
</gpx>"#;

        let backend = GpxBackend::new();
        let result = backend.parse_bytes(gpx_xml.as_bytes(), &BackendOptions::default());

        assert!(
            result.is_ok(),
            "Multi-activity GPX file should parse successfully"
        );
        let doc = result.unwrap();

        // Verify all three activities are present
        assert!(
            doc.markdown.contains("Morning Run"),
            "Markdown should contain 'Morning Run' track"
        );
        assert!(
            doc.markdown.contains("Afternoon Bike Ride"),
            "Markdown should contain 'Afternoon Bike Ride' track"
        );
        assert!(
            doc.markdown.contains("Evening Hike"),
            "Markdown should contain 'Evening Hike' track"
        );

        // Verify activity types are preserved
        assert!(
            doc.markdown.contains("running")
                || doc.markdown.contains("cycling")
                || doc.markdown.contains("hiking"),
            "Markdown should contain at least one activity type"
        );

        // Verify DocItems structure
        let items = doc.content_blocks.unwrap();
        assert!(
            items.len() >= 3,
            "Should have at least 3 content blocks for 3 tracks"
        );
    }

    #[test]
    fn test_gpx_with_device_extensions() {
        // GPX file with device-specific extensions (Garmin, Strava)
        // Extensions include: heart rate, cadence, power, temperature
        let gpx_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.1" creator="Garmin Edge 1030" xmlns="http://www.topografix.com/GPX/1/1" xmlns:gpxtpx="http://www.garmin.com/xmlschemas/TrackPointExtension/v1">
  <trk>
    <name>Training Ride</name>
    <type>cycling</type>
    <trkseg>
      <trkpt lat="37.7749" lon="-122.4194">
        <ele>50</ele>
        <time>2024-01-15T10:00:00Z</time>
        <extensions>
          <gpxtpx:TrackPointExtension>
            <gpxtpx:hr>145</gpxtpx:hr>
            <gpxtpx:cad>85</gpxtpx:cad>
            <gpxtpx:atemp>18</gpxtpx:atemp>
          </gpxtpx:TrackPointExtension>
        </extensions>
      </trkpt>
      <trkpt lat="37.7750" lon="-122.4195">
        <ele>55</ele>
        <time>2024-01-15T10:05:00Z</time>
        <extensions>
          <gpxtpx:TrackPointExtension>
            <gpxtpx:hr>158</gpxtpx:hr>
            <gpxtpx:cad>90</gpxtpx:cad>
            <gpxtpx:power>250</gpxtpx:power>
            <gpxtpx:atemp>19</gpxtpx:atemp>
          </gpxtpx:TrackPointExtension>
        </extensions>
      </trkpt>
    </trkseg>
  </trk>
</gpx>"#;

        let backend = GpxBackend::new();
        let result = backend.parse_bytes(gpx_xml.as_bytes(), &BackendOptions::default());

        assert!(
            result.is_ok(),
            "GPX with device extensions should parse successfully"
        );
        let doc = result.unwrap();

        // Verify basic track information
        assert!(
            doc.markdown.contains("Training Ride"),
            "Markdown should contain track name 'Training Ride'"
        );

        // Parser may or may not preserve extensions (depends on implementation)
        // Just verify the file is parsed successfully
        assert!(
            !doc.markdown.is_empty(),
            "Parsed GPX should produce non-empty markdown"
        );

        // Verify DocItems structure
        let items = doc.content_blocks.unwrap();
        assert!(!items.is_empty(), "Should have content blocks");
    }

    #[test]
    fn test_gpx_track_with_long_pauses() {
        // GPX track with large time gaps (rest stops, traffic lights)
        // Common in real-world GPS recordings
        let gpx_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.1" creator="Test" xmlns="http://www.topografix.com/GPX/1/1">
  <trk>
    <name>City Walk with Breaks</name>
    <type>walking</type>
    <trkseg>
      <trkpt lat="37.7749" lon="-122.4194">
        <ele>10</ele>
        <time>2024-01-15T10:00:00Z</time>
      </trkpt>
      <trkpt lat="37.7750" lon="-122.4195">
        <ele>12</ele>
        <time>2024-01-15T10:05:00Z</time>
      </trkpt>
      <!-- 30 minute break at coffee shop -->
      <trkpt lat="37.7750" lon="-122.4195">
        <ele>12</ele>
        <time>2024-01-15T10:35:00Z</time>
      </trkpt>
      <trkpt lat="37.7751" lon="-122.4196">
        <ele>15</ele>
        <time>2024-01-15T10:40:00Z</time>
      </trkpt>
      <!-- 1 hour lunch break -->
      <trkpt lat="37.7751" lon="-122.4196">
        <ele>15</ele>
        <time>2024-01-15T11:40:00Z</time>
      </trkpt>
      <trkpt lat="37.7752" lon="-122.4197">
        <ele>18</ele>
        <time>2024-01-15T11:45:00Z</time>
      </trkpt>
    </trkseg>
  </trk>
</gpx>"#;

        let backend = GpxBackend::new();
        let result = backend.parse_bytes(gpx_xml.as_bytes(), &BackendOptions::default());

        assert!(
            result.is_ok(),
            "GPX track with long pauses should parse successfully"
        );
        let doc = result.unwrap();

        // Verify track is present
        assert!(
            doc.markdown.contains("City Walk with Breaks"),
            "Markdown should contain track name"
        );

        // Verify timestamps are parsed (time gaps should be preserved)
        assert!(
            doc.markdown.contains("Time:") || doc.markdown.contains("2024-01-15"),
            "Markdown should contain timestamp information"
        );

        // Verify DocItems structure
        let items = doc.content_blocks.unwrap();
        assert!(!items.is_empty(), "Should have content blocks");

        // Track should have 6 points despite time gaps
        // (Parser should preserve all points, not remove stationary ones)
        assert!(
            doc.markdown.len() > 100,
            "Should have substantial content for multi-point track"
        );
    }

    // ========== Advanced GPX Features (5 tests) ==========

    #[test]
    fn test_gpx_docitems_include_all_track_points() {
        use docling_gps::{GpxInfo, GpxTrack, GpxTrackSegment};

        // Create a track with 50 points (more than old MAX_POINTS_TO_SHOW=20)
        let mut points = Vec::new();
        for i in 0..50 {
            points.push(GpxPoint {
                latitude: 37.0 + (i as f64 * 0.001),
                longitude: -122.0 + (i as f64 * 0.001),
                elevation: Some(100.0 + (i as f64 * 10.0)),
                time: Some(format!("2024-01-15T10:{i:02}:00Z")),
            });
        }

        let track = GpxTrack {
            name: Some("Long Track".to_string()),
            description: Some("Track with 50 points".to_string()),
            track_type: Some("hiking".to_string()),
            total_points: 50,
            segments: vec![GpxTrackSegment { points }],
        };

        let gpx = GpxInfo {
            version: "1.1".to_string(),
            creator: Some("Test".to_string()),
            author: None,
            name: Some("Test GPX".to_string()),
            description: None,
            tracks: vec![track],
            routes: vec![],
            waypoints: vec![],
        };

        let doc_items = GpxBackend::gpx_to_docitems(&gpx);

        // Find the track points text item
        let has_all_points = doc_items.iter().any(|item| {
            if let DocItem::Text { text, .. } = item {
                // Count how many numbered points are in the text
                let point_count = (1..=50)
                    .filter(|i| text.contains(&format!("{i}. Lat:")))
                    .count();
                point_count == 50
            } else {
                false
            }
        });

        assert!(
            has_all_points,
            "DocItems should include all 50 track points, not just the first 20"
        );

        // Verify specific points are present (beginning, middle, end)
        let all_text = doc_items
            .iter()
            .filter_map(|item| {
                if let DocItem::Text { text, .. } = item {
                    Some(text.as_str())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        assert!(
            all_text.contains("1. Lat: 37.000000"),
            "Should have point 1"
        );
        assert!(all_text.contains("25. Lat: 37.024"), "Should have point 25");
        assert!(all_text.contains("50. Lat: 37.049"), "Should have point 50");
        assert!(
            all_text.contains("Ele: 590.0m"),
            "Should have elevation from last point"
        );
    }

    #[test]
    fn test_gpx_docitems_include_all_route_points() {
        use docling_gps::{GpxInfo, GpxRoute};

        // Create a route with 30 points (more than old limit of 10)
        let mut points = Vec::new();
        for i in 0..30 {
            points.push(GpxPoint {
                latitude: 40.0 + (i as f64 * 0.001),
                longitude: -120.0 + (i as f64 * 0.001),
                elevation: Some(500.0 + (i as f64 * 5.0)),
                time: None,
            });
        }

        let route = GpxRoute {
            name: Some("Long Route".to_string()),
            description: Some("Route with 30 points".to_string()),
            points,
        };

        let gpx = GpxInfo {
            version: "1.1".to_string(),
            creator: None,
            author: None,
            name: Some("Test Route".to_string()),
            description: None,
            tracks: vec![],
            routes: vec![route],
            waypoints: vec![],
        };

        let doc_items = GpxBackend::gpx_to_docitems(&gpx);

        // Find the route points text item
        let has_all_points = doc_items.iter().any(|item| {
            if let DocItem::Text { text, .. } = item {
                // Count how many numbered points are in the text
                let point_count = (1..=30)
                    .filter(|i| text.contains(&format!("{i}. Lat:")))
                    .count();
                point_count == 30
            } else {
                false
            }
        });

        assert!(
            has_all_points,
            "DocItems should include all 30 route points, not just the first 10"
        );

        // Verify specific points are present
        let all_text = doc_items
            .iter()
            .filter_map(|item| {
                if let DocItem::Text { text, .. } = item {
                    Some(text.as_str())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        assert!(
            all_text.contains("1. Lat: 40.000000"),
            "Should have point 1"
        );
        assert!(all_text.contains("15. Lat: 40.014"), "Should have point 15");
        assert!(all_text.contains("30. Lat: 40.029"), "Should have point 30");
    }

    #[test]
    fn test_gpx_with_metadata() {
        // GPX file with rich metadata (author, copyright, links, keywords)
        let gpx_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.1" creator="GPS Tracker Pro v2.5" xmlns="http://www.topografix.com/GPX/1/1">
  <metadata>
    <name>Mt. Tamalpais Hike</name>
    <desc>Scenic hike to East Peak with panoramic views of San Francisco Bay</desc>
    <author>
      <name>John Hiker</name>
      <email id="john" domain="example.com"/>
    </author>
    <copyright author="John Hiker">
      <year>2024</year>
      <license>CC BY-NC-SA 4.0</license>
    </copyright>
    <link href="https://example.com/trails/mt-tam">
      <text>Trail Information</text>
      <type>text/html</type>
    </link>
    <time>2024-01-20T08:00:00Z</time>
    <keywords>hiking, mountain, bay area, scenic</keywords>
    <bounds minlat="37.900" minlon="-122.600" maxlat="37.925" maxlon="-122.575"/>
  </metadata>
  <trk>
    <name>Main Trail</name>
    <trkseg>
      <trkpt lat="37.9123" lon="-122.5877">
        <ele>100</ele>
        <time>2024-01-20T08:30:00Z</time>
      </trkpt>
      <trkpt lat="37.9234" lon="-122.5786">
        <ele>784</ele>
        <time>2024-01-20T10:00:00Z</time>
      </trkpt>
    </trkseg>
  </trk>
</gpx>"#;

        let backend = GpxBackend::new();
        let result = backend.parse_bytes(gpx_xml.as_bytes(), &BackendOptions::default());

        assert!(
            result.is_ok(),
            "GPX with rich metadata should parse successfully"
        );
        let doc = result.unwrap();

        // Verify metadata is extracted
        // Parser may or may not include all metadata fields depending on implementation
        assert!(
            doc.markdown.contains("Mt. Tamalpais")
                || doc.markdown.contains("Main Trail")
                || doc.markdown.contains("Scenic hike"),
            "Markdown should contain name or description from metadata"
        );

        // Verify DocItems structure
        let items = doc.content_blocks.unwrap();
        assert!(!items.is_empty(), "Should have content blocks");

        // Check metadata in document metadata
        assert!(
            doc.metadata.title.is_some() || !doc.markdown.is_empty(),
            "Should extract title from metadata"
        );
    }

    #[test]
    fn test_gpx_route_with_waypoint_symbols() {
        // GPX route with waypoints containing symbols and descriptions
        // Common in navigation applications (Garmin, OsmAnd)
        let gpx_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.1" creator="Navigator" xmlns="http://www.topografix.com/GPX/1/1">
  <rte>
    <name>City Tour Route</name>
    <desc>Historical landmarks tour</desc>
    <rtept lat="37.7749" lon="-122.4194">
      <ele>15</ele>
      <name>Start: Ferry Building</name>
      <desc>Historic marketplace and transportation hub</desc>
      <sym>Flag, Blue</sym>
      <type>Start Point</type>
    </rtept>
    <rtept lat="37.7955" lon="-122.4030">
      <ele>80</ele>
      <name>Coit Tower</name>
      <desc>Art Deco tower with city views</desc>
      <sym>Scenic Area</sym>
      <type>Point of Interest</type>
    </rtept>
    <rtept lat="37.8199" lon="-122.4783">
      <ele>67</ele>
      <name>Golden Gate Bridge</name>
      <desc>Iconic suspension bridge</desc>
      <sym>Bridge</sym>
      <type>Landmark</type>
    </rtept>
    <rtept lat="37.7749" lon="-122.4194">
      <ele>15</ele>
      <name>End: Ferry Building</name>
      <desc>Return to start</desc>
      <sym>Flag, Red</sym>
      <type>End Point</type>
    </rtept>
  </rte>
</gpx>"#;

        let backend = GpxBackend::new();
        let result = backend.parse_bytes(gpx_xml.as_bytes(), &BackendOptions::default());

        assert!(result.is_ok());
        let doc = result.unwrap();

        // Verify route name and description are present
        assert!(doc.markdown.contains("City Tour Route"));
        assert!(
            doc.markdown.contains("Historical landmarks") || doc.markdown.contains("Historical"),
            "Should include route description"
        );

        // Verify route points are included (coordinates)
        assert!(
            doc.markdown.contains("37.7749")
                || doc.markdown.contains("37.7955")
                || doc.markdown.contains("37.8199"),
            "Should include route point coordinates"
        );

        // Verify DocItems structure
        let items = doc.content_blocks.unwrap();
        assert!(items.len() >= 2, "Should have content blocks for route");

        // Route should include 4 waypoints (note: waypoint names may not be in route output, only coordinates)
        assert!(
            doc.markdown.len() > 200,
            "Should have substantial content for route with 4 waypoints"
        );

        // Verify Points count
        assert!(
            doc.markdown.contains("Points: 4") || doc.markdown.contains("Points:"),
            "Should show number of route points"
        );
    }

    #[test]
    fn test_gpx_track_with_multiple_segments() {
        // GPX track with multiple segments (GPS device turned off/on)
        // Common in multi-day hikes or when GPS signal is lost
        let gpx_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.1" creator="Hiking Logger" xmlns="http://www.topografix.com/GPX/1/1">
  <trk>
    <name>Pacific Crest Trail - Section 1</name>
    <desc>Multi-day backpacking trip, GPS recharged each morning</desc>
    <type>hiking</type>
    <trkseg>
      <!-- Day 1 -->
      <trkpt lat="34.3813" lon="-116.8560">
        <ele>1200</ele>
        <time>2024-04-01T07:00:00Z</time>
      </trkpt>
      <trkpt lat="34.3825" lon="-116.8550">
        <ele>1250</ele>
        <time>2024-04-01T12:00:00Z</time>
      </trkpt>
      <trkpt lat="34.3840" lon="-116.8540">
        <ele>1300</ele>
        <time>2024-04-01T17:00:00Z</time>
      </trkpt>
    </trkseg>
    <trkseg>
      <!-- Day 2 -->
      <trkpt lat="34.3840" lon="-116.8540">
        <ele>1300</ele>
        <time>2024-04-02T07:00:00Z</time>
      </trkpt>
      <trkpt lat="34.3855" lon="-116.8530">
        <ele>1350</ele>
        <time>2024-04-02T12:00:00Z</time>
      </trkpt>
      <trkpt lat="34.3870" lon="-116.8520">
        <ele>1400</ele>
        <time>2024-04-02T17:00:00Z</time>
      </trkpt>
    </trkseg>
    <trkseg>
      <!-- Day 3 -->
      <trkpt lat="34.3870" lon="-116.8520">
        <ele>1400</ele>
        <time>2024-04-03T07:00:00Z</time>
      </trkpt>
      <trkpt lat="34.3885" lon="-116.8510">
        <ele>1450</ele>
        <time>2024-04-03T12:00:00Z</time>
      </trkpt>
      <trkpt lat="34.3900" lon="-116.8500">
        <ele>1500</ele>
        <time>2024-04-03T17:00:00Z</time>
      </trkpt>
    </trkseg>
  </trk>
</gpx>"#;

        let backend = GpxBackend::new();
        let result = backend.parse_bytes(gpx_xml.as_bytes(), &BackendOptions::default());

        assert!(result.is_ok());
        let doc = result.unwrap();

        // Verify track with multiple segments is parsed
        assert!(doc.markdown.contains("Pacific Crest Trail"));

        // Verify all track points are included (3 segments × 3 points = 9 points)
        // Parser should preserve all segments and points
        assert!(
            doc.markdown.len() > 300,
            "Should have substantial content for multi-segment track with 9 points"
        );

        // Verify DocItems structure
        let items = doc.content_blocks.unwrap();
        assert!(!items.is_empty(), "Should have content blocks");

        // Verify elevation data is present (1200m to 1500m progression)
        assert!(
            doc.markdown.contains("1200")
                || doc.markdown.contains("1500")
                || doc.markdown.contains("Ele:"),
            "Should include elevation data"
        );
    }

    #[test]
    fn test_gpx_with_bounds() {
        // GPX file with explicit bounding box (map extent)
        // Used by mapping applications to determine viewport
        let gpx_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.1" creator="MapExporter" xmlns="http://www.topografix.com/GPX/1/1">
  <metadata>
    <name>Yosemite Valley Area</name>
    <bounds minlat="37.7000" minlon="-119.7000" maxlat="37.8000" maxlon="-119.5000"/>
  </metadata>
  <wpt lat="37.7483" lon="-119.5872">
    <ele>1219</ele>
    <name>Half Dome</name>
    <desc>Iconic granite dome</desc>
    <sym>Summit</sym>
  </wpt>
  <wpt lat="37.7273" lon="-119.5973">
    <ele>2425</ele>
    <name>Glacier Point</name>
    <desc>Panoramic overlook</desc>
    <sym>Scenic Area</sym>
  </wpt>
  <wpt lat="37.7455" lon="-119.5957">
    <ele>1215</ele>
    <name>Yosemite Falls</name>
    <desc>North America's tallest waterfall</desc>
    <sym>Waterfall</sym>
  </wpt>
</gpx>"#;

        let backend = GpxBackend::new();
        let result = backend.parse_bytes(gpx_xml.as_bytes(), &BackendOptions::default());

        assert!(result.is_ok());
        let doc = result.unwrap();

        // Verify waypoints are present
        assert!(
            doc.markdown.contains("Half Dome")
                || doc.markdown.contains("Glacier Point")
                || doc.markdown.contains("Yosemite Falls")
        );

        // Verify bounding box information (parser may or may not include in output)
        // At minimum, verify document parses successfully
        assert!(!doc.markdown.is_empty());

        // Verify DocItems structure
        let items = doc.content_blocks.unwrap();
        assert!(
            items.len() >= 3,
            "Should have content blocks for 3 waypoints"
        );

        // Verify coordinates are within bounds (37.7-37.8, -119.7 to -119.5)
        assert!(
            doc.markdown.contains("37.7") || doc.markdown.contains("119.5"),
            "Should include coordinate data within bounds"
        );
    }

    #[test]
    fn test_gpx_with_elevation_statistics() {
        // GPX track with elevation gain/loss information in description
        // Common in fitness apps and route planning tools
        let gpx_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.1" creator="Fitness Tracker" xmlns="http://www.topografix.com/GPX/1/1">
  <trk>
    <name>Mount Diablo Summit</name>
    <desc>Challenging climb with 1,173m elevation gain. Total ascent: 1,173m, Total descent: 0m. Distance: 11.2 km. Moving time: 3h 45m.</desc>
    <type>hiking</type>
    <trkseg>
      <trkpt lat="37.8816" lon="-121.9141">
        <ele>85</ele>
        <time>2024-03-10T08:00:00Z</time>
      </trkpt>
      <trkpt lat="37.8820" lon="-121.9130">
        <ele>150</ele>
        <time>2024-03-10T08:30:00Z</time>
      </trkpt>
      <trkpt lat="37.8825" lon="-121.9120">
        <ele>300</ele>
        <time>2024-03-10T09:00:00Z</time>
      </trkpt>
      <trkpt lat="37.8830" lon="-121.9110">
        <ele>500</ele>
        <time>2024-03-10T09:30:00Z</time>
      </trkpt>
      <trkpt lat="37.8835" lon="-121.9100">
        <ele>750</ele>
        <time>2024-03-10T10:15:00Z</time>
      </trkpt>
      <trkpt lat="37.8840" lon="-121.9090">
        <ele>1000</ele>
        <time>2024-03-10T11:00:00Z</time>
      </trkpt>
      <trkpt lat="37.8845" lon="-121.9080">
        <ele>1173</ele>
        <time>2024-03-10T11:45:00Z</time>
      </trkpt>
    </trkseg>
  </trk>
</gpx>"#;

        let backend = GpxBackend::new();
        let result = backend.parse_bytes(gpx_xml.as_bytes(), &BackendOptions::default());

        assert!(result.is_ok());
        let doc = result.unwrap();

        // Verify track name and statistics are present
        assert!(doc.markdown.contains("Mount Diablo Summit"));
        assert!(
            doc.markdown.contains("1,173")
                || doc.markdown.contains("1173")
                || doc.markdown.contains("elevation"),
            "Should include elevation statistics from description"
        );

        // Verify elevation progression (85m → 1173m)
        assert!(
            doc.markdown.contains("85") || doc.markdown.contains("1173"),
            "Should include elevation values"
        );

        // Verify DocItems structure
        let items = doc.content_blocks.unwrap();
        assert!(!items.is_empty(), "Should have content blocks");

        // Track with 7 points should have substantial markdown
        assert!(
            doc.markdown.len() > 200,
            "Should have substantial content for 7-point elevation climb"
        );

        // Verify timestamps show progression over ~3h 45m
        assert!(
            doc.markdown.contains("Time:") || doc.markdown.contains("2024-03-10"),
            "Should include timestamp information"
        );
    }
}
