//! Prototype VCF parser using `noodles-vcf`
//!
//! Purpose: Learn `noodles-vcf` API patterns before integrating into `docling-genomics`
//!
//! Usage: `cargo run --example vcf_prototype -- <path-to-vcf>`
//! Example: `cargo run --example vcf_prototype -- ../../test-corpus/genomics/vcf/small_variants.vcf`

#![allow(
    clippy::unnecessary_wraps,
    clippy::match_same_arms,
    clippy::needless_collect
)]

use std::{env, fs::File, io::BufReader};

use noodles_vcf as vcf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let vcf_path = if args.len() > 1 {
        &args[1]
    } else {
        "../../test-corpus/genomics/vcf/small_variants.vcf"
    };

    println!("Reading VCF file: {vcf_path}");
    println!();

    // Open and parse VCF file
    let mut reader = File::open(vcf_path)
        .map(BufReader::new)
        .map(vcf::io::Reader::new)?;

    let header = reader.read_header()?;

    // Print header information
    print_header(&header)?;

    // Print variant records (first 5)
    print_variants(&header, &mut reader)?;

    Ok(())
}

fn print_header(header: &vcf::Header) -> Result<(), Box<dyn std::error::Error>> {
    println!("=== VCF HEADER ===");
    println!();

    // File format
    println!("File Format: {:?}", header.file_format());

    // Samples
    let sample_names: Vec<_> = header.sample_names().iter().collect();
    println!("Samples ({} total): {:?}", sample_names.len(), sample_names);
    println!();

    // Contigs
    println!("Contigs:");
    for (name, contig) in header.contigs() {
        print!("  - {name}");
        if let Some(length) = contig.length() {
            print!(" (length: {length})");
        }
        println!();
    }
    println!();

    // INFO fields
    println!("INFO fields:");
    for (key, info) in header.infos() {
        println!("  - {}: {}", key, info.description());
    }
    println!();

    // FORMAT fields
    println!("FORMAT fields:");
    for (key, format) in header.formats() {
        println!("  - {}: {}", key, format.description());
    }
    println!();

    // FILTER fields
    println!("FILTER fields:");
    for (key, filter) in header.filters() {
        println!("  - {}: {}", key, filter.description());
    }
    println!();

    Ok(())
}

fn print_variants(
    header: &vcf::Header,
    reader: &mut vcf::io::Reader<BufReader<File>>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("=== VARIANT RECORDS (first 5) ===");
    println!();

    for (i, result) in reader.records().enumerate() {
        if i >= 5 {
            break;
        }

        let record = result?;

        println!("Variant {}:", i + 1);

        // Basic fields using Record trait methods
        println!("  CHROM: {}", record.reference_sequence_name());
        println!("  POS: {:?}", record.variant_start());
        println!("  ID: {:?}", record.ids());
        println!("  REF: {}", record.reference_bases());
        println!("  ALT: {:?}", record.alternate_bases());

        // Quality score (Option<Result<f32>>)
        match record.quality_score() {
            Some(Ok(qual)) => {
                println!("  QUAL: {qual}");
            }
            Some(Err(e)) => {
                println!("  QUAL: <error: {e}>");
            }
            None => {
                println!("  QUAL: .");
            }
        }

        // Filters
        println!("  FILTER: {:?}", record.filters());

        // INFO field
        println!("  INFO: {:?}", record.info());

        // Genotypes (if present)
        if !header.sample_names().is_empty() {
            println!("  Genotypes:");
            print_genotypes(header, &record)?;
        }

        println!();
    }

    Ok(())
}

fn print_genotypes(
    header: &vcf::Header,
    record: &vcf::Record,
) -> Result<(), Box<dyn std::error::Error>> {
    use vcf::variant::record::samples::Sample;

    let samples = record.samples();

    // Iterate through each sample
    for sample_name in header.sample_names() {
        print!("    {sample_name}: ");

        // Get sample data
        if let Some(sample) = samples.get(header, sample_name) {
            // Try to get genotype (GT field)
            if let Some(genotype_result) = sample.get(header, "GT") {
                match genotype_result {
                    Ok(Some(gt_value)) => {
                        print!("GT={gt_value:?}");
                    }
                    Ok(None) => {
                        print!("GT=.");
                    }
                    Err(e) => {
                        print!("GT=<error: {e}>");
                    }
                }
            }

            // Try to get other FORMAT fields
            for (key, _) in header.formats() {
                if key != "GT" {
                    if let Some(value_result) = sample.get(header, key) {
                        match value_result {
                            Ok(Some(value)) => {
                                print!(", {key}={value:?}");
                            }
                            Ok(None) => {
                                // Skip missing values
                            }
                            Err(_) => {
                                // Skip errors
                            }
                        }
                    }
                }
            }
            println!();
        } else {
            println!("(no data)");
        }
    }

    Ok(())
}
