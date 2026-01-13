//! GPS format backend for docling-core
//!
//! Processes GPX (GPS Exchange Format) files into markdown documents.

use std::fmt::Write;
use std::path::Path;

use crate::error::{DoclingError, Result};

/// Process a GPX file into markdown
///
/// # Arguments
///
/// * `path` - Path to the GPX file
///
/// # Returns
///
/// Returns markdown document with GPS track information.
///
/// # Errors
///
/// Returns an error if the file cannot be read or if GPX parsing fails.
///
/// # Examples
///
/// ```no_run
/// use docling_core::gps::process_gpx;
///
/// let markdown = process_gpx("hiking_trail.gpx")?;
/// println!("{}", markdown);
/// # Ok::<(), docling_core::error::DoclingError>(())
/// ```
#[must_use = "this function returns the extracted markdown content"]
#[allow(clippy::too_many_lines)] // Complex GPX processing - keeping together for clarity
pub fn process_gpx<P: AsRef<Path>>(path: P) -> Result<String> {
    let path = path.as_ref();

    // Parse GPX file to get GPS data
    let gpx = docling_gps::parse_gpx(path)
        .map_err(|e| DoclingError::ConversionError(format!("Failed to parse GPX: {e}")))?;

    // Start building markdown output
    let mut markdown = String::new();

    // Add title
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("track.gpx");

    let gpx_name = gpx.name.unwrap_or_else(|| filename.to_string());
    let _ = writeln!(markdown, "# GPS Track: {gpx_name}\n");

    // Add GPX description if available
    if let Some(desc) = gpx.description {
        let _ = writeln!(markdown, "{desc}\n");
    }

    // Add GPX metadata
    markdown.push_str("## GPS Information\n\n");
    markdown.push_str("- **Format:** GPX (GPS Exchange Format)\n");
    let _ = writeln!(markdown, "- **Version:** {}", gpx.version);
    if let Some(creator) = gpx.creator {
        let _ = writeln!(markdown, "- **Creator:** {creator}");
    }
    let _ = writeln!(markdown, "- **Tracks:** {}", gpx.tracks.len());
    let _ = writeln!(markdown, "- **Routes:** {}", gpx.routes.len());
    let _ = writeln!(markdown, "- **Waypoints:** {}", gpx.waypoints.len());
    markdown.push('\n');

    // Add tracks section
    if !gpx.tracks.is_empty() {
        markdown.push_str("## Tracks\n\n");
        for (i, track) in gpx.tracks.iter().enumerate() {
            let default_name = format!("Track {}", i + 1);
            let track_name = track.name.as_deref().unwrap_or(&default_name);
            let _ = writeln!(markdown, "### {}. {}\n", i + 1, track_name);

            if let Some(desc) = &track.description {
                let _ = writeln!(markdown, "{desc}\n");
            }

            markdown.push_str("**Track Details:**\n\n");
            if let Some(track_type) = &track.track_type {
                let _ = writeln!(markdown, "- **Type:** {track_type}");
            }
            let _ = writeln!(markdown, "- **Segments:** {}", track.segments.len());
            let _ = writeln!(markdown, "- **Total Points:** {}", track.total_points);

            // Calculate statistics if we have points
            if track.total_points > 0 {
                let mut min_lat = f64::MAX;
                let mut max_lat = f64::MIN;
                let mut min_lon = f64::MAX;
                let mut max_lon = f64::MIN;
                let mut min_ele = f64::MAX;
                let mut max_ele = f64::MIN;
                let mut has_elevation = false;

                for segment in &track.segments {
                    for point in &segment.points {
                        min_lat = min_lat.min(point.latitude);
                        max_lat = max_lat.max(point.latitude);
                        min_lon = min_lon.min(point.longitude);
                        max_lon = max_lon.max(point.longitude);

                        if let Some(ele) = point.elevation {
                            min_ele = min_ele.min(ele);
                            max_ele = max_ele.max(ele);
                            has_elevation = true;
                        }
                    }
                }

                let _ = writeln!(
                    markdown,
                    "- **Latitude Range:** {min_lat:.6}° to {max_lat:.6}°"
                );
                let _ = writeln!(
                    markdown,
                    "- **Longitude Range:** {min_lon:.6}° to {max_lon:.6}°"
                );

                if has_elevation {
                    let _ = writeln!(
                        markdown,
                        "- **Elevation Range:** {min_ele:.1}m to {max_ele:.1}m"
                    );
                }
            }

            markdown.push('\n');
        }
    }

    // Add routes section
    if !gpx.routes.is_empty() {
        markdown.push_str("## Routes\n\n");
        for (i, route) in gpx.routes.iter().enumerate() {
            let default_name = format!("Route {}", i + 1);
            let route_name = route.name.as_deref().unwrap_or(&default_name);
            let _ = writeln!(markdown, "### {}. {}\n", i + 1, route_name);

            if let Some(desc) = &route.description {
                let _ = writeln!(markdown, "{desc}\n");
            }

            markdown.push_str("**Route Details:**\n\n");
            let _ = writeln!(markdown, "- **Points:** {}", route.points.len());

            if let (Some(first), Some(last)) = (route.points.first(), route.points.last()) {
                let _ = writeln!(
                    markdown,
                    "- **Start:** {:.6}°, {:.6}°",
                    first.latitude, first.longitude
                );
                let _ = writeln!(
                    markdown,
                    "- **End:** {:.6}°, {:.6}°",
                    last.latitude, last.longitude
                );
            }

            markdown.push('\n');
        }
    }

    // Add waypoints section
    if !gpx.waypoints.is_empty() {
        markdown.push_str("## Waypoints\n\n");
        for (i, waypoint) in gpx.waypoints.iter().enumerate() {
            let default_name = format!("Waypoint {}", i + 1);
            let wp_name = waypoint.name.as_deref().unwrap_or(&default_name);
            let _ = writeln!(markdown, "{}. **{}**", i + 1, wp_name);

            if let Some(desc) = &waypoint.description {
                let _ = writeln!(markdown, "   - {desc}");
            }

            let _ = writeln!(
                markdown,
                "   - Location: {:.6}°, {:.6}°",
                waypoint.point.latitude, waypoint.point.longitude
            );

            if let Some(ele) = waypoint.point.elevation {
                let _ = writeln!(markdown, "   - Elevation: {ele:.1}m");
            }

            if let Some(time) = &waypoint.point.time {
                let _ = writeln!(markdown, "   - Time: {time}");
            }
        }
        markdown.push('\n');
    }

    // If GPX is empty, add a note
    if gpx.tracks.is_empty() && gpx.routes.is_empty() && gpx.waypoints.is_empty() {
        markdown.push_str("*This GPX file is empty or contains no parseable data.*\n\n");
    }

    Ok(markdown)
}
