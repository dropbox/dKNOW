// NotificationHandlers - View modifiers for handling menu/keyboard commands
// Extracted from ContentView to reduce complexity

import SwiftUI
import DoclingBridge

// MARK: - Main Notification Handlers

/// View modifier that handles all notification-based commands
/// Extracted to reduce complexity for the Swift compiler
struct NotificationHandlers: ViewModifier {
    @ObservedObject var viewModel: DocumentViewModel
    let appState: AppState

    func body(content: Content) -> some View {
        content
            .onReceive(NotificationCenter.default.publisher(for: .openPDF)) { _ in
                viewModel.openPDF()
            }
            .onReceive(NotificationCenter.default.publisher(for: .openRecentPDF)) { notification in
                if let url = notification.object as? URL {
                    viewModel.loadPDF(from: url)
                }
            }
            .onReceive(NotificationCenter.default.publisher(for: .toggleBoundingBoxes)) { _ in
                viewModel.showBoundingBoxes.toggle()
            }
            .onReceive(NotificationCenter.default.publisher(for: .toggleTextCells)) { _ in
                viewModel.showTextCells.toggle()
            }
            .onReceive(NotificationCenter.default.publisher(for: .toggleSnapToGrid)) { _ in
                viewModel.snapToGrid.toggle()
            }
            .onReceive(NotificationCenter.default.publisher(for: .toggleAlignmentGuides)) { _ in
                viewModel.showAlignmentGuides.toggle()
            }
            .modifier(PlaybackNotificationHandlers(viewModel: viewModel))
            .modifier(ZoomNotificationHandlers(viewModel: viewModel))
            .modifier(ExportNotificationHandlers(viewModel: viewModel))
            .modifier(EditNotificationHandlers(viewModel: viewModel))
            .onAppear {
                viewModel.pipeline = appState.pipeline
            }
            .navigationTitle(viewModel.windowTitle)
    }
}

// MARK: - Playback Notifications

/// Playback-related notifications
struct PlaybackNotificationHandlers: ViewModifier {
    @ObservedObject var viewModel: DocumentViewModel

    func body(content: Content) -> some View {
        content
            .onReceive(NotificationCenter.default.publisher(for: .togglePlayback)) { _ in
                viewModel.togglePlayback()
            }
            .onReceive(NotificationCenter.default.publisher(for: .previousStage)) { _ in
                viewModel.previousStage()
            }
            .onReceive(NotificationCenter.default.publisher(for: .nextStage)) { _ in
                viewModel.nextStage()
            }
            .onReceive(NotificationCenter.default.publisher(for: .firstStage)) { _ in
                viewModel.currentStage = .rawPdf
            }
            .onReceive(NotificationCenter.default.publisher(for: .lastStage)) { _ in
                viewModel.currentStage = .readingOrder
            }
            .onReceive(NotificationCenter.default.publisher(for: .previousPage)) { _ in
                viewModel.previousPage()
            }
            .onReceive(NotificationCenter.default.publisher(for: .nextPage)) { _ in
                viewModel.nextPage()
            }
    }
}

// MARK: - Zoom Notifications

/// Zoom-related notifications
struct ZoomNotificationHandlers: ViewModifier {
    @ObservedObject var viewModel: DocumentViewModel

    func body(content: Content) -> some View {
        content
            .onReceive(NotificationCenter.default.publisher(for: .zoomIn)) { _ in
                viewModel.zoomIn()
            }
            .onReceive(NotificationCenter.default.publisher(for: .zoomOut)) { _ in
                viewModel.zoomOut()
            }
            .onReceive(NotificationCenter.default.publisher(for: .zoomActualSize)) { _ in
                viewModel.zoomToActualSize()
            }
            .onReceive(NotificationCenter.default.publisher(for: .zoomFitToWindow)) { _ in
                viewModel.zoomToFit()
            }
            .onReceive(NotificationCenter.default.publisher(for: .zoomToSelection)) { _ in
                viewModel.zoomToSelection()
            }
    }
}

// MARK: - Export Notifications

/// Export-related notifications
struct ExportNotificationHandlers: ViewModifier {
    @ObservedObject var viewModel: DocumentViewModel

    func body(content: Content) -> some View {
        content
            .onReceive(NotificationCenter.default.publisher(for: .exportCOCOAll)) { _ in
                viewModel.exportCOCOAllPages()
            }
            .onReceive(NotificationCenter.default.publisher(for: .exportCOCOPage)) { _ in
                viewModel.exportCOCOCurrentPage()
            }
            .onReceive(NotificationCenter.default.publisher(for: .exportCOCOWithImages)) { _ in
                viewModel.exportCOCOWithImages()
            }
            .onReceive(NotificationCenter.default.publisher(for: .exportJSON)) { _ in
                viewModel.exportJSON()
            }
            .onReceive(NotificationCenter.default.publisher(for: .exportValidationMarkdown)) { _ in
                viewModel.exportValidationReportMarkdown()
            }
            .onReceive(NotificationCenter.default.publisher(for: .exportValidationJSON)) { _ in
                viewModel.exportValidationReportJSON()
            }
            .onReceive(NotificationCenter.default.publisher(for: .exportYOLO)) { _ in
                viewModel.exportYOLO()
            }
            // Import operations
            .onReceive(NotificationCenter.default.publisher(for: .importCOCO)) { _ in
                viewModel.importCOCOAnnotations()
            }
    }
}

// MARK: - Edit Notifications

/// Edit-related notifications (undo/redo)
struct EditNotificationHandlers: ViewModifier {
    @ObservedObject var viewModel: DocumentViewModel

    func body(content: Content) -> some View {
        content
            .onReceive(NotificationCenter.default.publisher(for: .undoEdit)) { _ in
                viewModel.undo()
            }
            .onReceive(NotificationCenter.default.publisher(for: .redoEdit)) { _ in
                viewModel.redo()
            }
            .onReceive(NotificationCenter.default.publisher(for: .discardEdits)) { _ in
                viewModel.discardEdits()
            }
            .onReceive(NotificationCenter.default.publisher(for: .deleteElement)) { _ in
                viewModel.deleteSelectedElement()
            }
            .onReceive(NotificationCenter.default.publisher(for: .duplicateElement)) { _ in
                viewModel.duplicateSelectedElement()
            }
            .onReceive(NotificationCenter.default.publisher(for: .toggleDrawMode)) { _ in
                viewModel.editTool = viewModel.editTool == .draw ? .select : .draw
            }
            .onReceive(NotificationCenter.default.publisher(for: .toggleLassoMode)) { _ in
                viewModel.editTool = viewModel.editTool == .lasso ? .select : .lasso
            }
            .onReceive(NotificationCenter.default.publisher(for: .toggleMarqueeMode)) { _ in
                viewModel.editTool = viewModel.editTool == .marquee ? .select : .marquee
            }
            .onReceive(NotificationCenter.default.publisher(for: .deselect)) { _ in
                viewModel.deselect()
            }
            .onReceive(NotificationCenter.default.publisher(for: .selectAll)) { _ in
                viewModel.selectAll()
            }
            .onReceive(NotificationCenter.default.publisher(for: .selectNext)) { _ in
                viewModel.selectNextElement()
            }
            .onReceive(NotificationCenter.default.publisher(for: .selectPrevious)) { _ in
                viewModel.selectPreviousElement()
            }
            .onReceive(NotificationCenter.default.publisher(for: .saveCorrections)) { _ in
                viewModel.saveCorrections()
            }
            .onReceive(NotificationCenter.default.publisher(for: .loadCorrections)) { _ in
                viewModel.loadCorrections()
            }
            .modifier(ClipboardNotificationHandlers(viewModel: viewModel))
            .modifier(QuickLabelNotificationHandlers(viewModel: viewModel))
            .modifier(NudgeNotificationHandlers(viewModel: viewModel))
            .modifier(SplitNotificationHandlers(viewModel: viewModel))
    }
}

// MARK: - Clipboard Notifications

/// Clipboard notifications for copy/cut/paste operations
struct ClipboardNotificationHandlers: ViewModifier {
    @ObservedObject var viewModel: DocumentViewModel

    func body(content: Content) -> some View {
        content
            .onReceive(NotificationCenter.default.publisher(for: .copyElements)) { _ in
                viewModel.copySelectedElements()
            }
            .onReceive(NotificationCenter.default.publisher(for: .cutElements)) { _ in
                viewModel.cutSelectedElements()
            }
            .onReceive(NotificationCenter.default.publisher(for: .pasteElements)) { _ in
                viewModel.pasteElements()
            }
            .onReceive(NotificationCenter.default.publisher(for: .mergeElements)) { _ in
                viewModel.mergeSelectedElements()
            }
            // Lock operations
            .onReceive(NotificationCenter.default.publisher(for: .toggleLock)) { _ in
                viewModel.toggleLockSelectedElements()
            }
            .onReceive(NotificationCenter.default.publisher(for: .lockAll)) { _ in
                viewModel.lockAllElements()
            }
            .onReceive(NotificationCenter.default.publisher(for: .unlockAll)) { _ in
                viewModel.unlockAllElements()
            }
    }
}

// MARK: - Quick Label Notifications

/// Quick label notifications for rapid annotation (number key shortcuts)
struct QuickLabelNotificationHandlers: ViewModifier {
    @ObservedObject var viewModel: DocumentViewModel

    func body(content: Content) -> some View {
        content
            .onReceive(NotificationCenter.default.publisher(for: .quickLabel1)) { _ in
                viewModel.changeSelectedLabels(to: .text)
            }
            .onReceive(NotificationCenter.default.publisher(for: .quickLabel2)) { _ in
                viewModel.changeSelectedLabels(to: .title)
            }
            .onReceive(NotificationCenter.default.publisher(for: .quickLabel3)) { _ in
                viewModel.changeSelectedLabels(to: .sectionHeader)
            }
            .onReceive(NotificationCenter.default.publisher(for: .quickLabel4)) { _ in
                viewModel.changeSelectedLabels(to: .table)
            }
            .onReceive(NotificationCenter.default.publisher(for: .quickLabel5)) { _ in
                viewModel.changeSelectedLabels(to: .picture)
            }
            .onReceive(NotificationCenter.default.publisher(for: .quickLabel6)) { _ in
                viewModel.changeSelectedLabels(to: .listItem)
            }
            .onReceive(NotificationCenter.default.publisher(for: .quickLabel7)) { _ in
                viewModel.changeSelectedLabels(to: .caption)
            }
            .onReceive(NotificationCenter.default.publisher(for: .quickLabel8)) { _ in
                viewModel.changeSelectedLabels(to: .footnote)
            }
            .onReceive(NotificationCenter.default.publisher(for: .quickLabel9)) { _ in
                viewModel.changeSelectedLabels(to: .code)
            }
    }
}

// MARK: - Nudge Notifications

/// Nudge notifications for fine element positioning
struct NudgeNotificationHandlers: ViewModifier {
    @ObservedObject var viewModel: DocumentViewModel

    func body(content: Content) -> some View {
        content
            .onReceive(NotificationCenter.default.publisher(for: .nudgeUp)) { _ in
                viewModel.nudgeSelectedElement(dx: 0, dy: 1)
            }
            .onReceive(NotificationCenter.default.publisher(for: .nudgeDown)) { _ in
                viewModel.nudgeSelectedElement(dx: 0, dy: -1)
            }
            .onReceive(NotificationCenter.default.publisher(for: .nudgeLeft)) { _ in
                viewModel.nudgeSelectedElement(dx: -1, dy: 0)
            }
            .onReceive(NotificationCenter.default.publisher(for: .nudgeRight)) { _ in
                viewModel.nudgeSelectedElement(dx: 1, dy: 0)
            }
    }
}

// MARK: - Split Notifications

/// Split notifications for dividing elements
struct SplitNotificationHandlers: ViewModifier {
    @ObservedObject var viewModel: DocumentViewModel

    func body(content: Content) -> some View {
        content
            .onReceive(NotificationCenter.default.publisher(for: .splitElement)) { _ in
                viewModel.enterSplitMode()
            }
            .onReceive(NotificationCenter.default.publisher(for: .splitHorizontal)) { _ in
                viewModel.quickSplit(direction: .horizontal)
            }
            .onReceive(NotificationCenter.default.publisher(for: .splitVertical)) { _ in
                viewModel.quickSplit(direction: .vertical)
            }
            .modifier(AlignmentNotificationHandlers(viewModel: viewModel))
    }
}

// MARK: - Alignment Notifications

/// Alignment notifications for multi-element alignment, distribution, and size matching
struct AlignmentNotificationHandlers: ViewModifier {
    @ObservedObject var viewModel: DocumentViewModel

    func body(content: Content) -> some View {
        content
            .onReceive(NotificationCenter.default.publisher(for: .alignLeft)) { _ in
                viewModel.alignLeft()
            }
            .onReceive(NotificationCenter.default.publisher(for: .alignRight)) { _ in
                viewModel.alignRight()
            }
            .onReceive(NotificationCenter.default.publisher(for: .alignTop)) { _ in
                viewModel.alignTop()
            }
            .onReceive(NotificationCenter.default.publisher(for: .alignBottom)) { _ in
                viewModel.alignBottom()
            }
            .onReceive(NotificationCenter.default.publisher(for: .alignCenterH)) { _ in
                viewModel.alignCenterHorizontal()
            }
            .onReceive(NotificationCenter.default.publisher(for: .alignCenterV)) { _ in
                viewModel.alignCenterVertical()
            }
            .onReceive(NotificationCenter.default.publisher(for: .distributeH)) { _ in
                viewModel.distributeHorizontally()
            }
            .onReceive(NotificationCenter.default.publisher(for: .distributeV)) { _ in
                viewModel.distributeVertically()
            }
            // Match size operations
            .onReceive(NotificationCenter.default.publisher(for: .matchWidth)) { _ in
                viewModel.matchWidth()
            }
            .onReceive(NotificationCenter.default.publisher(for: .matchHeight)) { _ in
                viewModel.matchHeight()
            }
            .onReceive(NotificationCenter.default.publisher(for: .matchBoth)) { _ in
                viewModel.matchBothDimensions()
            }
            .onReceive(NotificationCenter.default.publisher(for: .matchWidthSmallest)) { _ in
                viewModel.matchWidthToSmallest()
            }
            .onReceive(NotificationCenter.default.publisher(for: .matchHeightSmallest)) { _ in
                viewModel.matchHeightToSmallest()
            }
    }
}
