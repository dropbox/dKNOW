//! # docling-genomics
//!
//! Genomics and bioinformatics file format parsers for docling-rs.
//!
//! This crate provides parsing support for genomics file formats commonly used in
//! bioinformatics research, variant analysis, and clinical genomics workflows.
//!
//! ## Supported Formats
//!
//! | Format | Extension | Description |
//! |--------|-----------|-------------|
//! | VCF | `.vcf`, `.vcf.gz` | Variant Call Format - genetic variant data |
//!
//! ## What is VCF?
//!
//! VCF (Variant Call Format) is a text file format used in bioinformatics to store
//! gene sequence variations. It was developed for the 1000 Genomes Project and has
//! become the standard format for:
//!
//! - **SNPs** (Single Nucleotide Polymorphisms)
//! - **Insertions and deletions** (indels)
//! - **Structural variants**
//! - **Copy number variations**
//!
//! VCF files contain:
//! - **Header lines** (`##`) with metadata about the file, reference genome, and field definitions
//! - **Column header** (`#CHROM`) defining the data columns
//! - **Variant records** with chromosome, position, reference/alternate alleles, quality, and per-sample genotypes
//!
//! ## Quick Start
//!
//! ### Parse a VCF File
//!
//! ```rust,no_run
//! use docling_genomics::{VcfParser, VcfDocument};
//!
//! // Parse VCF file to structured document
//! let doc = VcfParser::parse_file("sample.vcf")?;
//!
//! // Access header information
//! println!("File format: {}", doc.header.file_format);
//! println!("Samples: {:?}", doc.header.samples);
//!
//! // Iterate over variants
//! for variant in &doc.variants {
//!     println!("{}:{} {} -> {:?}",
//!         variant.chrom, variant.pos,
//!         variant.ref_bases, variant.alt_alleles
//!     );
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ### Convert to Markdown
//!
//! ```rust,no_run
//! use docling_genomics::{VcfParser, vcf_to_markdown};
//!
//! let doc = VcfParser::parse_file("variants.vcf")?;
//! let markdown = vcf_to_markdown(&doc);
//!
//! // Markdown includes:
//! // - File metadata summary
//! // - Variant statistics
//! // - Variant table with key columns
//! println!("{}", markdown);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ### Analyze Variant Statistics
//!
//! ```rust,no_run
//! use docling_genomics::VcfParser;
//!
//! let doc = VcfParser::parse_file("variants.vcf")?;
//! let stats = doc.statistics();
//!
//! println!("Total variants: {}", stats.total_variants);
//! println!("SNPs: {}", stats.snp_count);
//! println!("Insertions: {}", stats.insertion_count);
//! println!("Deletions: {}", stats.deletion_count);
//! println!("PASS variants: {}", stats.pass_count);
//! println!("Mean quality: {:.2}", stats.mean_quality);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## VCF Structure
//!
//! ### Header Metadata
//!
//! The parser extracts key header information:
//!
//! - **File format version** (e.g., VCFv4.2)
//! - **Reference genome** path or identifier
//! - **INFO field definitions** - variant-level annotations
//! - **FORMAT field definitions** - per-sample genotype fields
//! - **FILTER definitions** - quality filter explanations
//! - **Contig/chromosome** definitions
//! - **Sample names** from the column header
//!
//! ### Variant Fields
//!
//! Each variant record contains:
//!
//! | Field | Description |
//! |-------|-------------|
//! | `chrom` | Chromosome (e.g., "chr1", "1", "X") |
//! | `pos` | 1-based position on chromosome |
//! | `id` | Variant identifier (e.g., rsID) |
//! | `ref_bases` | Reference allele bases |
//! | `alt_alleles` | Alternative allele(s) |
//! | `quality` | Phred-scaled quality score |
//! | `filter` | PASS or filter name(s) |
//! | `info` | Variant-level annotations |
//! | `genotypes` | Per-sample genotype data |
//!
//! ### Genotype Data
//!
//! For multi-sample VCF files, each variant includes genotype information:
//!
//! ```rust,ignore
//! for variant in &doc.variants {
//!     for genotype in &variant.genotypes {
//!         println!("Sample: {}", genotype.sample);
//!         println!("  GT: {:?}", genotype.gt);  // e.g., "0/1" for heterozygous
//!         for (key, value) in &genotype.fields {
//!             println!("  {}: {}", key, value);  // e.g., DP, GQ, AD
//!         }
//!     }
//! }
//! ```
//!
//! ## Use Cases
//!
//! - **Clinical reporting**: Extract variant summaries for patient reports
//! - **Research documentation**: Convert variant calls to readable format
//! - **Data validation**: Parse and verify VCF file structure
//! - **Pipeline integration**: Use as part of bioinformatics workflows
//!
//! ## Limitations
//!
//! - Compressed VCF (`.vcf.gz`) requires external decompression
//! - Very large VCF files (millions of variants) may require streaming
//! - Complex structural variants may have simplified representation
//!
//! ## Feature Flags
//!
//! This crate is included by default in `docling-backend`. No feature flags required.

/// VCF (Variant Call Format) file parser and document generator
pub mod vcf;

pub use vcf::{to_markdown as vcf_to_markdown, VcfDocument, VcfParser};
