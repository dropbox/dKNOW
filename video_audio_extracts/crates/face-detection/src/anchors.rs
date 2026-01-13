// UltraFace RFB-320 Prior/Anchor Box Generation
//
// This module generates the 4420 prior boxes used by the UltraFace model
// for decoding regression outputs into bounding boxes.
//
// Reference: https://github.com/Linzaer/Ultra-Light-Fast-Generic-Face-Detector-1MB
//            vision/utils/box_utils.py::generate_priors()

/// Prior box in center form [center_x, center_y, width, height]
/// All values are normalized to [0, 1] range
#[derive(Debug, Clone, Copy)]
pub struct PriorBox {
    pub center_x: f32,
    pub center_y: f32,
    pub width: f32,
    pub height: f32,
}

impl PriorBox {
    pub fn new(center_x: f32, center_y: f32, width: f32, height: f32) -> Self {
        Self {
            center_x,
            center_y,
            width,
            height,
        }
    }
}

/// Generate prior boxes for UltraFace RFB-320 model
///
/// Configuration for 320x240 input:
/// - Feature map sizes: [[40, 20, 10, 5], [30, 15, 8, 4]]
/// - Min box sizes: [[10, 16, 24], [32, 48], [64, 96], [128, 192, 256]]
/// - Shrinkage (strides): [[8, 16, 32, 64], [8, 16, 30, 60]]
/// - Total priors: 4420
///
/// Layout:
/// - Feature map 0 (40x30): 3 boxes/cell = 3600 priors
/// - Feature map 1 (20x15): 2 boxes/cell = 600 priors
/// - Feature map 2 (10x8):  2 boxes/cell = 160 priors
/// - Feature map 3 (5x4):   3 boxes/cell = 60 priors
pub fn generate_ultraface_320_priors() -> Vec<PriorBox> {
    // Configuration for 320x240 input
    const IMAGE_WIDTH: f32 = 320.0;
    const IMAGE_HEIGHT: f32 = 240.0;

    // Feature map dimensions [width, height]
    const FEATURE_MAPS: [(usize, usize); 4] = [
        (40, 30), // Feature map 0
        (20, 15), // Feature map 1
        (10, 8),  // Feature map 2
        (5, 4),   // Feature map 3
    ];

    // Shrinkage factors (strides) [width_stride, height_stride]
    const SHRINKAGE: [(f32, f32); 4] = [
        (8.0, 8.0),   // 320/40=8, 240/30=8
        (16.0, 16.0), // 320/20=16, 240/15=16
        (32.0, 30.0), // 320/10=32, 240/8=30
        (64.0, 60.0), // 320/5=64, 240/4=60
    ];

    // Min box sizes for each feature map level
    const MIN_BOXES: [&[f32]; 4] = [
        &[10.0, 16.0, 24.0],     // Feature map 0: small faces
        &[32.0, 48.0],           // Feature map 1: medium faces
        &[64.0, 96.0],           // Feature map 2: large faces
        &[128.0, 192.0, 256.0],  // Feature map 3: very large faces
    ];

    // Pre-allocate vector for all 4420 priors
    let mut priors = Vec::with_capacity(4420);

    // Generate priors for each feature map level
    for (level, &(feature_w, feature_h)) in FEATURE_MAPS.iter().enumerate() {
        let (stride_w, stride_h) = SHRINKAGE[level];
        let min_sizes = MIN_BOXES[level];

        // Iterate through feature map grid
        for j in 0..feature_h {
            for i in 0..feature_w {
                // Calculate center coordinates (normalized to [0, 1])
                // Center of cell = (i + 0.5) * stride / image_size
                let center_x = (i as f32 + 0.5) * stride_w / IMAGE_WIDTH;
                let center_y = (j as f32 + 0.5) * stride_h / IMAGE_HEIGHT;

                // Generate priors for each box size at this location
                for &min_size in min_sizes {
                    let width = min_size / IMAGE_WIDTH;
                    let height = min_size / IMAGE_HEIGHT;

                    // Clamp to [0, 1] range as per UltraFace implementation
                    let center_x = center_x.clamp(0.0, 1.0);
                    let center_y = center_y.clamp(0.0, 1.0);
                    let width = width.clamp(0.0, 1.0);
                    let height = height.clamp(0.0, 1.0);

                    priors.push(PriorBox::new(center_x, center_y, width, height));
                }
            }
        }
    }

    assert_eq!(
        priors.len(),
        4420,
        "Expected 4420 priors, generated {}",
        priors.len()
    );

    priors
}

/// Decode location regression outputs to bounding boxes
///
/// UltraFace outputs regression offsets (not absolute coordinates):
/// - locations[0:2]: center offsets [dx, dy]
/// - locations[2:4]: size offsets [dw, dh]
///
/// Decoding formulas (from box_utils.py::convert_locations_to_boxes):
/// ```
/// center_x = prior_center_x + dx * center_variance * prior_width
/// center_y = prior_center_y + dy * center_variance * prior_height
/// width = prior_width * exp(dw * size_variance)
/// height = prior_height * exp(dh * size_variance)
/// ```
///
/// Then convert from center form [cx, cy, w, h] to corner form [x1, y1, x2, y2]
pub fn decode_boxes(
    locations: &[f32],   // Model output: [N, 4] flattened
    priors: &[PriorBox], // Prior boxes: [N]
    center_variance: f32,
    size_variance: f32,
) -> Vec<[f32; 4]> {
    assert_eq!(
        locations.len(),
        priors.len() * 4,
        "Location data size mismatch"
    );

    let num_boxes = priors.len();
    let mut decoded_boxes = Vec::with_capacity(num_boxes);

    for i in 0..num_boxes {
        let prior = &priors[i];
        let loc_offset = i * 4;

        // Extract regression offsets from model output
        let dx = locations[loc_offset];
        let dy = locations[loc_offset + 1];
        let dw = locations[loc_offset + 2];
        let dh = locations[loc_offset + 3];

        // Decode center coordinates
        let center_x = prior.center_x + dx * center_variance * prior.width;
        let center_y = prior.center_y + dy * center_variance * prior.height;

        // Decode box size
        let width = prior.width * (dw * size_variance).exp();
        let height = prior.height * (dh * size_variance).exp();

        // Convert from center form to corner form [x1, y1, x2, y2]
        let x1 = center_x - width / 2.0;
        let y1 = center_y - height / 2.0;
        let x2 = center_x + width / 2.0;
        let y2 = center_y + height / 2.0;

        decoded_boxes.push([x1, y1, x2, y2]);
    }

    decoded_boxes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prior_generation_count() {
        let priors = generate_ultraface_320_priors();
        assert_eq!(priors.len(), 4420, "Should generate exactly 4420 priors");
    }

    #[test]
    fn test_prior_bounds() {
        let priors = generate_ultraface_320_priors();

        // All priors should be in valid [0, 1] range
        for (i, prior) in priors.iter().enumerate() {
            assert!(
                prior.center_x >= 0.0 && prior.center_x <= 1.0,
                "Prior {} center_x out of range: {}",
                i,
                prior.center_x
            );
            assert!(
                prior.center_y >= 0.0 && prior.center_y <= 1.0,
                "Prior {} center_y out of range: {}",
                i,
                prior.center_y
            );
            assert!(
                prior.width >= 0.0 && prior.width <= 1.0,
                "Prior {} width out of range: {}",
                i,
                prior.width
            );
            assert!(
                prior.height >= 0.0 && prior.height <= 1.0,
                "Prior {} height out of range: {}",
                i,
                prior.height
            );
        }
    }

    #[test]
    fn test_box_decoding() {
        // Create simple test case with identity transform
        let priors = vec![PriorBox::new(0.5, 0.5, 0.2, 0.2)];

        // Zero offsets should return box at prior location
        let locations = vec![0.0, 0.0, 0.0, 0.0];
        let decoded = decode_boxes(&locations, &priors, 0.1, 0.2);

        assert_eq!(decoded.len(), 1);
        let bbox = decoded[0];

        // Expected: center (0.5, 0.5), size (0.2, 0.2)
        // Corner form: x1 = 0.5 - 0.1 = 0.4, y1 = 0.4, x2 = 0.6, y2 = 0.6
        assert!((bbox[0] - 0.4).abs() < 0.001, "x1 should be ~0.4");
        assert!((bbox[1] - 0.4).abs() < 0.001, "y1 should be ~0.4");
        assert!((bbox[2] - 0.6).abs() < 0.001, "x2 should be ~0.6");
        assert!((bbox[3] - 0.6).abs() < 0.001, "y2 should be ~0.6");
    }
}
