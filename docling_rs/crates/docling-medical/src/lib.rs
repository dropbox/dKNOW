//! Medical imaging format parsers for docling-rs
//!
//! This crate provides parsing support for medical imaging formats:
//!
//! - **DICOM** (Digital Imaging and Communications in Medicine) - `.dcm`, `.dicom`
//!
//! # Examples
//!
//! ```rust,no_run
//! use docling_medical::parse_dicom;
//!
//! // Parse DICOM file (second arg: anonymize patient data)
//! let metadata = parse_dicom("scan.dcm", false)?;
//!
//! // Access patient information
//! let patient = &metadata.patient;
//! println!("Patient: {}", &patient.name);
//! # Ok::<(), docling_medical::DicomError>(())
//! ```
//!
//! DICOM is the international standard for medical images and related information.
//! The parser extracts metadata (patient info, study details, series info) without
//! processing pixel data.
//!
//! ## Future Support
//!
//! Planned formats:
//! - NIFTI (Neuroimaging Informatics Technology Initiative)
//! - HL7 FHIR (Fast Healthcare Interoperability Resources)
//! - MINC (Medical Imaging `NetCDF`)

/// DICOM file format parsing module
pub mod dicom;

pub use dicom::{
    format_dicom_date, format_dicom_time, parse_dicom, DicomError, DicomMetadata, ImageInfo,
    PatientInfo, SeriesInfo, StudyInfo,
};
