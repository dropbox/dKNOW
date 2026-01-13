//! Pure Rust implementation of rotated rectangle box points calculation
//!
//! This replaces opencv's `imgproc::box_points()` function to eliminate
//! the opencv/libclang dependency for OCR functionality.
//!
//! BUG #58 fix: Enable OCR without opencv dependency

/// A 2D point with f32 coordinates
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Point2f {
    pub x: f32,
    pub y: f32,
}

/// A rotated rectangle defined by center, size, and angle
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct RotatedRect {
    /// Center point of the rectangle
    pub center: Point2f,
    /// Size (width, height) of the rectangle
    pub size: Size2f,
    /// Rotation angle in degrees (counter-clockwise)
    pub angle: f32,
}

/// Size with width and height
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Size2f {
    pub width: f32,
    pub height: f32,
}

impl RotatedRect {
    /// Calculate the 4 corner points of this rotated rectangle
    ///
    /// This is equivalent to `OpenCV`'s `cv2.boxPoints()` / `imgproc::box_points()`
    ///
    /// Returns 4 points in order: bottom-left, top-left, top-right, bottom-right
    /// (matching `OpenCV`'s convention)
    #[inline]
    #[must_use = "returns the 4 corner points of the rotated rectangle"]
    pub fn box_points(&self) -> [Point2f; 4] {
        // Convert angle from degrees to radians
        let angle_rad = self.angle.to_radians();
        let cos_a = angle_rad.cos();
        let sin_a = angle_rad.sin();

        // Half dimensions
        let hw = self.size.width / 2.0;
        let hh = self.size.height / 2.0;

        // Calculate rotated corner offsets
        // OpenCV convention: angle rotates counter-clockwise
        // Corner offsets before rotation (relative to center):
        // bottom-left: (-hw, -hh)
        // top-left:    (-hw, +hh)
        // top-right:   (+hw, +hh)
        // bottom-right:(+hw, -hh)

        let corners = [
            (-hw, -hh), // bottom-left
            (-hw, hh),  // top-left
            (hw, hh),   // top-right
            (hw, -hh),  // bottom-right
        ];

        corners.map(|(dx, dy)| {
            // Apply rotation: x' = x*cos - y*sin, y' = x*sin + y*cos
            let rx = dx.mul_add(cos_a, -dy * sin_a);
            let ry = dx.mul_add(sin_a, dy * cos_a);

            Point2f {
                x: self.center.x + rx,
                y: self.center.y + ry,
            }
        })
    }
}

/// Get the 4 corner points of a rotated rectangle and sort them for OCR
///
/// This replaces the opencv-dependent `get_min_boxes` function.
///
/// Returns corners in specific order (clockwise from top-left) and max side length
#[inline]
#[must_use = "returns the sorted box points and max side length"]
pub fn get_min_boxes_pure(rect: &RotatedRect) -> (Vec<Point2f>, f32) {
    let max_side = rect.size.width.max(rect.size.height);

    // Get box points
    let mut box_points_unsorted: Vec<Point2f> = rect.box_points().to_vec();

    // Sort by x coordinate (Reference: OcrUtils.cpp:190, Python line 174)
    box_points_unsorted.sort_by(|a, b| a.x.total_cmp(&b.x));

    // Determine order (Reference: OcrUtils.cpp:191-210, Python lines 177-192)
    let (index1, index4) = if box_points_unsorted[1].y > box_points_unsorted[0].y {
        (0, 1)
    } else {
        (1, 0)
    };

    let (index2, index3) = if box_points_unsorted[3].y > box_points_unsorted[2].y {
        (2, 3)
    } else {
        (3, 2)
    };

    let min_box = vec![
        box_points_unsorted[index1],
        box_points_unsorted[index2],
        box_points_unsorted[index3],
        box_points_unsorted[index4],
    ];

    (min_box, max_side)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_box_points_no_rotation() {
        let rect = RotatedRect {
            center: Point2f { x: 50.0, y: 50.0 },
            size: Size2f {
                width: 20.0,
                height: 10.0,
            },
            angle: 0.0,
        };

        let points = rect.box_points();

        // No rotation: corners are at center ± half dimensions
        assert!((points[0].x - 40.0).abs() < 0.001); // bottom-left x
        assert!((points[0].y - 45.0).abs() < 0.001); // bottom-left y
        assert!((points[2].x - 60.0).abs() < 0.001); // top-right x
        assert!((points[2].y - 55.0).abs() < 0.001); // top-right y
    }

    #[test]
    fn test_box_points_90_degree_rotation() {
        let rect = RotatedRect {
            center: Point2f { x: 0.0, y: 0.0 },
            size: Size2f {
                width: 20.0,
                height: 10.0,
            },
            angle: 90.0,
        };

        let points = rect.box_points();

        // After 90° rotation, width becomes vertical, height becomes horizontal
        // Point that was at (-10, -5) goes to (5, -10)
        assert!((points[0].x - 5.0).abs() < 0.01);
        assert!((points[0].y - (-10.0)).abs() < 0.01);
    }

    #[test]
    fn test_get_min_boxes_pure() {
        let rect = RotatedRect {
            center: Point2f { x: 100.0, y: 100.0 },
            size: Size2f {
                width: 50.0,
                height: 30.0,
            },
            angle: 15.0,
        };

        let (points, max_side) = get_min_boxes_pure(&rect);

        assert_eq!(points.len(), 4);
        assert!((max_side - 50.0).abs() < 0.001);
    }
}
