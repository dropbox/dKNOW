//! Convert VCF file to Markdown
//!
//! Usage: `cargo run --example vcf_to_markdown -- <path-to-vcf>`
//! Example: `cargo run --example vcf_to_markdown -- ../../test-corpus/genomics/vcf/small_variants.vcf`

use std::env;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let vcf_path = if args.len() > 1 {
        &args[1]
    } else {
        "../../test-corpus/genomics/vcf/small_variants.vcf"
    };

    let path = Path::new(vcf_path);
    if !path.exists() {
        eprintln!("Error: File not found: {vcf_path}");
        return Err("File not found".into());
    }

    println!("Reading VCF file: {vcf_path}");
    println!();

    // Parse VCF file
    let doc = docling_genomics::vcf::VcfParser::parse_file(path)?;

    // Generate markdown
    let markdown = docling_genomics::vcf::to_markdown(&doc);

    // Print markdown
    println!("{markdown}");

    Ok(())
}
