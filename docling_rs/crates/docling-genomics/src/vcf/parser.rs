use super::types::{FormatFieldDef, Genotype, InfoFieldDef, Variant, VcfDocument, VcfHeader};
use anyhow::{Context, Result};
use noodles_vcf as vcf;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// VCF file parser
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct VcfParser;

impl VcfParser {
    /// Parse a VCF file from a path
    ///
    /// Ported from: `examples/vcf_prototype.rs` (N=52)
    /// Uses noodles-vcf crate for parsing
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file cannot be opened
    /// - The VCF header cannot be parsed
    /// - Any variant record is malformed
    #[must_use = "this function returns a parsed VCF document that should be processed"]
    pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<VcfDocument> {
        let path = path.as_ref();

        // Open file and create VCF reader
        // Pattern from vcf_prototype.rs:24-26
        let mut reader = File::open(path)
            .context(format!("Failed to open VCF file: {}", path.display()))
            .map(BufReader::new)
            .map(vcf::io::Reader::new)?;

        // Read header
        // Pattern from vcf_prototype.rs:28
        let header = reader.read_header().context("Failed to read VCF header")?;

        // Parse header into our types
        let parsed_header = Self::parse_header(&header);

        // Parse all variant records
        let variants = Self::parse_variants(&header, &mut reader)?;

        Ok(VcfDocument::new(parsed_header, variants))
    }

    /// Parse VCF content from a string
    ///
    /// # Errors
    ///
    /// Returns an error if the VCF content is malformed.
    #[must_use = "this function returns a parsed VCF document that should be processed"]
    pub fn parse_str(content: &str) -> Result<VcfDocument> {
        let cursor = std::io::Cursor::new(content.as_bytes());
        Self::parse_reader(cursor)
    }

    /// Parse VCF from a reader
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The VCF header cannot be parsed
    /// - Any variant record is malformed
    #[must_use = "this function returns a parsed VCF document that should be processed"]
    pub fn parse_reader<R: BufRead>(reader: R) -> Result<VcfDocument> {
        let mut vcf_reader = vcf::io::Reader::new(reader);

        let header = vcf_reader
            .read_header()
            .context("Failed to read VCF header")?;

        let parsed_header = Self::parse_header(&header);
        let variants = Self::parse_variants(&header, &mut vcf_reader)?;

        Ok(VcfDocument::new(parsed_header, variants))
    }

    /// Parse VCF header into our `VcfHeader` type
    ///
    /// Ported from: examples/vcf_prototype.rs:39-84 (`print_header` function)
    fn parse_header(header: &vcf::Header) -> VcfHeader {
        // Extract file format
        // Pattern from vcf_prototype.rs:44
        // Convert FileFormat to "VCFv{major}.{minor}" string
        let file_format = {
            let fmt = header.file_format();
            format!("VCFv{}.{}", fmt.major(), fmt.minor())
        };

        // Extract sample names
        // Pattern from vcf_prototype.rs:47
        let samples: Vec<String> = header
            .sample_names()
            .iter()
            .map(std::string::ToString::to_string)
            .collect();

        // Extract contigs
        // Pattern from vcf_prototype.rs:52-59
        let contigs: Vec<String> = header
            .contigs()
            .keys()
            .map(std::string::ToString::to_string)
            .collect();

        // Extract INFO field definitions
        // Pattern from vcf_prototype.rs:63-66
        let mut info_fields = HashMap::new();
        for (key, info) in header.infos() {
            info_fields.insert(
                key.clone(),
                InfoFieldDef {
                    id: key.clone(),
                    number: format!("{:?}", info.number()),
                    field_type: format!("{:?}", info.ty()),
                    description: info.description().to_string(),
                },
            );
        }

        // Extract FORMAT field definitions
        // Pattern from vcf_prototype.rs:69-73
        let mut format_fields = HashMap::new();
        for (key, format) in header.formats() {
            format_fields.insert(
                key.clone(),
                FormatFieldDef {
                    id: key.clone(),
                    number: format!("{:?}", format.number()),
                    field_type: format!("{:?}", format.ty()),
                    description: format.description().to_string(),
                },
            );
        }

        // Extract FILTER definitions
        // Pattern from vcf_prototype.rs:77-80
        let mut filters = HashMap::new();
        for (key, filter) in header.filters() {
            filters.insert(key.clone(), filter.description().to_string());
        }

        // Extract reference genome if present
        // VCF spec: ##reference=<URL or path>
        // Example: ##reference=file:///seq/references/1000GenomesPilot-NCBI36.fasta
        let reference = header.other_records().iter().find_map(|(key, value)| {
            (key.to_string() == "reference").then(|| format!("{value:?}"))
        });

        VcfHeader {
            file_format,
            reference,
            contigs,
            samples,
            info_fields,
            format_fields,
            filters,
        }
    }

    /// Parse all variant records
    ///
    /// Ported from: examples/vcf_prototype.rs:86-138 (`print_variants` function)
    fn parse_variants<R: BufRead>(
        header: &vcf::Header,
        reader: &mut vcf::io::Reader<R>,
    ) -> Result<Vec<Variant>> {
        let mut variants = Vec::new();

        // Pattern from vcf_prototype.rs:93-98
        for result in reader.records() {
            let record = result.context("Failed to read VCF record")?;
            let variant = Self::parse_variant(header, &record)?;
            variants.push(variant);
        }

        Ok(variants)
    }

    /// Parse a single variant record
    ///
    /// Ported from: examples/vcf_prototype.rs:100-135 (`print_variants` loop body)
    fn parse_variant(header: &vcf::Header, record: &vcf::Record) -> Result<Variant> {
        // CHROM - Pattern from vcf_prototype.rs:103
        let chrom = record.reference_sequence_name().to_string();

        // POS - Pattern from vcf_prototype.rs:104
        let pos = match record.variant_start() {
            Some(Ok(position)) => {
                // Position is 1-based, convert to u64
                usize::from(position) as u64
            }
            Some(Err(e)) => anyhow::bail!("Invalid position: {e}"),
            None => anyhow::bail!("Missing position"),
        };

        // ID - Pattern from vcf_prototype.rs:105
        let id = {
            let ids_str = format!("{:?}", record.ids());
            if ids_str == "[]" || ids_str == "[\".\"]" {
                None
            } else {
                // Parse ["rs6054257"] or ["rs123", "rs456"] format
                let cleaned = ids_str
                    .trim_matches('[')
                    .trim_matches(']')
                    .split(", ")
                    .map(|s| s.trim_matches('"'))
                    .collect::<Vec<_>>()
                    .join(",");
                Some(cleaned)
            }
        };

        // REF - Pattern from vcf_prototype.rs:106
        let ref_bases = record.reference_bases().to_string();

        // ALT - Pattern from vcf_prototype.rs:107
        let alt_alleles = {
            let alt_str = format!("{:?}", record.alternate_bases());
            // Parse "[Ok(\"A\"), Ok(\"G\")]" or "[]" format
            if alt_str == "[]" {
                // Missing ALT is represented as "."
                vec![".".to_string()]
            } else {
                alt_str
                    .trim_matches('[')
                    .trim_matches(']')
                    .split(", ")
                    .filter(|s| !s.is_empty())
                    .map(|s| {
                        s.trim_start_matches("Ok(\"")
                            .trim_end_matches("\")")
                            .to_string()
                    })
                    .collect()
            }
        };

        // QUAL - Pattern from vcf_prototype.rs:110-120
        let quality = match record.quality_score() {
            Some(Ok(qual)) => Some(qual),
            Some(Err(_)) | None => None,
        };

        // FILTER - Pattern from vcf_prototype.rs:123
        // Parse filter string - extract PASS, FAIL, or filter names
        let filter = {
            let filter_str = format!("{:?}", record.filters());
            // Parse "[Ok(\"PASS\")]" or "[Ok(\"q10\"), Ok(\"s50\")]" format
            if filter_str == "[]" || filter_str.contains("PASS") {
                "PASS".to_string()
            } else {
                // Extract filter names from debug format
                filter_str
                    .trim_matches('[')
                    .trim_matches(']')
                    .split(", ")
                    .filter(|s| !s.is_empty())
                    .map(|s| {
                        s.trim_start_matches("Ok(\"")
                            .trim_end_matches("\")")
                            .to_string()
                    })
                    .collect::<Vec<_>>()
                    .join(";")
            }
        };

        // INFO - Pattern from vcf_prototype.rs:126
        // For now, store as empty HashMap - will implement INFO parsing next
        let info = HashMap::new();

        // Genotypes - Pattern from vcf_prototype.rs:129-132
        let genotypes = if header.sample_names().is_empty() {
            Vec::new()
        } else {
            Self::parse_genotypes(header, record)
        };

        Ok(Variant {
            chrom,
            pos,
            id,
            ref_bases,
            alt_alleles,
            quality,
            filter,
            info,
            genotypes,
        })
    }

    /// Parse genotype information for all samples
    ///
    /// Ported from: examples/vcf_prototype.rs:140-194 (`print_genotypes` function)
    fn parse_genotypes(header: &vcf::Header, record: &vcf::Record) -> Vec<Genotype> {
        use vcf::variant::record::samples::Sample;

        let samples = record.samples();
        let mut genotypes = Vec::new();

        // Pattern from vcf_prototype.rs:149-191
        for sample_name in header.sample_names() {
            if let Some(sample) = samples.get(header, sample_name) {
                let mut gt = None;
                let mut fields = HashMap::new();

                // Get GT field - Pattern from vcf_prototype.rs:155-167
                if let Some(genotype_result) = sample.get(header, "GT") {
                    match genotype_result {
                        Ok(Some(gt_value)) => {
                            gt = Some(format!("{gt_value:?}"));
                        }
                        Ok(None) => {
                            gt = Some(".".to_string());
                        }
                        Err(_) => {
                            // Skip errors
                        }
                    }
                }

                // Get other FORMAT fields - Pattern from vcf_prototype.rs:170-186
                for (key, _) in header.formats() {
                    if key != "GT" {
                        if let Some(Ok(Some(value))) = sample.get(header, key) {
                            fields.insert(key.clone(), format!("{value:?}"));
                        }
                    }
                }

                genotypes.push(Genotype {
                    sample: sample_name.clone(),
                    gt,
                    fields,
                });
            }
        }

        genotypes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_vcf() {
        let vcf_content = r"##fileformat=VCFv4.2
#CHROM	POS	ID	REF	ALT	QUAL	FILTER	INFO
chr1	14370	rs6054257	G	A	29	PASS	NS=3;DP=14
";
        let doc = VcfParser::parse_str(vcf_content).unwrap();
        assert_eq!(doc.header.file_format, "VCFv4.2");
        assert_eq!(doc.variants.len(), 1);
        assert_eq!(doc.variants[0].chrom, "chr1");
        assert_eq!(doc.variants[0].pos, 14370);
        assert_eq!(doc.variants[0].id, Some("rs6054257".to_string()));
        assert_eq!(doc.variants[0].ref_bases, "G");
        assert_eq!(doc.variants[0].alt_alleles, vec!["A"]);
        assert_eq!(doc.variants[0].quality, Some(29.0));
        assert_eq!(doc.variants[0].filter, "PASS");
    }

    #[test]
    fn test_parse_multi_allelic() {
        let vcf_content = r"##fileformat=VCFv4.2
#CHROM	POS	ID	REF	ALT	QUAL	FILTER	INFO
chr1	1110696	rs6040355	A	G,T	67	PASS	NS=2;DP=10
";
        let doc = VcfParser::parse_str(vcf_content).unwrap();
        assert_eq!(doc.variants.len(), 1);
        assert_eq!(doc.variants[0].alt_alleles, vec!["G", "T"]);
    }

    #[test]
    fn test_parse_missing_data() {
        let vcf_content = r"##fileformat=VCFv4.2
#CHROM	POS	ID	REF	ALT	QUAL	FILTER	INFO
chr1	1230237	.	T	.	47	PASS	NS=3
";
        let doc = VcfParser::parse_str(vcf_content).unwrap();
        assert_eq!(doc.variants.len(), 1);
        assert_eq!(doc.variants[0].id, None);
        assert_eq!(doc.variants[0].alt_alleles, vec!["."]);
    }

    #[test]
    fn test_parse_with_genotypes() {
        let vcf_content = r#"##fileformat=VCFv4.2
##FORMAT=<ID=GT,Number=1,Type=String,Description="Genotype">
##FORMAT=<ID=GQ,Number=1,Type=Integer,Description="Genotype Quality">
#CHROM	POS	ID	REF	ALT	QUAL	FILTER	INFO	FORMAT	NA00001	NA00002
chr1	14370	rs6054257	G	A	29	PASS	NS=3	GT:GQ	0|0:48	1|0:48
"#;
        let doc = VcfParser::parse_str(vcf_content).unwrap();
        assert_eq!(doc.variants.len(), 1);
        assert_eq!(doc.variants[0].genotypes.len(), 2);
        assert_eq!(doc.variants[0].genotypes[0].sample, "NA00001");
        assert_eq!(doc.variants[0].genotypes[1].sample, "NA00002");
    }

    #[test]
    fn test_parse_reference_genome() {
        let vcf_content = r"##fileformat=VCFv4.2
##reference=file:///seq/references/1000GenomesPilot-NCBI36.fasta
#CHROM	POS	ID	REF	ALT	QUAL	FILTER	INFO
chr1	14370	rs6054257	G	A	29	PASS	NS=3
";
        let doc = VcfParser::parse_str(vcf_content).unwrap();
        assert!(doc.header.reference.is_some());
        let reference = doc.header.reference.as_ref().unwrap();
        assert!(reference.contains("1000GenomesPilot-NCBI36.fasta"));
    }

    #[test]
    fn test_parse_no_reference() {
        let vcf_content = r"##fileformat=VCFv4.2
#CHROM	POS	ID	REF	ALT	QUAL	FILTER	INFO
chr1	14370	rs6054257	G	A	29	PASS	NS=3
";
        let doc = VcfParser::parse_str(vcf_content).unwrap();
        assert!(doc.header.reference.is_none());
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn test_statistics() {
        let vcf_content = r"##fileformat=VCFv4.2
#CHROM	POS	ID	REF	ALT	QUAL	FILTER	INFO
chr1	14370	rs1	G	A	29	PASS	NS=3
chr1	14371	rs2	CT	C	50	PASS	NS=3
chr1	14372	rs3	C	CT	60	q10	NS=3
chr1	14373	rs4	A	G	40	PASS	NS=3
";
        let doc = VcfParser::parse_str(vcf_content).unwrap();
        let stats = doc.statistics();
        assert_eq!(stats.total_variants, 4);
        assert_eq!(stats.snp_count, 2); // rs1, rs4
        assert_eq!(stats.deletion_count, 1); // rs2
        assert_eq!(stats.insertion_count, 1); // rs3
        assert_eq!(stats.pass_count, 3);
        assert_eq!(stats.filtered_count, 1);
        assert_eq!(stats.mean_quality, (29.0 + 50.0 + 60.0 + 40.0) / 4.0);
    }

    #[test]
    fn test_parse_small_variants_file() {
        // Test parsing the actual test corpus file
        let path = std::path::Path::new("../../test-corpus/genomics/vcf/small_variants.vcf");
        if !path.exists() {
            // Skip if test corpus not available
            return;
        }

        let doc = VcfParser::parse_file(path).unwrap();

        // Verify header
        assert_eq!(doc.header.file_format, "VCFv4.2");
        assert_eq!(doc.header.samples.len(), 3);
        assert!(doc.header.samples.contains(&"NA00001".to_string()));
        assert!(doc.header.samples.contains(&"NA00002".to_string()));
        assert!(doc.header.samples.contains(&"NA00003".to_string()));

        // Verify reference genome extraction
        assert!(doc.header.reference.is_some());
        let reference = doc.header.reference.as_ref().unwrap();
        assert!(reference.contains("1000GenomesPilot-NCBI36.fasta"));

        // Verify contigs
        assert!(doc.header.contigs.contains(&"20".to_string()));

        // Verify variants
        assert_eq!(doc.variants.len(), 5); // small_variants.vcf has 5 variants

        // Verify first variant
        let v1 = &doc.variants[0];
        assert_eq!(v1.chrom, "20");
        assert_eq!(v1.pos, 14370);
        assert_eq!(v1.id, Some("rs6054257".to_string()));
        assert_eq!(v1.ref_bases, "G");
        assert_eq!(v1.alt_alleles, vec!["A"]);
        assert_eq!(v1.quality, Some(29.0));
        assert_eq!(v1.filter, "PASS");

        // Verify genotypes
        assert_eq!(v1.genotypes.len(), 3);
        assert_eq!(v1.genotypes[0].sample, "NA00001");
        assert!(v1.genotypes[0].gt.is_some());
    }
}
