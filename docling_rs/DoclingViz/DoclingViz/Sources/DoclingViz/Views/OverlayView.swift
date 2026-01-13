// OverlayView - Canvas overlay for bounding boxes and interactions
// Contains the element overlay rendering and gesture handling

import SwiftUI
import DoclingBridge

// MARK: - Grid Overlay Canvas

/// Renders a visual grid when snap-to-grid is enabled
struct GridOverlayCanvas: View {
    @ObservedObject var viewModel: DocumentViewModel

    var body: some View {
        GeometryReader { geometry in
            Canvas { context, size in
                let pageSize = viewModel.pageSize
                let scale = min(
                    size.width / CGFloat(pageSize.width),
                    size.height / CGFloat(pageSize.height)
                )

                let gridSize = CGFloat(viewModel.gridSize) * scale
                let gridColor = Color.gray.opacity(0.15)

                // Draw vertical lines
                var x: CGFloat = 0
                while x < size.width {
                    let path = Path { p in
                        p.move(to: CGPoint(x: x, y: 0))
                        p.addLine(to: CGPoint(x: x, y: size.height))
                    }
                    context.stroke(path, with: .color(gridColor), lineWidth: 0.5)
                    x += gridSize
                }

                // Draw horizontal lines
                var y: CGFloat = 0
                while y < size.height {
                    let path = Path { p in
                        p.move(to: CGPoint(x: 0, y: y))
                        p.addLine(to: CGPoint(x: size.width, y: y))
                    }
                    context.stroke(path, with: .color(gridColor), lineWidth: 0.5)
                    y += gridSize
                }
            }
        }
        .allowsHitTesting(false)  // Pass through all clicks
    }
}

// MARK: - Overlay View

struct OverlayView: View {
    @ObservedObject var viewModel: DocumentViewModel
    @State private var contextMenuPosition: CGPoint = .zero

    var body: some View {
        GeometryReader { geometry in
            ZStack {
                // Grid overlay (when snap-to-grid and show grid are enabled)
                if viewModel.snapToGrid && viewModel.showGridOverlay {
                    GridOverlayCanvas(viewModel: viewModel)
                }

                // Canvas for drawing bounding boxes
                Canvas { context, size in
                    guard viewModel.showBoundingBoxes,
                          viewModel.currentStageSnapshot != nil else { return }

                    let pageSize = viewModel.pageSize
                    let scale = min(
                        size.width / CGFloat(pageSize.width),
                        size.height / CGFloat(pageSize.height)
                    )

                    // Draw filtered elements (respects confidence threshold)
                    for element in viewModel.filteredElements {
                        let rect = scaledRect(for: element.bbox, scale: scale, pageHeight: pageSize.height)
                        let color = elementColor(for: element, colorByConfidence: viewModel.colorByConfidence)
                        let isSelected = viewModel.isElementSelected(element.id)
                        let isPrimary = element.id == viewModel.selectedElementId

                        // Fill
                        context.fill(
                            Path(rect),
                            with: .color(color.opacity(isSelected ? 0.25 : 0.15))
                        )

                        // Border - thicker for selected, slightly dashed for secondary selection
                        if isSelected && !isPrimary {
                            // Secondary selection: dashed border
                            context.stroke(
                                Path(rect),
                                with: .color(color.opacity(0.8)),
                                style: StrokeStyle(lineWidth: 2, dash: [4, 2])
                            )
                        } else {
                            context.stroke(
                                Path(rect),
                                with: .color(color.opacity(0.8)),
                                lineWidth: isSelected ? 3 : 1
                            )
                        }

                        // Draw lock indicator for locked elements
                        if viewModel.isElementLocked(element.id) {
                            let lockSize: CGFloat = 12
                            let lockX = rect.maxX - lockSize - 2
                            let lockY = rect.minY + 2

                            // Lock body (rounded rectangle)
                            let bodyRect = CGRect(x: lockX, y: lockY + 4, width: lockSize, height: lockSize - 4)
                            context.fill(
                                Path(roundedRect: bodyRect, cornerRadius: 2),
                                with: .color(.orange)
                            )

                            // Lock shackle (arc)
                            let shacklePath = Path { p in
                                let centerX = lockX + lockSize / 2
                                p.move(to: CGPoint(x: centerX - 3, y: lockY + 5))
                                p.addLine(to: CGPoint(x: centerX - 3, y: lockY + 2))
                                p.addArc(center: CGPoint(x: centerX, y: lockY + 2),
                                        radius: 3,
                                        startAngle: .degrees(180),
                                        endAngle: .degrees(0),
                                        clockwise: false)
                                p.addLine(to: CGPoint(x: centerX + 3, y: lockY + 5))
                            }
                            context.stroke(shacklePath, with: .color(.orange), lineWidth: 2)
                        }

                        // Draw resize handles for primary selected element only
                        if isPrimary {
                            drawResizeHandles(context: &context, rect: rect)
                        }
                    }

                    // Draw text cells if enabled
                    if viewModel.showTextCells, let snapshot = viewModel.currentStageSnapshot {
                        for cell in snapshot.textCells {
                            let rect = scaledRect(for: cell.bbox, scale: scale, pageHeight: pageSize.height)
                            context.stroke(
                                Path(rect),
                                with: .color(.blue.opacity(0.3)),
                                lineWidth: 0.5
                            )
                        }
                    }

                    // Draw the current drawing rectangle (when in draw mode)
                    if let drawingRect = viewModel.drawingRect(in: size) {
                        // Fill
                        context.fill(
                            Path(drawingRect),
                            with: .color(Color.green.opacity(0.2))
                        )
                        // Border
                        context.stroke(
                            Path(drawingRect),
                            with: .color(Color.green),
                            style: StrokeStyle(lineWidth: 2, dash: [6, 3])
                        )
                    }

                    // Draw alignment guides (during drag)
                    if viewModel.isDragging && viewModel.showAlignmentGuides {
                        let guides = viewModel.currentAlignmentGuides

                        // Draw vertical guides (blue lines)
                        for x in guides.verticalGuides {
                            let scaledX = CGFloat(x) * scale
                            let path = Path { p in
                                p.move(to: CGPoint(x: scaledX, y: 0))
                                p.addLine(to: CGPoint(x: scaledX, y: size.height))
                            }
                            context.stroke(
                                path,
                                with: .color(Color.cyan),
                                style: StrokeStyle(lineWidth: 1, dash: [4, 4])
                            )
                        }

                        // Draw horizontal guides (blue lines)
                        for y in guides.horizontalGuides {
                            // Convert from PDF coordinates to screen coordinates
                            let scaledY = (CGFloat(pageSize.height) - CGFloat(y)) * scale
                            let path = Path { p in
                                p.move(to: CGPoint(x: 0, y: scaledY))
                                p.addLine(to: CGPoint(x: size.width, y: scaledY))
                            }
                            context.stroke(
                                path,
                                with: .color(Color.cyan),
                                style: StrokeStyle(lineWidth: 1, dash: [4, 4])
                            )
                        }
                    }

                    // Draw snap indicators (during drag with snap-to-grid)
                    if viewModel.isDragging && viewModel.snapToGrid && !viewModel.currentSnapIndicators.isEmpty {
                        let indicators = viewModel.currentSnapIndicators

                        // Draw vertical snap lines (magenta for snapped edges)
                        for x in indicators.verticalLines {
                            let scaledX = CGFloat(x) * scale
                            let path = Path { p in
                                p.move(to: CGPoint(x: scaledX, y: 0))
                                p.addLine(to: CGPoint(x: scaledX, y: size.height))
                            }
                            context.stroke(
                                path,
                                with: .color(Color.pink.opacity(0.8)),
                                lineWidth: 2
                            )

                            // Draw snap markers at the line
                            let markerSize: CGFloat = 6
                            let topMarker = CGRect(
                                x: scaledX - markerSize/2,
                                y: 4,
                                width: markerSize,
                                height: markerSize
                            )
                            let bottomMarker = CGRect(
                                x: scaledX - markerSize/2,
                                y: size.height - markerSize - 4,
                                width: markerSize,
                                height: markerSize
                            )
                            context.fill(Path(ellipseIn: topMarker), with: .color(.pink))
                            context.fill(Path(ellipseIn: bottomMarker), with: .color(.pink))
                        }

                        // Draw horizontal snap lines (magenta for snapped edges)
                        for y in indicators.horizontalLines {
                            // Convert from PDF coordinates to screen coordinates
                            let scaledY = (CGFloat(pageSize.height) - CGFloat(y)) * scale
                            let path = Path { p in
                                p.move(to: CGPoint(x: 0, y: scaledY))
                                p.addLine(to: CGPoint(x: size.width, y: scaledY))
                            }
                            context.stroke(
                                path,
                                with: .color(Color.pink.opacity(0.8)),
                                lineWidth: 2
                            )

                            // Draw snap markers at the line
                            let markerSize: CGFloat = 6
                            let leftMarker = CGRect(
                                x: 4,
                                y: scaledY - markerSize/2,
                                width: markerSize,
                                height: markerSize
                            )
                            let rightMarker = CGRect(
                                x: size.width - markerSize - 4,
                                y: scaledY - markerSize/2,
                                width: markerSize,
                                height: markerSize
                            )
                            context.fill(Path(ellipseIn: leftMarker), with: .color(.pink))
                            context.fill(Path(ellipseIn: rightMarker), with: .color(.pink))
                        }
                    }

                    // Draw split preview line when in split mode
                    if viewModel.isSplitMode, let element = viewModel.selectedElement {
                        let rect = scaledRect(for: element.bbox, scale: scale, pageHeight: pageSize.height)

                        // Draw the split line
                        let splitPath: Path
                        switch viewModel.splitDirection {
                        case .horizontal:
                            // Horizontal split: draw horizontal line across element
                            let splitY = rect.minY + rect.height * CGFloat(viewModel.splitPosition)
                            splitPath = Path { p in
                                p.move(to: CGPoint(x: rect.minX - 10, y: splitY))
                                p.addLine(to: CGPoint(x: rect.maxX + 10, y: splitY))
                            }
                        case .vertical:
                            // Vertical split: draw vertical line across element
                            let splitX = rect.minX + rect.width * CGFloat(viewModel.splitPosition)
                            splitPath = Path { p in
                                p.move(to: CGPoint(x: splitX, y: rect.minY - 10))
                                p.addLine(to: CGPoint(x: splitX, y: rect.maxY + 10))
                            }
                        }

                        // Draw split line with prominent style
                        context.stroke(
                            splitPath,
                            with: .color(Color.red),
                            style: StrokeStyle(lineWidth: 2, dash: [6, 3])
                        )

                        // Draw endpoint handles
                        let handleSize: CGFloat = 8
                        switch viewModel.splitDirection {
                        case .horizontal:
                            let splitY = rect.minY + rect.height * CGFloat(viewModel.splitPosition)
                            let leftHandle = CGRect(x: rect.minX - 10 - handleSize/2, y: splitY - handleSize/2, width: handleSize, height: handleSize)
                            let rightHandle = CGRect(x: rect.maxX + 10 - handleSize/2, y: splitY - handleSize/2, width: handleSize, height: handleSize)
                            context.fill(Path(ellipseIn: leftHandle), with: .color(.red))
                            context.fill(Path(ellipseIn: rightHandle), with: .color(.red))
                        case .vertical:
                            let splitX = rect.minX + rect.width * CGFloat(viewModel.splitPosition)
                            let topHandle = CGRect(x: splitX - handleSize/2, y: rect.minY - 10 - handleSize/2, width: handleSize, height: handleSize)
                            let bottomHandle = CGRect(x: splitX - handleSize/2, y: rect.maxY + 10 - handleSize/2, width: handleSize, height: handleSize)
                            context.fill(Path(ellipseIn: topHandle), with: .color(.red))
                            context.fill(Path(ellipseIn: bottomHandle), with: .color(.red))
                        }
                    }

                    // Draw lasso selection path
                    if viewModel.isLassoDrawing, let lassoPath = viewModel.lassoSwiftUIPath() {
                        // Fill with semi-transparent blue
                        context.fill(
                            lassoPath,
                            with: .color(Color.blue.opacity(0.15))
                        )
                        // Stroke with dashed blue line
                        context.stroke(
                            lassoPath,
                            with: .color(Color.blue.opacity(0.8)),
                            style: StrokeStyle(lineWidth: 2, dash: [6, 3])
                        )
                    }

                    // Draw marquee selection rectangle
                    if viewModel.isMarqueeDrawing, let marqueeRect = viewModel.marqueeRect() {
                        // Fill with semi-transparent purple
                        context.fill(
                            Path(marqueeRect),
                            with: .color(Color.purple.opacity(0.15))
                        )
                        // Stroke with dashed purple line
                        context.stroke(
                            Path(marqueeRect),
                            with: .color(Color.purple.opacity(0.8)),
                            style: StrokeStyle(lineWidth: 2, dash: [6, 3])
                        )
                    }
                }
                .gesture(
                    DragGesture(minimumDistance: 0)
                        .onChanged { value in
                            if viewModel.editTool == .draw {
                                // Drawing mode
                                if viewModel.isDrawing {
                                    viewModel.updateDrawing(at: value.location, in: geometry.size)
                                } else {
                                    viewModel.startDrawing(at: value.location, in: geometry.size)
                                }
                            } else if viewModel.editTool == .lasso {
                                // Lasso selection mode
                                if viewModel.isLassoDrawing {
                                    viewModel.updateLasso(at: value.location)
                                } else {
                                    viewModel.startLasso(at: value.location, in: geometry.size)
                                }
                            } else if viewModel.editTool == .marquee {
                                // Marquee (box) selection mode
                                if viewModel.isMarqueeDrawing {
                                    viewModel.updateMarquee(at: value.location)
                                } else {
                                    viewModel.startMarquee(at: value.location, in: geometry.size)
                                }
                            } else if viewModel.isDragging {
                                // Continue dragging existing element
                                viewModel.updateDrag(at: value.location, in: geometry.size)
                            } else if viewModel.selectedElementId != nil {
                                // Start drag on selected element
                                viewModel.startDrag(at: value.location, in: geometry.size)
                            }
                        }
                        .onEnded { value in
                            if viewModel.editTool == .draw && viewModel.isDrawing {
                                // End drawing
                                viewModel.endDrawing(in: geometry.size)
                            } else if viewModel.editTool == .lasso && viewModel.isLassoDrawing {
                                // End lasso selection
                                let modifierFlags = NSEvent.modifierFlags
                                let addToSelection = modifierFlags.contains(.shift)
                                viewModel.endLasso(addToSelection: addToSelection)
                            } else if viewModel.editTool == .marquee && viewModel.isMarqueeDrawing {
                                // End marquee selection
                                let modifierFlags = NSEvent.modifierFlags
                                let addToSelection = modifierFlags.contains(.shift)
                                viewModel.endMarquee(addToSelection: addToSelection)
                            } else if viewModel.isDragging {
                                viewModel.endDrag()
                            } else {
                                // Single click - select element with modifier support
                                let modifierFlags = NSEvent.modifierFlags
                                var modifiers: EventModifiers = []
                                if modifierFlags.contains(.shift) {
                                    modifiers.insert(.shift)
                                }
                                if modifierFlags.contains(.command) {
                                    modifiers.insert(.command)
                                }
                                viewModel.handleTap(at: value.location, in: geometry.size, modifiers: modifiers)
                            }
                        }
                )
                .simultaneousGesture(
                    TapGesture()
                        .onEnded { _ in
                            // Handle tap for selection (backup in case drag gesture doesn't trigger)
                        }
                )
                .contextMenu {
                    if let element = viewModel.selectedElement {
                        // Clipboard operations
                        Button {
                            viewModel.copySelectedElements()
                        } label: {
                            Label("Copy", systemImage: "doc.on.doc")
                        }
                        .keyboardShortcut("c", modifiers: .command)

                        Button {
                            viewModel.cutSelectedElements()
                        } label: {
                            Label("Cut", systemImage: "scissors")
                        }
                        .keyboardShortcut("x", modifiers: .command)

                        if viewModel.canPaste {
                            Button {
                                viewModel.pasteElements()
                            } label: {
                                Label("Paste (\(viewModel.clipboard.count))", systemImage: "doc.on.clipboard")
                            }
                            .keyboardShortcut("v", modifiers: .command)
                        }

                        Divider()

                        Button {
                            viewModel.duplicateSelectedElement()
                        } label: {
                            Label("Duplicate", systemImage: "plus.square.on.square")
                        }
                        .keyboardShortcut("d", modifiers: .command)

                        if viewModel.canMergeSelection {
                            Button {
                                viewModel.mergeSelectedElements()
                            } label: {
                                Label("Merge (\(viewModel.selectedElementCount))", systemImage: "rectangle.arrowtriangle.2.inward")
                            }
                            .keyboardShortcut("m", modifiers: .command)
                        }

                        Divider()

                        // Label change menu (supports batch change for multi-selection)
                        Menu(viewModel.selectedElementCount > 1 ? "Change Label (\(viewModel.selectedElementCount))" : "Change Label") {
                            ForEach(DocItemLabel.allCases) { label in
                                Button(action: {
                                    if viewModel.selectedElementCount > 1 {
                                        viewModel.changeSelectedLabels(to: label)
                                    } else {
                                        viewModel.changeLabel(elementId: element.id, to: label)
                                    }
                                }) {
                                    HStack {
                                        Circle()
                                            .fill(label.swiftUIColor)
                                            .frame(width: 8, height: 8)
                                        Text(label.description)
                                        if viewModel.selectedElementCount == 1 && element.label == label {
                                            Spacer()
                                            Image(systemName: "checkmark")
                                        }
                                    }
                                }
                            }
                        }

                        Divider()

                        // Element info
                        Text("ID: \(element.id)")
                            .foregroundColor(.secondary)
                        Text("Confidence: \(Int(element.confidence * 100))%")
                            .foregroundColor(.secondary)
                        if element.hasReadingOrder {
                            Text("Reading Order: \(element.readingOrder)")
                                .foregroundColor(.secondary)
                        }
                        if viewModel.selectedElementCount > 1 {
                            Text("Selected: \(viewModel.selectedElementCount) elements")
                                .foregroundColor(.secondary)
                        }

                        Divider()

                        Button(role: .destructive) {
                            viewModel.deleteSelectedElement()
                        } label: {
                            if viewModel.selectedElementCount > 1 {
                                Label("Delete \(viewModel.selectedElementCount) Elements", systemImage: "trash")
                            } else {
                                Label("Delete Element", systemImage: "trash")
                            }
                        }
                    } else {
                        // No selection - show paste option if available
                        if viewModel.canPaste {
                            Button {
                                viewModel.pasteElements()
                            } label: {
                                Label("Paste (\(viewModel.clipboard.count))", systemImage: "doc.on.clipboard")
                            }
                            .keyboardShortcut("v", modifiers: .command)

                            Divider()
                        }

                        Text("Right-click on an element to edit")
                            .foregroundColor(.secondary)
                    }
                }
            }
        }
    }

    /// Draw resize handles for the selected element
    func drawResizeHandles(context: inout GraphicsContext, rect: CGRect) {
        let handleSize: CGFloat = 8.0
        let halfHandle = handleSize / 2

        // Handle positions
        let handles: [CGPoint] = [
            CGPoint(x: rect.minX, y: rect.minY),           // topLeft
            CGPoint(x: rect.midX, y: rect.minY),           // topCenter
            CGPoint(x: rect.maxX, y: rect.minY),           // topRight
            CGPoint(x: rect.minX, y: rect.midY),           // middleLeft
            CGPoint(x: rect.maxX, y: rect.midY),           // middleRight
            CGPoint(x: rect.minX, y: rect.maxY),           // bottomLeft
            CGPoint(x: rect.midX, y: rect.maxY),           // bottomCenter
            CGPoint(x: rect.maxX, y: rect.maxY),           // bottomRight
        ]

        for center in handles {
            let handleRect = CGRect(
                x: center.x - halfHandle,
                y: center.y - halfHandle,
                width: handleSize,
                height: handleSize
            )

            // White fill
            context.fill(Path(handleRect), with: .color(.white))

            // Blue border
            context.stroke(
                Path(handleRect),
                with: .color(.accentColor),
                lineWidth: 2
            )
        }
    }

    func scaledRect(for bbox: BoundingBox, scale: CGFloat, pageHeight: Float) -> CGRect {
        // Convert from PDF coordinates (bottom-left origin) to screen coordinates (top-left origin)
        let x = CGFloat(bbox.x) * scale
        let y = (CGFloat(pageHeight) - CGFloat(bbox.y) - CGFloat(bbox.height)) * scale
        let width = CGFloat(bbox.width) * scale
        let height = CGFloat(bbox.height) * scale
        return CGRect(x: x, y: y, width: width, height: height)
    }
}
