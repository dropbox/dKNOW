//! Input format types for document conversion
//!
//! This module defines the `InputFormat` enum which represents the various
//! document formats that docling can process.

use serde::{Deserialize, Serialize};

/// Input document format
///
/// Matches Python's `docling.datamodel.base_models.InputFormat`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum InputFormat {
    /// PDF document
    #[serde(rename = "PDF")]
    Pdf,
    /// Microsoft Word document (.docx)
    #[serde(rename = "DOCX")]
    Docx,
    /// Microsoft Word 97-2003 document (.doc)
    #[serde(rename = "DOC")]
    Doc,
    /// Microsoft `PowerPoint` (.pptx)
    #[serde(rename = "PPTX")]
    Pptx,
    /// Microsoft Excel (.xlsx)
    #[serde(rename = "XLSX")]
    Xlsx,
    /// HTML document
    #[serde(rename = "HTML")]
    Html,
    /// CSV file
    #[serde(rename = "CSV")]
    Csv,
    /// Markdown document
    #[serde(rename = "MD")]
    Md,
    /// `AsciiDoc` document
    #[serde(rename = "ASCIIDOC")]
    Asciidoc,
    /// JATS XML (journal article format)
    #[serde(rename = "JATS")]
    Jats,
    /// `WebVTT` (video captions)
    #[serde(rename = "WEBVTT")]
    Webvtt,
    /// SRT subtitle (`SubRip`)
    #[serde(rename = "SRT")]
    Srt,
    /// PNG image
    #[serde(rename = "PNG")]
    Png,
    /// JPEG image
    #[serde(rename = "JPEG")]
    Jpeg,
    /// TIFF image
    #[serde(rename = "TIFF")]
    Tiff,
    /// WebP image
    #[serde(rename = "WEBP")]
    Webp,
    /// BMP image
    #[serde(rename = "BMP")]
    Bmp,
    /// GIF image
    #[serde(rename = "GIF")]
    Gif,
    /// EPUB e-book
    #[serde(rename = "EPUB")]
    Epub,
    /// `FictionBook` e-book (.fb2, .fb2.zip)
    #[serde(rename = "FB2")]
    Fb2,
    /// Mobipocket e-book (.mobi, .prc, .azw)
    #[serde(rename = "MOBI")]
    Mobi,
    /// Email message (.eml)
    #[serde(rename = "EML")]
    Eml,
    /// Mailbox archive (.mbox, .mbx)
    #[serde(rename = "MBOX")]
    Mbox,
    /// vCard contact file (.vcf, .vcard)
    #[serde(rename = "VCF")]
    Vcf,
    /// Microsoft Outlook message (.msg)
    #[serde(rename = "MSG")]
    Msg,
    /// ZIP archive (.zip)
    #[serde(rename = "ZIP")]
    Zip,
    /// TAR archive (.tar, .tar.gz, .tgz, .tar.bz2)
    #[serde(rename = "TAR")]
    Tar,
    /// 7Z archive (.7z)
    #[serde(rename = "7Z")]
    SevenZ,
    /// RAR archive (.rar)
    #[serde(rename = "RAR")]
    Rar,
    /// WAV audio (.wav)
    #[serde(rename = "WAV")]
    Wav,
    /// MP3 audio (.mp3)
    #[serde(rename = "MP3")]
    Mp3,
    /// MP4 video (.mp4, .m4v)
    #[serde(rename = "MP4")]
    Mp4,
    /// MKV video (.mkv)
    #[serde(rename = "MKV")]
    Mkv,
    /// MOV video (.mov, .qt)
    #[serde(rename = "MOV")]
    Mov,
    /// AVI video (.avi)
    #[serde(rename = "AVI")]
    Avi,
    /// `OpenDocument` Text (.odt)
    #[serde(rename = "ODT")]
    Odt,
    /// `OpenDocument` Spreadsheet (.ods)
    #[serde(rename = "ODS")]
    Ods,
    /// `OpenDocument` Presentation (.odp)
    #[serde(rename = "ODP")]
    Odp,
    /// XPS Document (.xps, .oxps)
    #[serde(rename = "XPS")]
    Xps,
    /// SVG (Scalable Vector Graphics) (.svg)
    #[serde(rename = "SVG")]
    Svg,
    /// HEIF/HEIC image (High Efficiency Image Format)
    #[serde(rename = "HEIF")]
    Heif,
    /// AVIF image (AV1 Image File Format)
    #[serde(rename = "AVIF")]
    Avif,
    /// ICS/iCalendar (.ics, .ical)
    #[serde(rename = "ICS")]
    Ics,
    /// Jupyter Notebook (.ipynb)
    #[serde(rename = "IPYNB")]
    Ipynb,
    /// GPX (GPS Exchange Format) (.gpx)
    #[serde(rename = "GPX")]
    Gpx,
    /// KML (Keyhole Markup Language) (.kml)
    #[serde(rename = "KML")]
    Kml,
    /// KMZ (compressed KML) (.kmz)
    #[serde(rename = "KMZ")]
    Kmz,
    /// DICOM (Digital Imaging and Communications in Medicine) (.dcm, .dicom)
    #[serde(rename = "DICOM")]
    Dicom,
    /// RTF (Rich Text Format) (.rtf)
    #[serde(rename = "RTF")]
    Rtf,
    /// STL (`STereoLithography`) (.stl) - 3D mesh format
    #[serde(rename = "STL")]
    Stl,
    /// OBJ (Wavefront Object) (.obj) - 3D mesh format
    #[serde(rename = "OBJ")]
    Obj,
    /// GLTF (GL Transmission Format) (.gltf) - Modern 3D format
    #[serde(rename = "GLTF")]
    Gltf,
    /// GLB (Binary glTF) (.glb) - Binary GLTF format
    #[serde(rename = "GLB")]
    Glb,
    /// DXF (Drawing Exchange Format) (.dxf) - `AutoCAD` interchange format
    #[serde(rename = "DXF")]
    Dxf,
    /// IDML (`InDesign` Markup Language) (.idml) - Adobe `InDesign` interchange format
    #[serde(rename = "IDML")]
    Idml,
    /// Microsoft Publisher (.pub)
    #[serde(rename = "PUB")]
    Pub,
    /// LaTeX document (.tex)
    #[serde(rename = "TEX")]
    Tex,
    /// Apple Pages document (.pages)
    #[serde(rename = "PAGES")]
    Pages,
    /// Apple Numbers spreadsheet (.numbers)
    #[serde(rename = "NUMBERS")]
    Numbers,
    /// Apple Keynote presentation (.key)
    #[serde(rename = "KEY")]
    Key,
    /// Microsoft Visio drawing (.vsdx)
    #[serde(rename = "VSDX")]
    Vsdx,
    /// Microsoft Project (.mpp)
    #[serde(rename = "MPP")]
    Mpp,
    /// Microsoft `OneNote` (.one)
    #[serde(rename = "ONE")]
    One,
    /// Microsoft Access database (.mdb, .accdb)
    #[serde(rename = "MDB")]
    Mdb,
    /// Docling JSON document (native format)
    #[serde(rename = "JSON_DOCLING")]
    JsonDocling,
}

impl InputFormat {
    /// Detect format from file extension
    #[inline]
    #[must_use = "detects format from file extension"]
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "pdf" => Some(Self::Pdf),
            "docx" => Some(Self::Docx),
            "doc" => Some(Self::Doc),
            "pptx" => Some(Self::Pptx),
            "xlsx" | "xlsm" => Some(Self::Xlsx),
            "html" | "htm" => Some(Self::Html),
            "csv" => Some(Self::Csv),
            "md" | "markdown" => Some(Self::Md),
            "asciidoc" | "adoc" => Some(Self::Asciidoc),
            "nxml" | "xml" => Some(Self::Jats),
            "vtt" => Some(Self::Webvtt),
            "srt" => Some(Self::Srt),
            "png" => Some(Self::Png),
            "jpg" | "jpeg" => Some(Self::Jpeg),
            "tif" | "tiff" => Some(Self::Tiff),
            "webp" => Some(Self::Webp),
            "bmp" => Some(Self::Bmp),
            "gif" => Some(Self::Gif),
            "epub" => Some(Self::Epub),
            "fb2" => Some(Self::Fb2),
            "mobi" | "prc" | "azw" => Some(Self::Mobi),
            "eml" => Some(Self::Eml),
            "mbox" | "mbx" => Some(Self::Mbox),
            "vcf" | "vcard" => Some(Self::Vcf),
            "msg" => Some(Self::Msg),
            "zip" => Some(Self::Zip),
            "tar" | "tgz" | "tbz2" | "tbz" => Some(Self::Tar),
            "7z" => Some(Self::SevenZ),
            "rar" => Some(Self::Rar),
            "wav" => Some(Self::Wav),
            "mp3" => Some(Self::Mp3),
            "mp4" | "m4v" => Some(Self::Mp4),
            "mkv" => Some(Self::Mkv),
            "mov" | "qt" => Some(Self::Mov),
            "avi" => Some(Self::Avi),
            "odt" => Some(Self::Odt),
            "ods" => Some(Self::Ods),
            "odp" => Some(Self::Odp),
            "xps" | "oxps" => Some(Self::Xps),
            "svg" => Some(Self::Svg),
            "heif" | "heic" => Some(Self::Heif),
            "avif" => Some(Self::Avif),
            "ics" | "ical" => Some(Self::Ics),
            "ipynb" => Some(Self::Ipynb),
            "gpx" => Some(Self::Gpx),
            "kml" => Some(Self::Kml),
            "kmz" => Some(Self::Kmz),
            "dcm" | "dicom" => Some(Self::Dicom),
            "rtf" => Some(Self::Rtf),
            "stl" => Some(Self::Stl),
            "obj" => Some(Self::Obj),
            "gltf" => Some(Self::Gltf),
            "glb" => Some(Self::Glb),
            "dxf" => Some(Self::Dxf),
            "idml" => Some(Self::Idml),
            "pub" => Some(Self::Pub),
            "tex" | "latex" => Some(Self::Tex),
            "pages" => Some(Self::Pages),
            "numbers" => Some(Self::Numbers),
            "key" => Some(Self::Key),
            "vsdx" => Some(Self::Vsdx),
            "mpp" => Some(Self::Mpp),
            "one" => Some(Self::One),
            "mdb" | "accdb" => Some(Self::Mdb),
            "json" => Some(Self::JsonDocling),
            "gz" => {
                // Check if it's .tar.gz by looking at the full path
                // This will be handled more robustly in the converter
                Some(Self::Tar)
            }
            "bz2" => {
                // Check if it's .tar.bz2
                Some(Self::Tar)
            }
            _ => None,
        }
    }

    /// Get file extensions associated with this format
    #[inline]
    #[must_use = "returns file extensions for this format"]
    pub const fn extensions(&self) -> &'static [&'static str] {
        match self {
            Self::Pdf => &["pdf"],
            Self::Docx => &["docx"],
            Self::Doc => &["doc"],
            Self::Pptx => &["pptx"],
            Self::Xlsx => &["xlsx", "xlsm"],
            Self::Html => &["html", "htm"],
            Self::Csv => &["csv"],
            Self::Md => &["md", "markdown"],
            Self::Asciidoc => &["asciidoc", "adoc"],
            Self::Jats => &["nxml", "xml"],
            Self::Webvtt => &["vtt"],
            Self::Srt => &["srt"],
            Self::Png => &["png"],
            Self::Jpeg => &["jpg", "jpeg"],
            Self::Tiff => &["tif", "tiff"],
            Self::Webp => &["webp"],
            Self::Bmp => &["bmp"],
            Self::Gif => &["gif"],
            Self::Epub => &["epub"],
            Self::Fb2 => &["fb2"],
            Self::Mobi => &["mobi", "prc", "azw"],
            Self::Eml => &["eml"],
            Self::Mbox => &["mbox", "mbx"],
            Self::Vcf => &["vcf", "vcard"],
            Self::Msg => &["msg"],
            Self::Zip => &["zip"],
            Self::Tar => &["tar", "tgz", "tar.gz", "tbz2", "tar.bz2"],
            Self::SevenZ => &["7z"],
            Self::Rar => &["rar"],
            Self::Wav => &["wav"],
            Self::Mp3 => &["mp3"],
            Self::Mp4 => &["mp4", "m4v"],
            Self::Mkv => &["mkv"],
            Self::Mov => &["mov", "qt"],
            Self::Avi => &["avi"],
            Self::Odt => &["odt"],
            Self::Ods => &["ods"],
            Self::Odp => &["odp"],
            Self::Xps => &["xps", "oxps"],
            Self::Svg => &["svg"],
            Self::Heif => &["heif", "heic"],
            Self::Avif => &["avif"],
            Self::Ics => &["ics", "ical"],
            Self::Ipynb => &["ipynb"],
            Self::Gpx => &["gpx"],
            Self::Kml => &["kml"],
            Self::Kmz => &["kmz"],
            Self::Dicom => &["dcm", "dicom"],
            Self::Rtf => &["rtf"],
            Self::Stl => &["stl"],
            Self::Obj => &["obj"],
            Self::Gltf => &["gltf"],
            Self::Glb => &["glb"],
            Self::Dxf => &["dxf"],
            Self::Idml => &["idml"],
            Self::Pub => &["pub"],
            Self::Tex => &["tex", "latex"],
            Self::Pages => &["pages"],
            Self::Numbers => &["numbers"],
            Self::Key => &["key"],
            Self::Vsdx => &["vsdx"],
            Self::Mpp => &["mpp"],
            Self::One => &["one"],
            Self::Mdb => &["mdb", "accdb"],
            Self::JsonDocling => &["json"],
        }
    }

    /// Check if this is an image format
    #[inline]
    #[must_use = "returns whether this is an image format"]
    pub const fn is_image(&self) -> bool {
        matches!(
            self,
            Self::Png
                | Self::Jpeg
                | Self::Tiff
                | Self::Webp
                | Self::Bmp
                | Self::Gif
                | Self::Heif
                | Self::Avif
        )
    }

    /// Check if this is a document format (text-based)
    #[inline]
    #[must_use = "returns whether this is a document format"]
    pub const fn is_document(&self) -> bool {
        matches!(
            self,
            Self::Pdf
                | Self::Docx
                | Self::Doc
                | Self::Pptx
                | Self::Xlsx
                | Self::Html
                | Self::Md
                | Self::Asciidoc
                | Self::Rtf
        )
    }

    /// Check if this is an e-book format
    #[inline]
    #[must_use = "returns whether this is an ebook format"]
    pub const fn is_ebook(&self) -> bool {
        matches!(self, Self::Epub | Self::Fb2 | Self::Mobi)
    }

    /// Check if this is an email format
    #[inline]
    #[must_use = "returns whether this is an email format"]
    pub const fn is_email(&self) -> bool {
        matches!(self, Self::Eml | Self::Mbox | Self::Vcf | Self::Msg)
    }

    /// Check if this is an archive format
    #[inline]
    #[must_use = "returns whether this is an archive format"]
    pub const fn is_archive(&self) -> bool {
        matches!(self, Self::Zip | Self::Tar | Self::SevenZ | Self::Rar)
    }

    /// Check if this is an audio format
    #[inline]
    #[must_use = "returns whether this is an audio format"]
    pub const fn is_audio(&self) -> bool {
        matches!(self, Self::Wav | Self::Mp3)
    }

    /// Check if this is a video format
    #[inline]
    #[must_use = "returns whether this is a video format"]
    pub const fn is_video(&self) -> bool {
        matches!(self, Self::Mp4 | Self::Mkv | Self::Mov | Self::Avi)
    }

    /// Check if this is a subtitle format
    #[inline]
    #[must_use = "returns whether this is a subtitle format"]
    pub const fn is_subtitle(&self) -> bool {
        matches!(self, Self::Webvtt | Self::Srt)
    }

    /// Check if this is an `OpenDocument` format
    #[inline]
    #[must_use = "returns whether this is an OpenDocument format"]
    pub const fn is_opendocument(&self) -> bool {
        matches!(self, Self::Odt | Self::Ods | Self::Odp)
    }

    /// Check if this is a CAD/3D format
    #[inline]
    #[must_use = "returns whether this is a CAD/3D format"]
    pub const fn is_cad(&self) -> bool {
        matches!(
            self,
            Self::Stl | Self::Obj | Self::Gltf | Self::Glb | Self::Dxf
        )
    }

    /// Check if this is an Apple iWork format
    #[inline]
    #[must_use = "returns whether this is an Apple iWork format"]
    pub const fn is_apple(&self) -> bool {
        matches!(self, Self::Pages | Self::Numbers | Self::Key)
    }
}

impl std::fmt::Display for InputFormat {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Pdf => "PDF",
            Self::Docx => "DOCX",
            Self::Doc => "DOC",
            Self::Pptx => "PPTX",
            Self::Xlsx => "XLSX",
            Self::Html => "HTML",
            Self::Csv => "CSV",
            Self::Md => "MD",
            Self::Asciidoc => "ASCIIDOC",
            Self::Jats => "JATS",
            Self::Webvtt => "WEBVTT",
            Self::Srt => "SRT",
            Self::Png => "PNG",
            Self::Jpeg => "JPEG",
            Self::Tiff => "TIFF",
            Self::Webp => "WEBP",
            Self::Bmp => "BMP",
            Self::Gif => "GIF",
            Self::Epub => "EPUB",
            Self::Fb2 => "FB2",
            Self::Mobi => "MOBI",
            Self::Eml => "EML",
            Self::Mbox => "MBOX",
            Self::Vcf => "VCF",
            Self::Msg => "MSG",
            Self::Zip => "ZIP",
            Self::Tar => "TAR",
            Self::SevenZ => "7Z",
            Self::Rar => "RAR",
            Self::Wav => "WAV",
            Self::Mp3 => "MP3",
            Self::Mp4 => "MP4",
            Self::Mkv => "MKV",
            Self::Mov => "MOV",
            Self::Avi => "AVI",
            Self::Odt => "ODT",
            Self::Ods => "ODS",
            Self::Odp => "ODP",
            Self::Xps => "XPS",
            Self::Svg => "SVG",
            Self::Heif => "HEIF",
            Self::Avif => "AVIF",
            Self::Ics => "ICS",
            Self::Ipynb => "IPYNB",
            Self::Gpx => "GPX",
            Self::Kml => "KML",
            Self::Kmz => "KMZ",
            Self::Dicom => "DICOM",
            Self::Rtf => "RTF",
            Self::Stl => "STL",
            Self::Obj => "OBJ",
            Self::Gltf => "GLTF",
            Self::Glb => "GLB",
            Self::Dxf => "DXF",
            Self::Idml => "IDML",
            Self::Pub => "PUB",
            Self::Tex => "TEX",
            Self::Pages => "PAGES",
            Self::Numbers => "NUMBERS",
            Self::Key => "KEY",
            Self::Vsdx => "VSDX",
            Self::Mpp => "MPP",
            Self::One => "ONE",
            Self::Mdb => "MDB",
            Self::JsonDocling => "JSON_DOCLING",
        };
        write!(f, "{s}")
    }
}

impl std::str::FromStr for InputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "PDF" => Ok(Self::Pdf),
            "DOCX" => Ok(Self::Docx),
            "DOC" => Ok(Self::Doc),
            "PPTX" => Ok(Self::Pptx),
            "XLSX" => Ok(Self::Xlsx),
            "HTML" | "HTM" => Ok(Self::Html),
            "CSV" => Ok(Self::Csv),
            "MD" | "MARKDOWN" => Ok(Self::Md),
            "ASCIIDOC" | "ADOC" => Ok(Self::Asciidoc),
            "JATS" | "NXML" => Ok(Self::Jats),
            "WEBVTT" | "VTT" => Ok(Self::Webvtt),
            "SRT" => Ok(Self::Srt),
            "PNG" => Ok(Self::Png),
            "JPEG" | "JPG" => Ok(Self::Jpeg),
            "TIFF" | "TIF" => Ok(Self::Tiff),
            "WEBP" => Ok(Self::Webp),
            "BMP" => Ok(Self::Bmp),
            "GIF" => Ok(Self::Gif),
            "EPUB" => Ok(Self::Epub),
            "FB2" => Ok(Self::Fb2),
            "MOBI" | "PRC" | "AZW" => Ok(Self::Mobi),
            "EML" => Ok(Self::Eml),
            "MBOX" | "MBX" => Ok(Self::Mbox),
            "VCF" | "VCARD" => Ok(Self::Vcf),
            "MSG" => Ok(Self::Msg),
            "ZIP" => Ok(Self::Zip),
            "TAR" | "TGZ" | "TBZ2" => Ok(Self::Tar),
            "7Z" => Ok(Self::SevenZ),
            "RAR" => Ok(Self::Rar),
            "WAV" => Ok(Self::Wav),
            "MP3" => Ok(Self::Mp3),
            "MP4" | "M4V" => Ok(Self::Mp4),
            "MKV" => Ok(Self::Mkv),
            "MOV" | "QT" => Ok(Self::Mov),
            "AVI" => Ok(Self::Avi),
            "ODT" => Ok(Self::Odt),
            "ODS" => Ok(Self::Ods),
            "ODP" => Ok(Self::Odp),
            "XPS" | "OXPS" => Ok(Self::Xps),
            "SVG" => Ok(Self::Svg),
            "HEIF" | "HEIC" => Ok(Self::Heif),
            "AVIF" => Ok(Self::Avif),
            "ICS" | "ICAL" => Ok(Self::Ics),
            "IPYNB" => Ok(Self::Ipynb),
            "GPX" => Ok(Self::Gpx),
            "KML" => Ok(Self::Kml),
            "KMZ" => Ok(Self::Kmz),
            "DICOM" | "DCM" => Ok(Self::Dicom),
            "RTF" => Ok(Self::Rtf),
            "STL" => Ok(Self::Stl),
            "OBJ" => Ok(Self::Obj),
            "GLTF" => Ok(Self::Gltf),
            "GLB" => Ok(Self::Glb),
            "DXF" => Ok(Self::Dxf),
            "IDML" => Ok(Self::Idml),
            "PUB" => Ok(Self::Pub),
            "TEX" | "LATEX" => Ok(Self::Tex),
            "PAGES" => Ok(Self::Pages),
            "NUMBERS" => Ok(Self::Numbers),
            "KEY" => Ok(Self::Key),
            "VSDX" => Ok(Self::Vsdx),
            "MPP" => Ok(Self::Mpp),
            "ONE" => Ok(Self::One),
            "MDB" | "ACCDB" => Ok(Self::Mdb),
            "JSON_DOCLING" | "JSONDOCLING" | "JSON-DOCLING" => Ok(Self::JsonDocling),
            _ => Err(format!("unknown input format: '{s}'")),
        }
    }
}

/// Legacy type alias for backward compatibility
pub type DocumentFormat = InputFormat;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_extension() {
        assert_eq!(InputFormat::from_extension("pdf"), Some(InputFormat::Pdf));
        assert_eq!(InputFormat::from_extension("PDF"), Some(InputFormat::Pdf));
        assert_eq!(InputFormat::from_extension("docx"), Some(InputFormat::Docx));
        assert_eq!(InputFormat::from_extension("unknown"), None);
    }

    #[test]
    fn test_is_image() {
        assert!(InputFormat::Png.is_image());
        assert!(InputFormat::Jpeg.is_image());
        assert!(!InputFormat::Pdf.is_image());
        assert!(!InputFormat::Docx.is_image());
    }

    #[test]
    fn test_serialization() {
        let format = InputFormat::Pdf;
        let json = serde_json::to_string(&format).unwrap();
        assert_eq!(json, r#""PDF""#);

        let deserialized: InputFormat = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, InputFormat::Pdf);
    }

    #[test]
    fn test_from_extension_multi_part() {
        // Test multi-part extensions
        assert_eq!(InputFormat::from_extension("tar.gz"), None);
        assert_eq!(InputFormat::from_extension("gz"), Some(InputFormat::Tar));
        assert_eq!(InputFormat::from_extension("bz2"), Some(InputFormat::Tar));
        assert_eq!(InputFormat::from_extension("tar"), Some(InputFormat::Tar));
    }

    #[test]
    fn test_extensions_roundtrip() {
        // Test that extensions() returns valid extensions that can be parsed back
        for format in [
            InputFormat::Pdf,
            InputFormat::Docx,
            InputFormat::Xlsx,
            InputFormat::Html,
            InputFormat::Png,
        ] {
            let exts = format.extensions();
            assert!(!exts.is_empty(), "Format {format:?} should have extensions");
            for ext in exts {
                let parsed = InputFormat::from_extension(ext);
                assert_eq!(
                    parsed,
                    Some(format),
                    "Extension '{ext}' should parse back to {format:?}"
                );
            }
        }
    }

    #[test]
    fn test_is_document_classification() {
        // Test document format classification
        assert!(InputFormat::Pdf.is_document());
        assert!(InputFormat::Docx.is_document());
        assert!(InputFormat::Doc.is_document());
        assert!(InputFormat::Pptx.is_document());
        assert!(InputFormat::Xlsx.is_document());
        assert!(InputFormat::Html.is_document());
        assert!(InputFormat::Md.is_document());
        assert!(InputFormat::Asciidoc.is_document());
        assert!(InputFormat::Rtf.is_document());

        // Non-document formats
        assert!(!InputFormat::Png.is_document());
        assert!(!InputFormat::Epub.is_document());
        assert!(!InputFormat::Zip.is_document());
        assert!(!InputFormat::Mp4.is_document());
    }

    #[test]
    fn test_is_ebook_classification() {
        // Test e-book format classification
        assert!(InputFormat::Epub.is_ebook());
        assert!(InputFormat::Fb2.is_ebook());
        assert!(InputFormat::Mobi.is_ebook());

        // Non-ebook formats
        assert!(!InputFormat::Pdf.is_ebook());
        assert!(!InputFormat::Docx.is_ebook());
        assert!(!InputFormat::Html.is_ebook());
    }

    #[test]
    fn test_is_archive_classification() {
        // Test archive format classification
        assert!(InputFormat::Zip.is_archive());
        assert!(InputFormat::Tar.is_archive());
        assert!(InputFormat::SevenZ.is_archive());
        assert!(InputFormat::Rar.is_archive());

        // Non-archive formats
        assert!(!InputFormat::Pdf.is_archive());
        assert!(!InputFormat::Docx.is_archive());
        assert!(!InputFormat::Png.is_archive());
    }

    #[test]
    fn test_is_subtitle_classification() {
        // Test subtitle format classification
        assert!(InputFormat::Webvtt.is_subtitle());
        assert!(InputFormat::Srt.is_subtitle());

        // Non-subtitle formats
        assert!(!InputFormat::Mp4.is_subtitle());
        assert!(!InputFormat::Pdf.is_subtitle());
        assert!(!InputFormat::Html.is_subtitle());
    }

    #[test]
    fn test_is_cad_classification() {
        // Test CAD/3D format classification
        assert!(InputFormat::Stl.is_cad());
        assert!(InputFormat::Obj.is_cad());
        assert!(InputFormat::Gltf.is_cad());
        assert!(InputFormat::Glb.is_cad());
        assert!(InputFormat::Dxf.is_cad());

        // Non-CAD formats
        assert!(!InputFormat::Pdf.is_cad());
        assert!(!InputFormat::Png.is_cad());
        assert!(!InputFormat::Svg.is_cad());
    }

    #[test]
    fn test_is_apple_classification() {
        // Test Apple iWork format classification
        assert!(InputFormat::Pages.is_apple());
        assert!(InputFormat::Numbers.is_apple());
        assert!(InputFormat::Key.is_apple());

        // Non-Apple formats
        assert!(!InputFormat::Docx.is_apple());
        assert!(!InputFormat::Xlsx.is_apple());
        assert!(!InputFormat::Pptx.is_apple());
    }

    #[test]
    fn test_display_trait() {
        // Test Display trait implementation
        assert_eq!(format!("{}", InputFormat::Pdf), "PDF");
        assert_eq!(format!("{}", InputFormat::Docx), "DOCX");
        assert_eq!(format!("{}", InputFormat::Html), "HTML");
        assert_eq!(format!("{}", InputFormat::Png), "PNG");
        assert_eq!(format!("{}", InputFormat::JsonDocling), "JSON_DOCLING");
    }

    #[test]
    fn test_input_format_from_str() {
        use std::str::FromStr;

        // Standard formats (uppercase)
        assert_eq!(InputFormat::from_str("PDF").unwrap(), InputFormat::Pdf);
        assert_eq!(InputFormat::from_str("DOCX").unwrap(), InputFormat::Docx);
        assert_eq!(InputFormat::from_str("DOC").unwrap(), InputFormat::Doc);
        assert_eq!(InputFormat::from_str("PPTX").unwrap(), InputFormat::Pptx);
        assert_eq!(InputFormat::from_str("XLSX").unwrap(), InputFormat::Xlsx);
        assert_eq!(InputFormat::from_str("HTML").unwrap(), InputFormat::Html);
        assert_eq!(InputFormat::from_str("CSV").unwrap(), InputFormat::Csv);
        assert_eq!(InputFormat::from_str("MD").unwrap(), InputFormat::Md);
        assert_eq!(InputFormat::from_str("PNG").unwrap(), InputFormat::Png);
        assert_eq!(InputFormat::from_str("JPEG").unwrap(), InputFormat::Jpeg);
        assert_eq!(InputFormat::from_str("EPUB").unwrap(), InputFormat::Epub);

        // Lowercase (case insensitive)
        assert_eq!(InputFormat::from_str("pdf").unwrap(), InputFormat::Pdf);
        assert_eq!(InputFormat::from_str("docx").unwrap(), InputFormat::Docx);
        assert_eq!(InputFormat::from_str("html").unwrap(), InputFormat::Html);
        assert_eq!(InputFormat::from_str("png").unwrap(), InputFormat::Png);

        // Mixed case
        assert_eq!(InputFormat::from_str("Pdf").unwrap(), InputFormat::Pdf);
        assert_eq!(InputFormat::from_str("HtMl").unwrap(), InputFormat::Html);

        // Alternative names
        assert_eq!(InputFormat::from_str("HTM").unwrap(), InputFormat::Html);
        assert_eq!(InputFormat::from_str("MARKDOWN").unwrap(), InputFormat::Md);
        assert_eq!(InputFormat::from_str("JPG").unwrap(), InputFormat::Jpeg);
        assert_eq!(InputFormat::from_str("TIF").unwrap(), InputFormat::Tiff);
        assert_eq!(InputFormat::from_str("NXML").unwrap(), InputFormat::Jats);
        assert_eq!(InputFormat::from_str("VTT").unwrap(), InputFormat::Webvtt);
        assert_eq!(
            InputFormat::from_str("ADOC").unwrap(),
            InputFormat::Asciidoc
        );
        assert_eq!(InputFormat::from_str("HEIC").unwrap(), InputFormat::Heif);
        assert_eq!(InputFormat::from_str("LATEX").unwrap(), InputFormat::Tex);
        assert_eq!(InputFormat::from_str("ACCDB").unwrap(), InputFormat::Mdb);

        // JSON_DOCLING variants
        assert_eq!(
            InputFormat::from_str("JSON_DOCLING").unwrap(),
            InputFormat::JsonDocling
        );
        assert_eq!(
            InputFormat::from_str("JSONDOCLING").unwrap(),
            InputFormat::JsonDocling
        );
        assert_eq!(
            InputFormat::from_str("JSON-DOCLING").unwrap(),
            InputFormat::JsonDocling
        );

        // Invalid
        assert!(InputFormat::from_str("invalid").is_err());
        assert!(InputFormat::from_str("").is_err());
        assert!(InputFormat::from_str("UNKNOWN").is_err());
    }

    #[test]
    fn test_input_format_roundtrip() {
        use std::str::FromStr;

        // Test all variants roundtrip: Display -> FromStr -> original
        let all_formats = [
            InputFormat::Pdf,
            InputFormat::Docx,
            InputFormat::Doc,
            InputFormat::Pptx,
            InputFormat::Xlsx,
            InputFormat::Html,
            InputFormat::Csv,
            InputFormat::Md,
            InputFormat::Asciidoc,
            InputFormat::Jats,
            InputFormat::Webvtt,
            InputFormat::Srt,
            InputFormat::Png,
            InputFormat::Jpeg,
            InputFormat::Tiff,
            InputFormat::Webp,
            InputFormat::Bmp,
            InputFormat::Gif,
            InputFormat::Epub,
            InputFormat::Fb2,
            InputFormat::Mobi,
            InputFormat::Eml,
            InputFormat::Mbox,
            InputFormat::Vcf,
            InputFormat::Msg,
            InputFormat::Zip,
            InputFormat::Tar,
            InputFormat::SevenZ,
            InputFormat::Rar,
            InputFormat::Wav,
            InputFormat::Mp3,
            InputFormat::Mp4,
            InputFormat::Mkv,
            InputFormat::Mov,
            InputFormat::Avi,
            InputFormat::Odt,
            InputFormat::Ods,
            InputFormat::Odp,
            InputFormat::Xps,
            InputFormat::Svg,
            InputFormat::Heif,
            InputFormat::Avif,
            InputFormat::Ics,
            InputFormat::Ipynb,
            InputFormat::Gpx,
            InputFormat::Kml,
            InputFormat::Kmz,
            InputFormat::Dicom,
            InputFormat::Rtf,
            InputFormat::Stl,
            InputFormat::Obj,
            InputFormat::Gltf,
            InputFormat::Glb,
            InputFormat::Dxf,
            InputFormat::Idml,
            InputFormat::Pub,
            InputFormat::Tex,
            InputFormat::Pages,
            InputFormat::Numbers,
            InputFormat::Key,
            InputFormat::Vsdx,
            InputFormat::Mpp,
            InputFormat::One,
            InputFormat::Mdb,
            InputFormat::JsonDocling,
        ];

        for format in all_formats {
            let s = format.to_string();
            let parsed = InputFormat::from_str(&s).unwrap();
            assert_eq!(
                format, parsed,
                "Roundtrip failed for {format:?}: '{s}' -> {parsed:?}"
            );
        }
    }
}
