/// DICOM (Digital Imaging and Communications in Medicine) format parser
///
/// Extracts metadata from DICOM medical imaging files including:
/// - Patient information (anonymized by default)
/// - Study information (date, description, physician)
/// - Series information (modality, series number, description)
/// - Image information (dimensions, instance number)
///
/// Note: This parser focuses on metadata extraction only. Pixel data is not extracted or converted.
use dicom_object::{open_file, DefaultDicomObject, Tag};
use std::path::Path;

/// DICOM parsing error
#[derive(Debug)]
pub enum DicomError {
    /// Failed to open DICOM file
    IoError(std::io::Error),
    /// DICOM parsing error
    ParseError(String),
    /// Missing required DICOM tag
    MissingTag(String),
}

impl std::fmt::Display for DicomError {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IoError(e) => write!(f, "DICOM I/O error: {e}"),
            Self::ParseError(msg) => write!(f, "DICOM parse error: {msg}"),
            Self::MissingTag(tag) => write!(f, "Missing DICOM tag: {tag}"),
        }
    }
}

impl std::error::Error for DicomError {}

impl From<std::io::Error> for DicomError {
    #[inline]
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err)
    }
}

impl From<dicom_object::ReadError> for DicomError {
    #[inline]
    fn from(err: dicom_object::ReadError) -> Self {
        Self::ParseError(format!("{err}"))
    }
}

/// Extracted DICOM metadata
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct DicomMetadata {
    /// Patient information
    pub patient: PatientInfo,
    /// Study information
    pub study: StudyInfo,
    /// Series information
    pub series: SeriesInfo,
    /// Image information
    pub image: ImageInfo,
    /// Equipment information
    pub equipment: Option<EquipmentInfo>,
    /// Acquisition parameters
    pub acquisition: Option<AcquisitionInfo>,
    /// File size in bytes
    pub file_size: u64,
}

/// Patient information from DICOM file
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct PatientInfo {
    /// Patient name (DICOM tag 0010,0010)
    pub name: String,
    /// Patient ID (DICOM tag 0010,0020)
    pub id: String,
    /// Patient birth date in DICOM format (YYYYMMDD) (DICOM tag 0010,0030)
    pub birth_date: Option<String>,
    /// Patient sex: M (Male), F (Female), O (Other), U (Unknown) (DICOM tag 0010,0040)
    pub sex: Option<String>,
}

/// Study information from DICOM file
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct StudyInfo {
    /// Study Instance UID (DICOM tag 0020,000D) - unique identifier for the study
    pub uid: String,
    /// Study date in DICOM format (YYYYMMDD) (DICOM tag 0008,0020)
    pub date: Option<String>,
    /// Study time in DICOM format (HHMMSS) (DICOM tag 0008,0030)
    pub time: Option<String>,
    /// Study description (DICOM tag 0008,1030)
    pub description: Option<String>,
    /// Study ID assigned by the institution (DICOM tag 0020,0010)
    pub id: Option<String>,
    /// Referring physician name (DICOM tag 0008,0090)
    pub referring_physician: Option<String>,
}

/// Series information from DICOM file
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct SeriesInfo {
    /// Series Instance UID (DICOM tag 0020,000E) - unique identifier for the series
    pub uid: String,
    /// Series number within the study (DICOM tag 0020,0011)
    pub number: Option<String>,
    /// Imaging modality (e.g., CT, MR, US, XA) (DICOM tag 0008,0060)
    pub modality: String,
    /// Series description (DICOM tag 0008,103E)
    pub description: Option<String>,
}

/// Image information from DICOM file
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct ImageInfo {
    /// SOP Class UID identifying the image type (DICOM tag 0008,0016)
    pub sop_class_uid: String,
    /// SOP Instance UID - unique identifier for this image (DICOM tag 0008,0018)
    pub sop_instance_uid: String,
    /// Instance number within the series (DICOM tag 0020,0013)
    pub instance_number: Option<String>,
    /// Image height in pixels (DICOM tag 0028,0010)
    pub rows: Option<u16>,
    /// Image width in pixels (DICOM tag 0028,0011)
    pub columns: Option<u16>,
    /// Number of frames for multi-frame images (DICOM tag 0028,0008)
    pub number_of_frames: Option<String>,
    /// Image type classification (DICOM tag 0008,0008)
    pub image_type: Option<String>,
    /// Body part examined (e.g., CHEST, HEAD) (DICOM tag 0018,0015)
    pub body_part_examined: Option<String>,
    /// Patient position during acquisition (DICOM tag 0018,5100)
    pub patient_position: Option<String>,
}

/// Equipment information from DICOM file
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct EquipmentInfo {
    /// Equipment manufacturer (DICOM tag 0008,0070)
    pub manufacturer: Option<String>,
    /// Manufacturer's model name (DICOM tag 0008,1090)
    pub model_name: Option<String>,
    /// Station name where the image was acquired (DICOM tag 0008,1010)
    pub station_name: Option<String>,
    /// Software version(s) used (DICOM tag 0018,1020)
    pub software_version: Option<String>,
}

/// Acquisition parameters from DICOM file
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct AcquisitionInfo {
    /// Pixel spacing in mm (row × column) (DICOM tag 0028,0030)
    pub pixel_spacing: Option<String>,
    /// Slice thickness in mm (DICOM tag 0018,0050)
    pub slice_thickness: Option<String>,
    /// Image position in patient coordinates (DICOM tag 0020,0032)
    pub image_position: Option<String>,
    /// Window center for display (DICOM tag 0028,1050)
    pub window_center: Option<String>,
    /// Window width for display (DICOM tag 0028,1051)
    pub window_width: Option<String>,
    /// Peak kilovoltage output of X-ray generator (DICOM tag 0018,0060)
    pub kvp: Option<String>,
    /// X-ray tube exposure in mAs (DICOM tag 0018,1152)
    pub exposure: Option<String>,
}

/// Parse a DICOM file and extract metadata
///
/// # Arguments
/// * `path` - Path to the DICOM file
/// * `anonymize` - If true, anonymize patient name and ID
///
/// # Errors
///
/// Returns an error if:
/// - The file cannot be read (I/O error)
/// - The DICOM file structure is invalid
/// - Required metadata fields are missing
#[must_use = "this function returns DICOM metadata that should be processed"]
pub fn parse_dicom<P: AsRef<Path>>(path: P, anonymize: bool) -> Result<DicomMetadata, DicomError> {
    let path_ref = path.as_ref();

    // Get file size
    let file_size = std::fs::metadata(path_ref)?.len();

    // Open DICOM file
    let obj = open_file(path_ref)?;

    // Extract metadata
    let metadata = DicomMetadata {
        patient: extract_patient_info(&obj, anonymize)?,
        study: extract_study_info(&obj)?,
        series: extract_series_info(&obj)?,
        image: extract_image_info(&obj)?,
        equipment: extract_equipment_info(&obj),
        acquisition: extract_acquisition_info(&obj),
        file_size,
    };

    Ok(metadata)
}

/// Extract patient information from DICOM object
#[allow(clippy::unnecessary_wraps, reason = "Result kept for API consistency")]
fn extract_patient_info(
    obj: &DefaultDicomObject,
    anonymize: bool,
) -> Result<PatientInfo, DicomError> {
    // Patient Name (0010,0010)
    let patient_name = get_string_tag(obj, 0x0010, 0x0010).unwrap_or_else(|| "UNKNOWN".to_string());
    let patient_name = if anonymize {
        "[PATIENT-ANONYMIZED]".to_string()
    } else {
        patient_name
    };

    // Patient ID (0010,0020)
    let patient_id = get_string_tag(obj, 0x0010, 0x0020).unwrap_or_else(|| "UNKNOWN".to_string());
    let patient_id = if anonymize {
        "[ID-ANONYMIZED]".to_string()
    } else {
        patient_id
    };

    // Patient Birth Date (0010,0030)
    let birth_date = get_string_tag(obj, 0x0010, 0x0030);

    // Patient Sex (0010,0040)
    // DICOM standard values: M (Male), F (Female), O (Other), U (Unknown)
    // Map non-standard values to standard equivalents
    let sex = get_string_tag(obj, 0x0010, 0x0040).map(|s| {
        match s.to_uppercase().as_str() {
            "M" | "MALE" => "M".to_string(),
            "F" | "FEMALE" => "F".to_string(),
            // Any other value (O, OTHER, U, UNKNOWN, or non-standard) maps to U
            _ => "U".to_string(),
        }
    });

    Ok(PatientInfo {
        name: patient_name,
        id: patient_id,
        birth_date,
        sex,
    })
}

/// Extract study information from DICOM object
fn extract_study_info(obj: &DefaultDicomObject) -> Result<StudyInfo, DicomError> {
    // Study Instance UID (0020,000D) - Required
    let uid = get_string_tag(obj, 0x0020, 0x000D)
        .ok_or_else(|| DicomError::MissingTag("StudyInstanceUID (0020,000D)".to_string()))?;

    // Study Date (0008,0020)
    let date = get_string_tag(obj, 0x0008, 0x0020);

    // Study Time (0008,0030)
    let time = get_string_tag(obj, 0x0008, 0x0030);

    // Study Description (0008,1030)
    let description = get_string_tag(obj, 0x0008, 0x1030);

    // Study ID (0020,0010)
    let id = get_string_tag(obj, 0x0020, 0x0010);

    // Referring Physician Name (0008,0090)
    let referring_physician = get_string_tag(obj, 0x0008, 0x0090);

    Ok(StudyInfo {
        uid,
        date,
        time,
        description,
        id,
        referring_physician,
    })
}

/// Extract series information from DICOM object
fn extract_series_info(obj: &DefaultDicomObject) -> Result<SeriesInfo, DicomError> {
    // Series Instance UID (0020,000E) - Required
    let uid = get_string_tag(obj, 0x0020, 0x000E)
        .ok_or_else(|| DicomError::MissingTag("SeriesInstanceUID (0020,000E)".to_string()))?;

    // Series Number (0020,0011)
    let number = get_string_tag(obj, 0x0020, 0x0011);

    // Modality (0008,0060) - Required
    let modality = get_string_tag(obj, 0x0008, 0x0060)
        .ok_or_else(|| DicomError::MissingTag("Modality (0008,0060)".to_string()))?;

    // Series Description (0008,103E)
    let description = get_string_tag(obj, 0x0008, 0x103E);

    Ok(SeriesInfo {
        uid,
        number,
        modality,
        description,
    })
}

/// Extract image information from DICOM object
fn extract_image_info(obj: &DefaultDicomObject) -> Result<ImageInfo, DicomError> {
    // SOP Class UID (0008,0016) - Required
    let sop_class_uid = get_string_tag(obj, 0x0008, 0x0016)
        .ok_or_else(|| DicomError::MissingTag("SOPClassUID (0008,0016)".to_string()))?;

    // SOP Instance UID (0008,0018) - Required
    let sop_instance_uid = get_string_tag(obj, 0x0008, 0x0018)
        .ok_or_else(|| DicomError::MissingTag("SOPInstanceUID (0008,0018)".to_string()))?;

    // Instance Number (0020,0013)
    let instance_number = get_string_tag(obj, 0x0020, 0x0013);

    // Rows (0028,0010)
    let rows = get_u16_tag(obj, 0x0028, 0x0010);

    // Columns (0028,0011)
    let columns = get_u16_tag(obj, 0x0028, 0x0011);

    // Number of Frames (0028,0008)
    let number_of_frames = get_string_tag(obj, 0x0028, 0x0008);

    // Image Type (0008,0008)
    // DICOM uses backslashes as separators. Convert to commas for readability.
    let image_type = get_string_tag(obj, 0x0008, 0x0008)
        .map(|s| s.split('\\').collect::<Vec<&str>>().join(", "));

    // Body Part Examined (0018,0015)
    let body_part_examined = get_string_tag(obj, 0x0018, 0x0015);

    // Patient Position (0018,5100)
    let patient_position = get_string_tag(obj, 0x0018, 0x5100);

    Ok(ImageInfo {
        sop_class_uid,
        sop_instance_uid,
        instance_number,
        rows,
        columns,
        number_of_frames,
        image_type,
        body_part_examined,
        patient_position,
    })
}

/// Get string value for a DICOM tag
#[inline]
fn get_string_tag(obj: &DefaultDicomObject, group: u16, element: u16) -> Option<String> {
    let tag = Tag(group, element);
    obj.element(tag)
        .ok()
        .and_then(|elem| elem.to_str().ok())
        .map(|s| s.trim().to_string())
}

/// Get u16 value for a DICOM tag
#[inline]
fn get_u16_tag(obj: &DefaultDicomObject, group: u16, element: u16) -> Option<u16> {
    let tag = Tag(group, element);
    obj.element(tag)
        .ok()
        .and_then(|elem| elem.to_int::<i32>().ok())
        .and_then(|val| u16::try_from(val).ok())
}

/// Get float value for a DICOM tag
#[inline]
fn get_float_tag(obj: &DefaultDicomObject, group: u16, element: u16) -> Option<f64> {
    let tag = Tag(group, element);
    obj.element(tag)
        .ok()
        .and_then(|elem| elem.to_float64().ok())
}

/// Extract equipment information from DICOM object
fn extract_equipment_info(obj: &DefaultDicomObject) -> Option<EquipmentInfo> {
    // Manufacturer (0008,0070)
    let manufacturer = get_string_tag(obj, 0x0008, 0x0070);

    // Manufacturer's Model Name (0008,1090)
    let model_name = get_string_tag(obj, 0x0008, 0x1090);

    // Station Name (0008,1010)
    let station_name = get_string_tag(obj, 0x0008, 0x1010);

    // Software Versions (0018,1020)
    let software_version = get_string_tag(obj, 0x0018, 0x1020);

    // Return None if no equipment info is available
    if manufacturer.is_none()
        && model_name.is_none()
        && station_name.is_none()
        && software_version.is_none()
    {
        return None;
    }

    Some(EquipmentInfo {
        manufacturer,
        model_name,
        station_name,
        software_version,
    })
}

/// Extract acquisition information from DICOM object
fn extract_acquisition_info(obj: &DefaultDicomObject) -> Option<AcquisitionInfo> {
    // Pixel Spacing (0028,0030) - stored as "row_spacing\column_spacing"
    let pixel_spacing = get_string_tag(obj, 0x0028, 0x0030).map(|s| {
        let parts: Vec<&str> = s.split('\\').collect();
        if parts.len() == 2 {
            format!("{} × {} mm", parts[0], parts[1])
        } else {
            format!("{s} mm")
        }
    });

    // Slice Thickness (0018,0050)
    let slice_thickness = get_string_tag(obj, 0x0018, 0x0050)
        .or_else(|| get_float_tag(obj, 0x0018, 0x0050).map(|f| format!("{f:.1}")))
        .map(|s| format!("{s} mm"));

    // Image Position (Patient) (0020,0032)
    let image_position = get_string_tag(obj, 0x0020, 0x0032).map(|s| {
        let parts: Vec<&str> = s.split('\\').collect();
        if parts.len() == 3 {
            format!("({}, {}, {})", parts[0], parts[1], parts[2])
        } else {
            s
        }
    });

    // Window Center (0028,1050)
    let window_center = get_string_tag(obj, 0x0028, 0x1050);

    // Window Width (0028,1051)
    let window_width = get_string_tag(obj, 0x0028, 0x1051);

    // KVP (0018,0060)
    let kvp = get_string_tag(obj, 0x0018, 0x0060)
        .or_else(|| get_float_tag(obj, 0x0018, 0x0060).map(|f| format!("{f:.0}")))
        .map(|s| format!("{s} kV"));

    // Exposure (0018,1152)
    let exposure = get_string_tag(obj, 0x0018, 0x1152)
        .or_else(|| get_float_tag(obj, 0x0018, 0x1152).map(|f| format!("{f:.0}")))
        .map(|s| format!("{s} mAs"));

    // Return None if no acquisition info is available
    if pixel_spacing.is_none()
        && slice_thickness.is_none()
        && image_position.is_none()
        && window_center.is_none()
        && window_width.is_none()
        && kvp.is_none()
        && exposure.is_none()
    {
        return None;
    }

    Some(AcquisitionInfo {
        pixel_spacing,
        slice_thickness,
        image_position,
        window_center,
        window_width,
        kvp,
        exposure,
    })
}

/// Format DICOM date (YYYYMMDD) to readable format (YYYY-MM-DD)
#[inline]
#[must_use = "formats DICOM date to readable format"]
pub fn format_dicom_date(date_str: &str) -> String {
    if date_str.len() == 8 {
        format!(
            "{}-{}-{}",
            &date_str[0..4],
            &date_str[4..6],
            &date_str[6..8]
        )
    } else {
        date_str.to_string()
    }
}

/// Format DICOM time (HHMMSS.FFFFFF) to readable format (HH:MM:SS)
#[inline]
#[must_use = "formats DICOM time to readable format"]
pub fn format_dicom_time(time_str: &str) -> String {
    if time_str.len() >= 6 {
        format!(
            "{}:{}:{}",
            &time_str[0..2],
            &time_str[2..4],
            &time_str[4..6]
        )
    } else {
        time_str.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_dicom_date() {
        assert_eq!(format_dicom_date("20231107"), "2023-11-07");
        assert_eq!(format_dicom_date("invalid"), "invalid");
    }

    #[test]
    fn test_format_dicom_time() {
        assert_eq!(format_dicom_time("143025"), "14:30:25");
        assert_eq!(format_dicom_time("143025.123456"), "14:30:25");
        assert_eq!(format_dicom_time("short"), "short"); // < 6 chars
    }
}
