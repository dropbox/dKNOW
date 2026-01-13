// PDFCanvasView - PDF rendering and zoom controls
// Contains PDFKit integration and zoom gestures

import SwiftUI
import PDFKit
import AppKit
import DoclingBridge

// MARK: - PDF Canvas View

struct PDFCanvasView: NSViewRepresentable {
    @ObservedObject var viewModel: DocumentViewModel

    func makeCoordinator() -> Coordinator {
        Coordinator(viewModel: viewModel)
    }

    func makeNSView(context: Context) -> ZoomablePDFView {
        let pdfView = ZoomablePDFView()
        pdfView.autoScales = false
        pdfView.displayMode = .singlePage
        pdfView.displayDirection = .vertical
        pdfView.backgroundColor = .windowBackgroundColor
        pdfView.scaleFactor = 1.0

        // Set coordinator for zoom handling
        pdfView.coordinator = context.coordinator

        // Add magnification gesture for pinch-to-zoom
        let magnificationGesture = NSMagnificationGestureRecognizer(
            target: context.coordinator,
            action: #selector(Coordinator.handleMagnification(_:))
        )
        pdfView.addGestureRecognizer(magnificationGesture)

        // Store reference to coordinator for scroll wheel handling
        context.coordinator.pdfView = pdfView

        return pdfView
    }

    func updateNSView(_ pdfView: ZoomablePDFView, context: Context) {
        if pdfView.document != viewModel.pdfDocument {
            pdfView.document = viewModel.pdfDocument
        }
        if let doc = viewModel.pdfDocument,
           viewModel.currentPage < doc.pageCount,
           let page = doc.page(at: viewModel.currentPage) {
            pdfView.go(to: page)
        }
        // Apply zoom level
        pdfView.scaleFactor = viewModel.zoomLevel

        // Handle zoom to selection
        if let targetRect = viewModel.zoomTargetRect,
           let doc = viewModel.pdfDocument,
           viewModel.currentPage < doc.pageCount,
           let page = doc.page(at: viewModel.currentPage) {
            // Schedule scroll after current update
            DispatchQueue.main.async {
                pdfView.go(to: targetRect, on: page)
                // Clear the target after navigating
                Task { @MainActor in
                    self.viewModel.clearZoomTarget()
                }
            }
        }
    }

    // MARK: - Coordinator for Gesture Handling

    class Coordinator: NSObject {
        let viewModel: DocumentViewModel
        weak var pdfView: PDFView?

        /// Starting zoom level when pinch begins
        private var startingZoom: Double = 1.0

        init(viewModel: DocumentViewModel) {
            self.viewModel = viewModel
        }

        @MainActor @objc func handleMagnification(_ gesture: NSMagnificationGestureRecognizer) {
            switch gesture.state {
            case .began:
                // Store the starting zoom level
                startingZoom = viewModel.zoomLevel

            case .changed:
                // Magnification is the change from 1.0 (no change)
                // gesture.magnification: -1.0 to +inf, where 0.0 = no change
                // Convert to scale factor: 1.0 + magnification
                let scaleFactor = 1.0 + gesture.magnification
                let newZoom = startingZoom * scaleFactor
                viewModel.setZoom(newZoom)

            case .ended, .cancelled:
                // Optionally snap to nearest step
                break

            default:
                break
            }
        }

        /// Handle scroll wheel zoom with Option key
        @MainActor func handleScrollWheelZoom(deltaY: CGFloat) {
            // Scroll up = zoom in, scroll down = zoom out
            // Scale factor: each unit of deltaY changes zoom by 5%
            let zoomDelta = Double(deltaY) * 0.05
            let newZoom = viewModel.zoomLevel * (1.0 + zoomDelta)
            viewModel.setZoom(newZoom)
        }
    }
}

// MARK: - ZoomablePDFView

/// Custom PDFView subclass that supports Option+scroll wheel zoom
class ZoomablePDFView: PDFView {
    weak var coordinator: PDFCanvasView.Coordinator?

    override func scrollWheel(with event: NSEvent) {
        // Check if Option key is held
        if event.modifierFlags.contains(.option) {
            // Option + scroll = zoom
            // Use scrollingDeltaY for smooth scrolling (trackpad) or deltaY for mouse wheel
            let delta = event.hasPreciseScrollingDeltas ? event.scrollingDeltaY : event.deltaY * 10
            Task { @MainActor in
                coordinator?.handleScrollWheelZoom(deltaY: delta)
            }
        } else {
            // Normal scroll behavior
            super.scrollWheel(with: event)
        }
    }
}
