// Image preprocessing for Idefics3 model (CodeFormula)
//
// Preprocessing steps:
// 1. Crop image to region bbox
// 2. Resize to 512x512 (model input size)
// 3. Normalize with ImageNet statistics
// 4. Convert to tensor [1, 3, H, W] format
//
// Reference: ~/docling/docling/models/code_formula_model.py
// - images_scale: 1.67 (120 DPI)
// - expansion_factor: 0.18

use crate::ocr::utils::{IMAGENET_MEAN, IMAGENET_STD};
use image::DynamicImage;
use tch::Tensor;

/// Image preprocessor for Idefics3 model
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Idefics3Preprocessor {
    image_size: u32,
    mean: [f32; 3],
    std: [f32; 3],
}

impl Idefics3Preprocessor {
    /// Create a new preprocessor with default ImageNet normalization
    #[inline]
    #[must_use = "returns a new preprocessor instance"]
    pub const fn new() -> Self {
        Self {
            image_size: 512,
            // ImageNet mean/std (RGB order)
            mean: IMAGENET_MEAN,
            std: IMAGENET_STD,
        }
    }

    /// Preprocess image for model input
    ///
    /// Steps:
    /// 1. Crop to bbox (if provided)
    /// 2. Resize to image_size x image_size
    /// 3. Normalize with ImageNet stats
    /// 4. Convert to tensor [1, 3, H, W]
    ///
    /// Returns: Tensor [1, 3, image_size, image_size]
    pub fn preprocess(
        &self,
        image: &DynamicImage,
        bbox: Option<(u32, u32, u32, u32)>, // (x, y, width, height)
    ) -> Result<Tensor, Box<dyn std::error::Error>> {
        // Step 1: Crop if bbox provided
        let image = if let Some((x, y, w, h)) = bbox {
            image.crop_imm(x, y, w, h)
        } else {
            image.clone()
        };

        // Step 2: Resize to model input size
        let resized = image.resize_exact(
            self.image_size,
            self.image_size,
            image::imageops::FilterType::Lanczos3,
        );

        // Step 3: Convert to RGB and normalize
        let rgb = resized.to_rgb8();
        let (width, height) = rgb.dimensions();

        // Create tensor [3, H, W]
        let mut data = vec![0.0f32; (3 * width * height) as usize];

        for y in 0..height {
            for x in 0..width {
                let pixel = rgb.get_pixel(x, y);
                let idx = (y * width + x) as usize;

                // Normalize each channel
                // Formula: (pixel / 255.0 - mean) / std
                for c in 0..3 {
                    let normalized = (pixel[c] as f32 / 255.0 - self.mean[c]) / self.std[c];
                    data[c * (width * height) as usize + idx] = normalized;
                }
            }
        }

        // Convert to tensor [3, H, W]
        let tensor = Tensor::from_slice(&data).view([3, height as i64, width as i64]);

        // Add batch dimension: [1, 3, H, W]
        let batched = tensor.unsqueeze(0);

        Ok(batched)
    }

    /// Preprocess image from file path
    pub fn preprocess_from_path(
        &self,
        image_path: &std::path::Path,
        bbox: Option<(u32, u32, u32, u32)>,
    ) -> Result<Tensor, Box<dyn std::error::Error>> {
        let image = image::open(image_path)
            .map_err(|e| format!("Failed to load image from {:?}: {}", image_path, e))?;
        self.preprocess(&image, bbox)
    }

    /// Set custom image size (default: 512)
    #[inline]
    #[must_use = "returns the modified preprocessor with new image size"]
    pub fn with_image_size(mut self, size: u32) -> Self {
        self.image_size = size;
        self
    }

    /// Set custom normalization parameters
    #[inline]
    #[must_use = "returns the modified preprocessor with new normalization"]
    pub fn with_normalization(mut self, mean: [f32; 3], std: [f32; 3]) -> Self {
        self.mean = mean;
        self.std = std;
        self
    }

    /// Preprocess multiple images into a batched tensor
    ///
    /// Processes N images and stacks them into a single batched tensor.
    /// All images are resized to the same size (image_size x image_size).
    ///
    /// # Arguments
    /// * `images` - Slice of DynamicImage references
    /// * `bboxes` - Optional slice of bboxes (must match images length if provided)
    ///
    /// # Returns
    /// * Tensor [N, 3, image_size, image_size] where N = images.len()
    ///
    /// # Example
    /// ```ignore
    /// let images = vec![img1, img2, img3];
    /// let batched = preprocessor.preprocess_batch(&images, None)?;
    /// // batched.size() == [3, 3, 512, 512]
    /// ```
    pub fn preprocess_batch(
        &self,
        images: &[DynamicImage],
        bboxes: Option<&[Option<(u32, u32, u32, u32)>]>,
    ) -> Result<Tensor, Box<dyn std::error::Error>> {
        if images.is_empty() {
            return Err("Cannot preprocess empty batch".into());
        }

        // Validate bboxes length if provided
        if let Some(bboxes) = bboxes {
            if bboxes.len() != images.len() {
                return Err(format!(
                    "bboxes length ({}) must match images length ({})",
                    bboxes.len(),
                    images.len()
                )
                .into());
            }
        }

        // Preprocess each image individually (returns [1, 3, H, W])
        let mut tensors = Vec::with_capacity(images.len());
        for (i, image) in images.iter().enumerate() {
            let bbox = bboxes.and_then(|b| b[i]);
            let tensor = self.preprocess(image, bbox)?;
            tensors.push(tensor);
        }

        // Stack tensors along batch dimension: [1, 3, H, W] x N → [N, 3, H, W]
        let batched = Tensor::cat(&tensors, 0);

        Ok(batched)
    }
}

impl Default for Idefics3Preprocessor {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb};
    use tch::Kind;

    fn create_test_image(width: u32, height: u32) -> DynamicImage {
        // Create a simple gradient image for testing
        let mut img = ImageBuffer::new(width, height);
        for y in 0..height {
            for x in 0..width {
                let r = ((x as f32 / width as f32) * 255.0) as u8;
                let g = ((y as f32 / height as f32) * 255.0) as u8;
                let b = 128u8;
                img.put_pixel(x, y, Rgb([r, g, b]));
            }
        }
        DynamicImage::ImageRgb8(img)
    }

    #[test]
    fn test_preprocess_no_crop() {
        let preprocessor = Idefics3Preprocessor::new();
        let image = create_test_image(1024, 768);

        let result = preprocessor.preprocess(&image, None);
        assert!(result.is_ok());

        let tensor = result.unwrap();
        // Expected shape: [1, 3, 512, 512]
        assert_eq!(tensor.size(), vec![1, 3, 512, 512]);

        // Check dtype
        assert_eq!(tensor.kind(), Kind::Float);
    }

    #[test]
    fn test_preprocess_with_crop() {
        let preprocessor = Idefics3Preprocessor::new();
        let image = create_test_image(1024, 768);

        // Crop to center region (100x100)
        let bbox = (462, 334, 100, 100);
        let result = preprocessor.preprocess(&image, Some(bbox));
        assert!(result.is_ok());

        let tensor = result.unwrap();
        // Expected shape: [1, 3, 512, 512] (cropped region is resized)
        assert_eq!(tensor.size(), vec![1, 3, 512, 512]);
    }

    #[test]
    fn test_custom_image_size() {
        let preprocessor = Idefics3Preprocessor::new().with_image_size(224);
        let image = create_test_image(512, 512);

        let result = preprocessor.preprocess(&image, None);
        assert!(result.is_ok());

        let tensor = result.unwrap();
        // Expected shape: [1, 3, 224, 224]
        assert_eq!(tensor.size(), vec![1, 3, 224, 224]);
    }

    #[test]
    fn test_normalization_range() {
        let preprocessor = Idefics3Preprocessor::new();
        let image = create_test_image(512, 512);

        let tensor = preprocessor.preprocess(&image, None).unwrap();

        // Check that values are normalized (should be roughly in [-3, 3] range)
        // ImageNet normalization: (pixel/255 - mean) / std
        // With mean ~0.4 and std ~0.2, normalized range is roughly [-2, 3]
        let min = tensor.min().double_value(&[]);
        let max = tensor.max().double_value(&[]);

        assert!(min >= -3.0, "Min value {} is too small", min);
        assert!(max <= 3.0, "Max value {} is too large", max);
    }

    #[test]
    fn test_batch_dimension() {
        let preprocessor = Idefics3Preprocessor::new();
        let image = create_test_image(512, 512);

        let tensor = preprocessor.preprocess(&image, None).unwrap();

        // Verify batch dimension is 1
        assert_eq!(tensor.size()[0], 1);
        assert_eq!(tensor.size()[1], 3); // RGB channels
    }

    #[test]
    fn test_custom_normalization() {
        let preprocessor =
            Idefics3Preprocessor::new().with_normalization([0.5, 0.5, 0.5], [0.5, 0.5, 0.5]);
        let image = create_test_image(256, 256);

        let tensor = preprocessor.preprocess(&image, None).unwrap();

        // With mean=0.5 and std=0.5, normalized range is [-1, 1]
        let min = tensor.min().double_value(&[]);
        let max = tensor.max().double_value(&[]);

        assert!(min >= -1.1, "Min value {} is too small", min);
        assert!(max <= 1.1, "Max value {} is too large", max);
    }

    #[test]
    fn test_preprocess_batch() {
        let preprocessor = Idefics3Preprocessor::new();

        // Create 3 test images
        let images = vec![
            create_test_image(1024, 768),
            create_test_image(512, 512),
            create_test_image(800, 600),
        ];

        let result = preprocessor.preprocess_batch(&images, None);
        assert!(result.is_ok());

        let tensor = result.unwrap();
        // Expected shape: [3, 3, 512, 512] (batch_size=3, channels=3, H=512, W=512)
        assert_eq!(tensor.size(), vec![3, 3, 512, 512]);

        // Check dtype
        assert_eq!(tensor.kind(), Kind::Float);
    }

    #[test]
    fn test_preprocess_batch_with_bboxes() {
        let preprocessor = Idefics3Preprocessor::new();

        // Create 2 test images
        let images = vec![create_test_image(1024, 768), create_test_image(512, 512)];

        // Crop first image, no crop for second
        let bboxes = vec![Some((100, 100, 200, 200)), None];

        let result = preprocessor.preprocess_batch(&images, Some(&bboxes));
        assert!(result.is_ok());

        let tensor = result.unwrap();
        // Expected shape: [2, 3, 512, 512]
        assert_eq!(tensor.size(), vec![2, 3, 512, 512]);
    }

    #[test]
    fn test_preprocess_batch_empty() {
        let preprocessor = Idefics3Preprocessor::new();
        let images: Vec<DynamicImage> = vec![];

        let result = preprocessor.preprocess_batch(&images, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty batch"));
    }

    #[test]
    fn test_preprocess_batch_length_mismatch() {
        let preprocessor = Idefics3Preprocessor::new();
        let images = vec![create_test_image(512, 512), create_test_image(512, 512)];
        let bboxes = vec![Some((0, 0, 100, 100))]; // Only 1 bbox for 2 images

        let result = preprocessor.preprocess_batch(&images, Some(&bboxes));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must match"));
    }

    #[test]
    fn test_preprocess_batch_matches_individual() {
        let preprocessor = Idefics3Preprocessor::new();

        // Create test images
        let images = vec![create_test_image(512, 512), create_test_image(1024, 768)];

        // Process as batch
        let batched = preprocessor.preprocess_batch(&images, None).unwrap();

        // Process individually
        let individual1 = preprocessor.preprocess(&images[0], None).unwrap();
        let individual2 = preprocessor.preprocess(&images[1], None).unwrap();

        // Compare: batched[0] should match individual1
        let batch_0 = batched.select(0, 0);
        let diff1 = (&batch_0 - &individual1.squeeze_dim(0))
            .abs()
            .max()
            .double_value(&[]);
        assert!(
            diff1 < 1e-5,
            "Batch element 0 differs from individual: {}",
            diff1
        );

        // Compare: batched[1] should match individual2
        let batch_1 = batched.select(0, 1);
        let diff2 = (&batch_1 - &individual2.squeeze_dim(0))
            .abs()
            .max()
            .double_value(&[]);
        assert!(
            diff2 < 1e-5,
            "Batch element 1 differs from individual: {}",
            diff2
        );
    }
}
