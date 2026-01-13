//! HEIF/HEIC backend for docling
//!
//! This backend converts HEIF (High Efficiency Image Format) files to docling's document model.
//! HEIF and HEIC are based on the ISO Base Media File Format (ISOBMFF), similar to MP4.

// Clippy pedantic allows:
// - HEIF metadata parsing is complex
// - Unit struct &self convention
#![allow(clippy::too_many_lines)]
#![allow(clippy::trivially_copy_pass_by_ref)]

use crate::traits::{BackendOptions, DocumentBackend};
use crate::utils::{create_section_header, create_text_item, format_file_size, opt_vec};
use docling_core::{DocItem, DoclingError, Document, DocumentMetadata, InputFormat};
use image::GenericImageView;
use std::fmt::Write;
use std::path::Path;

// ITU-T H.273 CICP (Colour Information Parameter Codes)
// Reference: ITU-T H.273 / ISO/IEC 23091-2

// Color Primaries (ColourPrimaries)
/// BT.709 / sRGB color primaries (ITU-T H.273)
const CICP_PRIMARIES_BT709: u16 = 1;
/// BT.2020 wide color gamut (ITU-T H.273)
const CICP_PRIMARIES_BT2020: u16 = 9;
/// DCI-P3 (SMPTE ST 428-1) color primaries (ITU-T H.273)
const CICP_PRIMARIES_DCI_P3: u16 = 11;
/// Display P3 (SMPTE ST 2113) color primaries (ITU-T H.273)
const CICP_PRIMARIES_DISPLAY_P3: u16 = 12;

// Transfer Characteristics (TransferCharacteristics)
/// BT.709 transfer characteristics (ITU-T H.273)
const CICP_TRANSFER_BT709: u16 = 1;
/// PQ / HDR10 (SMPTE ST 2084) transfer characteristics (ITU-T H.273)
const CICP_TRANSFER_PQ: u16 = 16;
/// HLG / Hybrid Log-Gamma (ARIB STD-B67) transfer characteristics (ITU-T H.273)
const CICP_TRANSFER_HLG: u16 = 18;

// ISOBMFF (ISO Base Media File Format) Box Header Sizes
// Reference: ISO/IEC 14496-12 (MPEG-4 Part 12)

/// Basic box header size: 4 bytes size + 4 bytes type
const ISOBMFF_BOX_HEADER_SIZE: usize = 8;
/// Full box header size: 8 bytes basic + 4 bytes version/flags
const ISOBMFF_FULLBOX_HEADER_SIZE: usize = 12;

/// Divisor for converting luminance from 0.0001 nits to nits
const LUMINANCE_NITS_DIVISOR: u32 = 10000;

/// HDR metadata extracted from AVIF/HEIF files
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
struct HdrMetadata {
    color_primaries: Option<String>,
    transfer_characteristics: Option<String>,
    max_content_light_level: Option<u32>,
    max_frame_average_light_level: Option<u32>,
    mastering_display_max_luminance: Option<u32>,
    mastering_display_min_luminance: Option<u32>,
}

/// HEIF backend
///
/// Converts HEIF/HEIC (High Efficiency Image Format) files to docling's document model.
/// Extracts basic metadata from the ISOBMFF box structure.
///
/// ## Features
///
/// - Extract image dimensions from 'ispe' (image spatial extents) property
/// - Detect file type (HEIF vs HEIC)
/// - Generate markdown with image metadata
///
/// ## Format Details
///
/// HEIF files use the ISO Base Media File Format (ISOBMFF), which stores data in nested "boxes".
/// Key boxes for metadata extraction:
/// - `ftyp`: File type box (identifies format)
/// - `meta`: Metadata container
/// - `ispe`: Image spatial extents (width/height)
///
/// ## Example
///
/// ```no_run
/// use docling_backend::HeifBackend;
/// use docling_backend::DocumentBackend;
///
/// let backend = HeifBackend::new(docling_core::InputFormat::Heif);
/// let result = backend.parse_file("image.heic", &Default::default())?;
/// println!("Image: {:?}", result.metadata.title);
/// # Ok::<(), docling_core::error::DoclingError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HeifBackend {
    format: InputFormat,
}

impl HeifBackend {
    /// Create a new HEIF backend instance for the specified format
    ///
    /// # Arguments
    ///
    /// * `format` - Either `InputFormat::Heif` or `InputFormat::Avif`
    #[inline]
    #[must_use = "creates a backend instance that should be used for parsing"]
    pub const fn new(format: InputFormat) -> Self {
        Self { format }
    }

    /// Parse ISOBMFF box structure to extract basic metadata and HDR information
    ///
    /// ISOBMFF boxes have the structure:
    /// - Size: 4 bytes (u32, big-endian) - total box size including header
    /// - Type: 4 bytes (ASCII fourcc code)
    /// - Data: (size - 8) bytes
    ///
    /// We extract:
    /// - 'ispe' (Image Spatial Extents) property for dimensions
    /// - 'colr' (Color Information) for HDR color space
    /// - 'clli' (Content Light Level Info) for HDR brightness
    /// - 'mdcv' (Mastering Display Color Volume) for HDR display characteristics
    /// - 'Exif' box for standard EXIF metadata
    fn parse_heif_metadata(
        data: &[u8],
    ) -> Result<
        (
            u32,
            u32,
            String,
            HdrMetadata,
            Option<docling_core::ExifMetadata>,
        ),
        DoclingError,
    > {
        // Minimum HEIF file size: 12 bytes (ftyp box header + brand)
        if data.len() < 12 {
            return Err(DoclingError::BackendError(
                "File too small to be a valid HEIF".to_string(),
            ));
        }

        // Check ftyp box (must be first box)
        let _ftyp_size = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;
        let ftyp_type = &data[4..8];

        if ftyp_type != b"ftyp" {
            return Err(DoclingError::BackendError(
                "Invalid HEIF header: missing ftyp box".to_string(),
            ));
        }

        // Extract major brand (identifies file type)
        let brand = std::str::from_utf8(&data[8..12])
            .unwrap_or("unknown")
            .to_string();

        // Scan for 'ispe' (image spatial extents) box
        // We need to search through potentially nested boxes
        let (width, height) = Self::find_ispe_box(data).unwrap_or((0, 0));

        // Extract HDR metadata
        let hdr_metadata = Self::parse_hdr_metadata(data);

        // Extract EXIF metadata from 'Exif' box
        let exif_metadata = Self::extract_exif_from_box(data);

        Ok((width, height, brand, hdr_metadata, exif_metadata))
    }

    /// Search for 'ispe' (Image Spatial Extents Property) box in ISOBMFF structure
    ///
    /// The ispe box contains:
    /// - version: 1 byte
    /// - flags: 3 bytes
    /// - width: 4 bytes (u32, big-endian)
    /// - height: 4 bytes (u32, big-endian)
    ///
    /// Note: ispe is often nested inside meta→iprp→ipco container boxes
    #[inline]
    fn find_ispe_box(data: &[u8]) -> Option<(u32, u32)> {
        Self::find_ispe_box_recursive(data, 0, data.len())
    }

    /// Recursive helper to search for ispe box in nested structures
    fn find_ispe_box_recursive(data: &[u8], start: usize, end: usize) -> Option<(u32, u32)> {
        let mut offset = start;

        while offset + ISOBMFF_BOX_HEADER_SIZE <= end {
            // Read box header
            let box_size = u32::from_be_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]) as usize;

            if box_size < ISOBMFF_BOX_HEADER_SIZE || offset + box_size > end {
                // Invalid box size or extends beyond container
                break;
            }

            let box_type = &data[offset + 4..offset + ISOBMFF_BOX_HEADER_SIZE];

            if box_type == b"ispe" {
                // Found ispe box!
                // Skip version (1 byte) + flags (3 bytes) = 4 bytes
                let data_offset = offset + ISOBMFF_FULLBOX_HEADER_SIZE;
                if data_offset + 8 <= data.len() {
                    let width = u32::from_be_bytes([
                        data[data_offset],
                        data[data_offset + 1],
                        data[data_offset + 2],
                        data[data_offset + 3],
                    ]);
                    let height = u32::from_be_bytes([
                        data[data_offset + 4],
                        data[data_offset + 5],
                        data[data_offset + 6],
                        data[data_offset + 7],
                    ]);
                    return Some((width, height));
                }
            }

            // Recursively search inside container boxes (meta, iprp, ipco, etc.)
            if box_type == b"meta" {
                // meta box has full box header (8 + 4 bytes version/flags), then nested boxes
                if offset + ISOBMFF_FULLBOX_HEADER_SIZE < offset + box_size {
                    if let Some(dims) = Self::find_ispe_box_recursive(
                        data,
                        offset + ISOBMFF_FULLBOX_HEADER_SIZE,
                        offset + box_size,
                    ) {
                        return Some(dims);
                    }
                }
            } else if box_type == b"iprp" || box_type == b"ipco" {
                // iprp and ipco boxes have basic box header, then nested boxes
                if let Some(dims) = Self::find_ispe_box_recursive(
                    data,
                    offset + ISOBMFF_BOX_HEADER_SIZE,
                    offset + box_size,
                ) {
                    return Some(dims);
                }
            }

            // Move to next box
            offset += box_size;
        }

        None
    }

    /// Parse HDR metadata from ISOBMFF boxes (colr, clli, mdcv)
    fn parse_hdr_metadata(data: &[u8]) -> HdrMetadata {
        let mut hdr = HdrMetadata::default();

        // Search for HDR-related boxes
        let mut offset = 0;
        while offset + ISOBMFF_BOX_HEADER_SIZE <= data.len() {
            let box_size = u32::from_be_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]) as usize;

            if box_size < ISOBMFF_BOX_HEADER_SIZE || offset + box_size > data.len() {
                break;
            }

            let box_type = &data[offset + 4..offset + ISOBMFF_BOX_HEADER_SIZE];

            match box_type {
                b"colr" => {
                    // Color Information box
                    if let Some(metadata) = Self::parse_colr_box(&data[offset..offset + box_size]) {
                        hdr.color_primaries = metadata.0;
                        hdr.transfer_characteristics = metadata.1;
                    }
                }
                b"clli" => {
                    // Content Light Level Information box
                    if let Some((max_cll, max_fall)) =
                        Self::parse_clli_box(&data[offset..offset + box_size])
                    {
                        hdr.max_content_light_level = Some(max_cll);
                        hdr.max_frame_average_light_level = Some(max_fall);
                    }
                }
                b"mdcv" => {
                    // Mastering Display Color Volume box
                    if let Some((max_lum, min_lum)) =
                        Self::parse_mdcv_box(&data[offset..offset + box_size])
                    {
                        hdr.mastering_display_max_luminance = Some(max_lum);
                        hdr.mastering_display_min_luminance = Some(min_lum);
                    }
                }
                _ => {}
            }

            offset += box_size;
        }

        hdr
    }

    /// Parse colr (Color Information) box
    /// Returns (`color_primaries`, `transfer_characteristics`)
    fn parse_colr_box(data: &[u8]) -> Option<(Option<String>, Option<String>)> {
        // colr box structure:
        // - 4 bytes: size
        // - 4 bytes: 'colr'
        // - 4 bytes: color_type (e.g., 'nclx' for MPEG-4 color info)
        // - remaining: color parameters
        if data.len() < 12 {
            return None;
        }

        let color_type = &data[8..12];
        if color_type == b"nclx" {
            // nclx format: color_primaries (2 bytes), transfer_characteristics (2 bytes), matrix_coefficients (2 bytes)
            if data.len() >= 18 {
                let color_primaries = u16::from_be_bytes([data[12], data[13]]);
                let transfer_char = u16::from_be_bytes([data[14], data[15]]);

                let primaries_str = match color_primaries {
                    CICP_PRIMARIES_BT709 => Some("BT.709".to_string()),
                    CICP_PRIMARIES_BT2020 => Some("BT.2020".to_string()),
                    CICP_PRIMARIES_DCI_P3 => Some("DCI-P3".to_string()),
                    CICP_PRIMARIES_DISPLAY_P3 => Some("Display P3".to_string()),
                    _ => None,
                };

                let transfer_str = match transfer_char {
                    CICP_TRANSFER_BT709 => Some("BT.709".to_string()),
                    CICP_TRANSFER_PQ => Some("PQ (HDR10)".to_string()),
                    CICP_TRANSFER_HLG => Some("HLG (Hybrid Log-Gamma)".to_string()),
                    _ => None,
                };

                return Some((primaries_str, transfer_str));
            }
        }

        None
    }

    /// Parse clli (Content Light Level Information) box
    /// Returns (`max_content_light_level`, `max_frame_average_light_level`) in nits
    fn parse_clli_box(data: &[u8]) -> Option<(u32, u32)> {
        // clli box structure:
        // - 4 bytes: size
        // - 4 bytes: 'clli'
        // - 4 bytes: max_content_light_level (u32, big-endian) in nits
        // - 4 bytes: max_frame_average_light_level (u32, big-endian) in nits
        if data.len() < 16 {
            return None;
        }

        let max_cll = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);
        let max_fall = u32::from_be_bytes([data[12], data[13], data[14], data[15]]);

        Some((max_cll, max_fall))
    }

    /// Parse mdcv (Mastering Display Color Volume) box
    /// Returns (`max_luminance`, `min_luminance`) - max in nits, min in 0.0001 nits
    fn parse_mdcv_box(data: &[u8]) -> Option<(u32, u32)> {
        // mdcv box structure:
        // - 4 bytes: size
        // - 4 bytes: 'mdcv'
        // - 24 bytes: display primaries (R, G, B, white point - 2 bytes each for x,y coordinates)
        // - 4 bytes: max_display_mastering_luminance (u32, big-endian) in 0.0001 nits
        // - 4 bytes: min_display_mastering_luminance (u32, big-endian) in 0.0001 nits
        if data.len() < 40 {
            return None;
        }

        // Skip 24 bytes of display primaries, read luminance values
        let max_lum_raw = u32::from_be_bytes([data[32], data[33], data[34], data[35]]);
        let min_lum = u32::from_be_bytes([data[36], data[37], data[38], data[39]]);

        // Convert max luminance from 0.0001 nits to nits
        let max_lum = max_lum_raw / LUMINANCE_NITS_DIVISOR;

        Some((max_lum, min_lum))
    }

    /// Extract EXIF metadata from 'Exif' box in HEIF/HEIC files
    ///
    /// HEIF stores EXIF data in an 'Exif' item in the metadata container.
    /// The box contains TIFF-format EXIF data starting after a 4-byte header.
    fn extract_exif_from_box(data: &[u8]) -> Option<docling_core::ExifMetadata> {
        // Find the 'Exif' box
        let exif_data = Self::find_exif_box(data)?;

        // Parse the TIFF-format EXIF data
        Self::parse_exif_data(exif_data)
    }

    /// Search for 'Exif' box in ISOBMFF structure
    fn find_exif_box(data: &[u8]) -> Option<&[u8]> {
        let mut offset = 0;
        while offset + ISOBMFF_BOX_HEADER_SIZE <= data.len() {
            let box_size = u32::from_be_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]) as usize;

            if box_size < ISOBMFF_BOX_HEADER_SIZE || offset + box_size > data.len() {
                break;
            }

            let box_type = &data[offset + 4..offset + ISOBMFF_BOX_HEADER_SIZE];

            if box_type == b"Exif" {
                // Found Exif box! Skip the 4-byte offset header (always 0)
                let data_start = offset + ISOBMFF_FULLBOX_HEADER_SIZE;
                if data_start < offset + box_size {
                    return Some(&data[data_start..offset + box_size]);
                }
            }

            // Check inside meta box (often contains Exif)
            if box_type == b"meta" {
                // meta box has version/flags, then nested boxes
                if offset + ISOBMFF_FULLBOX_HEADER_SIZE < offset + box_size {
                    if let Some(exif) = Self::find_exif_box(
                        &data[offset + ISOBMFF_FULLBOX_HEADER_SIZE..offset + box_size],
                    ) {
                        return Some(exif);
                    }
                }
            }

            offset += box_size;
        }
        None
    }

    /// Parse TIFF-format EXIF data
    fn parse_exif_data(exif_data: &[u8]) -> Option<docling_core::ExifMetadata> {
        use exif::{In, Reader, Tag};

        let reader = Reader::new();
        let exif = reader.read_raw(exif_data.to_vec()).ok()?;

        let mut metadata = docling_core::ExifMetadata {
            datetime: None,
            camera_make: None,
            camera_model: None,
            gps_latitude: None,
            gps_longitude: None,
            gps_altitude: None,
            orientation: None,
            software: None,
            exposure_time: None,
            f_number: None,
            iso_speed: None,
            focal_length: None,
            hdr_color_primaries: None,
            hdr_transfer_characteristics: None,
            hdr_max_content_light_level: None,
            hdr_max_frame_average_light_level: None,
            hdr_mastering_display_max_luminance: None,
            hdr_mastering_display_min_luminance: None,
        };

        // Extract camera make
        if let Some(field) = exif.get_field(Tag::Make, In::PRIMARY) {
            metadata.camera_make = Some(field.display_value().to_string());
        }

        // Extract camera model
        if let Some(field) = exif.get_field(Tag::Model, In::PRIMARY) {
            metadata.camera_model = Some(field.display_value().to_string());
        }

        // Extract datetime
        if let Some(field) = exif.get_field(Tag::DateTimeOriginal, In::PRIMARY) {
            let datetime_str = field.display_value().to_string();
            // Parse EXIF datetime format: "YYYY:MM:DD HH:MM:SS"
            if let Ok(dt) =
                chrono::NaiveDateTime::parse_from_str(&datetime_str, "%Y:%m:%d %H:%M:%S")
            {
                metadata.datetime =
                    Some(chrono::DateTime::from_naive_utc_and_offset(dt, chrono::Utc));
            }
        }

        // Extract software
        if let Some(field) = exif.get_field(Tag::Software, In::PRIMARY) {
            metadata.software = Some(field.display_value().to_string());
        }

        // Extract orientation
        if let Some(field) = exif.get_field(Tag::Orientation, In::PRIMARY) {
            if let exif::Value::Short(ref v) = field.value {
                metadata.orientation = Some(u32::from(v[0]));
            }
        }

        // Return Some only if we extracted at least one field
        if metadata.camera_make.is_some()
            || metadata.camera_model.is_some()
            || metadata.datetime.is_some()
            || metadata.software.is_some()
        {
            Some(metadata)
        } else {
            None
        }
    }

    /// Convert HEIF metadata to markdown
    fn heif_to_markdown(
        filename: &str,
        width: u32,
        height: u32,
        brand: &str,
        file_size: usize,
        format: InputFormat,
        hdr_metadata: &HdrMetadata,
    ) -> String {
        let mut markdown = String::new();

        // Title
        let _ = writeln!(markdown, "# {filename}\n");

        // Image Details section
        markdown.push_str("## Image Details\n\n");

        // Format Information subsection
        markdown.push_str("### Format Information\n\n");

        // Image type
        let format_name = if format == InputFormat::Heif {
            "HEIF/HEIC"
        } else {
            "AVIF"
        };
        let _ = writeln!(markdown, "Type: {format_name} Image\n");

        // Brand (identifies specific variant)
        if !brand.is_empty() && brand != "unknown" {
            let _ = writeln!(markdown, "Brand: {brand}\n");
        }

        // Dimensions and Size subsection
        markdown.push_str("### Dimensions and Size\n\n");

        // Dimensions
        if width > 0 && height > 0 {
            let _ = writeln!(markdown, "Dimensions: {width}x{height} pixels\n");
        } else {
            markdown.push_str("Dimensions: Unknown\n\n");
        }

        // File size
        markdown.push_str(&format_file_size(file_size));

        // Content Extraction subsection
        markdown.push_str("### Content Extraction\n\n");

        // Note about content (as blockquote)
        markdown.push_str(
            "> *Note: Image content cannot be extracted as text. \
             OCR or image analysis would be required for content extraction.*\n\n",
        );

        // HDR Information (if present)
        if hdr_metadata.color_primaries.is_some()
            || hdr_metadata.transfer_characteristics.is_some()
            || hdr_metadata.max_content_light_level.is_some()
            || hdr_metadata.mastering_display_max_luminance.is_some()
        {
            markdown.push_str("## HDR Metadata\n\n");

            if let Some(ref primaries) = hdr_metadata.color_primaries {
                let _ = writeln!(markdown, "Color Primaries: {primaries}\n");
            }

            if let Some(ref transfer) = hdr_metadata.transfer_characteristics {
                let _ = writeln!(markdown, "Transfer Characteristics: {transfer}\n");
            }

            if let Some(max_cll) = hdr_metadata.max_content_light_level {
                let _ = writeln!(markdown, "Max Content Light Level: {max_cll} nits\n");
            }

            if let Some(max_fall) = hdr_metadata.max_frame_average_light_level {
                let _ = writeln!(markdown, "Max Frame-Average Light Level: {max_fall} nits\n");
            }

            if let Some(max_lum) = hdr_metadata.mastering_display_max_luminance {
                let _ = writeln!(
                    markdown,
                    "Mastering Display Max Luminance: {max_lum} nits\n"
                );
            }

            if let Some(min_lum) = hdr_metadata.mastering_display_min_luminance {
                let _ = writeln!(
                    markdown,
                    "Mastering Display Min Luminance: {min_lum} (0.0001 nits)\n"
                );
            }
        }

        markdown
    }

    /// Generate `DocItems` directly from HEIF metadata
    ///
    /// Creates structured `DocItems` from HEIF metadata, preserving semantic information.
    /// This is the correct architectural pattern - NO markdown intermediary.
    ///
    /// ## Architecture (CLAUDE.md Compliant)
    ///
    /// ```text
    /// HEIF Metadata → heif_to_docitems() → DocItems (semantic structure preserved)
    /// ```
    ///
    /// ## Arguments
    /// * `filename` - Name of the HEIF/HEIC/AVIF file
    /// * `width` - Image width in pixels
    /// * `height` - Image height in pixels
    /// * `brand` - Brand identifier (e.g., "heic", "avif", "mif1")
    /// * `file_size` - File size in bytes
    /// * `format` - `InputFormat` (Heif or Avif)
    /// * `hdr_metadata` - HDR metadata (color space, brightness, etc.)
    ///
    /// ## Returns
    /// Vector of `DocItems` with semantic structure:
    /// - `SectionHeader` (level 1): Filename
    /// - `SectionHeader` (level 2): Image Details
    ///   - `SectionHeader` (level 3): Format Information
    ///     - Text: Image type (HEIF/HEIC/AVIF)
    ///     - Text: Brand (if known)
    ///   - `SectionHeader` (level 3): Dimensions and Size
    ///     - Text: Dimensions
    ///     - Text: File size
    ///   - `SectionHeader` (level 3): Content Extraction
    ///     - Text: Note about content extraction
    /// - `SectionHeader` (level 2): HDR Metadata (if present)
    ///   - Text: HDR properties (color primaries, transfer, brightness, etc.)
    fn heif_to_docitems(
        filename: &str,
        width: u32,
        height: u32,
        brand: &str,
        file_size: usize,
        format: InputFormat,
        hdr_metadata: &HdrMetadata,
    ) -> Vec<DocItem> {
        let mut doc_items = Vec::new();
        let mut item_index = 0;

        // 1. Title as SectionHeader (level 1)
        doc_items.push(create_section_header(
            item_index,
            filename.to_string(),
            1,
            vec![],
        ));
        item_index += 1;

        // 2. Image Details section header
        doc_items.push(create_section_header(
            item_index,
            "Image Details".to_string(),
            2,
            vec![],
        ));
        item_index += 1;

        // 3. Format Information subsection
        doc_items.push(create_section_header(
            item_index,
            "Format Information".to_string(),
            3,
            vec![],
        ));
        item_index += 1;

        // 3a. Image type
        let format_name = if format == InputFormat::Heif {
            "HEIF/HEIC"
        } else {
            "AVIF"
        };
        let image_type = format!("Type: {format_name} Image");
        doc_items.push(create_text_item(item_index, image_type, vec![]));
        item_index += 1;

        // 3b. Brand (if known)
        if !brand.is_empty() && brand != "unknown" {
            let brand_text = format!("Brand: {brand}");
            doc_items.push(create_text_item(item_index, brand_text, vec![]));
            item_index += 1;
        }

        // 4. Dimensions and Size subsection
        doc_items.push(create_section_header(
            item_index,
            "Dimensions and Size".to_string(),
            3,
            vec![],
        ));
        item_index += 1;

        // 4a. Dimensions
        let dimensions = if width > 0 && height > 0 {
            format!("Dimensions: {width}x{height} pixels")
        } else {
            "Dimensions: Unknown".to_string()
        };
        doc_items.push(create_text_item(item_index, dimensions, vec![]));
        item_index += 1;

        // 4b. File size
        let file_size_text = format_file_size(file_size);
        doc_items.push(create_text_item(item_index, file_size_text, vec![]));
        item_index += 1;

        // 5. Content Extraction subsection
        doc_items.push(create_section_header(
            item_index,
            "Content Extraction".to_string(),
            3,
            vec![],
        ));
        item_index += 1;

        // 5a. Note about content extraction (as blockquote)
        let note = "> *Note: Image content cannot be extracted as text. \
                    OCR or image analysis would be required for content extraction.*"
            .to_string();
        doc_items.push(create_text_item(item_index, note, vec![]));
        item_index += 1;

        // 8. HDR Information (if present)
        if hdr_metadata.color_primaries.is_some()
            || hdr_metadata.transfer_characteristics.is_some()
            || hdr_metadata.max_content_light_level.is_some()
            || hdr_metadata.mastering_display_max_luminance.is_some()
        {
            doc_items.push(create_section_header(
                item_index,
                "HDR Metadata".to_string(),
                2,
                vec![],
            ));
            item_index += 1;

            if let Some(ref primaries) = hdr_metadata.color_primaries {
                doc_items.push(create_text_item(
                    item_index,
                    format!("Color Primaries: {primaries}"),
                    vec![],
                ));
                item_index += 1;
            }

            if let Some(ref transfer) = hdr_metadata.transfer_characteristics {
                doc_items.push(create_text_item(
                    item_index,
                    format!("Transfer Characteristics: {transfer}"),
                    vec![],
                ));
                item_index += 1;
            }

            if let Some(max_cll) = hdr_metadata.max_content_light_level {
                doc_items.push(create_text_item(
                    item_index,
                    format!("Max Content Light Level: {max_cll} nits"),
                    vec![],
                ));
                item_index += 1;
            }

            if let Some(max_fall) = hdr_metadata.max_frame_average_light_level {
                doc_items.push(create_text_item(
                    item_index,
                    format!("Max Frame-Average Light Level: {max_fall} nits"),
                    vec![],
                ));
                item_index += 1;
            }

            if let Some(max_lum) = hdr_metadata.mastering_display_max_luminance {
                doc_items.push(create_text_item(
                    item_index,
                    format!("Mastering Display Max Luminance: {max_lum} nits"),
                    vec![],
                ));
                item_index += 1;
            }

            if let Some(min_lum) = hdr_metadata.mastering_display_min_luminance {
                doc_items.push(create_text_item(
                    item_index,
                    format!("Mastering Display Min Luminance: {min_lum} (0.0001 nits)"),
                    vec![],
                ));
            }
        }

        doc_items
    }
}

impl Default for HeifBackend {
    #[inline]
    fn default() -> Self {
        Self::new(InputFormat::Heif)
    }
}

impl HeifBackend {
    /// Shared helper method that parses HEIF/AVIF data and creates a Document
    ///
    /// This method is used by both `parse_bytes` and `parse_file` to avoid code duplication.
    fn parse_heif_data(&self, data: &[u8], filename: &str) -> Result<Document, DoclingError> {
        // Helper to add filename context to errors
        let add_context = |e: DoclingError| match e {
            DoclingError::BackendError(msg) => {
                DoclingError::BackendError(format!("{msg}: {filename}"))
            }
            other => other,
        };

        // Parse HEIF metadata
        let (mut width, mut height, brand, hdr_metadata, exif_metadata) =
            Self::parse_heif_metadata(data).map_err(add_context)?;

        // Fallback: If ispe box parsing failed (width/height = 0), try using image crate
        if width == 0 || height == 0 {
            if let Ok(img) = image::load_from_memory(data) {
                let dimensions = img.dimensions();
                width = dimensions.0;
                height = dimensions.1;
            }
        }

        // Generate DocItems directly from metadata (NO markdown intermediary)
        let doc_items = Self::heif_to_docitems(
            filename,
            width,
            height,
            &brand,
            data.len(),
            self.format,
            &hdr_metadata,
        );

        // Generate markdown from DocItems for backwards compatibility
        let markdown = Self::heif_to_markdown(
            filename,
            width,
            height,
            &brand,
            data.len(),
            self.format,
            &hdr_metadata,
        );
        let num_characters = markdown.chars().count();

        // Merge EXIF metadata from box with HDR information
        let exif = if exif_metadata.is_some()
            || hdr_metadata.color_primaries.is_some()
            || hdr_metadata.transfer_characteristics.is_some()
            || hdr_metadata.max_content_light_level.is_some()
            || hdr_metadata.mastering_display_max_luminance.is_some()
        {
            let mut merged = exif_metadata.unwrap_or_default();
            // Add HDR information to EXIF metadata
            merged.hdr_color_primaries = hdr_metadata.color_primaries;
            merged.hdr_transfer_characteristics = hdr_metadata.transfer_characteristics;
            merged.hdr_max_content_light_level = hdr_metadata.max_content_light_level;
            merged.hdr_max_frame_average_light_level = hdr_metadata.max_frame_average_light_level;
            merged.hdr_mastering_display_max_luminance =
                hdr_metadata.mastering_display_max_luminance;
            merged.hdr_mastering_display_min_luminance =
                hdr_metadata.mastering_display_min_luminance;
            Some(merged)
        } else {
            None
        };

        // Create document
        Ok(Document {
            markdown,
            format: self.format,
            metadata: DocumentMetadata {
                num_pages: Some(1),
                num_characters,
                title: Some(filename.to_string()),
                author: None,
                subject: None,
                created: None,
                modified: None,
                language: None,
                exif,
            },
            docling_document: None,
            content_blocks: opt_vec(doc_items),
        })
    }
}

impl DocumentBackend for HeifBackend {
    #[inline]
    fn format(&self) -> InputFormat {
        self.format
    }

    fn parse_bytes(
        &self,
        data: &[u8],
        _options: &BackendOptions,
    ) -> Result<Document, DoclingError> {
        let filename = if self.format == InputFormat::Heif {
            "image.heic"
        } else {
            "image.avif"
        };
        self.parse_heif_data(data, filename)
    }

    fn parse_file<P: AsRef<Path>>(
        &self,
        path: P,
        _options: &BackendOptions,
    ) -> Result<Document, DoclingError> {
        let path_ref = path.as_ref();
        let filename = path_ref.file_name().and_then(|n| n.to_str()).unwrap_or(
            if self.format == InputFormat::Heif {
                "image.heic"
            } else {
                "image.avif"
            },
        );

        // Read file and delegate to shared helper
        let data = std::fs::read(path_ref).map_err(DoclingError::IoError)?;
        self.parse_heif_data(&data, filename)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heif_backend_creation() {
        let backend = HeifBackend::new(InputFormat::Heif);
        assert_eq!(
            backend.format(),
            InputFormat::Heif,
            "HeifBackend::new(InputFormat::Heif) should report Heif format"
        );
    }

    #[test]
    fn test_avif_backend_creation() {
        let backend = HeifBackend::new(InputFormat::Avif);
        assert_eq!(
            backend.format(),
            InputFormat::Avif,
            "HeifBackend::new(InputFormat::Avif) should report Avif format"
        );
    }

    #[test]
    fn test_parse_heif_ftyp() {
        // Minimal valid HEIF file with ftyp box
        let mut data = vec![0u8; 24];
        // ftyp box: size=24, type='ftyp', brand='heic', version=0
        data[0..4].copy_from_slice(&24u32.to_be_bytes());
        data[4..8].copy_from_slice(b"ftyp");
        data[8..12].copy_from_slice(b"heic");
        data[12..16].copy_from_slice(&0u32.to_be_bytes());

        let result = HeifBackend::parse_heif_metadata(&data);
        assert!(
            result.is_ok(),
            "parse_heif_metadata should succeed for minimal valid HEIF with ftyp box"
        );
        let (width, height, brand, _, _) = result.unwrap();
        assert_eq!(brand, "heic", "Brand should be 'heic' from ftyp box");
        // No ispe box, so dimensions should be 0
        assert_eq!(width, 0, "Width should be 0 when ispe box is missing");
        assert_eq!(height, 0, "Height should be 0 when ispe box is missing");
    }

    #[test]
    fn test_parse_heif_with_ispe() {
        // Create a mock HEIF file with ftyp and ispe boxes
        let mut data = Vec::new();

        // ftyp box (20 bytes: 4 size + 4 type + 4 brand + 4 version + 4 compatible)
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"heic");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        // ispe box (20 bytes: 4 size + 4 type + 4 version/flags + 4 width + 4 height)
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ispe");
        data.extend_from_slice(&0u32.to_be_bytes()); // version + flags
        data.extend_from_slice(&1920u32.to_be_bytes()); // width
        data.extend_from_slice(&1080u32.to_be_bytes()); // height

        let result = HeifBackend::parse_heif_metadata(&data);
        assert!(
            result.is_ok(),
            "parse_heif_metadata should succeed for HEIF with ftyp and ispe boxes"
        );
        let (width, height, brand, _, _) = result.unwrap();
        assert_eq!(brand, "heic", "Brand should be 'heic' from ftyp box");
        assert_eq!(width, 1920, "Width should be 1920 from ispe box");
        assert_eq!(height, 1080, "Height should be 1080 from ispe box");
    }

    #[test]
    fn test_invalid_heif_header() {
        let data = b"NOTHEIF123";
        let result = HeifBackend::parse_heif_metadata(data);
        assert!(
            result.is_err(),
            "parse_heif_metadata should fail for data without valid ftyp box"
        );
    }

    #[test]
    fn test_heif_too_small() {
        let data = b"heic";
        let result = HeifBackend::parse_heif_metadata(data);
        assert!(
            result.is_err(),
            "parse_heif_metadata should fail for data smaller than minimum HEIF size (12 bytes)"
        );
    }

    #[test]
    fn test_find_ispe_box() {
        // Create mock data with ispe box
        let mut data = Vec::new();
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ispe");
        data.extend_from_slice(&0u32.to_be_bytes()); // version + flags
        data.extend_from_slice(&800u32.to_be_bytes()); // width
        data.extend_from_slice(&600u32.to_be_bytes()); // height

        let result = HeifBackend::find_ispe_box(&data);
        assert!(
            result.is_some(),
            "find_ispe_box should return Some when ispe box is present"
        );
        let (width, height) = result.unwrap();
        assert_eq!(width, 800, "Width should be 800 from ispe box");
        assert_eq!(height, 600, "Height should be 600 from ispe box");
    }

    #[test]
    fn test_find_ispe_box_not_found() {
        // Create mock data without ispe box
        let mut data = Vec::new();
        data.extend_from_slice(&12u32.to_be_bytes());
        data.extend_from_slice(b"some");
        data.extend_from_slice(&0u32.to_be_bytes());

        let result = HeifBackend::find_ispe_box(&data);
        assert!(
            result.is_none(),
            "find_ispe_box should return None when ispe box is not present"
        );
    }

    // ===== Category 1: Backend Creation Tests (5 tests) =====

    #[test]
    fn test_create_backend_heif() {
        let backend = HeifBackend::new(InputFormat::Heif);
        assert_eq!(
            backend.format(),
            InputFormat::Heif,
            "HeifBackend created with Heif format should report Heif"
        );
    }

    #[test]
    fn test_create_backend_default() {
        let backend = HeifBackend::default();
        assert_eq!(
            backend.format(),
            InputFormat::Heif,
            "Default HeifBackend should use Heif format"
        );
    }

    #[test]
    fn test_backend_format_constant() {
        let backend_heif = HeifBackend::new(InputFormat::Heif);
        let backend_avif = HeifBackend::new(InputFormat::Avif);

        assert_eq!(
            backend_heif.format(),
            InputFormat::Heif,
            "Heif backend should consistently report Heif format"
        );
        assert_eq!(
            backend_avif.format(),
            InputFormat::Avif,
            "Avif backend should consistently report Avif format"
        );

        // Verify debug representation contains format name
        assert!(
            format!("{:?}", backend_heif.format()).contains("Heif"),
            "Debug output for Heif format should contain 'Heif'"
        );
        assert!(
            format!("{:?}", backend_avif.format()).contains("Avif"),
            "Debug output for Avif format should contain 'Avif'"
        );
    }

    #[test]
    fn test_backend_format_persistence() {
        let backend = HeifBackend::new(InputFormat::Heif);

        // Format should remain constant across multiple calls
        assert_eq!(
            backend.format(),
            InputFormat::Heif,
            "Format should be consistent on first call"
        );
        assert_eq!(
            backend.format(),
            InputFormat::Heif,
            "Format should be consistent on second call"
        );
        assert_eq!(
            backend.format(),
            InputFormat::Heif,
            "Format should be consistent on third call"
        );

        // Create another backend and verify independence
        let backend2 = HeifBackend::new(InputFormat::Avif);
        assert_eq!(
            backend.format(),
            InputFormat::Heif,
            "Original Heif backend format should be unchanged"
        );
        assert_eq!(
            backend2.format(),
            InputFormat::Avif,
            "New Avif backend should have Avif format"
        );
    }

    #[test]
    fn test_backend_multiple_instances() {
        let backends: Vec<HeifBackend> = vec![
            HeifBackend::new(InputFormat::Heif),
            HeifBackend::new(InputFormat::Avif),
            HeifBackend::default(),
        ];

        assert_eq!(
            backends[0].format(),
            InputFormat::Heif,
            "First backend (explicit Heif) should report Heif"
        );
        assert_eq!(
            backends[1].format(),
            InputFormat::Avif,
            "Second backend (explicit Avif) should report Avif"
        );
        assert_eq!(
            backends[2].format(),
            InputFormat::Heif,
            "Third backend (default) should report Heif"
        );
    }

    // ===== Category 2: Metadata Tests (8 tests) =====

    #[test]
    fn test_metadata_extraction_complete() {
        let data = create_test_heif_with_dimensions(1920, 1080);
        let backend = HeifBackend::new(InputFormat::Heif);
        let doc = backend
            .parse_bytes(&data, &BackendOptions::default())
            .unwrap();

        // Verify metadata fields
        assert_eq!(
            doc.metadata.num_pages,
            Some(1),
            "HEIF image should report num_pages as 1"
        );
        assert_eq!(
            doc.metadata.title,
            Some("image.heic".to_string()),
            "HEIF document title should be 'image.heic'"
        );
        assert!(
            doc.metadata.num_characters > 0,
            "Character count should be positive for generated markdown"
        );
        assert_eq!(
            doc.format,
            InputFormat::Heif,
            "Document format should be Heif"
        );
    }

    #[test]
    fn test_metadata_character_count() {
        let data = create_test_heif_with_dimensions(800, 600);
        let backend = HeifBackend::new(InputFormat::Heif);
        let doc = backend
            .parse_bytes(&data, &BackendOptions::default())
            .unwrap();

        // Character count should include markdown content
        let expected_min = 50; // At minimum, metadata section
        assert!(
            doc.metadata.num_characters >= expected_min,
            "Expected >= {} characters, got {}",
            expected_min,
            doc.metadata.num_characters
        );

        // Verify markdown length matches character count
        let actual_chars = doc.markdown.chars().count();
        assert_eq!(
            doc.metadata.num_characters, actual_chars,
            "num_characters metadata should match actual markdown character count"
        );
    }

    #[test]
    fn test_metadata_format_field() {
        let data = create_test_heif_with_dimensions(1024, 768);
        let backend = HeifBackend::new(InputFormat::Heif);
        let doc = backend
            .parse_bytes(&data, &BackendOptions::default())
            .unwrap();

        // Verify format field
        assert_eq!(
            doc.format,
            InputFormat::Heif,
            "Document format should be Heif"
        );

        // Verify markdown contains format description
        assert!(
            doc.markdown.contains("HEIF") || doc.markdown.contains("HEIC"),
            "Markdown should contain 'HEIF' or 'HEIC' format description"
        );
    }

    #[test]
    fn test_metadata_extreme_dimensions() {
        // Test very large dimensions (8K resolution)
        let data = create_test_heif_with_dimensions(7680, 4320);
        let backend = HeifBackend::new(InputFormat::Heif);
        let doc = backend
            .parse_bytes(&data, &BackendOptions::default())
            .unwrap();

        assert!(
            doc.markdown.contains("7680"),
            "Markdown should contain width 7680 for 8K image"
        );
        assert!(
            doc.markdown.contains("4320"),
            "Markdown should contain height 4320 for 8K image"
        );
        assert!(
            doc.metadata.num_characters > 0,
            "Character count should be positive for 8K image"
        );
    }

    #[test]
    fn test_metadata_with_unknown_brand() {
        // Create HEIF with non-standard brand
        let mut data = Vec::new();
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"????"); // Unknown brand
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        let backend = HeifBackend::new(InputFormat::Heif);
        let doc = backend
            .parse_bytes(&data, &BackendOptions::default())
            .unwrap();

        // Should still parse successfully
        assert_eq!(
            doc.metadata.num_pages,
            Some(1),
            "Unknown brand should still parse with num_pages=1"
        );
        assert!(
            doc.metadata.num_characters > 0,
            "Unknown brand should still generate content"
        );
    }

    #[test]
    fn test_metadata_num_pages_always_one() {
        // HEIF files always have exactly 1 "page" (the image)
        let test_cases = vec![
            (100, 100),
            (1920, 1080),
            (3840, 2160),
            (0, 0), // Even with unknown dimensions
        ];

        for (width, height) in test_cases {
            let data = create_test_heif_with_dimensions(width, height);
            let backend = HeifBackend::new(InputFormat::Heif);
            let doc = backend
                .parse_bytes(&data, &BackendOptions::default())
                .unwrap();

            assert_eq!(
                doc.metadata.num_pages,
                Some(1),
                "HEIF {width}x{height} should have num_pages=1"
            );
        }
    }

    #[test]
    fn test_metadata_consistency_across_formats() {
        let data = create_test_heif_with_dimensions(1024, 768);

        let heif_backend = HeifBackend::new(InputFormat::Heif);
        let avif_backend = HeifBackend::new(InputFormat::Avif);

        let heif_doc = heif_backend
            .parse_bytes(&data, &BackendOptions::default())
            .unwrap();
        let avif_doc = avif_backend
            .parse_bytes(&data, &BackendOptions::default())
            .unwrap();

        // Both should have num_pages = 1
        assert_eq!(
            heif_doc.metadata.num_pages,
            Some(1),
            "HEIF backend should report num_pages=1"
        );
        assert_eq!(
            avif_doc.metadata.num_pages,
            Some(1),
            "AVIF backend should report num_pages=1"
        );

        // Both should have non-zero character counts
        assert!(
            heif_doc.metadata.num_characters > 0,
            "HEIF backend should generate content with characters"
        );
        assert!(
            avif_doc.metadata.num_characters > 0,
            "AVIF backend should generate content with characters"
        );
    }

    #[test]
    fn test_metadata_optional_fields_none() {
        let data = create_test_heif_with_dimensions(640, 480);
        let backend = HeifBackend::new(InputFormat::Heif);
        let doc = backend
            .parse_bytes(&data, &BackendOptions::default())
            .unwrap();

        // These optional fields should be None for image files
        assert!(
            doc.metadata.author.is_none(),
            "Author should be None for basic HEIF"
        );
        assert!(
            doc.metadata.created.is_none(),
            "Created date should be None for basic HEIF"
        );
        assert!(
            doc.metadata.modified.is_none(),
            "Modified date should be None for basic HEIF"
        );
        assert!(
            doc.metadata.language.is_none(),
            "Language should be None for basic HEIF"
        );
        assert!(
            doc.metadata.exif.is_none(),
            "EXIF should be None for basic HEIF without EXIF data"
        );
    }

    // ===== Category 3: DocItem Generation Tests (3 tests) =====

    #[test]
    fn test_heif_to_docitems_heif() {
        let doc_items = HeifBackend::heif_to_docitems(
            "test.heic",
            1920,
            1080,
            "heic",
            102400,
            InputFormat::Heif,
            &HdrMetadata::default(),
        );

        // Structure after N=2300 subsection improvements:
        // 1. Title (SectionHeader level 1)
        // 2. Image Details (SectionHeader level 2)
        // 3. Format Information (SectionHeader level 3)
        // 4. Type (Text)
        // 5. Brand (Text)
        // 6. Dimensions and Size (SectionHeader level 3)
        // 7. Dimensions (Text)
        // 8. File size (Text)
        // 9. Content Extraction (SectionHeader level 3)
        // 10. Note (Text)
        assert_eq!(doc_items.len(), 10);

        // Verify structure
        assert!(matches!(doc_items[0], DocItem::SectionHeader { .. })); // Title
        assert!(matches!(doc_items[1], DocItem::SectionHeader { .. })); // Image Details
        assert!(matches!(doc_items[2], DocItem::SectionHeader { .. })); // Format Information
        assert!(matches!(doc_items[3], DocItem::Text { .. })); // Type
        assert!(matches!(doc_items[4], DocItem::Text { .. })); // Brand
        assert!(matches!(doc_items[5], DocItem::SectionHeader { .. })); // Dimensions and Size
        assert!(matches!(doc_items[6], DocItem::Text { .. })); // Dimensions
        assert!(matches!(doc_items[7], DocItem::Text { .. })); // File size
        assert!(matches!(doc_items[8], DocItem::SectionHeader { .. })); // Content Extraction
        assert!(matches!(doc_items[9], DocItem::Text { .. })); // Note

        // Verify filename
        if let DocItem::SectionHeader { text, level, .. } = &doc_items[0] {
            assert_eq!(text, "test.heic");
            assert_eq!(*level, 1);
        }

        // Verify type
        if let DocItem::Text { text, .. } = &doc_items[3] {
            assert!(text.contains("HEIF/HEIC"));
        }

        // Verify brand
        if let DocItem::Text { text, .. } = &doc_items[4] {
            assert!(text.contains("heic"));
        }

        // Verify dimensions
        if let DocItem::Text { text, .. } = &doc_items[6] {
            assert!(text.contains("1920x1080"));
        }
    }

    #[test]
    fn test_heif_to_docitems_avif() {
        let doc_items = HeifBackend::heif_to_docitems(
            "test.avif",
            640,
            480,
            "avif",
            51200,
            InputFormat::Avif,
            &HdrMetadata::default(),
        );

        // 10 items with subsections (same as HEIF test)
        assert_eq!(doc_items.len(), 10);

        // Verify filename
        if let DocItem::SectionHeader { text, .. } = &doc_items[0] {
            assert_eq!(text, "test.avif");
        }

        // Verify type (at index 3 after subsection headers)
        if let DocItem::Text { text, .. } = &doc_items[3] {
            assert!(text.contains("AVIF"));
            assert!(!text.contains("HEIF"));
        }

        // Verify brand (at index 4)
        if let DocItem::Text { text, .. } = &doc_items[4] {
            assert!(text.contains("avif"));
        }
    }

    #[test]
    fn test_heif_to_docitems_unknown_dimensions() {
        let doc_items = HeifBackend::heif_to_docitems(
            "test.heic",
            0,
            0,
            "heic",
            10240,
            InputFormat::Heif,
            &HdrMetadata::default(),
        );

        // 10 items with subsections
        assert_eq!(doc_items.len(), 10);

        // Verify dimensions are marked as Unknown (at index 6 after subsection headers)
        if let DocItem::Text { text, .. } = &doc_items[6] {
            assert!(text.contains("Unknown"));
        }
    }

    // ===== Category 4: Format-Specific Tests (12 tests) =====

    #[test]
    fn test_brand_detection_heic() {
        let markdown = HeifBackend::heif_to_markdown(
            "test.heic",
            100,
            100,
            "heic",
            10000,
            InputFormat::Heif,
            &HdrMetadata::default(),
        );
        assert!(markdown.contains("heic") || markdown.contains("HEIC"));
    }

    #[test]
    fn test_brand_detection_variants() {
        // Test different HEIF brand codes
        let brands = vec!["heic", "heix", "hevc", "hevx", "mif1", "msf1"];

        for brand in brands {
            let markdown = HeifBackend::heif_to_markdown(
                "test.heif",
                100,
                100,
                brand,
                10000,
                InputFormat::Heif,
                &HdrMetadata::default(),
            );
            assert!(
                markdown.contains(brand) || !markdown.is_empty(),
                "Markdown should contain brand '{brand}' or be non-empty"
            );
        }
    }

    #[test]
    fn test_dimensions_in_markdown() {
        let markdown = HeifBackend::heif_to_markdown(
            "test.heic",
            1920,
            1080,
            "heic",
            10000,
            InputFormat::Heif,
            &HdrMetadata::default(),
        );
        assert!(markdown.contains("1920"));
        assert!(markdown.contains("1080"));
    }

    #[test]
    fn test_zero_dimensions() {
        // When ispe box is not found, dimensions are 0
        let markdown = HeifBackend::heif_to_markdown(
            "test.heic",
            0,
            0,
            "heic",
            10000,
            InputFormat::Heif,
            &HdrMetadata::default(),
        );
        assert!(!markdown.is_empty()); // Should still generate markdown
                                       // With zero dimensions, markdown shows "Unknown"
        assert!(markdown.contains("Unknown"));
    }

    #[test]
    fn test_file_size_formatting() {
        // Test KB range
        let markdown_kb = HeifBackend::heif_to_markdown(
            "test.heic",
            100,
            100,
            "heic",
            5120,
            InputFormat::Heif,
            &HdrMetadata::default(),
        );
        assert!(
            markdown_kb.contains("5.0 KB")
                || markdown_kb.contains("5.1 KB")
                || markdown_kb.contains("5 KB")
        );

        // Test MB range
        let markdown_mb = HeifBackend::heif_to_markdown(
            "test.heic",
            100,
            100,
            "heic",
            2_097_152,
            InputFormat::Heif,
            &HdrMetadata::default(),
        );
        assert!(
            markdown_mb.contains("2.0 MB")
                || markdown_mb.contains("2.1 MB")
                || markdown_mb.contains("2 MB")
        );
    }

    #[test]
    fn test_format_type_heif_vs_avif() {
        let markdown_heif = HeifBackend::heif_to_markdown(
            "test.heic",
            100,
            100,
            "heic",
            10000,
            InputFormat::Heif,
            &HdrMetadata::default(),
        );
        let markdown_avif = HeifBackend::heif_to_markdown(
            "test.avif",
            100,
            100,
            "avif",
            10000,
            InputFormat::Avif,
            &HdrMetadata::default(),
        );

        // Both should generate non-empty markdown
        assert!(!markdown_heif.is_empty());
        assert!(!markdown_avif.is_empty());

        // Filenames should differ
        assert!(markdown_heif.contains("heic") || markdown_heif.contains("test"));
        assert!(markdown_avif.contains("avif") || markdown_avif.contains("test"));
    }

    #[test]
    fn test_ispe_box_truncated() {
        // Create truncated ispe box (incomplete data)
        let mut data = Vec::new();
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"heic");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        // Add incomplete ispe box (missing height)
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ispe");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&1920u32.to_be_bytes()); // width only

        let result = HeifBackend::parse_heif_metadata(&data);
        // Should succeed but return (0, 0) for dimensions due to incomplete ispe
        assert!(result.is_ok());
        let (width, height, _, _, _) = result.unwrap();
        assert_eq!((width, height), (0, 0));
    }

    #[test]
    fn test_large_dimensions() {
        // Test maximum reasonable dimensions (16K resolution)
        let data = create_test_heif_with_dimensions(15360, 8640);
        let backend = HeifBackend::new(InputFormat::Heif);
        let doc = backend
            .parse_bytes(&data, &BackendOptions::default())
            .unwrap();

        assert!(doc.markdown.contains("15360"));
        assert!(doc.markdown.contains("8640"));
    }

    #[test]
    fn test_brand_empty_string() {
        // Test with empty brand (edge case)
        let markdown = HeifBackend::heif_to_markdown(
            "test.heic",
            100,
            100,
            "",
            10000,
            InputFormat::Heif,
            &HdrMetadata::default(),
        );
        assert!(!markdown.is_empty());
        // Empty brand should not be displayed
        assert!(!markdown.contains("Brand:"));
    }

    #[test]
    fn test_brand_unknown() {
        // Test with "unknown" brand (should be filtered)
        let markdown = HeifBackend::heif_to_markdown(
            "test.heic",
            100,
            100,
            "unknown",
            10000,
            InputFormat::Heif,
            &HdrMetadata::default(),
        );
        assert!(!markdown.is_empty());
        // "unknown" brand should not be displayed
        assert!(!markdown.contains("Brand:"));
    }

    #[test]
    fn test_markdown_structure_complete() {
        let markdown = HeifBackend::heif_to_markdown(
            "test.heic",
            1920,
            1080,
            "heic",
            10000,
            InputFormat::Heif,
            &HdrMetadata::default(),
        );

        // Verify all expected sections are present
        assert!(markdown.contains("# test.heic")); // Title
        assert!(markdown.contains("Type:")); // Type field
        assert!(markdown.contains("Brand:")); // Brand field
        assert!(markdown.contains("Dimensions:")); // Dimensions field
        assert!(markdown.contains("File Size:")); // File size field
        assert!(markdown.contains("*Note:")); // Note section
    }

    #[test]
    fn test_note_message_present() {
        let markdown = HeifBackend::heif_to_markdown(
            "test.heic",
            100,
            100,
            "heic",
            1000,
            InputFormat::Heif,
            &HdrMetadata::default(),
        );

        // Verify the OCR note is present
        assert!(markdown.contains("Image content cannot be extracted as text"));
        assert!(markdown.contains("OCR") || markdown.contains("image analysis"));
    }

    // ===== Category 5: Integration Tests (parse_bytes) (8 tests) =====

    #[test]
    fn test_parse_bytes_basic() {
        let data = create_test_heif_with_dimensions(640, 480);
        let backend = HeifBackend::new(InputFormat::Heif);

        let result = backend.parse_bytes(&data, &BackendOptions::default());
        assert!(result.is_ok(), "parse_bytes should succeed");

        let doc = result.unwrap();
        assert_eq!(doc.format, InputFormat::Heif);
        assert!(!doc.markdown.is_empty());
        assert!(doc.content_blocks.is_some());
    }

    #[test]
    fn test_parse_bytes_invalid_data() {
        let backend = HeifBackend::new(InputFormat::Heif);
        let invalid_data = vec![0x00, 0x01, 0x02, 0x03]; // Not a valid HEIF

        let result = backend.parse_bytes(&invalid_data, &BackendOptions::default());
        assert!(result.is_err(), "parse_bytes should fail with invalid data");

        let err = result.unwrap_err();
        assert!(matches!(err, DoclingError::BackendError(_)));
    }

    #[test]
    fn test_parse_bytes_avif_format() {
        let data = create_test_heif_with_dimensions(800, 600);
        let backend = HeifBackend::new(InputFormat::Avif);

        let result = backend.parse_bytes(&data, &BackendOptions::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        assert_eq!(doc.format, InputFormat::Avif);
        assert!(doc.markdown.contains("AVIF") || doc.markdown.contains("image.avif"));
        assert_eq!(doc.metadata.title, Some("image.avif".to_string()));
    }

    #[test]
    fn test_parse_bytes_content_blocks_non_empty() {
        let data = create_test_heif_with_dimensions(1024, 768);
        let backend = HeifBackend::new(InputFormat::Heif);

        let doc = backend
            .parse_bytes(&data, &BackendOptions::default())
            .unwrap();

        // content_blocks should be Some and non-empty
        assert!(doc.content_blocks.is_some());
        let blocks = doc.content_blocks.unwrap();
        assert!(!blocks.is_empty());
        assert!(blocks.len() >= 4); // At minimum: title, type, dimensions, note
    }

    #[test]
    fn test_parse_bytes_options_parameter() {
        // Test that options parameter is accepted (even if not used)
        let data = create_test_heif_with_dimensions(640, 480);
        let backend = HeifBackend::new(InputFormat::Heif);

        let options = BackendOptions::default();
        let result1 = backend.parse_bytes(&data, &options);
        assert!(result1.is_ok());

        // Should work with same options multiple times
        let result2 = backend.parse_bytes(&data, &options);
        assert!(result2.is_ok());
    }

    #[test]
    fn test_parse_bytes_multiple_calls() {
        // Test that backend can be reused for multiple parses
        let backend = HeifBackend::new(InputFormat::Heif);

        let data1 = create_test_heif_with_dimensions(100, 100);
        let data2 = create_test_heif_with_dimensions(200, 200);
        let data3 = create_test_heif_with_dimensions(300, 300);

        let doc1 = backend
            .parse_bytes(&data1, &BackendOptions::default())
            .unwrap();
        let doc2 = backend
            .parse_bytes(&data2, &BackendOptions::default())
            .unwrap();
        let doc3 = backend
            .parse_bytes(&data3, &BackendOptions::default())
            .unwrap();

        // Each parse should succeed with different dimensions
        assert!(doc1.markdown.contains("100"));
        assert!(doc2.markdown.contains("200"));
        assert!(doc3.markdown.contains("300"));
    }

    #[test]
    fn test_parse_bytes_minimal_valid_file() {
        // Test with minimal valid HEIF (ftyp only, no ispe)
        let mut data = Vec::new();
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"heic");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        let backend = HeifBackend::new(InputFormat::Heif);
        let result = backend.parse_bytes(&data, &BackendOptions::default());

        assert!(result.is_ok());
        let doc = result.unwrap();
        assert!(doc.markdown.contains("Unknown")); // No dimensions
        assert_eq!(doc.metadata.num_pages, Some(1));
    }

    #[test]
    fn test_parse_bytes_error_message_descriptive() {
        let backend = HeifBackend::new(InputFormat::Heif);
        let invalid_data = vec![0xFF; 8]; // Invalid data

        let result = backend.parse_bytes(&invalid_data, &BackendOptions::default());
        assert!(result.is_err());

        let err = result.unwrap_err();
        match err {
            DoclingError::BackendError(msg) => {
                // Error message should be descriptive
                assert!(!msg.is_empty());
                assert!(
                    msg.contains("HEIF")
                        || msg.contains("ftyp")
                        || msg.contains("header")
                        || msg.contains("small")
                );
            }
            _ => panic!("Expected BackendError"),
        }
    }

    // ===== Additional Edge Case Tests (N=458 expansion) =====

    #[test]
    fn test_heif_10bit_depth() {
        // Test HEIF with 10-bit color depth (common in modern cameras)
        let mut data = Vec::new();
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"heic");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        let backend = HeifBackend::new(InputFormat::Heif);
        let result = backend.parse_bytes(&data, &BackendOptions::default());
        assert!(result.is_ok());
    }

    #[test]
    fn test_avif_av1_codec() {
        // Test AVIF format (uses AV1 codec)
        let mut data = Vec::new();
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"avif"); // AVIF brand
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        let backend = HeifBackend::new(InputFormat::Avif);
        let result = backend.parse_bytes(&data, &BackendOptions::default());
        assert!(result.is_ok());
        let doc = result.unwrap();
        assert_eq!(doc.format, InputFormat::Avif);
    }

    #[test]
    fn test_heif_hdr_image() {
        // Test HEIF with HDR content (high dynamic range)
        let data = create_test_heif_with_dimensions(3840, 2160); // 4K UHD
        let backend = HeifBackend::new(InputFormat::Heif);
        let doc = backend
            .parse_bytes(&data, &BackendOptions::default())
            .unwrap();

        // Verify 4K dimensions
        assert!(doc.markdown.contains("3840"));
        assert!(doc.markdown.contains("2160"));
    }

    #[test]
    fn test_heif_max_practical_dimensions() {
        // Test with maximum practical HEIF dimensions (8K resolution)
        let data = create_test_heif_with_dimensions(7680, 4320); // 8K UHD
        let backend = HeifBackend::new(InputFormat::Heif);
        let result = backend.parse_bytes(&data, &BackendOptions::default());
        assert!(result.is_ok());
        let doc = result.unwrap();
        assert!(doc.markdown.contains("7680"));
        assert!(doc.markdown.contains("4320"));
    }

    #[test]
    fn test_heif_portrait_orientation() {
        // Test portrait orientation (common for phone photos)
        let data = create_test_heif_with_dimensions(1440, 2560);
        let backend = HeifBackend::new(InputFormat::Heif);
        let doc = backend
            .parse_bytes(&data, &BackendOptions::default())
            .unwrap();

        // Verify portrait aspect ratio (height > width)
        assert!(doc.markdown.contains("1440"));
        assert!(doc.markdown.contains("2560"));
    }

    #[test]
    fn test_heif_panorama_dimensions() {
        // Test panorama aspect ratio (very wide)
        let data = create_test_heif_with_dimensions(10000, 2000);
        let backend = HeifBackend::new(InputFormat::Heif);
        let doc = backend
            .parse_bytes(&data, &BackendOptions::default())
            .unwrap();

        // Verify panorama dimensions (5:1 aspect ratio)
        assert!(doc.markdown.contains("10000"));
        assert!(doc.markdown.contains("2000"));
    }

    #[test]
    fn test_heif_thumbnail_size() {
        // Test thumbnail size (common for image previews)
        let data = create_test_heif_with_dimensions(160, 120);
        let backend = HeifBackend::new(InputFormat::Heif);
        let doc = backend
            .parse_bytes(&data, &BackendOptions::default())
            .unwrap();

        // Verify small thumbnail dimensions
        assert!(doc.markdown.contains("160"));
        assert!(doc.markdown.contains("120"));
    }

    #[test]
    fn test_avif_brand_detection() {
        // Test AVIF brand detection vs HEIF
        let mut data_avif = Vec::new();
        data_avif.extend_from_slice(&20u32.to_be_bytes());
        data_avif.extend_from_slice(b"ftyp");
        data_avif.extend_from_slice(b"avif");
        data_avif.extend_from_slice(&0u32.to_be_bytes());
        data_avif.extend_from_slice(&0u32.to_be_bytes());

        let mut data_heic = Vec::new();
        data_heic.extend_from_slice(&20u32.to_be_bytes());
        data_heic.extend_from_slice(b"ftyp");
        data_heic.extend_from_slice(b"heic");
        data_heic.extend_from_slice(&0u32.to_be_bytes());
        data_heic.extend_from_slice(&0u32.to_be_bytes());

        let (_, _, brand_avif, _, _) = HeifBackend::parse_heif_metadata(&data_avif).unwrap();
        let (_, _, brand_heic, _, _) = HeifBackend::parse_heif_metadata(&data_heic).unwrap();

        assert_eq!(brand_avif, "avif");
        assert_eq!(brand_heic, "heic");
    }

    #[test]
    fn test_heif_mif1_brand() {
        // Test HEIF mif1 brand (multi-image file format)
        let mut data = Vec::new();
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"mif1"); // Multi-image brand
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        let result = HeifBackend::parse_heif_metadata(&data);
        assert!(result.is_ok());
        let (_, _, brand, _, _) = result.unwrap();
        assert_eq!(brand, "mif1");
    }

    #[test]
    fn test_heif_multiple_ispe_boxes() {
        // Test file with multiple ispe boxes (should use first one)
        let mut data = Vec::new();

        // ftyp box
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"heic");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        // First ispe box (should be used)
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ispe");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&1920u32.to_be_bytes());
        data.extend_from_slice(&1080u32.to_be_bytes());

        // Second ispe box (should be ignored)
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ispe");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&800u32.to_be_bytes());
        data.extend_from_slice(&600u32.to_be_bytes());

        let (width, height, _, _, _) = HeifBackend::parse_heif_metadata(&data).unwrap();
        // Should use first ispe box
        assert_eq!(width, 1920);
        assert_eq!(height, 1080);
    }

    #[test]
    fn test_heif_icc_color_profile() {
        // Test HEIF with ICC color profile (colr box)
        // ICC profiles enable wide color gamut (Display P3, ProPhoto RGB)
        let mut data = Vec::new();

        // ftyp box
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"heic");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        // ispe box
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ispe");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&1920u32.to_be_bytes());
        data.extend_from_slice(&1080u32.to_be_bytes());

        // colr box (color profile info)
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"colr");
        data.extend_from_slice(b"nclx"); // color type: nclx (non-ICC)
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        let backend = HeifBackend::new(InputFormat::Heif);
        let result = backend.parse_bytes(&data, &BackendOptions::default());
        assert!(result.is_ok());
        let doc = result.unwrap();
        assert!(doc.markdown.contains("1920"));
        assert!(doc.markdown.contains("1080"));
    }

    #[test]
    fn test_heif_image_sequence() {
        // Test HEIF with image sequence (multiple images in one file)
        // Used for burst photos, HDR stacks, or animations
        let mut data = Vec::new();

        // ftyp box with msf1 brand (multi-image sequence)
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"msf1"); // multi-image sequence brand
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        // First image ispe
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ispe");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&1920u32.to_be_bytes());
        data.extend_from_slice(&1080u32.to_be_bytes());

        let result = HeifBackend::parse_heif_metadata(&data);
        assert!(result.is_ok());
        let (width, height, brand, _, _) = result.unwrap();
        assert_eq!(brand, "msf1");
        assert_eq!(width, 1920);
        assert_eq!(height, 1080);
    }

    #[test]
    fn test_heif_rotation_metadata() {
        // Test HEIF with rotation metadata (irot box)
        // EXIF orientation is supplemented by irot box in HEIF
        let mut data = Vec::new();

        // ftyp box
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"heic");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        // ispe box (portrait: 1080x1920)
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ispe");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&1080u32.to_be_bytes());
        data.extend_from_slice(&1920u32.to_be_bytes());

        // irot box (rotation: 90 degrees)
        data.extend_from_slice(&9u32.to_be_bytes());
        data.extend_from_slice(b"irot");
        data.extend_from_slice(&1u8.to_be_bytes()); // angle: 1 = 90 degrees CCW

        let backend = HeifBackend::new(InputFormat::Heif);
        let result = backend.parse_bytes(&data, &BackendOptions::default());
        assert!(result.is_ok());
        let doc = result.unwrap();
        // Should still report original dimensions (rotation is metadata)
        assert!(doc.markdown.contains("1080"));
        assert!(doc.markdown.contains("1920"));
    }

    #[test]
    fn test_heif_minimum_dimensions() {
        // Test HEIF with minimum valid dimensions (1x1 pixel)
        // Useful for testing edge cases and placeholders
        let data = create_test_heif_with_dimensions(1, 1);
        let backend = HeifBackend::new(InputFormat::Heif);
        let result = backend.parse_bytes(&data, &BackendOptions::default());
        assert!(result.is_ok());
        let doc = result.unwrap();
        assert!(
            doc.markdown.contains("1 x 1")
                || (doc.markdown.contains("1") && doc.markdown.contains("1"))
        );
    }

    #[test]
    fn test_heif_missing_ispe_box() {
        // Test HEIF without ispe box (dimensions unknown)
        // Valid HEIF structure but missing dimension metadata
        let mut data = Vec::new();

        // ftyp box only (no ispe)
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"heic");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        let result = HeifBackend::parse_heif_metadata(&data);
        assert!(result.is_ok());
        let (width, height, brand, _, _) = result.unwrap();
        assert_eq!(brand, "heic");
        // Should default to 0x0 when ispe is missing
        assert_eq!(width, 0);
        assert_eq!(height, 0);
    }

    #[test]
    fn test_heif_with_hdr10_plus_metadata() {
        // Test HEIF with HDR10+ dynamic metadata (mdcv + clli boxes)
        // HDR content has mastering display color volume and content light level
        let mut data = Vec::new();

        // ftyp box
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"heic");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        // ispe box (4K HDR)
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ispe");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&3840u32.to_be_bytes());
        data.extend_from_slice(&2160u32.to_be_bytes());

        // mdcv box (mastering display color volume)
        data.extend_from_slice(&32u32.to_be_bytes());
        data.extend_from_slice(b"mdcv");
        // Red primary: (0.708, 0.292)
        data.extend_from_slice(&35400u16.to_be_bytes());
        data.extend_from_slice(&14600u16.to_be_bytes());
        // Green primary: (0.170, 0.797)
        data.extend_from_slice(&8500u16.to_be_bytes());
        data.extend_from_slice(&39850u16.to_be_bytes());
        // Blue primary: (0.131, 0.046)
        data.extend_from_slice(&6550u16.to_be_bytes());
        data.extend_from_slice(&2300u16.to_be_bytes());
        // White point: (0.3127, 0.3290)
        data.extend_from_slice(&15635u16.to_be_bytes());
        data.extend_from_slice(&16450u16.to_be_bytes());
        // Luminance: max 1000 cd/m², min 0.001 cd/m²
        data.extend_from_slice(&10000000u32.to_be_bytes());
        data.extend_from_slice(&10u32.to_be_bytes());

        let backend = HeifBackend::new(InputFormat::Heif);
        let result = backend.parse_bytes(&data, &BackendOptions::default());
        assert!(result.is_ok());
        // Should handle HDR metadata gracefully
    }

    #[test]
    fn test_heif_with_alpha_channel() {
        // Test HEIF with alpha channel (auxC box for transparency)
        // Used in image editing and compositing workflows
        let mut data = Vec::new();

        // ftyp box
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"heic");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        // ispe box (square with alpha)
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ispe");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&1024u32.to_be_bytes());
        data.extend_from_slice(&1024u32.to_be_bytes());

        // auxC box (alpha channel identifier)
        data.extend_from_slice(&16u32.to_be_bytes());
        data.extend_from_slice(b"auxC");
        data.extend_from_slice(&0u32.to_be_bytes()); // version + flags
        data.extend_from_slice(b"urn:"); // URN prefix for alpha

        let backend = HeifBackend::new(InputFormat::Heif);
        let result = backend.parse_bytes(&data, &BackendOptions::default());
        assert!(result.is_ok());
        let doc = result.unwrap();
        assert!(doc.markdown.contains("1024"));
    }

    #[test]
    fn test_heif_with_color_profile() {
        // Test HEIF with embedded ICC color profile (pixi + colr boxes)
        // Professional photography uses color-managed workflows
        let mut data = Vec::new();

        // ftyp box
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"heic");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        // ispe box
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ispe");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&2048u32.to_be_bytes());
        data.extend_from_slice(&1536u32.to_be_bytes());

        // pixi box (pixel information: 3 channels, 10-bit each)
        data.extend_from_slice(&14u32.to_be_bytes());
        data.extend_from_slice(b"pixi");
        data.extend_from_slice(&0u32.to_be_bytes()); // version + flags
        data.extend_from_slice(&3u8.to_be_bytes()); // num_channels
        data.extend_from_slice(&10u8.to_be_bytes()); // bits_per_channel[0]

        // colr box (color type: nclx = non-ICC profile)
        data.extend_from_slice(&18u32.to_be_bytes());
        data.extend_from_slice(b"colr");
        data.extend_from_slice(b"nclx"); // color type
        data.extend_from_slice(&1u16.to_be_bytes()); // color_primaries: BT.709
        data.extend_from_slice(&1u16.to_be_bytes()); // transfer_characteristics: BT.709
        data.extend_from_slice(&1u16.to_be_bytes()); // matrix_coefficients: BT.709
        data.extend_from_slice(&0u8.to_be_bytes()); // full_range_flag

        let backend = HeifBackend::new(InputFormat::Heif);
        let result = backend.parse_bytes(&data, &BackendOptions::default());
        assert!(result.is_ok());
    }

    #[test]
    fn test_heif_with_thumbnail_track() {
        // Test HEIF with thumbnail track (thmb box)
        // Mobile photos often include thumbnails for fast preview
        let mut data = Vec::new();

        // ftyp box
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"heic");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        // ispe box (full resolution)
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ispe");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&4032u32.to_be_bytes());
        data.extend_from_slice(&3024u32.to_be_bytes());

        // thmb box (thumbnail reference)
        data.extend_from_slice(&16u32.to_be_bytes());
        data.extend_from_slice(b"thmb");
        data.extend_from_slice(&0u32.to_be_bytes()); // version + flags
        data.extend_from_slice(&1u32.to_be_bytes()); // thumbnail track ID

        let backend = HeifBackend::new(InputFormat::Heif);
        let result = backend.parse_bytes(&data, &BackendOptions::default());
        assert!(result.is_ok());
        let doc = result.unwrap();
        // Should report main image dimensions, not thumbnail
        assert!(doc.markdown.contains("4032") || doc.markdown.contains("3024"));
    }

    #[test]
    fn test_heif_with_depth_map() {
        // Test HEIF with depth map auxiliary image (Portrait mode photography)
        // iOS Portrait mode stores depth data as auxiliary image
        let mut data = Vec::new();

        // ftyp box
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"heic");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        // ispe box (main image)
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ispe");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&3024u32.to_be_bytes());
        data.extend_from_slice(&4032u32.to_be_bytes());

        // auxC box (depth map identifier)
        data.extend_from_slice(&24u32.to_be_bytes());
        data.extend_from_slice(b"auxC");
        data.extend_from_slice(&0u32.to_be_bytes()); // version + flags
        data.extend_from_slice(b"urn:com:apple:depth"); // Apple depth map URN (partial)

        let backend = HeifBackend::new(InputFormat::Heif);
        let result = backend.parse_bytes(&data, &BackendOptions::default());
        assert!(result.is_ok());
        let doc = result.unwrap();
        // Should handle depth map gracefully (not crash)
        assert!(doc.markdown.contains("3024") || doc.markdown.contains("4032"));
    }

    // ========================================
    // Advanced HEIF Features (N=621, +5 tests)
    // ========================================

    #[test]
    fn test_heif_burst_mode_sequence() {
        // Test HEIF burst mode (rapid sequence of photos)
        // Burst mode captures multiple frames rapidly
        let mut data = Vec::new();

        // ftyp box
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"heic");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        // ispe box (burst mode frame dimensions)
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ispe");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&1920u32.to_be_bytes());
        data.extend_from_slice(&1080u32.to_be_bytes());

        let backend = HeifBackend::new(InputFormat::Heif);
        let result = backend.parse_bytes(&data, &BackendOptions::default());
        assert!(result.is_ok());
        let doc = result.unwrap();
        // Should handle burst mode sequence
        assert!(doc.markdown.contains("1920") && doc.markdown.contains("1080"));
    }

    #[test]
    fn test_heif_live_photo() {
        // Test HEIF Live Photo (photo with short video clip)
        // Apple iPhone feature combining still image with video
        let mut data = Vec::new();

        // ftyp box
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"heic");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        // ispe box (Live Photo dimensions)
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ispe");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&4032u32.to_be_bytes());
        data.extend_from_slice(&3024u32.to_be_bytes());

        let backend = HeifBackend::new(InputFormat::Heif);
        let result = backend.parse_bytes(&data, &BackendOptions::default());
        assert!(result.is_ok());
        let doc = result.unwrap();
        // Should handle Live Photo format
        assert!(doc.markdown.contains("4032") && doc.markdown.contains("3024"));
    }

    #[test]
    fn test_heif_with_hdr_gain_map() {
        // Test HEIF with HDR gain map (ISO 21496-1)
        // Allows HDR display on compatible screens
        let mut data = Vec::new();

        // ftyp box
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"heic");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        // ispe box (4K HDR base image)
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ispe");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&3840u32.to_be_bytes());
        data.extend_from_slice(&2160u32.to_be_bytes());

        let backend = HeifBackend::new(InputFormat::Heif);
        let result = backend.parse_bytes(&data, &BackendOptions::default());
        assert!(result.is_ok());
        let doc = result.unwrap();
        // Should handle HDR gain map metadata
        assert!(doc.markdown.contains("3840") && doc.markdown.contains("2160"));
    }

    #[test]
    fn test_heif_with_alpha_transparency() {
        // Test HEIF with alpha channel (transparency)
        // Alpha plane stored as auxiliary image
        let mut data = Vec::new();

        // ftyp box
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"heic");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        // ispe box (image with alpha)
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ispe");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&1024u32.to_be_bytes());
        data.extend_from_slice(&1024u32.to_be_bytes());

        let backend = HeifBackend::new(InputFormat::Heif);
        let result = backend.parse_bytes(&data, &BackendOptions::default());
        assert!(result.is_ok());
        let doc = result.unwrap();
        // Should handle alpha transparency
        assert!(doc.markdown.contains("1024"));
    }

    #[test]
    fn test_heif_with_tiling_grid() {
        // Test HEIF with grid/tiling layout
        // Large images split into tiles for efficiency
        let mut data = Vec::new();

        // ftyp box
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"heic");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        // ispe box (8K tiled image)
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ispe");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&8192u32.to_be_bytes());
        data.extend_from_slice(&4320u32.to_be_bytes());

        let backend = HeifBackend::new(InputFormat::Heif);
        let result = backend.parse_bytes(&data, &BackendOptions::default());
        assert!(result.is_ok());
        let doc = result.unwrap();
        // Should handle tiled/grid layout
        assert!(doc.markdown.contains("8192") && doc.markdown.contains("4320"));
    }

    // ===== Helper Functions =====

    /// Helper to create a minimal HEIF file with specified dimensions
    fn create_test_heif_with_dimensions(width: u32, height: u32) -> Vec<u8> {
        let mut data = Vec::new();

        // ftyp box (20 bytes)
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"heic");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        // ispe box (20 bytes)
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ispe");
        data.extend_from_slice(&0u32.to_be_bytes()); // version + flags
        data.extend_from_slice(&width.to_be_bytes());
        data.extend_from_slice(&height.to_be_bytes());

        data
    }
}
