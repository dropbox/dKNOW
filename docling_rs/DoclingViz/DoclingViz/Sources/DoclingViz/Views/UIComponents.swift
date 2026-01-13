// UIComponents - Reusable UI components for DoclingViz
// Contains indicators, HUDs, and small reusable views

import SwiftUI
import DoclingBridge

// MARK: - Zoom Indicator View

/// HUD that displays current zoom level
struct ZoomIndicatorView: View {
    let zoomLevel: Double

    var body: some View {
        VStack {
            Spacer()
            HStack {
                Spacer()
                HStack(spacing: 8) {
                    Image(systemName: "magnifyingglass")
                        .font(.title3)
                    Text("\(Int(zoomLevel * 100))%")
                        .font(.title2.monospacedDigit())
                        .fontWeight(.semibold)
                }
                .padding(.horizontal, 16)
                .padding(.vertical, 10)
                .background(.ultraThinMaterial)
                .cornerRadius(10)
                .shadow(radius: 4)
                Spacer()
            }
            .padding(.bottom, 40)
        }
    }
}

// MARK: - Corrections Indicator View

/// Status bar indicator showing unsaved corrections count
struct CorrectionsIndicatorView: View {
    let editCount: Int

    var body: some View {
        HStack(spacing: 4) {
            Image(systemName: "pencil.circle.fill")
                .foregroundColor(.orange)
            Text("\(editCount) unsaved correction\(editCount == 1 ? "" : "s")")
                .font(.caption)
                .foregroundColor(.orange)
            Text("(Cmd+Shift+S to save)")
                .font(.caption2)
                .foregroundColor(.secondary)
        }
        .padding(.horizontal, 8)
        .padding(.vertical, 4)
        .background(Color.orange.opacity(0.15))
        .cornerRadius(4)
    }
}

// MARK: - DocItemLabel Extension

extension DocItemLabel {
    /// Short description for compact UI
    var shortDescription: String {
        switch self {
        case .caption: return "Caption"
        case .footnote: return "Footnote"
        case .formula: return "Formula"
        case .listItem: return "List"
        case .pageFooter: return "Footer"
        case .pageHeader: return "Header"
        case .picture: return "Picture"
        case .sectionHeader: return "Section"
        case .table: return "Table"
        case .text: return "Text"
        case .title: return "Title"
        case .code: return "Code"
        case .checkboxSelected: return "Check+"
        case .checkboxUnselected: return "Check-"
        case .documentIndex: return "Index"
        case .form: return "Form"
        case .keyValueRegion: return "K/V"
        }
    }
}
