//! GPX (GPS Exchange Format) parsing module
//!
//! Provides parsing for GPX files containing GPS tracks, routes, and waypoints.

use crate::error::{GpsError, Result};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

/// GPS track information extracted from GPX file
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GpxInfo {
    /// GPX metadata name
    pub name: Option<String>,
    /// GPX metadata description
    pub description: Option<String>,
    /// GPX version
    pub version: String,
    /// Creator application (software that created the file)
    pub creator: Option<String>,
    /// Author name (person who created the content)
    pub author: Option<String>,
    /// List of tracks in the GPX file
    pub tracks: Vec<GpxTrack>,
    /// List of routes in the GPX file
    pub routes: Vec<GpxRoute>,
    /// List of waypoints in the GPX file
    pub waypoints: Vec<GpxWaypoint>,
}

/// GPS track containing a sequence of track segments
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GpxTrack {
    /// Track name
    pub name: Option<String>,
    /// Track description
    pub description: Option<String>,
    /// Track type (e.g., "hiking", "running")
    pub track_type: Option<String>,
    /// Track segments
    pub segments: Vec<GpxTrackSegment>,
    /// Number of points in all segments
    pub total_points: usize,
}

/// GPS track segment containing a sequence of track points
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GpxTrackSegment {
    /// Track points in this segment
    pub points: Vec<GpxPoint>,
}

/// GPS route containing a sequence of route points
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GpxRoute {
    /// Route name
    pub name: Option<String>,
    /// Route description
    pub description: Option<String>,
    /// Route points
    pub points: Vec<GpxPoint>,
}

/// GPS waypoint (point of interest)
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GpxWaypoint {
    /// Waypoint name
    pub name: Option<String>,
    /// Waypoint description
    pub description: Option<String>,
    /// Point data
    pub point: GpxPoint,
}

/// GPS point with coordinates and optional metadata
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GpxPoint {
    /// Latitude in degrees
    pub latitude: f64,
    /// Longitude in degrees
    pub longitude: f64,
    /// Elevation in meters (optional)
    pub elevation: Option<f64>,
    /// Timestamp (optional)
    pub time: Option<String>,
}

/// Parse a GPX file and extract GPS information
///
/// # Arguments
///
/// * `path` - Path to the GPX file
///
/// # Returns
///
/// Returns a `GpxInfo` struct containing all GPS data from the file.
///
/// # Examples
///
/// ```no_run
/// use docling_gps::parse_gpx;
///
/// let gpx = parse_gpx("hiking_trail.gpx")?;
/// println!("GPX: {}", gpx.name.unwrap_or_default());
/// println!("Tracks: {}", gpx.tracks.len());
/// # Ok::<(), docling_gps::error::GpsError>(())
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The file cannot be opened (`GpsError::Io`)
/// - The GPX file cannot be parsed (`GpsError::GpxParse`)
#[must_use = "this function returns parsed GPX data that should be processed"]
pub fn parse_gpx<P: AsRef<Path>>(path: P) -> Result<GpxInfo> {
    let path = path.as_ref();
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    // Parse GPX file using gpx crate
    let gpx: gpx::Gpx = gpx::read(reader)
        .map_err(|e| GpsError::GpxParse(format!("Failed to parse GPX file: {e}")))?;

    // Extract metadata
    let name = gpx.metadata.as_ref().and_then(|m| m.name.clone());
    let description = gpx.metadata.as_ref().and_then(|m| m.description.clone());

    // Creator is the software (from <gpx creator="..."> attribute, not metadata)
    let creator = gpx.creator.clone();

    // Author is the person (from <metadata><author><name> element)
    let author = gpx
        .metadata
        .as_ref()
        .and_then(|m| m.author.as_ref())
        .and_then(|a| a.name.clone());

    // Extract tracks
    let tracks = gpx
        .tracks
        .iter()
        .map(|track| {
            let segments: Vec<GpxTrackSegment> = track
                .segments
                .iter()
                .map(|seg| {
                    let points: Vec<GpxPoint> = seg
                        .points
                        .iter()
                        .map(|pt| GpxPoint {
                            latitude: pt.point().y(),
                            longitude: pt.point().x(),
                            elevation: pt.elevation,
                            time: pt.time.and_then(|t| t.format().ok()),
                        })
                        .collect();

                    GpxTrackSegment { points }
                })
                .collect();

            let total_points = segments.iter().map(|s| s.points.len()).sum();

            GpxTrack {
                name: track.name.clone(),
                description: track.description.clone(),
                track_type: track.type_.clone(),
                segments,
                total_points,
            }
        })
        .collect();

    // Extract routes
    let routes = gpx
        .routes
        .iter()
        .map(|route| {
            let points: Vec<GpxPoint> = route
                .points
                .iter()
                .map(|pt| GpxPoint {
                    latitude: pt.point().y(),
                    longitude: pt.point().x(),
                    elevation: pt.elevation,
                    time: pt.time.and_then(|t| t.format().ok()),
                })
                .collect();

            GpxRoute {
                name: route.name.clone(),
                description: route.description.clone(),
                points,
            }
        })
        .collect();

    // Extract waypoints
    let waypoints = gpx
        .waypoints
        .iter()
        .map(|wp| GpxWaypoint {
            name: wp.name.clone(),
            description: wp.description.clone(),
            point: GpxPoint {
                latitude: wp.point().y(),
                longitude: wp.point().x(),
                elevation: wp.elevation,
                time: wp.time.and_then(|t| t.format().ok()),
            },
        })
        .collect();

    // Format version properly: Gpx10 -> "1.0", Gpx11 -> "1.1"
    let version = match gpx.version {
        gpx::GpxVersion::Gpx10 => "1.0".to_string(),
        gpx::GpxVersion::Gpx11 => "1.1".to_string(),
        gpx::GpxVersion::Unknown => gpx.version.to_string(),
    };

    Ok(GpxInfo {
        name,
        description,
        version,
        creator,
        author,
        tracks,
        routes,
        waypoints,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// Helper to create a temp GPX file with given content
    fn create_temp_gpx(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::with_suffix(".gpx").unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file.flush().unwrap();
        file
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn test_gpx_point() {
        let point = GpxPoint {
            latitude: 47.644_548,
            longitude: -122.326_897,
            elevation: Some(4.46),
            time: Some("2009-10-17T18:37:26Z".to_string()),
        };

        assert_eq!(point.latitude, 47.644_548);
        assert_eq!(point.longitude, -122.326_897);
        assert_eq!(point.elevation, Some(4.46));
    }

    #[test]
    fn test_parse_gpx_nonexistent() {
        let result = parse_gpx("nonexistent.gpx");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_gpx_simple_waypoint() {
        let gpx_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.1" creator="TestApp">
  <wpt lat="47.644548" lon="-122.326897">
    <name>Seattle Space Needle</name>
    <desc>Famous Seattle landmark</desc>
    <ele>184.0</ele>
  </wpt>
</gpx>"#;

        let temp_file = create_temp_gpx(gpx_content);
        let result = parse_gpx(temp_file.path()).unwrap();

        assert_eq!(result.version, "1.1");
        assert_eq!(result.creator, Some("TestApp".to_string()));
        assert_eq!(result.waypoints.len(), 1);

        let waypoint = &result.waypoints[0];
        assert_eq!(waypoint.name, Some("Seattle Space Needle".to_string()));
        assert_eq!(
            waypoint.description,
            Some("Famous Seattle landmark".to_string())
        );
        assert!((waypoint.point.latitude - 47.644_548).abs() < 0.0001);
        assert!((waypoint.point.longitude - (-122.326_897)).abs() < 0.0001);
        assert_eq!(waypoint.point.elevation, Some(184.0));
    }

    #[test]
    fn test_parse_gpx_multiple_waypoints() {
        let gpx_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.1" creator="TestApp">
  <wpt lat="48.8584" lon="2.2945">
    <name>Eiffel Tower</name>
  </wpt>
  <wpt lat="51.5014" lon="-0.1419">
    <name>Big Ben</name>
  </wpt>
  <wpt lat="40.6892" lon="-74.0445">
    <name>Statue of Liberty</name>
  </wpt>
</gpx>"#;

        let temp_file = create_temp_gpx(gpx_content);
        let result = parse_gpx(temp_file.path()).unwrap();

        assert_eq!(result.waypoints.len(), 3);
        assert_eq!(result.waypoints[0].name, Some("Eiffel Tower".to_string()));
        assert_eq!(result.waypoints[1].name, Some("Big Ben".to_string()));
        assert_eq!(
            result.waypoints[2].name,
            Some("Statue of Liberty".to_string())
        );
    }

    #[test]
    fn test_parse_gpx_with_track() {
        let gpx_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.1" creator="TestApp">
  <trk>
    <name>Morning Run</name>
    <desc>My daily jogging route</desc>
    <trkseg>
      <trkpt lat="47.644548" lon="-122.326897">
        <ele>10.0</ele>
        <time>2025-12-08T07:00:00Z</time>
      </trkpt>
      <trkpt lat="47.645000" lon="-122.327000">
        <ele>12.0</ele>
        <time>2025-12-08T07:01:00Z</time>
      </trkpt>
      <trkpt lat="47.645500" lon="-122.327500">
        <ele>15.0</ele>
        <time>2025-12-08T07:02:00Z</time>
      </trkpt>
    </trkseg>
  </trk>
</gpx>"#;

        let temp_file = create_temp_gpx(gpx_content);
        let result = parse_gpx(temp_file.path()).unwrap();

        assert_eq!(result.tracks.len(), 1);
        let track = &result.tracks[0];
        assert_eq!(track.name, Some("Morning Run".to_string()));
        assert_eq!(
            track.description,
            Some("My daily jogging route".to_string())
        );
        assert_eq!(track.segments.len(), 1);
        assert_eq!(track.total_points, 3);

        let segment = &track.segments[0];
        assert_eq!(segment.points.len(), 3);
        assert!((segment.points[0].latitude - 47.644_548).abs() < 0.0001);
        assert_eq!(segment.points[0].elevation, Some(10.0));
    }

    #[test]
    fn test_parse_gpx_with_route() {
        let gpx_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.1" creator="TestApp">
  <rte>
    <name>Scenic Drive</name>
    <desc>Weekend road trip route</desc>
    <rtept lat="37.7749" lon="-122.4194">
      <name>San Francisco</name>
    </rtept>
    <rtept lat="34.0522" lon="-118.2437">
      <name>Los Angeles</name>
    </rtept>
  </rte>
</gpx>"#;

        let temp_file = create_temp_gpx(gpx_content);
        let result = parse_gpx(temp_file.path()).unwrap();

        assert_eq!(result.routes.len(), 1);
        let route = &result.routes[0];
        assert_eq!(route.name, Some("Scenic Drive".to_string()));
        assert_eq!(
            route.description,
            Some("Weekend road trip route".to_string())
        );
        assert_eq!(route.points.len(), 2);
    }

    #[test]
    fn test_parse_gpx_with_metadata() {
        let gpx_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.1" creator="Garmin Edge 530">
  <metadata>
    <name>Summer Bike Ride</name>
    <desc>100km cycling event</desc>
    <author>
      <name>John Doe</name>
    </author>
  </metadata>
</gpx>"#;

        let temp_file = create_temp_gpx(gpx_content);
        let result = parse_gpx(temp_file.path()).unwrap();

        assert_eq!(result.name, Some("Summer Bike Ride".to_string()));
        assert_eq!(result.description, Some("100km cycling event".to_string()));
        assert_eq!(result.creator, Some("Garmin Edge 530".to_string()));
        assert_eq!(result.author, Some("John Doe".to_string()));
    }

    #[test]
    fn test_parse_gpx_version_1_0() {
        let gpx_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.0" creator="OldApp">
  <wpt lat="45.0" lon="-90.0">
    <name>Test Point</name>
  </wpt>
</gpx>"#;

        let temp_file = create_temp_gpx(gpx_content);
        let result = parse_gpx(temp_file.path()).unwrap();

        assert_eq!(result.version, "1.0");
        assert_eq!(result.waypoints.len(), 1);
    }

    #[test]
    fn test_parse_gpx_multiple_track_segments() {
        let gpx_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.1" creator="TestApp">
  <trk>
    <name>Split Track</name>
    <trkseg>
      <trkpt lat="47.0" lon="-122.0"><ele>10.0</ele></trkpt>
      <trkpt lat="47.1" lon="-122.1"><ele>20.0</ele></trkpt>
    </trkseg>
    <trkseg>
      <trkpt lat="47.2" lon="-122.2"><ele>30.0</ele></trkpt>
      <trkpt lat="47.3" lon="-122.3"><ele>40.0</ele></trkpt>
      <trkpt lat="47.4" lon="-122.4"><ele>50.0</ele></trkpt>
    </trkseg>
  </trk>
</gpx>"#;

        let temp_file = create_temp_gpx(gpx_content);
        let result = parse_gpx(temp_file.path()).unwrap();

        assert_eq!(result.tracks.len(), 1);
        let track = &result.tracks[0];
        assert_eq!(track.segments.len(), 2);
        assert_eq!(track.segments[0].points.len(), 2);
        assert_eq!(track.segments[1].points.len(), 3);
        assert_eq!(track.total_points, 5);
    }

    #[test]
    fn test_parse_gpx_empty() {
        let gpx_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.1" creator="EmptyApp">
</gpx>"#;

        let temp_file = create_temp_gpx(gpx_content);
        let result = parse_gpx(temp_file.path()).unwrap();

        assert_eq!(result.version, "1.1");
        assert!(result.tracks.is_empty());
        assert!(result.routes.is_empty());
        assert!(result.waypoints.is_empty());
    }

    #[test]
    fn test_parse_gpx_mixed_content() {
        let gpx_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.1" creator="MixedApp">
  <metadata>
    <name>Adventure Trip</name>
  </metadata>
  <wpt lat="45.0" lon="-90.0">
    <name>Start Point</name>
  </wpt>
  <trk>
    <name>Hiking Trail</name>
    <trkseg>
      <trkpt lat="45.1" lon="-90.1"></trkpt>
      <trkpt lat="45.2" lon="-90.2"></trkpt>
    </trkseg>
  </trk>
  <rte>
    <name>Return Route</name>
    <rtept lat="45.0" lon="-90.0"></rtept>
  </rte>
  <wpt lat="46.0" lon="-91.0">
    <name>End Point</name>
  </wpt>
</gpx>"#;

        let temp_file = create_temp_gpx(gpx_content);
        let result = parse_gpx(temp_file.path()).unwrap();

        assert_eq!(result.name, Some("Adventure Trip".to_string()));
        assert_eq!(result.waypoints.len(), 2);
        assert_eq!(result.tracks.len(), 1);
        assert_eq!(result.routes.len(), 1);
    }
}
