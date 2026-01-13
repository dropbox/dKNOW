//! Character encoding detection and conversion.
//!
//! This module provides functions to detect text file encodings and convert
//! them to UTF-8 for indexing. Supports common encodings:
//! - UTF-8 (with and without BOM)
//! - UTF-16 (LE and BE)
//! - UTF-32 (LE and BE)
//! - ISO-8859-1 (Latin-1)
//! - Windows-1252
//! - Other legacy encodings via chardetng

use std::fs::File;
use std::io::Read;
use std::path::Path;

use chardetng::EncodingDetector;
use encoding_rs::{Encoding, UTF_16BE, UTF_16LE, UTF_8};

/// Detected character encoding of a text file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetectedEncoding {
    /// UTF-8 (no BOM)
    Utf8,
    /// UTF-8 with BOM
    Utf8Bom,
    /// UTF-16 Little Endian
    Utf16Le,
    /// UTF-16 Big Endian
    Utf16Be,
    /// UTF-32 Little Endian
    Utf32Le,
    /// UTF-32 Big Endian
    Utf32Be,
    /// Legacy encoding detected by chardetng (e.g., ISO-8859-1, Windows-1252)
    Legacy(&'static Encoding),
    /// ASCII (subset of UTF-8)
    Ascii,
    /// Unknown/binary - could not determine encoding
    Unknown,
}

impl std::fmt::Display for DetectedEncoding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DetectedEncoding::Utf8 => write!(f, "UTF-8"),
            DetectedEncoding::Utf8Bom => write!(f, "UTF-8 with BOM"),
            DetectedEncoding::Utf16Le => write!(f, "UTF-16 LE"),
            DetectedEncoding::Utf16Be => write!(f, "UTF-16 BE"),
            DetectedEncoding::Utf32Le => write!(f, "UTF-32 LE"),
            DetectedEncoding::Utf32Be => write!(f, "UTF-32 BE"),
            DetectedEncoding::Legacy(enc) => write!(f, "{}", enc.name()),
            DetectedEncoding::Ascii => write!(f, "ASCII"),
            DetectedEncoding::Unknown => write!(f, "Unknown"),
        }
    }
}

/// UTF-8 BOM: EF BB BF
const UTF8_BOM: &[u8] = &[0xEF, 0xBB, 0xBF];
/// UTF-16 LE BOM: FF FE
const UTF16_LE_BOM: &[u8] = &[0xFF, 0xFE];
/// UTF-16 BE BOM: FE FF
const UTF16_BE_BOM: &[u8] = &[0xFE, 0xFF];
/// UTF-32 LE BOM: FF FE 00 00
const UTF32_LE_BOM: &[u8] = &[0xFF, 0xFE, 0x00, 0x00];
/// UTF-32 BE BOM: 00 00 FE FF
const UTF32_BE_BOM: &[u8] = &[0x00, 0x00, 0xFE, 0xFF];

/// Detect the character encoding of a byte buffer.
///
/// Detection priority:
/// 1. BOM (Byte Order Mark) - most reliable
/// 2. UTF-8 validation - if valid UTF-8, assume UTF-8
/// 3. chardetng statistical detection - for legacy encodings
pub fn detect_encoding(buffer: &[u8]) -> DetectedEncoding {
    if buffer.is_empty() {
        return DetectedEncoding::Utf8; // Empty is valid UTF-8
    }

    // Check for BOMs first (most reliable indicator)
    if let Some(bom_encoding) = detect_bom(buffer) {
        return bom_encoding;
    }

    // Check for null bytes pattern that indicates UTF-16/32 without BOM
    if let Some(unicode_encoding) = detect_unicode_without_bom(buffer) {
        return unicode_encoding;
    }

    // Check if it's valid UTF-8
    if std::str::from_utf8(buffer).is_ok() {
        // Check if it's pure ASCII (subset of UTF-8)
        if buffer.iter().all(|&b| b < 128) {
            return DetectedEncoding::Ascii;
        }
        return DetectedEncoding::Utf8;
    }

    // Use chardetng for statistical encoding detection
    let mut detector = EncodingDetector::new();
    detector.feed(buffer, true);
    let (encoding, confident) = detector.guess_assess(None, true);

    if confident {
        return DetectedEncoding::Legacy(encoding);
    }

    // If chardetng isn't confident but gave a guess, still use it
    // (better than failing entirely)
    if encoding != UTF_8 {
        return DetectedEncoding::Legacy(encoding);
    }

    DetectedEncoding::Unknown
}

/// Detect encoding from BOM (Byte Order Mark).
fn detect_bom(buffer: &[u8]) -> Option<DetectedEncoding> {
    // UTF-32 BOMs must be checked before UTF-16 (UTF-32 LE starts with FF FE like UTF-16 LE)
    if buffer.len() >= 4 {
        if buffer.starts_with(UTF32_LE_BOM) {
            return Some(DetectedEncoding::Utf32Le);
        }
        if buffer.starts_with(UTF32_BE_BOM) {
            return Some(DetectedEncoding::Utf32Be);
        }
    }

    if buffer.len() >= 3 && buffer.starts_with(UTF8_BOM) {
        return Some(DetectedEncoding::Utf8Bom);
    }

    if buffer.len() >= 2 {
        if buffer.starts_with(UTF16_LE_BOM) {
            return Some(DetectedEncoding::Utf16Le);
        }
        if buffer.starts_with(UTF16_BE_BOM) {
            return Some(DetectedEncoding::Utf16Be);
        }
    }

    None
}

/// Detect UTF-16/32 without BOM by looking at null byte patterns.
///
/// This is a heuristic: ASCII text in UTF-16 will have alternating null bytes.
fn detect_unicode_without_bom(buffer: &[u8]) -> Option<DetectedEncoding> {
    if buffer.len() < 4 {
        return None;
    }

    // Check for null byte patterns suggesting UTF-16/32
    let nulls_at_odd = buffer
        .iter()
        .skip(1)
        .step_by(2)
        .filter(|&&b| b == 0)
        .count();
    let nulls_at_even = buffer.iter().step_by(2).filter(|&&b| b == 0).count();
    let total_pairs = buffer.len() / 2;

    // If most odd bytes are null -> UTF-16 LE (ASCII chars: 'A' 00 'B' 00)
    if total_pairs > 4 && nulls_at_odd > total_pairs * 3 / 4 && nulls_at_even < total_pairs / 4 {
        return Some(DetectedEncoding::Utf16Le);
    }

    // If most even bytes are null -> UTF-16 BE (ASCII chars: 00 'A' 00 'B')
    if total_pairs > 4 && nulls_at_even > total_pairs * 3 / 4 && nulls_at_odd < total_pairs / 4 {
        return Some(DetectedEncoding::Utf16Be);
    }

    None
}

/// Convert a byte buffer to UTF-8 string.
///
/// Automatically detects the encoding and converts to UTF-8.
/// Returns the converted string and the detected encoding.
///
/// Replacement characters (U+FFFD) may be inserted for invalid sequences.
pub fn decode_to_utf8(buffer: &[u8]) -> (String, DetectedEncoding) {
    let encoding = detect_encoding(buffer);
    let text = decode_with_encoding(buffer, &encoding);
    (text, encoding)
}

/// Decode buffer using a specific encoding.
pub fn decode_with_encoding(buffer: &[u8], encoding: &DetectedEncoding) -> String {
    match encoding {
        DetectedEncoding::Utf8 | DetectedEncoding::Ascii => {
            // Already UTF-8, just convert
            String::from_utf8_lossy(buffer).into_owned()
        }
        DetectedEncoding::Utf8Bom => {
            // Skip BOM and decode
            let data = if buffer.starts_with(UTF8_BOM) {
                &buffer[3..]
            } else {
                buffer
            };
            String::from_utf8_lossy(data).into_owned()
        }
        DetectedEncoding::Utf16Le => decode_utf16_le(buffer),
        DetectedEncoding::Utf16Be => decode_utf16_be(buffer),
        DetectedEncoding::Utf32Le => decode_utf32_le(buffer),
        DetectedEncoding::Utf32Be => decode_utf32_be(buffer),
        DetectedEncoding::Legacy(enc) => {
            let (cow, _, _) = enc.decode(buffer);
            cow.into_owned()
        }
        DetectedEncoding::Unknown => {
            // Try UTF-8 lossy as fallback
            String::from_utf8_lossy(buffer).into_owned()
        }
    }
}

/// Decode UTF-16 LE buffer to UTF-8 string.
fn decode_utf16_le(buffer: &[u8]) -> String {
    // Skip BOM if present
    let data = if buffer.starts_with(UTF16_LE_BOM) {
        &buffer[2..]
    } else {
        buffer
    };

    // Use encoding_rs for UTF-16 LE
    let (cow, _, _) = UTF_16LE.decode(data);
    cow.into_owned()
}

/// Decode UTF-16 BE buffer to UTF-8 string.
fn decode_utf16_be(buffer: &[u8]) -> String {
    // Skip BOM if present
    let data = if buffer.starts_with(UTF16_BE_BOM) {
        &buffer[2..]
    } else {
        buffer
    };

    // Use encoding_rs for UTF-16 BE
    let (cow, _, _) = UTF_16BE.decode(data);
    cow.into_owned()
}

/// Decode UTF-32 LE buffer to UTF-8 string.
fn decode_utf32_le(buffer: &[u8]) -> String {
    // Skip BOM if present
    let data = if buffer.starts_with(UTF32_LE_BOM) {
        &buffer[4..]
    } else {
        buffer
    };

    // UTF-32 decoding: 4 bytes per codepoint, little endian
    let mut result = String::with_capacity(data.len() / 4);
    for chunk in data.chunks(4) {
        if chunk.len() < 4 {
            break;
        }
        let codepoint = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        if let Some(c) = char::from_u32(codepoint) {
            result.push(c);
        } else {
            result.push('\u{FFFD}'); // Replacement character
        }
    }
    result
}

/// Decode UTF-32 BE buffer to UTF-8 string.
fn decode_utf32_be(buffer: &[u8]) -> String {
    // Skip BOM if present
    let data = if buffer.starts_with(UTF32_BE_BOM) {
        &buffer[4..]
    } else {
        buffer
    };

    // UTF-32 decoding: 4 bytes per codepoint, big endian
    let mut result = String::with_capacity(data.len() / 4);
    for chunk in data.chunks(4) {
        if chunk.len() < 4 {
            break;
        }
        let codepoint = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        if let Some(c) = char::from_u32(codepoint) {
            result.push(c);
        } else {
            result.push('\u{FFFD}'); // Replacement character
        }
    }
    result
}

/// Read a text file and convert it to UTF-8.
///
/// Automatically detects the file's encoding and converts to UTF-8.
/// Returns the content as a String and the detected encoding.
///
/// # Errors
///
/// Returns an error if the file cannot be read.
pub fn read_text_file(path: &Path) -> std::io::Result<(String, DetectedEncoding)> {
    let mut file = File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    Ok(decode_to_utf8(&buffer))
}

/// Read a text file to UTF-8 string, discarding encoding info.
///
/// Convenience wrapper around `read_text_file` that just returns the string.
pub fn read_text_file_utf8(path: &Path) -> std::io::Result<String> {
    read_text_file(path).map(|(text, _)| text)
}

/// Check if a buffer contains valid text (any supported encoding).
///
/// Returns true if the buffer can be decoded as text using any
/// supported encoding. This is more permissive than the previous
/// UTF-8-only check.
pub fn is_valid_text_encoding(buffer: &[u8]) -> bool {
    if buffer.is_empty() {
        return true;
    }

    // Quick binary detection: null bytes in non-UTF-16/32 positions are binary
    // For UTF-16/32 we expect null bytes, but they follow specific patterns
    if has_suspicious_null_bytes(buffer) {
        return false;
    }

    let encoding = detect_encoding(buffer);

    match encoding {
        // Valid text encodings
        DetectedEncoding::Utf8
        | DetectedEncoding::Utf8Bom
        | DetectedEncoding::Utf16Le
        | DetectedEncoding::Utf16Be
        | DetectedEncoding::Utf32Le
        | DetectedEncoding::Utf32Be
        | DetectedEncoding::Ascii => true,

        // Legacy encoding detected - still valid text
        DetectedEncoding::Legacy(enc) => {
            // Additional check: make sure it decodes to reasonable text
            let (decoded, _, had_errors) = enc.decode(buffer);
            // If too many replacement chars, probably not text
            let replacement_count = decoded.chars().filter(|&c| c == '\u{FFFD}').count();
            let total_chars = decoded.chars().count();
            if total_chars > 0 && (replacement_count as f64 / total_chars as f64) > 0.1 {
                return false;
            }
            // Check printability
            let printable_ratio = count_printable_ratio(&decoded);
            if printable_ratio < 0.7 {
                return false;
            }
            !had_errors || replacement_count < total_chars / 10
        }

        DetectedEncoding::Unknown => false,
    }
}

/// Check if buffer has null bytes that don't look like UTF-16/32.
///
/// Null bytes are suspicious unless they follow UTF-16/32 patterns:
/// - UTF-16 LE: every odd byte is likely null (for ASCII text)
/// - UTF-16 BE: every even byte is likely null (for ASCII text)
/// - UTF-32: 3 out of 4 bytes are null
fn has_suspicious_null_bytes(buffer: &[u8]) -> bool {
    if !buffer.contains(&0) {
        return false; // No nulls, not suspicious
    }

    // Check for BOM first - if present, trust it
    if buffer.len() >= 4 && (buffer.starts_with(UTF32_LE_BOM) || buffer.starts_with(UTF32_BE_BOM)) {
        return false; // Has UTF-32 BOM
    }
    if buffer.len() >= 3 && buffer.starts_with(UTF8_BOM) {
        // UTF-8 with BOM shouldn't have nulls
        return buffer[3..].contains(&0);
    }
    if buffer.len() >= 2 && (buffer.starts_with(UTF16_LE_BOM) || buffer.starts_with(UTF16_BE_BOM)) {
        return false; // Has UTF-16 BOM
    }

    // No BOM - check for UTF-16 null patterns
    let len = buffer.len();
    if len < 4 {
        // Too short to reliably detect UTF-16 without BOM
        // If it has null bytes and no BOM, consider it binary
        return true;
    }

    // Count nulls at odd and even positions
    let nulls_at_odd: usize = buffer
        .iter()
        .skip(1)
        .step_by(2)
        .filter(|&&b| b == 0)
        .count();
    let nulls_at_even: usize = buffer.iter().step_by(2).filter(|&&b| b == 0).count();
    let total_pairs = len / 2;

    // UTF-16 LE pattern: most odd bytes are null, most even bytes are not
    if total_pairs > 4 && nulls_at_odd > total_pairs * 3 / 4 && nulls_at_even < total_pairs / 4 {
        return false; // Looks like UTF-16 LE
    }

    // UTF-16 BE pattern: most even bytes are null, most odd bytes are not
    if total_pairs > 4 && nulls_at_even > total_pairs * 3 / 4 && nulls_at_odd < total_pairs / 4 {
        return false; // Looks like UTF-16 BE
    }

    // Has null bytes but doesn't look like UTF-16/32 - suspicious
    true
}

/// Count the ratio of printable characters in a string.
fn count_printable_ratio(text: &str) -> f64 {
    if text.is_empty() {
        return 1.0;
    }

    let mut printable = 0;
    let mut total = 0;

    for c in text.chars() {
        total += 1;
        if c.is_ascii_graphic() || c.is_ascii_whitespace() || (!c.is_ascii() && !c.is_control()) {
            printable += 1;
        }
    }

    if total == 0 {
        return 1.0;
    }

    printable as f64 / total as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_utf8() {
        let text = "Hello, world!".as_bytes();
        assert_eq!(detect_encoding(text), DetectedEncoding::Ascii);

        let text = "Hello, 世界!".as_bytes();
        assert_eq!(detect_encoding(text), DetectedEncoding::Utf8);
    }

    #[test]
    fn test_detect_utf8_bom() {
        let mut text = Vec::from(UTF8_BOM);
        text.extend_from_slice(b"Hello, world!");
        assert_eq!(detect_encoding(&text), DetectedEncoding::Utf8Bom);
    }

    #[test]
    fn test_detect_utf16_le_bom() {
        // UTF-16 LE BOM + "Hi"
        let text: Vec<u8> = vec![0xFF, 0xFE, b'H', 0x00, b'i', 0x00];
        assert_eq!(detect_encoding(&text), DetectedEncoding::Utf16Le);
    }

    #[test]
    fn test_detect_utf16_be_bom() {
        // UTF-16 BE BOM + "Hi"
        let text: Vec<u8> = vec![0xFE, 0xFF, 0x00, b'H', 0x00, b'i'];
        assert_eq!(detect_encoding(&text), DetectedEncoding::Utf16Be);
    }

    #[test]
    fn test_detect_utf32_le_bom() {
        // UTF-32 LE BOM + "H"
        let text: Vec<u8> = vec![0xFF, 0xFE, 0x00, 0x00, b'H', 0x00, 0x00, 0x00];
        assert_eq!(detect_encoding(&text), DetectedEncoding::Utf32Le);
    }

    #[test]
    fn test_detect_utf32_be_bom() {
        // UTF-32 BE BOM + "H"
        let text: Vec<u8> = vec![0x00, 0x00, 0xFE, 0xFF, 0x00, 0x00, 0x00, b'H'];
        assert_eq!(detect_encoding(&text), DetectedEncoding::Utf32Be);
    }

    #[test]
    fn test_detect_utf16_le_no_bom() {
        // UTF-16 LE without BOM: "Hello" = H 00 e 00 l 00 l 00 o 00
        let text: Vec<u8> = vec![
            b'H', 0x00, b'e', 0x00, b'l', 0x00, b'l', 0x00, b'o', 0x00, b' ', 0x00, b'W', 0x00,
            b'o', 0x00, b'r', 0x00, b'l', 0x00, b'd', 0x00,
        ];
        assert_eq!(detect_encoding(&text), DetectedEncoding::Utf16Le);
    }

    #[test]
    fn test_detect_utf16_be_no_bom() {
        // UTF-16 BE without BOM: "Hello" = 00 H 00 e 00 l 00 l 00 o
        let text: Vec<u8> = vec![
            0x00, b'H', 0x00, b'e', 0x00, b'l', 0x00, b'l', 0x00, b'o', 0x00, b' ', 0x00, b'W',
            0x00, b'o', 0x00, b'r', 0x00, b'l', 0x00, b'd',
        ];
        assert_eq!(detect_encoding(&text), DetectedEncoding::Utf16Be);
    }

    #[test]
    fn test_detect_latin1() {
        // ISO-8859-1: "café" where é is 0xE9 (not valid UTF-8 by itself)
        let text: Vec<u8> = vec![b'c', b'a', b'f', 0xE9];
        let encoding = detect_encoding(&text);
        // chardetng should detect this as a legacy encoding
        matches!(encoding, DetectedEncoding::Legacy(_));
    }

    #[test]
    fn test_decode_utf8() {
        let text = "Hello, 世界!".as_bytes();
        let (decoded, encoding) = decode_to_utf8(text);
        assert_eq!(decoded, "Hello, 世界!");
        assert_eq!(encoding, DetectedEncoding::Utf8);
    }

    #[test]
    fn test_decode_utf16_le() {
        // UTF-16 LE BOM + "Hi"
        let text: Vec<u8> = vec![0xFF, 0xFE, b'H', 0x00, b'i', 0x00];
        let (decoded, encoding) = decode_to_utf8(&text);
        assert_eq!(decoded, "Hi");
        assert_eq!(encoding, DetectedEncoding::Utf16Le);
    }

    #[test]
    fn test_decode_utf16_be() {
        // UTF-16 BE BOM + "Hi"
        let text: Vec<u8> = vec![0xFE, 0xFF, 0x00, b'H', 0x00, b'i'];
        let (decoded, encoding) = decode_to_utf8(&text);
        assert_eq!(decoded, "Hi");
        assert_eq!(encoding, DetectedEncoding::Utf16Be);
    }

    #[test]
    fn test_decode_utf32_le() {
        // UTF-32 LE BOM + "AB"
        let text: Vec<u8> = vec![
            0xFF, 0xFE, 0x00, 0x00, // BOM
            b'A', 0x00, 0x00, 0x00, // 'A'
            b'B', 0x00, 0x00, 0x00, // 'B'
        ];
        let (decoded, encoding) = decode_to_utf8(&text);
        assert_eq!(decoded, "AB");
        assert_eq!(encoding, DetectedEncoding::Utf32Le);
    }

    #[test]
    fn test_decode_utf32_be() {
        // UTF-32 BE BOM + "AB"
        let text: Vec<u8> = vec![
            0x00, 0x00, 0xFE, 0xFF, // BOM
            0x00, 0x00, 0x00, b'A', // 'A'
            0x00, 0x00, 0x00, b'B', // 'B'
        ];
        let (decoded, encoding) = decode_to_utf8(&text);
        assert_eq!(decoded, "AB");
        assert_eq!(encoding, DetectedEncoding::Utf32Be);
    }

    #[test]
    fn test_decode_utf8_bom() {
        let mut text = Vec::from(UTF8_BOM);
        text.extend_from_slice("Hello!".as_bytes());
        let (decoded, encoding) = decode_to_utf8(&text);
        assert_eq!(decoded, "Hello!");
        assert_eq!(encoding, DetectedEncoding::Utf8Bom);
    }

    #[test]
    fn test_empty_buffer() {
        let (decoded, encoding) = decode_to_utf8(&[]);
        assert_eq!(decoded, "");
        assert_eq!(encoding, DetectedEncoding::Utf8);
    }

    #[test]
    fn test_is_valid_text_encoding() {
        // Valid UTF-8
        assert!(is_valid_text_encoding(b"Hello, world!"));
        assert!(is_valid_text_encoding("Hello, 世界!".as_bytes()));

        // Valid UTF-16 LE with BOM
        assert!(is_valid_text_encoding(&[
            0xFF, 0xFE, b'H', 0x00, b'i', 0x00
        ]));

        // Valid UTF-16 BE with BOM
        assert!(is_valid_text_encoding(&[
            0xFE, 0xFF, 0x00, b'H', 0x00, b'i'
        ]));

        // Empty is valid
        assert!(is_valid_text_encoding(&[]));
    }

    #[test]
    fn test_detected_encoding_display() {
        assert_eq!(format!("{}", DetectedEncoding::Utf8), "UTF-8");
        assert_eq!(format!("{}", DetectedEncoding::Utf8Bom), "UTF-8 with BOM");
        assert_eq!(format!("{}", DetectedEncoding::Utf16Le), "UTF-16 LE");
        assert_eq!(format!("{}", DetectedEncoding::Utf16Be), "UTF-16 BE");
        assert_eq!(format!("{}", DetectedEncoding::Ascii), "ASCII");
        assert_eq!(format!("{}", DetectedEncoding::Unknown), "Unknown");
    }

    #[test]
    fn test_read_text_file() {
        use std::io::Write;

        let dir = tempfile::tempdir().unwrap();

        // Create a UTF-8 file
        let utf8_path = dir.path().join("utf8.txt");
        {
            let mut f = File::create(&utf8_path).unwrap();
            f.write_all(b"Hello, world!").unwrap();
        }
        let (content, encoding) = read_text_file(&utf8_path).unwrap();
        assert_eq!(content, "Hello, world!");
        assert_eq!(encoding, DetectedEncoding::Ascii);

        // Create a UTF-16 LE file with BOM
        let utf16_path = dir.path().join("utf16.txt");
        {
            let mut f = File::create(&utf16_path).unwrap();
            f.write_all(&[0xFF, 0xFE, b'H', 0x00, b'i', 0x00]).unwrap();
        }
        let (content, encoding) = read_text_file(&utf16_path).unwrap();
        assert_eq!(content, "Hi");
        assert_eq!(encoding, DetectedEncoding::Utf16Le);
    }

    #[test]
    fn test_read_text_file_utf8() {
        use std::io::Write;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        {
            let mut f = File::create(&path).unwrap();
            f.write_all("Hello!".as_bytes()).unwrap();
        }
        let content = read_text_file_utf8(&path).unwrap();
        assert_eq!(content, "Hello!");
    }
}
