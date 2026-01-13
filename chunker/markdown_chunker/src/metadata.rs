// src/metadata.rs

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub content: String,
    pub metadata: ChunkMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMetadata {
    pub position: usize,
    pub token_count: usize,
    pub char_count: usize,
    pub language: Option<String>,
    pub chunk_type: ChunkType,
    pub header_hierarchy: Vec<(usize, String)>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ChunkType {
    Paragraph,
    CodeBlock,
    List,
    Table,
    Quote,
    Heading,
}
