mod parser;
mod serializer;
mod types;

pub use parser::VcfParser;
pub use serializer::to_markdown;
pub use types::{Genotype, InfoValue, Variant, VcfDocument, VcfHeader};
