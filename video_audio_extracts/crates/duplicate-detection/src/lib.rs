//! Duplicate Detection Library
//!
//! Provides perceptual hashing and similarity detection for:
//! - Images (pHash, aHash, dHash, block hash)
//! - Videos (keyframe-based perceptual hashing)
//! - Audio (spectral fingerprinting)

pub mod plugin;

use anyhow::{Context, Result};
use img_hash::image::DynamicImage;
use img_hash::{HashAlg, HasherConfig};
use rustfft::{num_complex::Complex, FftPlanner};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

/// Duplicate detection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateDetectionConfig {
    /// Hash algorithm to use
    pub hash_algorithm: HashAlgorithm,
    /// Hash size (larger = more accurate but slower comparison)
    pub hash_size: u32,
    /// Similarity threshold (0.0-1.0, where 1.0 = identical)
    pub similarity_threshold: f32,
    /// For video: number of keyframes to sample
    pub video_keyframes: usize,
    /// For audio: spectrogram resolution
    pub audio_resolution: usize,
}

impl Default for DuplicateDetectionConfig {
    fn default() -> Self {
        Self {
            hash_algorithm: HashAlgorithm::Gradient,
            hash_size: 8,              // 8x8 = 64-bit hash (good balance)
            similarity_threshold: 0.9, // 90% similar = duplicate
            video_keyframes: 10,
            audio_resolution: 32,
        }
    }
}

/// Perceptual hash algorithms
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum HashAlgorithm {
    /// Average Hash (aHash) - fast, simple
    Mean,
    /// Gradient Hash (gradient-based) - robust to color/brightness changes
    Gradient,
    /// Discrete Cosine Transform Hash (pHash) - most accurate
    DCT,
    /// Block Hash - good for varied content
    Block,
    /// Difference Hash (dHash) - fast, gradient-based
    VertGradient,
    /// Double Gradient Hash
    DoubleGradient,
}

impl HashAlgorithm {
    /// Convert to img_hash HashAlg
    fn to_img_hash_alg(self) -> HashAlg {
        match self {
            HashAlgorithm::Mean => HashAlg::Mean,
            HashAlgorithm::Gradient => HashAlg::Gradient,
            HashAlgorithm::DCT => HashAlg::DoubleGradient, // DCT not directly available, use DoubleGradient
            HashAlgorithm::Block => HashAlg::Blockhash,
            HashAlgorithm::VertGradient => HashAlg::VertGradient,
            HashAlgorithm::DoubleGradient => HashAlg::DoubleGradient,
        }
    }
}

/// Perceptual hash result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerceptualHash {
    /// Hash algorithm used
    pub algorithm: HashAlgorithm,
    /// Hash size (e.g., 8 for 8x8 = 64 bits)
    pub hash_size: u32,
    /// Hash bytes (base64-encoded for JSON serialization)
    #[serde(with = "base64_serde")]
    pub hash: Vec<u8>,
    /// Media type
    pub media_type: MediaType,
}

/// Media type for duplicate detection
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MediaType {
    Image,
    Video,
    Audio,
}

/// Similarity result between two media items
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarityResult {
    /// Similarity score (0.0-1.0, where 1.0 = identical)
    pub similarity: f32,
    /// Hamming distance (number of differing bits)
    pub hamming_distance: u32,
    /// Total bits compared
    pub total_bits: u32,
    /// Whether items are considered duplicates based on threshold
    pub is_duplicate: bool,
}

/// Duplicate detector
pub struct DuplicateDetector {
    hasher: img_hash::Hasher,
    config: DuplicateDetectionConfig,
}

impl DuplicateDetector {
    /// Create a new duplicate detector with configuration
    pub fn new(config: DuplicateDetectionConfig) -> Self {
        let hasher = HasherConfig::new()
            .hash_alg(config.hash_algorithm.to_img_hash_alg())
            .hash_size(config.hash_size, config.hash_size)
            .to_hasher();

        info!(
            "Duplicate detector initialized: algorithm={:?}, hash_size={}x{}, threshold={}",
            config.hash_algorithm, config.hash_size, config.hash_size, config.similarity_threshold
        );

        Self { hasher, config }
    }

    /// Compute perceptual hash for an image
    pub fn hash_image(&self, image: &DynamicImage) -> Result<PerceptualHash> {
        debug!("Computing perceptual hash for image");

        let img_hash = self.hasher.hash_image(image);

        Ok(PerceptualHash {
            algorithm: self.config.hash_algorithm,
            hash_size: self.config.hash_size,
            hash: img_hash.as_bytes().to_vec(),
            media_type: MediaType::Image,
        })
    }

    /// Compute perceptual hash for multiple images (e.g., video keyframes)
    pub fn hash_images(&self, images: &[DynamicImage]) -> Result<Vec<PerceptualHash>> {
        debug!("Computing perceptual hashes for {} images", images.len());

        let mut hashes = Vec::with_capacity(images.len());
        for (i, img) in images.iter().enumerate() {
            let hash = self
                .hash_image(img)
                .with_context(|| format!("Failed to hash image {}", i))?;
            hashes.push(hash);
        }
        Ok(hashes)
    }

    /// Compute perceptual hash for video (samples keyframes)
    pub fn hash_video_keyframes(&self, keyframes: &[DynamicImage]) -> Result<PerceptualHash> {
        debug!(
            "Computing video perceptual hash from {} keyframes",
            keyframes.len()
        );

        if keyframes.is_empty() {
            anyhow::bail!("No keyframes provided for video hashing");
        }

        // Sample keyframes evenly
        let sample_count = self.config.video_keyframes.min(keyframes.len());
        let stride = if keyframes.len() > 1 {
            keyframes.len() / sample_count
        } else {
            1
        };

        let sampled: Vec<&DynamicImage> = keyframes
            .iter()
            .step_by(stride)
            .take(sample_count)
            .collect();

        debug!(
            "Sampled {} keyframes (stride={}) for video hash",
            sampled.len(),
            stride
        );

        // Hash each sampled keyframe
        let frame_hashes: Vec<Vec<u8>> = sampled
            .iter()
            .map(|img| {
                let hash = self.hasher.hash_image(*img);
                hash.as_bytes().to_vec()
            })
            .collect();

        // Concatenate all hashes to create video signature
        let hash_size = frame_hashes.first().map(|h| h.len()).unwrap_or(0);
        let mut combined_hash: Vec<u8> = Vec::with_capacity(frame_hashes.len() * hash_size);
        combined_hash.extend(frame_hashes.into_iter().flatten());

        Ok(PerceptualHash {
            algorithm: self.config.hash_algorithm,
            hash_size: self.config.hash_size,
            hash: combined_hash,
            media_type: MediaType::Video,
        })
    }

    /// Compute perceptual hash for audio (spectral fingerprint)
    pub fn hash_audio(&self, audio_samples: &[f32], sample_rate: u32) -> Result<PerceptualHash> {
        debug!(
            "Computing audio perceptual hash: {} samples @ {} Hz",
            audio_samples.len(),
            sample_rate
        );

        if audio_samples.is_empty() {
            anyhow::bail!("No audio samples provided");
        }

        // Compute spectrogram-based fingerprint
        let fingerprint = self.compute_audio_fingerprint(audio_samples, sample_rate)?;

        Ok(PerceptualHash {
            algorithm: self.config.hash_algorithm,
            hash_size: self.config.audio_resolution as u32,
            hash: fingerprint,
            media_type: MediaType::Audio,
        })
    }

    /// Compare two perceptual hashes and return similarity
    pub fn compare_hashes(
        &self,
        hash1: &PerceptualHash,
        hash2: &PerceptualHash,
    ) -> Result<SimilarityResult> {
        // Validate hashes are comparable
        if hash1.media_type != hash2.media_type {
            anyhow::bail!(
                "Cannot compare hashes of different media types: {:?} vs {:?}",
                hash1.media_type,
                hash2.media_type
            );
        }

        if hash1.algorithm != hash2.algorithm {
            anyhow::bail!(
                "Cannot compare hashes from different algorithms: {:?} vs {:?}",
                hash1.algorithm,
                hash2.algorithm
            );
        }

        // Compute Hamming distance
        let hamming_distance = Self::hamming_distance(&hash1.hash, &hash2.hash)?;
        let total_bits = (hash1.hash.len() * 8) as u32;

        // Compute similarity (1.0 - normalized hamming distance)
        let similarity = 1.0 - (hamming_distance as f32 / total_bits as f32);

        let is_duplicate = similarity >= self.config.similarity_threshold;

        debug!(
            "Hash comparison: hamming={}, total_bits={}, similarity={:.3}, duplicate={}",
            hamming_distance, total_bits, similarity, is_duplicate
        );

        Ok(SimilarityResult {
            similarity,
            hamming_distance,
            total_bits,
            is_duplicate,
        })
    }

    /// Compute Hamming distance between two byte arrays
    fn hamming_distance(bytes1: &[u8], bytes2: &[u8]) -> Result<u32> {
        if bytes1.len() != bytes2.len() {
            anyhow::bail!(
                "Hash length mismatch: {} vs {} bytes",
                bytes1.len(),
                bytes2.len()
            );
        }

        let distance: u32 = bytes1
            .iter()
            .zip(bytes2.iter())
            .map(|(b1, b2)| (b1 ^ b2).count_ones())
            .sum();

        Ok(distance)
    }

    /// Compute audio fingerprint using spectrogram-based hashing
    fn compute_audio_fingerprint(&self, samples: &[f32], _sample_rate: u32) -> Result<Vec<u8>> {
        let resolution = self.config.audio_resolution;

        // Downsample audio to fixed resolution
        let stride = samples.len() / resolution;
        let downsampled: Vec<f32> = if stride > 0 {
            samples
                .iter()
                .step_by(stride)
                .take(resolution)
                .copied()
                .collect()
        } else {
            samples.to_vec()
        };

        // Compute FFT for spectral representation
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(downsampled.len());

        let mut spectrum = Vec::with_capacity(downsampled.len());
        spectrum.extend(downsampled.iter().map(|&s| Complex::new(s, 0.0)));

        fft.process(&mut spectrum);

        // Extract magnitude spectrum (first half, positive frequencies)
        let half_len = spectrum.len() / 2;
        let mut magnitudes = Vec::with_capacity(half_len);
        magnitudes.extend(spectrum[..half_len].iter().map(|c| c.norm()));

        // Compute threshold (median magnitude)
        let mut sorted_mags = magnitudes.clone();
        sorted_mags.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let threshold = sorted_mags[sorted_mags.len() / 2];

        // Create binary hash: 1 if magnitude > threshold, 0 otherwise
        let mut hash_bits = Vec::with_capacity(magnitudes.len());
        hash_bits.extend(magnitudes.iter().map(|&m| m > threshold));

        // Pack bits into bytes
        let hash_bytes = Self::pack_bits(&hash_bits);

        Ok(hash_bytes)
    }

    /// Pack boolean bits into bytes
    fn pack_bits(bits: &[bool]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(bits.len().div_ceil(8));

        for chunk in bits.chunks(8) {
            let mut byte = 0u8;
            for (i, &bit) in chunk.iter().enumerate() {
                if bit {
                    byte |= 1 << i;
                }
            }
            bytes.push(byte);
        }

        bytes
    }
}

/// Helper module for base64 serialization of hash bytes
mod base64_serde {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use base64::{engine::general_purpose, Engine};
        serializer.serialize_str(&general_purpose::STANDARD.encode(bytes))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        use base64::{engine::general_purpose, Engine};
        let s = String::deserialize(deserializer)?;
        general_purpose::STANDARD
            .decode(&s)
            .map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_algorithm_conversion() {
        assert_eq!(HashAlgorithm::Mean.to_img_hash_alg(), HashAlg::Mean);
        assert_eq!(HashAlgorithm::Gradient.to_img_hash_alg(), HashAlg::Gradient);
        assert_eq!(HashAlgorithm::Block.to_img_hash_alg(), HashAlg::Blockhash);
    }

    #[test]
    fn test_hamming_distance() {
        let bytes1 = vec![0b00000000, 0b11111111];
        let bytes2 = vec![0b00000000, 0b11111111];
        assert_eq!(
            DuplicateDetector::hamming_distance(&bytes1, &bytes2).unwrap(),
            0
        );

        let bytes3 = vec![0b00000000, 0b11111111];
        let bytes4 = vec![0b11111111, 0b00000000];
        assert_eq!(
            DuplicateDetector::hamming_distance(&bytes3, &bytes4).unwrap(),
            16
        );

        let bytes5 = vec![0b00000000];
        let bytes6 = vec![0b00000001];
        assert_eq!(
            DuplicateDetector::hamming_distance(&bytes5, &bytes6).unwrap(),
            1
        );
    }

    #[test]
    fn test_pack_bits() {
        let bits = vec![true, false, true, false, true, false, true, false];
        let packed = DuplicateDetector::pack_bits(&bits);
        assert_eq!(packed, vec![0b01010101]);

        let bits2 = vec![true, true, true, true, true, true, true, true];
        let packed2 = DuplicateDetector::pack_bits(&bits2);
        assert_eq!(packed2, vec![0b11111111]);

        let bits3 = vec![false; 16];
        let packed3 = DuplicateDetector::pack_bits(&bits3);
        assert_eq!(packed3, vec![0b00000000, 0b00000000]);
    }

    #[test]
    fn test_default_config() {
        let config = DuplicateDetectionConfig::default();
        assert_eq!(config.hash_size, 8);
        assert_eq!(config.similarity_threshold, 0.9);
        assert_eq!(config.hash_algorithm, HashAlgorithm::Gradient);
    }
}
