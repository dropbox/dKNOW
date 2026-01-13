//! Build logo embedding database using CLIP
//!
//! This tool scans a directory of logo images, extracts CLIP embeddings,
//! and saves them to a binary database file for fast similarity search.

use anyhow::{Context, Result};
use image::ImageReader;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

// Import from video_audio_embeddings crate
use video_audio_embeddings::{CLIPModel, VisionEmbeddingConfig, VisionEmbeddings};

/// Logo metadata entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogoEntry {
    /// Unique logo ID
    pub id: String,
    /// Brand name
    pub brand: String,
    /// Category (e.g., "tech", "sportswear", "food")
    pub category: String,
    /// Path to logo image (relative to database directory)
    pub image_path: String,
    /// CLIP embedding (512-dim for ViT-B/32)
    pub embedding: Vec<f32>,
}

/// Logo database containing all logo embeddings
#[derive(Debug, Serialize, Deserialize)]
pub struct LogoDatabase {
    /// Model used for embeddings
    pub model: String,
    /// Embedding dimension
    pub embedding_dim: usize,
    /// Logo entries with embeddings
    pub logos: Vec<LogoEntry>,
}

impl LogoDatabase {
    /// Create a new empty logo database
    pub fn new(model: String, embedding_dim: usize) -> Self {
        Self {
            model,
            embedding_dim,
            logos: Vec::new(),
        }
    }

    /// Add a logo entry
    pub fn add_logo(&mut self, entry: LogoEntry) {
        self.logos.push(entry);
    }

    /// Save database to JSON file
    pub fn save_to_json<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let json = serde_json::to_string_pretty(self)
            .context("Failed to serialize logo database")?;
        fs::write(path.as_ref(), json)
            .with_context(|| format!("Failed to write database to {:?}", path.as_ref()))?;
        println!("Saved logo database to: {:?}", path.as_ref());
        Ok(())
    }

    /// Load database from JSON file
    pub fn load_from_json<P: AsRef<Path>>(path: P) -> Result<Self> {
        let json = fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read database from {:?}", path.as_ref()))?;
        let db = serde_json::from_str(&json).context("Failed to deserialize logo database")?;
        Ok(db)
    }
}

/// Scan directory for logo images and categorize them
fn scan_logo_directory<P: AsRef<Path>>(logo_dir: P) -> Result<Vec<(String, String, PathBuf)>> {
    let mut logos = Vec::new();

    // Expected structure: logo_dir/<category>/<brand_name>.png
    // Example: logos/tech/apple.png, logos/sportswear/nike.png

    for category_entry in fs::read_dir(logo_dir.as_ref())
        .with_context(|| format!("Failed to read logo directory: {:?}", logo_dir.as_ref()))?
    {
        let category_entry = category_entry?;
        let category_path = category_entry.path();

        if !category_path.is_dir() {
            continue; // Skip non-directory files
        }

        let category_name = category_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        // Scan images in category directory
        for image_entry in fs::read_dir(&category_path)? {
            let image_entry = image_entry?;
            let image_path = image_entry.path();

            // Only process image files (png, jpg, jpeg, webp)
            if let Some(ext) = image_path.extension() {
                let ext = ext.to_str().unwrap_or("");
                if !matches!(ext.to_lowercase().as_str(), "png" | "jpg" | "jpeg" | "webp") {
                    continue;
                }
            } else {
                continue;
            }

            // Extract brand name from filename (remove extension)
            let brand_name = image_path
                .file_stem()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            logos.push((brand_name, category_name.clone(), image_path));
        }
    }

    Ok(logos)
}

/// Build logo embedding database
fn build_logo_database<P: AsRef<Path>>(
    logo_dir: P,
    model_path: &str,
    output_path: P,
) -> Result<()> {
    println!("Building logo embedding database...");
    println!("Logo directory: {:?}", logo_dir.as_ref());
    println!("CLIP model: {}", model_path);

    // Scan logo directory
    let logos = scan_logo_directory(&logo_dir)?;
    println!("Found {} logo images", logos.len());

    if logos.is_empty() {
        anyhow::bail!("No logo images found in directory: {:?}", logo_dir.as_ref());
    }

    // Initialize CLIP embeddings extractor
    let config = VisionEmbeddingConfig {
        model: CLIPModel::VitB32,
        model_path: model_path.to_string(),
        normalize: true,
        image_size: 224,
    };

    println!("Loading CLIP model...");
    let mut extractor = VisionEmbeddings::new(config.clone())?;
    println!("CLIP model loaded successfully");

    // Create database
    let mut database = LogoDatabase::new(
        "clip-vit-b32".to_string(),
        CLIPModel::VitB32.embedding_dim(),
    );

    // Process logos in batches
    const BATCH_SIZE: usize = 32;
    for (i, batch) in logos.chunks(BATCH_SIZE).enumerate() {
        println!(
            "Processing batch {}/{} ({} logos)...",
            i + 1,
            (logos.len() + BATCH_SIZE - 1) / BATCH_SIZE,
            batch.len()
        );

        // Load images for this batch
        let mut images = Vec::new();
        let mut valid_logos = Vec::new();

        for (brand, category, path) in batch {
            match ImageReader::open(path)
                .with_context(|| format!("Failed to open image: {:?}", path))?
                .decode()
            {
                Ok(img) => {
                    images.push(img);
                    valid_logos.push((brand.clone(), category.clone(), path.clone()));
                }
                Err(e) => {
                    eprintln!("Warning: Failed to decode image {:?}: {}", path, e);
                }
            }
        }

        if images.is_empty() {
            continue;
        }

        // Extract embeddings for batch
        let embeddings = extractor
            .extract_embeddings(&images)
            .context("Failed to extract embeddings")?;

        // Add to database
        for (embedding, (brand, category, path)) in embeddings.into_iter().zip(valid_logos) {
            let logo_id = format!("{}_{}", category, brand);
            let relative_path = path
                .strip_prefix(logo_dir.as_ref())
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();

            database.add_logo(LogoEntry {
                id: logo_id,
                brand,
                category,
                image_path: relative_path,
                embedding,
            });
        }

        println!(
            "  Extracted {} embeddings (total: {})",
            images.len(),
            database.logos.len()
        );
    }

    // Save database
    println!("Saving logo database...");
    database.save_to_json(output_path)?;

    println!("✅ Logo database created successfully!");
    println!("  Total logos: {}", database.logos.len());
    println!("  Embedding dimension: {}", database.embedding_dim);

    Ok(())
}

fn main() -> Result<()> {
    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 4 {
        eprintln!("Usage: {} <logo_dir> <clip_model_path> <output_json>", args[0]);
        eprintln!();
        eprintln!("Example:");
        eprintln!(
            "  {} models/logo-detection/clip_database/logos \\",
            args[0]
        );
        eprintln!("     models/embeddings/clip_vit_b32.onnx \\");
        eprintln!("     models/logo-detection/clip_database/logo_database.json");
        std::process::exit(1);
    }

    let logo_dir = &args[1];
    let model_path = &args[2];
    let output_path = &args[3];

    // Verify paths exist
    if !Path::new(logo_dir).exists() {
        anyhow::bail!("Logo directory does not exist: {}", logo_dir);
    }
    if !Path::new(model_path).exists() {
        anyhow::bail!("CLIP model file does not exist: {}", model_path);
    }

    build_logo_database(logo_dir, model_path, output_path)?;

    Ok(())
}
