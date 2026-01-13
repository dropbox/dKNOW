use super::types::VcfDocument;
use std::fmt::Write;

/// Convert a VCF document to Markdown format
#[must_use = "serialization returns markdown string"]
#[allow(clippy::too_many_lines)] // Complex VCF serialization - keeping together for clarity
pub fn to_markdown(doc: &VcfDocument) -> String {
    let mut md = String::new();

    // Title
    md.push_str("# VCF - Genomic Variant Call Format\n\n");

    // File Information
    md.push_str("## File Information\n\n");
    let _ = writeln!(md, "- **File Format:** {}", doc.header.file_format);

    if let Some(ref reference) = doc.header.reference {
        let _ = writeln!(md, "- **Reference Genome:** {reference}");
    }

    if !doc.header.samples.is_empty() {
        let _ = writeln!(
            md,
            "- **Samples:** {} ({})",
            doc.header.samples.len(),
            doc.header.samples.join(", ")
        );
    }

    if !doc.header.contigs.is_empty() {
        let _ = writeln!(
            md,
            "- **Contigs:** {} ({})",
            doc.header.contigs.len(),
            if doc.header.contigs.len() <= 5 {
                doc.header.contigs.join(", ")
            } else {
                format!(
                    "{}, ... and {} more",
                    doc.header.contigs[..5].join(", "),
                    doc.header.contigs.len() - 5
                )
            }
        );
    }

    md.push('\n');

    // Variant Summary Statistics
    let stats = doc.statistics();
    md.push_str("## Variant Summary\n\n");
    let _ = writeln!(md, "- **Total Variants:** {}", stats.total_variants);
    #[allow(
        clippy::cast_precision_loss,
        reason = "percentage calculation from integer counts, precision loss is acceptable"
    )]
    let snp_pct = if stats.total_variants > 0 {
        (stats.snp_count as f32 / stats.total_variants as f32) * 100.0
    } else {
        0.0
    };
    let _ = writeln!(md, "- **SNPs:** {} ({snp_pct:.1}%)", stats.snp_count);
    #[allow(
        clippy::cast_precision_loss,
        reason = "percentage calculation from integer counts, precision loss is acceptable"
    )]
    let ins_pct = if stats.total_variants > 0 {
        (stats.insertion_count as f32 / stats.total_variants as f32) * 100.0
    } else {
        0.0
    };
    let _ = writeln!(
        md,
        "- **Insertions:** {} ({ins_pct:.1}%)",
        stats.insertion_count
    );
    #[allow(
        clippy::cast_precision_loss,
        reason = "percentage calculation from integer counts, precision loss is acceptable"
    )]
    let del_pct = if stats.total_variants > 0 {
        (stats.deletion_count as f32 / stats.total_variants as f32) * 100.0
    } else {
        0.0
    };
    let _ = writeln!(
        md,
        "- **Deletions:** {} ({del_pct:.1}%)",
        stats.deletion_count
    );

    md.push('\n');

    // Sample Variants table (if any)
    if !doc.variants.is_empty() {
        md.push_str("## Sample Variants\n\n");
        md.push_str("| CHROM | POS | ID | REF | ALT | QUAL | FILTER |\n");
        md.push_str("|-------|-----|----|-----|-----|------|--------|");
        md.push('\n');

        for variant in doc.variants.iter().take(20) {
            let id_str = variant.id.as_deref().unwrap_or(".");
            let qual_str = variant
                .quality
                .map_or_else(|| ".".to_string(), |q| format!("{q:.1}"));
            let alt_str = variant.alt_alleles.join(",");

            let _ = writeln!(
                md,
                "| {} | {} | {} | {} | {} | {} | {} |",
                variant.chrom,
                variant.pos,
                id_str,
                variant.ref_bases,
                alt_str,
                qual_str,
                variant.filter
            );
        }

        if doc.variants.len() > 20 {
            let _ = writeln!(
                md,
                "\n*({} more variants not shown)*",
                doc.variants.len() - 20
            );
        }

        md.push('\n');
    }

    // Genotype section (if samples present, show first variant)
    if !doc.header.samples.is_empty() && !doc.variants.is_empty() {
        md.push_str("## Sample Genotypes (First Variant)\n\n");
        md.push_str("| Sample | Genotype | Additional Fields |\n");
        md.push_str("|--------|----------|-------------------|\n");

        for genotype in &doc.variants[0].genotypes {
            // Clean up genotype field - extract inner value from debug format
            let gt = genotype.gt.as_deref().map_or_else(
                || ".".to_string(),
                |s| {
                    // Parse "Genotype(Genotype(\"0|0\"))" to "0|0"
                    if s.starts_with("Genotype(Genotype(\"") {
                        s.trim_start_matches("Genotype(Genotype(\"")
                            .trim_end_matches("\"))")
                            .to_string()
                    } else {
                        s.to_string()
                    }
                },
            );

            let fields_str = if genotype.fields.is_empty() {
                "-".to_string()
            } else {
                genotype
                    .fields
                    .iter()
                    .map(|(k, v)| {
                        // Clean up field values - extract from Integer(), Float(), etc.
                        let cleaned_value = if v.starts_with("Integer(") {
                            v.trim_start_matches("Integer(")
                                .trim_end_matches(')')
                                .to_string()
                        } else if v.starts_with("Float(") {
                            v.trim_start_matches("Float(")
                                .trim_end_matches(')')
                                .to_string()
                        } else if v.starts_with("Array([") {
                            // Simplify array display: "Array([Ok(Some(51)), Ok(Some(51))])" -> "[51, 51]"
                            v.replace("Ok(Some(", "")
                                .replace("Ok(None)", ".")
                                .replace("))", "")
                                .replace("Array([", "[")
                                .replace("])", "]")
                        } else {
                            v.clone()
                        };
                        format!("{k}={cleaned_value}")
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            };

            let _ = writeln!(md, "| {} | {} | {} |", genotype.sample, gt, fields_str);
        }

        md.push('\n');
    }

    md
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vcf::types::{VcfDocument, VcfHeader};

    #[test]
    fn test_to_markdown_empty() {
        let doc = VcfDocument::new(VcfHeader::default(), Vec::new());
        let md = to_markdown(&doc);

        assert!(md.contains("# VCF - Genomic Variant Call Format"));
        assert!(md.contains("VCFv4.2"));
        assert!(md.contains("Total Variants:** 0"));
    }

    #[test]
    fn test_to_markdown_with_variants() {
        use crate::vcf::parser::VcfParser;

        let vcf_content = r"##fileformat=VCFv4.2
#CHROM	POS	ID	REF	ALT	QUAL	FILTER	INFO
chr1	14370	rs6054257	G	A	29	PASS	NS=3;DP=14
chr1	14371	rs2	CT	C	50	PASS	NS=3
";
        let doc = VcfParser::parse_str(vcf_content).unwrap();
        let md = to_markdown(&doc);

        // Check header
        assert!(md.contains("# VCF - Genomic Variant Call Format"));
        assert!(md.contains("VCFv4.2"));

        // Check statistics
        assert!(md.contains("Total Variants:** 2"));
        assert!(md.contains("SNPs:"));
        assert!(md.contains("Insertions:"));
        assert!(md.contains("Deletions:"));

        // Check variant table
        assert!(md.contains("| CHROM | POS | ID | REF | ALT | QUAL | FILTER |"));
        assert!(md.contains("| chr1 | 14370 | rs6054257 | G | A | 29.0 | PASS |"));
        assert!(md.contains("| chr1 | 14371 | rs2 | CT | C | 50.0 | PASS |"));
    }

    #[test]
    fn test_to_markdown_with_genotypes() {
        use crate::vcf::parser::VcfParser;

        let vcf_content = r#"##fileformat=VCFv4.2
##FORMAT=<ID=GT,Number=1,Type=String,Description="Genotype">
##FORMAT=<ID=GQ,Number=1,Type=Integer,Description="Genotype Quality">
#CHROM	POS	ID	REF	ALT	QUAL	FILTER	INFO	FORMAT	NA00001	NA00002
chr1	14370	rs6054257	G	A	29	PASS	NS=3	GT:GQ	0|0:48	1|0:48
"#;
        let doc = VcfParser::parse_str(vcf_content).unwrap();
        let md = to_markdown(&doc);

        // Check genotype section exists
        assert!(md.contains("## Sample Genotypes (First Variant)"));
        assert!(md.contains("| Sample | Genotype | Additional Fields |"));

        // Check sample data
        assert!(md.contains("| NA00001 |"));
        assert!(md.contains("| NA00002 |"));
    }

    #[test]
    fn test_to_markdown_many_variants() {
        use crate::vcf::types::Variant;

        // Create a document with many variants (more than 20)
        let header = VcfHeader::default();
        let mut variants = Vec::new();

        for i in 1..=25 {
            variants.push(Variant {
                chrom: "chr1".to_string(),
                pos: 10000 + i,
                id: Some(format!("rs{i}")),
                ref_bases: "A".to_string(),
                alt_alleles: vec!["G".to_string()],
                quality: Some(30.0),
                filter: "PASS".to_string(),
                info: std::collections::HashMap::new(),
                genotypes: Vec::new(),
            });
        }

        let doc = VcfDocument::new(header, variants);
        let md = to_markdown(&doc);

        // Should show first 20 variants
        assert!(md.contains("| chr1 | 10001 |"));
        assert!(md.contains("| chr1 | 10020 |"));

        // Should have note about remaining variants
        assert!(md.contains("(5 more variants not shown)"));
    }
}
