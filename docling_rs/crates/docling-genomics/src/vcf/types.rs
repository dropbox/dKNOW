use std::collections::HashMap;

/// Represents a complete VCF document
#[derive(Debug, Clone, PartialEq)]
pub struct VcfDocument {
    /// Header section with metadata and field definitions
    pub header: VcfHeader,
    /// List of variant records in the file
    pub variants: Vec<Variant>,
}

/// VCF file header information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VcfHeader {
    /// VCF format version (e.g., "VCFv4.2")
    pub file_format: String,
    /// Reference genome file path or identifier
    pub reference: Option<String>,
    /// List of contig/chromosome definitions
    pub contigs: Vec<String>,
    /// Sample names from the header
    pub samples: Vec<String>,
    /// INFO field definitions keyed by field ID
    pub info_fields: HashMap<String, InfoFieldDef>,
    /// FORMAT field definitions keyed by field ID
    pub format_fields: HashMap<String, FormatFieldDef>,
    /// FILTER definitions keyed by filter ID with descriptions
    pub filters: HashMap<String, String>,
}

/// INFO field definition from header
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct InfoFieldDef {
    /// Field identifier (e.g., "DP", "AF")
    pub id: String,
    /// Number of values (e.g., "1", "A", "R", ".")
    pub number: String,
    /// Data type (e.g., "Integer", "Float", "String", "Flag")
    pub field_type: String,
    /// Human-readable field description
    pub description: String,
}

/// FORMAT field definition from header
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct FormatFieldDef {
    /// Field identifier (e.g., "GT", "DP", "GQ")
    pub id: String,
    /// Number of values (e.g., "1", "A", "R", ".")
    pub number: String,
    /// Data type (e.g., "Integer", "Float", "String")
    pub field_type: String,
    /// Human-readable field description
    pub description: String,
}

/// A single variant record
#[derive(Debug, Clone, PartialEq)]
pub struct Variant {
    /// Chromosome name (e.g., "chr1", "1", "X")
    pub chrom: String,
    /// 1-based position on the chromosome
    pub pos: u64,
    /// Variant identifier (e.g., rs number) if provided
    pub id: Option<String>,
    /// Reference allele bases
    pub ref_bases: String,
    /// Alternate allele(s)
    pub alt_alleles: Vec<String>,
    /// Phred-scaled quality score if available
    pub quality: Option<f32>,
    /// Filter status (PASS or filter names)
    pub filter: String,
    /// INFO field values keyed by field ID
    pub info: HashMap<String, InfoValue>,
    /// Genotype data for each sample
    pub genotypes: Vec<Genotype>,
}

/// INFO field value (can be various types)
#[derive(Debug, Clone, PartialEq)]
pub enum InfoValue {
    /// Single integer value
    Integer(i32),
    /// Single floating point value
    Float(f32),
    /// Boolean flag (presence indicates true)
    Flag,
    /// Single string value
    String(String),
    /// Array of integer values
    IntArray(Vec<i32>),
    /// Array of floating point values
    FloatArray(Vec<f32>),
    /// Array of string values
    StringArray(Vec<String>),
}

impl std::fmt::Display for InfoValue {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Integer(v) => write!(f, "{v}"),
            Self::Float(v) => write!(f, "{v}"),
            Self::Flag => write!(f, "true"),
            Self::String(v) => write!(f, "{v}"),
            Self::IntArray(arr) => {
                let s = arr
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(",");
                write!(f, "{s}")
            }
            Self::FloatArray(arr) => {
                let s = arr
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(",");
                write!(f, "{s}")
            }
            Self::StringArray(arr) => write!(f, "{}", arr.join(",")),
        }
    }
}

/// Per-sample genotype information
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Genotype {
    /// Sample name from header
    pub sample: String,
    /// Genotype call (e.g., "0/1", "1|1")
    pub gt: Option<String>,
    /// Additional FORMAT field values keyed by field ID
    pub fields: HashMap<String, String>,
}

impl VcfDocument {
    /// Create a new VCF document from header and variant records
    #[inline]
    #[must_use = "creates VCF document from header and variants"]
    pub const fn new(header: VcfHeader, variants: Vec<Variant>) -> Self {
        Self { header, variants }
    }

    /// Calculate variant statistics
    #[must_use = "calculates variant statistics"]
    #[allow(clippy::cast_precision_loss)] // Quality count is small, f32 precision is fine
    pub fn statistics(&self) -> VcfStatistics {
        let mut stats = VcfStatistics {
            total_variants: self.variants.len(),
            ..Default::default()
        };

        for variant in &self.variants {
            // Count SNPs vs indels
            let is_snp = variant.ref_bases.len() == 1
                && variant
                    .alt_alleles
                    .iter()
                    .all(|alt| alt.len() == 1 && alt != ".");

            if is_snp {
                stats.snp_count += 1;
            } else {
                // Check if insertion or deletion
                for alt in &variant.alt_alleles {
                    if alt.len() > variant.ref_bases.len() {
                        stats.insertion_count += 1;
                    } else if alt.len() < variant.ref_bases.len() {
                        stats.deletion_count += 1;
                    }
                }
            }

            // Track PASS vs filtered
            if variant.filter == "PASS" || variant.filter == "." {
                stats.pass_count += 1;
            } else {
                stats.filtered_count += 1;
            }

            // Track quality scores
            if let Some(qual) = variant.quality {
                stats.total_quality += qual;
                if qual < stats.min_quality {
                    stats.min_quality = qual;
                }
                if qual > stats.max_quality {
                    stats.max_quality = qual;
                }
                stats.quality_count += 1;
            }
        }

        if stats.quality_count > 0 {
            stats.mean_quality = stats.total_quality / stats.quality_count as f32;
        }

        stats
    }
}

/// Variant statistics summary
#[derive(Debug, Default, Clone, PartialEq)]
pub struct VcfStatistics {
    /// Total number of variant records
    pub total_variants: usize,
    /// Count of single nucleotide polymorphisms
    pub snp_count: usize,
    /// Count of insertions
    pub insertion_count: usize,
    /// Count of deletions
    pub deletion_count: usize,
    /// Count of variants passing filters
    pub pass_count: usize,
    /// Count of filtered-out variants
    pub filtered_count: usize,
    /// Average quality score (0.0 if no quality data)
    pub mean_quality: f32,
    /// Minimum quality score observed
    pub min_quality: f32,
    /// Maximum quality score observed
    pub max_quality: f32,
    /// Sum of all quality scores (for mean calculation)
    pub total_quality: f32,
    /// Number of variants with quality scores
    pub quality_count: usize,
}

impl Default for VcfHeader {
    #[inline]
    fn default() -> Self {
        Self {
            file_format: "VCFv4.2".to_string(),
            reference: None,
            contigs: Vec::new(),
            samples: Vec::new(),
            info_fields: HashMap::new(),
            format_fields: HashMap::new(),
            filters: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_info_value_display() {
        assert_eq!(format!("{}", InfoValue::Integer(42)), "42");
        assert_eq!(format!("{}", InfoValue::Float(2.5)), "2.5");
        assert_eq!(format!("{}", InfoValue::Flag), "true");
        assert_eq!(format!("{}", InfoValue::String("test".to_string())), "test");
        assert_eq!(format!("{}", InfoValue::IntArray(vec![1, 2, 3])), "1,2,3");
        assert_eq!(
            format!("{}", InfoValue::FloatArray(vec![1.1, 2.2])),
            "1.1,2.2"
        );
        assert_eq!(
            format!(
                "{}",
                InfoValue::StringArray(vec!["a".to_string(), "b".to_string()])
            ),
            "a,b"
        );
    }
}
