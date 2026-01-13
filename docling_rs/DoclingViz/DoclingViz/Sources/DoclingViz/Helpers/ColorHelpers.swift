// ColorHelpers - Shared color utilities for DoclingViz
// Consolidates duplicate labelColor functions

import SwiftUI
import DoclingBridge

// MARK: - DocItemLabel Color Extension

extension DocItemLabel {
    /// Get SwiftUI Color for this label
    var swiftUIColor: Color {
        let rgb = self.color
        return Color(
            red: Double(rgb.r) / 255.0,
            green: Double(rgb.g) / 255.0,
            blue: Double(rgb.b) / 255.0
        )
    }
}

// MARK: - Confidence Color Helpers

/// Color based on confidence level
/// - Parameter confidence: 0.0 to 1.0 confidence value
/// - Returns: Green for high (≥80%), orange for medium (50-80%), red for low (<50%)
func confidenceColor(for confidence: Float) -> Color {
    if confidence >= 0.8 {
        return .green
    } else if confidence >= 0.5 {
        return .orange
    } else {
        return .red
    }
}

// MARK: - Element Color Helpers

/// Get display color for an element
/// - Parameters:
///   - element: The element to get color for
///   - colorByConfidence: If true, color by confidence level; otherwise by label
/// - Returns: SwiftUI Color for the element
func elementColor(for element: Element, colorByConfidence: Bool) -> Color {
    if colorByConfidence {
        return confidenceColor(for: element.confidence)
    }
    return element.label.swiftUIColor
}
