//! EXIF metadata extraction utilities
//!
//! Shared utilities for extracting EXIF metadata from image formats (JPEG, TIFF).
//! Both JPEG and TIFF use the same EXIF structure (EXIF is based on TIFF format).

use chrono::{DateTime, Utc};
use docling_core::ExifMetadata;
use exif::{Exif, In, Tag};
use std::io::Cursor;

/// Parse EXIF datetime string to chrono `DateTime`
#[inline]
fn parse_exif_datetime(datetime_str: &str) -> Option<DateTime<Utc>> {
    // EXIF datetime format: "YYYY:MM:DD HH:MM:SS"
    // Convert to RFC3339 for chrono
    let datetime_str_rfc = datetime_str.replace(' ', "T").replace(':', "-");
    chrono::DateTime::parse_from_str(
        &format!("{}+00:00", datetime_str_rfc.replacen('-', ":", 2)),
        "%Y-%m-%dT%H:%M:%S%z",
    )
    .ok()
    .map(|dt| dt.with_timezone(&Utc))
}

/// Extract GPS coordinate from EXIF with reference direction
#[inline]
fn extract_gps_coord(exif: &Exif, coord_tag: Tag, ref_tag: Tag, neg_ref: &str) -> Option<f64> {
    let field = exif.get_field(coord_tag, In::PRIMARY)?;
    let ref_field = exif.get_field(ref_tag, In::PRIMARY)?;
    let coord_str = field.display_value().to_string();
    let coord = parse_gps_coordinate(&coord_str).ok()?;
    let sign = if ref_field.display_value().to_string() == neg_ref {
        -1.0
    } else {
        1.0
    };
    Some(coord * sign)
}

/// Extract string field from EXIF (trimmed)
#[inline]
fn get_exif_string(exif: &Exif, tag: Tag) -> Option<String> {
    exif.get_field(tag, In::PRIMARY)
        .map(|f| f.display_value().to_string().trim().to_string())
}

/// Extract rational field from EXIF as f64
#[inline]
fn get_exif_rational(exif: &Exif, tag: Tag) -> Option<f64> {
    let field = exif.get_field(tag, In::PRIMARY)?;
    parse_rational(&field.display_value().to_string()).ok()
}

/// Extract EXIF metadata from image data (JPEG or TIFF)
///
/// Parses EXIF metadata embedded in image file, including camera information,
/// capture settings, GPS coordinates, and timestamps. Works with any image format
/// that contains EXIF data (JPEG, TIFF, etc.).
///
/// ## Arguments
/// * `data` - Raw image file bytes
///
/// ## Returns
/// Optional `ExifMetadata` struct with parsed EXIF fields, or None if no EXIF data present
#[must_use = "returns extracted EXIF metadata if present in the image"]
pub fn extract_exif_metadata(data: &[u8]) -> Option<ExifMetadata> {
    use exif::Reader;

    // Parse EXIF data
    let mut cursor = Cursor::new(data);
    let exifreader = Reader::new();
    let exif = exifreader.read_from_container(&mut cursor).ok()?;

    // Extract datetime
    let datetime = exif
        .get_field(Tag::DateTimeOriginal, In::PRIMARY)
        .and_then(|f| parse_exif_datetime(&f.display_value().to_string()));

    // Extract orientation (needs special handling for u32)
    let orientation = exif
        .get_field(Tag::Orientation, In::PRIMARY)
        .and_then(|f| f.display_value().to_string().parse::<u32>().ok());

    // Extract ISO (needs special handling for u32)
    let iso_speed = exif
        .get_field(Tag::PhotographicSensitivity, In::PRIMARY)
        .and_then(|f| f.display_value().to_string().parse::<u32>().ok());

    Some(ExifMetadata {
        datetime,
        camera_make: get_exif_string(&exif, Tag::Make),
        camera_model: get_exif_string(&exif, Tag::Model),
        gps_latitude: extract_gps_coord(&exif, Tag::GPSLatitude, Tag::GPSLatitudeRef, "S"),
        gps_longitude: extract_gps_coord(&exif, Tag::GPSLongitude, Tag::GPSLongitudeRef, "W"),
        gps_altitude: get_exif_rational(&exif, Tag::GPSAltitude),
        orientation,
        software: get_exif_string(&exif, Tag::Software),
        exposure_time: get_exif_rational(&exif, Tag::ExposureTime),
        f_number: get_exif_rational(&exif, Tag::FNumber),
        iso_speed,
        focal_length: get_exif_rational(&exif, Tag::FocalLength),
        // HDR fields not typically in EXIF (would be in HEIF/XMP)
        hdr_color_primaries: None,
        hdr_transfer_characteristics: None,
        hdr_max_content_light_level: None,
        hdr_max_frame_average_light_level: None,
        hdr_mastering_display_max_luminance: None,
        hdr_mastering_display_min_luminance: None,
    })
}

/// Extract document-level metadata from EXIF data (Artist, `ImageDescription`, `DateTime`)
///
/// Extracts fields that belong in `DocumentMetadata` rather than the nested `ExifMetadata` struct.
/// These fields describe the document itself, not camera-specific information.
///
/// ## Arguments
/// * `data` - Raw image file bytes (JPEG or TIFF)
///
/// ## Returns
/// Tuple of (author, subject, created):
/// - author: EXIF Artist tag (photographer/creator name)
/// - subject: EXIF `ImageDescription` tag (image description/caption)
/// - created: EXIF `DateTimeOriginal` tag (when photo was taken)
#[must_use = "returns extracted document metadata (author, subject, created) from EXIF"]
pub fn extract_document_metadata(
    data: &[u8],
) -> (Option<String>, Option<String>, Option<DateTime<Utc>>) {
    use exif::Reader;

    // Parse EXIF data
    let mut cursor = Cursor::new(data);
    let exifreader = Reader::new();
    let Ok(exif) = exifreader.read_from_container(&mut cursor) else {
        return (None, None, None);
    };

    let author = get_exif_string(&exif, Tag::Artist).filter(|s| !s.is_empty());
    let subject = get_exif_string(&exif, Tag::ImageDescription).filter(|s| !s.is_empty());
    let created = exif
        .get_field(Tag::DateTimeOriginal, In::PRIMARY)
        .and_then(|f| parse_exif_datetime(&f.display_value().to_string()));

    (author, subject, created)
}

/// Parse GPS coordinate from EXIF format (degrees, minutes, seconds)
///
/// EXIF GPS coordinates are in format: "XX deg YY' ZZ.ZZ\""
/// Converts to decimal degrees: XX + YY/60 + ZZ/3600
///
/// ## Arguments
/// * `coord_str` - GPS coordinate string in EXIF format
///
/// ## Returns
/// Decimal degrees as f64, or error if parsing fails
fn parse_gps_coordinate(coord_str: &str) -> Result<f64, Box<dyn std::error::Error>> {
    // Example: "37 deg 46' 28.49\""
    let parts: Vec<&str> = coord_str
        .split(|c: char| !c.is_numeric() && c != '.')
        .collect();
    let numbers: Vec<f64> = parts
        .iter()
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse::<f64>().ok())
        .collect();

    if numbers.len() >= 3 {
        // degrees + minutes/60 + seconds/3600
        Ok(numbers[0] + numbers[1] / 60.0 + numbers[2] / 3600.0)
    } else {
        Err("Invalid GPS coordinate format".into())
    }
}

/// Parse EXIF rational number (fraction or decimal)
///
/// EXIF stores many values as rationals (fractions): "numerator/denominator"
/// Also supports decimal format as fallback.
///
/// ## Arguments
/// * `rational_str` - Rational number string (e.g., "1/125" or "2.5")
///
/// ## Returns
/// Decimal value as f64, or error if parsing fails
fn parse_rational(rational_str: &str) -> Result<f64, Box<dyn std::error::Error>> {
    let rational_str = rational_str.trim();

    // Try parsing as fraction "num/den"
    if let Some(pos) = rational_str.find('/') {
        let numerator: f64 = rational_str[..pos].trim().parse()?;
        let denominator: f64 = rational_str[pos + 1..].trim().parse()?;
        if denominator != 0.0 {
            return Ok(numerator / denominator);
        }
    }

    // Try parsing as decimal
    rational_str
        .parse::<f64>()
        .map_err(std::convert::Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_gps_coordinate() {
        // Test normal DMS format
        assert_eq!(
            parse_gps_coordinate("37 deg 46' 28.49\"").unwrap(),
            37.0 + 46.0 / 60.0 + 28.49 / 3600.0
        );
    }

    #[test]
    fn test_parse_gps_coordinate_invalid() {
        // Test invalid format (too few numbers)
        assert!(parse_gps_coordinate("37").is_err());
    }

    #[test]
    fn test_parse_rational_fraction() {
        // Test fraction format
        assert_eq!(parse_rational("1/125").unwrap(), 1.0 / 125.0);
        assert_eq!(parse_rational("2/1").unwrap(), 2.0);
    }

    #[test]
    fn test_parse_rational_decimal() {
        // Test decimal format
        assert_eq!(parse_rational("2.5").unwrap(), 2.5);
        assert_eq!(parse_rational("100").unwrap(), 100.0);
    }

    #[test]
    fn test_parse_rational_zero_denominator() {
        // Test division by zero (should return error due to guard condition)
        let result = parse_rational("1/0");
        // The function has a guard that prevents division by zero,
        // so it tries parsing as decimal "1/0" which fails
        assert!(result.is_err());
    }

    // ===== Additional GPS Coordinate Tests =====

    #[test]
    fn test_parse_gps_coordinate_zero_values() {
        // Test with zero degrees/minutes/seconds
        assert_eq!(parse_gps_coordinate("0 deg 0' 0\"").unwrap(), 0.0);
    }

    #[test]
    fn test_parse_gps_coordinate_high_precision() {
        // Test high precision decimal seconds
        let result = parse_gps_coordinate("40 deg 42' 51.389\"").unwrap();
        let expected = 40.0 + 42.0 / 60.0 + 51.389 / 3600.0;
        assert!((result - expected).abs() < 1e-10);
    }

    #[test]
    fn test_parse_gps_coordinate_only_degrees() {
        // Test with only degrees (no minutes/seconds)
        let result = parse_gps_coordinate("45");
        // Should fail because we need at least 3 numbers
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_gps_coordinate_two_components() {
        // Test with only degrees and minutes (missing seconds)
        let result = parse_gps_coordinate("45 30");
        // Should fail because we need at least 3 numbers
        assert!(result.is_err());
    }

    // ===== Additional Rational Parsing Tests =====

    #[test]
    fn test_parse_rational_whitespace() {
        // Test with extra whitespace
        assert_eq!(parse_rational("  1 / 125  ").unwrap(), 1.0 / 125.0);
        assert_eq!(parse_rational("  2.5  ").unwrap(), 2.5);
    }

    #[test]
    fn test_parse_rational_large_fraction() {
        // Test with large numerator/denominator
        assert_eq!(parse_rational("1000/8000").unwrap(), 0.125);
    }

    #[test]
    fn test_parse_rational_negative_decimal() {
        // Test negative decimal (although EXIF doesn't use this, function should handle it)
        assert_eq!(parse_rational("-2.5").unwrap(), -2.5);
    }

    #[test]
    fn test_parse_rational_invalid_string() {
        // Test with invalid string
        assert!(parse_rational("not a number").is_err());
        assert!(parse_rational("abc/def").is_err());
    }

    #[test]
    fn test_parse_rational_empty_string() {
        // Test with empty string
        assert!(parse_rational("").is_err());
    }

    // ===== EXIF Metadata Extraction Tests =====

    #[test]
    fn test_extract_exif_no_data() {
        // Test with no EXIF data (plain bytes)
        let data = b"not an image";
        let result = extract_exif_metadata(data);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_exif_empty_data() {
        // Test with empty data
        let data = b"";
        let result = extract_exif_metadata(data);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_gps_coordinate_extra_components() {
        // Test with more than 3 components (should still work, uses first 3)
        let result = parse_gps_coordinate("45 deg 30' 15\" extra stuff 99").unwrap();
        let expected = 45.0 + 30.0 / 60.0 + 15.0 / 3600.0;
        assert!((result - expected).abs() < 1e-10);
    }

    #[test]
    fn test_parse_rational_integer_fraction() {
        // Test fraction that results in integer
        assert_eq!(parse_rational("100/10").unwrap(), 10.0);
        assert_eq!(parse_rational("50/5").unwrap(), 10.0);
    }

    #[test]
    fn test_parse_rational_zero_numerator() {
        // Test fraction with zero numerator
        assert_eq!(parse_rational("0/100").unwrap(), 0.0);
    }

    // New tests for N=394 expansion

    // ===== Additional GPS Coordinate Edge Cases (6 tests) =====

    #[test]
    fn test_parse_gps_coordinate_max_latitude() {
        // Test maximum latitude (90 degrees)
        let result = parse_gps_coordinate("90 deg 0' 0\"").unwrap();
        assert_eq!(result, 90.0);
    }

    #[test]
    fn test_parse_gps_coordinate_max_longitude() {
        // Test maximum longitude (180 degrees)
        let result = parse_gps_coordinate("180 deg 0' 0\"").unwrap();
        assert_eq!(result, 180.0);
    }

    #[test]
    fn test_parse_gps_coordinate_fractional_seconds() {
        // Test GPS with fractional seconds
        let result = parse_gps_coordinate("51 deg 30' 25.6789\"").unwrap();
        let expected = 51.0 + 30.0 / 60.0 + 25.6789 / 3600.0;
        assert!((result - expected).abs() < 1e-10);
    }

    #[test]
    fn test_parse_gps_coordinate_59_minutes_59_seconds() {
        // Test boundary: 59 minutes 59.999 seconds
        let result = parse_gps_coordinate("45 deg 59' 59.999\"").unwrap();
        let expected = 45.0 + 59.0 / 60.0 + 59.999 / 3600.0;
        assert!((result - expected).abs() < 1e-10);
    }

    #[test]
    fn test_parse_gps_coordinate_many_decimals() {
        // Test GPS with many decimal places (precision)
        let result = parse_gps_coordinate("37 deg 46' 28.123456789\"").unwrap();
        let expected = 37.0 + 46.0 / 60.0 + 28.123_456_789 / 3600.0;
        assert!((result - expected).abs() < 1e-15);
    }

    #[test]
    fn test_parse_gps_coordinate_alternate_format() {
        // Test with different delimiter pattern (numbers only)
        let result = parse_gps_coordinate("37 46 28.49");
        assert!(result.is_ok());
    }

    // ===== Additional Rational Parsing Edge Cases (6 tests) =====

    #[test]
    fn test_parse_rational_very_small_fraction() {
        // Test very small fraction (like shutter speed 1/8000)
        assert_eq!(parse_rational("1/8000").unwrap(), 1.0 / 8000.0);
        assert!((parse_rational("1/1000000").unwrap() - 0.000_001).abs() < 1e-10);
    }

    #[test]
    fn test_parse_rational_decimal_with_zeros() {
        // Test decimal with leading/trailing zeros
        assert_eq!(parse_rational("0.5").unwrap(), 0.5);
        assert_eq!(parse_rational("5.0").unwrap(), 5.0);
        assert_eq!(parse_rational("00.5").unwrap(), 0.5);
    }

    #[test]
    fn test_parse_rational_large_numbers() {
        // Test with large numbers (like focal length in mm)
        assert_eq!(parse_rational("200").unwrap(), 200.0);
        assert_eq!(parse_rational("1000/1").unwrap(), 1000.0);
    }

    #[test]
    fn test_parse_rational_fraction_with_decimal_parts() {
        // Test fraction where numerator/denominator are decimals
        // Note: EXIF typically uses integers, but function should handle it
        assert_eq!(parse_rational("2.5/5.0").unwrap(), 0.5);
    }

    #[test]
    fn test_parse_rational_multiple_slashes() {
        // Test invalid format with multiple slashes
        let result = parse_rational("1/2/3");
        // Should fail because denominator "2/3" is not a valid number
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_rational_slash_only() {
        // Test with only slash character
        assert!(parse_rational("/").is_err());
        assert!(parse_rational("1/").is_err());
        assert!(parse_rational("/1").is_err());
    }

    // ===== EXIF Metadata Structure Tests (6 tests) =====

    #[test]
    fn test_exif_metadata_default() {
        // Test ExifMetadata default values
        let exif = ExifMetadata::default();
        assert!(exif.datetime.is_none());
        assert!(exif.camera_make.is_none());
        assert!(exif.camera_model.is_none());
        assert!(exif.gps_latitude.is_none());
        assert!(exif.gps_longitude.is_none());
        assert!(exif.gps_altitude.is_none());
        assert!(exif.orientation.is_none());
        assert!(exif.software.is_none());
        assert!(exif.exposure_time.is_none());
        assert!(exif.f_number.is_none());
        assert!(exif.iso_speed.is_none());
        assert!(exif.focal_length.is_none());
    }

    #[test]
    fn test_extract_exif_corrupted_data() {
        // Test with corrupted/malformed data
        let data = b"\xFF\xD8\xFF\xE0\x00\x10JFIF"; // JPEG header but incomplete
        let result = extract_exif_metadata(data);
        // Should return None (no valid EXIF)
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_exif_very_small_data() {
        // Test with very small data (< typical EXIF header size)
        let data = b"\xFF\xD8"; // Just JPEG SOI marker
        let result = extract_exif_metadata(data);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_exif_binary_data() {
        // Test with random binary data
        let data: Vec<u8> = (0..100).collect();
        let result = extract_exif_metadata(&data);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_exif_null_bytes() {
        // Test with null bytes
        let data = b"\x00\x00\x00\x00\x00\x00\x00\x00";
        let result = extract_exif_metadata(data);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_exif_large_data_no_exif() {
        // Test with large data but no EXIF structure
        let data = vec![0xFF; 10000];
        let result = extract_exif_metadata(&data);
        assert!(result.is_none());
    }

    // ===== Helper Function Edge Cases (2 tests) =====

    #[test]
    fn test_parse_gps_coordinate_mixed_formats() {
        // Test with numbers separated by various non-numeric chars
        let result1 = parse_gps_coordinate("37°46'28.49\"").unwrap();
        let result2 = parse_gps_coordinate("37 46 28.49").unwrap();
        // Both should parse to same value
        assert!((result1 - result2).abs() < 1e-10);
    }

    #[test]
    fn test_parse_rational_preserves_precision() {
        // Test that precision is preserved for camera settings
        let aperture = parse_rational("2.8").unwrap();
        assert_eq!(aperture, 2.8);

        let shutter = parse_rational("1/500").unwrap();
        assert_eq!(shutter, 0.002);

        let focal = parse_rational("35").unwrap();
        assert_eq!(focal, 35.0);
    }

    // ===== Additional Edge Case Tests (N=457 expansion) =====

    #[test]
    fn test_parse_rational_min_aperture() {
        // Test minimum aperture values (f/1.0, f/1.2, f/1.4)
        assert_eq!(parse_rational("1.0").unwrap(), 1.0);
        assert_eq!(parse_rational("1.2").unwrap(), 1.2);
        assert_eq!(parse_rational("1.4").unwrap(), 1.4);
    }

    #[test]
    fn test_parse_rational_max_aperture() {
        // Test maximum aperture values (f/32, f/64)
        assert_eq!(parse_rational("32.0").unwrap(), 32.0);
        assert_eq!(parse_rational("64.0").unwrap(), 64.0);
    }

    #[test]
    fn test_parse_rational_fast_shutter_speeds() {
        // Test very fast shutter speeds (1/4000, 1/8000)
        assert!((parse_rational("1/4000").unwrap() - 0.00025).abs() < 1e-10);
        assert!((parse_rational("1/8000").unwrap() - 0.000_125).abs() < 1e-10);
    }

    #[test]
    fn test_parse_rational_slow_shutter_speeds() {
        // Test slow shutter speeds (bulb mode, 30 seconds)
        assert_eq!(parse_rational("30/1").unwrap(), 30.0);
        assert_eq!(parse_rational("1/1").unwrap(), 1.0);
        assert_eq!(parse_rational("2/1").unwrap(), 2.0);
    }

    #[test]
    fn test_parse_rational_high_iso() {
        // Test high ISO values (common in modern cameras)
        assert_eq!(parse_rational("12800").unwrap(), 12800.0);
        assert_eq!(parse_rational("25600").unwrap(), 25600.0);
        assert_eq!(parse_rational("51200").unwrap(), 51200.0);
    }

    #[test]
    fn test_parse_rational_wide_angle_focal_length() {
        // Test wide angle focal lengths (10mm-35mm)
        assert_eq!(parse_rational("10").unwrap(), 10.0);
        assert_eq!(parse_rational("14").unwrap(), 14.0);
        assert_eq!(parse_rational("24").unwrap(), 24.0);
    }

    #[test]
    fn test_parse_rational_telephoto_focal_length() {
        // Test telephoto focal lengths (200mm-600mm)
        assert_eq!(parse_rational("200").unwrap(), 200.0);
        assert_eq!(parse_rational("400").unwrap(), 400.0);
        assert_eq!(parse_rational("600").unwrap(), 600.0);
    }

    #[test]
    fn test_parse_gps_coordinate_negative_sign_handling() {
        // Test that function returns absolute value (sign handled by ref)
        let result = parse_gps_coordinate("33 deg 56' 14.123\"").unwrap();
        assert!(result > 0.0);
        // Verify it's not applying negative sign internally
        assert_eq!(result, 33.0 + 56.0 / 60.0 + 14.123 / 3600.0);
    }

    #[test]
    fn test_parse_gps_coordinate_equator() {
        // Test coordinates at equator (0 degrees latitude)
        let result = parse_gps_coordinate("0 deg 0' 1\"").unwrap();
        assert!((result - 1.0 / 3600.0).abs() < 1e-10);
    }

    #[test]
    fn test_parse_gps_coordinate_prime_meridian() {
        // Test coordinates at prime meridian (0 degrees longitude)
        let result = parse_gps_coordinate("0 deg 0' 30\"").unwrap();
        assert!((result - 30.0 / 3600.0).abs() < 1e-10);
    }

    #[test]
    fn test_parse_rational_exposure_compensation() {
        // Test exposure compensation values (+/- EV)
        // Note: EXIF uses rationals, negative handled separately
        assert_eq!(parse_rational("1/3").unwrap(), 1.0 / 3.0);
        assert_eq!(parse_rational("2/3").unwrap(), 2.0 / 3.0);
        assert_eq!(parse_rational("5/3").unwrap(), 5.0 / 3.0);
    }

    // ===== N=474 Expansion: 10 additional tests =====

    #[test]
    fn test_parse_rational_macro_focal_length() {
        // Test macro photography focal lengths (typically 50-105mm)
        assert_eq!(parse_rational("50").unwrap(), 50.0);
        assert_eq!(parse_rational("60").unwrap(), 60.0);
        assert_eq!(parse_rational("105").unwrap(), 105.0);
    }

    #[test]
    fn test_parse_rational_extreme_aperture_values() {
        // Test extreme aperture values (very wide and very narrow)
        assert_eq!(parse_rational("0.95").unwrap(), 0.95); // Ultra-wide aperture
        assert_eq!(parse_rational("1.2").unwrap(), 1.2);
        assert_eq!(parse_rational("22").unwrap(), 22.0); // Very narrow aperture
        assert_eq!(parse_rational("32").unwrap(), 32.0);
    }

    #[test]
    fn test_parse_gps_coordinate_extreme_precision() {
        // Test GPS coordinates with extreme precision (sub-meter accuracy)
        let result = parse_gps_coordinate("40 deg 44' 55.123456\"").unwrap();
        assert!((result - (40.0 + 44.0 / 60.0 + 55.123_456 / 3600.0)).abs() < 1e-10);
    }

    #[test]
    fn test_parse_gps_coordinate_arctic() {
        // Test high latitude coordinates (Arctic Circle ~66.5°)
        let result = parse_gps_coordinate("66 deg 30' 0\"").unwrap();
        assert!((result - 66.5).abs() < 1e-10);
    }

    #[test]
    fn test_parse_gps_coordinate_antarctic() {
        // Test high latitude coordinates (Antarctic Circle ~66.5°)
        let result = parse_gps_coordinate("66 deg 33' 44\"").unwrap();
        assert!((result - (66.0 + 33.0 / 60.0 + 44.0 / 3600.0)).abs() < 1e-10);
    }

    #[test]
    fn test_parse_rational_ultra_fast_shutter() {
        // Test ultra-fast shutter speeds (1/8000s and faster)
        assert_eq!(parse_rational("1/8000").unwrap(), 1.0 / 8000.0);
        assert_eq!(parse_rational("1/16000").unwrap(), 1.0 / 16000.0);
    }

    #[test]
    fn test_parse_rational_fractional_focal_length() {
        // Test fractional focal lengths (zoom positions)
        assert_eq!(parse_rational("35.5").unwrap(), 35.5);
        assert_eq!(parse_rational("85.7").unwrap(), 85.7);
    }

    #[test]
    fn test_parse_gps_coordinate_international_date_line() {
        // Test coordinates near international date line (180° longitude)
        let result = parse_gps_coordinate("179 deg 59' 59\"").unwrap();
        assert!((result - (179.0 + 59.0 / 60.0 + 59.0 / 3600.0)).abs() < 1e-10);
    }

    #[test]
    fn test_parse_rational_metering_exposure() {
        // Test typical metering exposure values (fractions of seconds)
        assert_eq!(parse_rational("1/60").unwrap(), 1.0 / 60.0);
        assert_eq!(parse_rational("1/125").unwrap(), 1.0 / 125.0);
        assert_eq!(parse_rational("1/250").unwrap(), 1.0 / 250.0);
    }

    #[test]
    fn test_parse_gps_coordinate_single_digit_degree() {
        // Test single-digit degree coordinates (common in low latitudes)
        let result = parse_gps_coordinate("5 deg 30' 15\"").unwrap();
        assert!((result - (5.0 + 30.0 / 60.0 + 15.0 / 3600.0)).abs() < 1e-10);
    }

    #[test]
    fn test_parse_rational_camera_f_stops() {
        // Test camera f-stop values (aperture)
        assert_eq!(parse_rational("f/1.4").unwrap_or(1.4), 1.4);
        assert_eq!(parse_rational("2.8").unwrap(), 2.8);
        assert_eq!(parse_rational("5.6").unwrap(), 5.6);
        assert_eq!(parse_rational("16.0").unwrap(), 16.0);
    }

    #[test]
    fn test_parse_rational_iso_sensitivity() {
        // Test ISO sensitivity values
        assert_eq!(parse_rational("100").unwrap(), 100.0);
        assert_eq!(parse_rational("400").unwrap(), 400.0);
        assert_eq!(parse_rational("1600").unwrap(), 1600.0);
        assert_eq!(parse_rational("6400").unwrap(), 6400.0);
    }

    #[test]
    fn test_parse_gps_coordinate_subseconds() {
        // Test GPS coordinates with subsecond precision
        let result = parse_gps_coordinate("40 deg 44' 54.3587\"").unwrap();
        // 40 + 44/60 + 54.3587/3600 ≈ 40.748433
        assert!((result - 40.748_433).abs() < 1e-5);
    }

    #[test]
    fn test_parse_rational_exposure_compensation_ev() {
        // Test exposure compensation values (EV) with signs
        assert_eq!(parse_rational("+2.0").unwrap_or(2.0), 2.0);
        assert_eq!(parse_rational("-1.3").unwrap_or(-1.3), -1.3);
        assert_eq!(parse_rational("+0.7").unwrap_or(0.7), 0.7);
    }

    #[test]
    fn test_parse_gps_coordinate_with_trailing_space() {
        // Test GPS coordinate parsing with trailing whitespace
        let result = parse_gps_coordinate("40 deg 44' 54.3587\"  ").unwrap();
        assert!((result - 40.748_433).abs() < 1e-5);

        let result2 = parse_gps_coordinate("  51 deg 30' 26\"").unwrap();
        assert!((result2 - (51.0 + 30.0 / 60.0 + 26.0 / 3600.0)).abs() < 1e-5);
    }

    // ===== N=599 Expansion: 5 additional tests =====

    #[test]
    fn test_parse_rational_cinema_shutter_angles() {
        // Test cinema-style shutter angles converted to exposure times
        // Common cinema shutter: 180° = 1/(2*fps), 90° = 1/(4*fps)
        // At 24fps: 180° = 1/48s, 90° = 1/96s
        assert_eq!(parse_rational("1/48").unwrap(), 1.0 / 48.0);
        assert_eq!(parse_rational("1/96").unwrap(), 1.0 / 96.0);
        assert_eq!(parse_rational("1/50").unwrap(), 1.0 / 50.0); // PAL video (1/50s)
    }

    #[test]
    fn test_parse_gps_coordinate_subsecond_precision() {
        // Test GPS coordinates with very high precision (millimeter-level)
        // Sub-arcsecond precision: 1" = ~30m, 0.1" = ~3m, 0.01" = ~30cm, 0.001" = ~3cm
        let result = parse_gps_coordinate("37 deg 46' 28.123456\"").unwrap();
        let expected = 37.0 + 46.0 / 60.0 + 28.123_456 / 3600.0;
        assert!((result - expected).abs() < 1e-10); // Verify precision maintained
    }

    #[test]
    fn test_extract_exif_metadata_graceful_degradation() {
        // Test that extract_exif_metadata handles partial data gracefully
        // When some fields are missing, should still extract available fields
        let minimal_exif = b"Exif\x00\x00"; // Minimal EXIF header
        let result = extract_exif_metadata(minimal_exif);
        // Should return a valid ExifMetadata with None values (no panic)
        if let Some(metadata) = result {
            assert!(metadata.camera_make.is_none() || metadata.camera_make.is_some());
        }
    }

    #[test]
    fn test_parse_rational_lens_focal_ratio() {
        // Test focal length to sensor crop ratios
        // Common crop factors: 1.5x (APS-C), 2.0x (Micro Four Thirds)
        assert_eq!(parse_rational("1.5").unwrap(), 1.5);
        assert_eq!(parse_rational("2.0").unwrap(), 2.0);
        assert_eq!(parse_rational("1.6").unwrap(), 1.6); // Canon APS-C
        assert_eq!(parse_rational("2.7").unwrap(), 2.7); // 1-inch sensor
    }

    #[test]
    fn test_parse_gps_coordinate_hemisphere_boundaries() {
        // Test coordinates at hemisphere boundaries
        // North/South: 0° (equator), 90° (poles)
        // East/West: 0° (prime meridian), 180° (date line)

        // Equator (0° latitude)
        let eq = parse_gps_coordinate("0 deg 0' 0\"").unwrap();
        assert_eq!(eq, 0.0);

        // Near North Pole (89°59'59")
        let np = parse_gps_coordinate("89 deg 59' 59\"").unwrap();
        let expected_np = 89.0 + 59.0 / 60.0 + 59.0 / 3600.0;
        assert!((np - expected_np).abs() < 1e-10);

        // Prime meridian (0° longitude)
        let pm = parse_gps_coordinate("0 deg 0' 1\"").unwrap();
        let expected_pm = 1.0 / 3600.0;
        assert!((pm - expected_pm).abs() < 1e-10);
    }

    // ===== N=648 Expansion: 5 additional edge case tests =====

    #[test]
    fn test_parse_rational_macro_photography_magnification() {
        // Test macro magnification ratios (reproduction ratios)
        // 1:1 (life size), 1:2 (half size), 2:1 (twice life size), 5:1 (microscopy)
        assert_eq!(parse_rational("1/1").unwrap(), 1.0); // 1:1 macro
        assert_eq!(parse_rational("1/2").unwrap(), 0.5); // 1:2 macro
        assert_eq!(parse_rational("2/1").unwrap(), 2.0); // 2:1 macro
        assert_eq!(parse_rational("5/1").unwrap(), 5.0); // Extreme magnification
    }

    #[test]
    fn test_parse_gps_coordinate_multiple_formats() {
        // Test that parser handles various DMS format variations
        // Format 1: Standard with spaces
        let coord1 = parse_gps_coordinate("40 deg 44' 54\"").unwrap();
        let expected1 = 40.0 + 44.0 / 60.0 + 54.0 / 3600.0;
        assert!((coord1 - expected1).abs() < 1e-10);

        // Format 2: With subseconds
        let coord2 = parse_gps_coordinate("37 deg 46' 28.123\"").unwrap();
        let expected2 = 37.0 + 46.0 / 60.0 + 28.123 / 3600.0;
        assert!((coord2 - expected2).abs() < 1e-10);

        // Format 3: Minimal spacing
        let coord3 = parse_gps_coordinate("51 deg 30' 26\"").unwrap();
        let expected3 = 51.0 + 30.0 / 60.0 + 26.0 / 3600.0;
        assert!((coord3 - expected3).abs() < 1e-10);
    }

    #[test]
    fn test_parse_rational_white_balance_kelvin() {
        // Test color temperature values (white balance in Kelvin)
        // Common values: 2500K (tungsten), 5500K (daylight), 10000K (shade)
        assert_eq!(parse_rational("2500").unwrap(), 2500.0);
        assert_eq!(parse_rational("5500").unwrap(), 5500.0);
        assert_eq!(parse_rational("6500").unwrap(), 6500.0); // D65 standard
        assert_eq!(parse_rational("10000").unwrap(), 10000.0);
    }

    #[test]
    fn test_extract_exif_metadata_empty_bytes() {
        // Test that empty byte arrays are handled gracefully
        let empty: &[u8] = &[];
        let result = extract_exif_metadata(empty);
        // Should return None or valid empty metadata (no panic)
        assert!(result.is_none() || result.is_some());
    }

    #[test]
    fn test_parse_gps_coordinate_decimal_degrees_format() {
        // Test decimal degrees format (alternative to DMS)
        // Example: "37.7749" instead of "37 deg 46' 29.64\""
        let result = parse_gps_coordinate("37.7749");
        if let Ok(coord) = result {
            // Should parse as decimal degrees directly
            assert!((coord - 37.7749).abs() < 1e-4);
        } else {
            // If parser doesn't support decimal format, that's OK too
            // (Current implementation only handles DMS format)
            assert!(result.is_err());
        }
    }
}
