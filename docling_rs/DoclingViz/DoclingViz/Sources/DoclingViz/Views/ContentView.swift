// ContentView - Main application layout
// Provides PDF canvas, stage timeline, and inspector panels
// Note: Components have been extracted to separate files for maintainability

import SwiftUI
import PDFKit
import UniformTypeIdentifiers
import AppKit
import DoclingBridge

// MARK: - Content View

struct ContentView: View {
    @EnvironmentObject var appState: AppState
    @StateObject private var viewModel = DocumentViewModel()
    @State private var isDragOver = false
    @State private var showGoToPage = false
    @State private var goToPageNumber = ""
    @State private var showZoomIndicator = false
    @State private var lastZoomLevel: Double = 1.0
    @State private var showKeyboardShortcuts = false

    var body: some View {
        NavigationSplitView {
            // Sidebar: Page list
            PageListView(viewModel: viewModel)
                .frame(minWidth: 120)
        } content: {
            // Main content: PDF canvas with overlays
            VStack(spacing: 0) {
                // Status bar (shows errors, progress, etc.)
                StatusBarView(viewModel: viewModel)

                // PDF canvas with overlay
                ZStack {
                    if viewModel.pdfDocument != nil {
                        PDFCanvasView(viewModel: viewModel)
                        OverlayView(viewModel: viewModel)

                        // Zoom indicator HUD
                        if showZoomIndicator {
                            ZoomIndicatorView(zoomLevel: viewModel.zoomLevel)
                                .transition(.opacity)
                        }
                    } else {
                        // Empty state - no PDF loaded
                        EmptyStateView()
                    }
                }
                .frame(minHeight: 400)

                Divider()

                // Stage timeline
                StageTimelineView(viewModel: viewModel)
                    .environmentObject(appState)
                    .frame(height: 100)
            }
        } detail: {
            // Detail: Element inspector
            InspectorView(viewModel: viewModel)
                .frame(minWidth: 250)
        }
        .toolbar {
            MainToolbarItems(viewModel: viewModel, appState: appState)
        }
        .modifier(NotificationHandlers(viewModel: viewModel, appState: appState))
        // Drag and drop support
        .onDrop(of: [.pdf, .fileURL], isTargeted: $isDragOver) { providers in
            handleDrop(providers: providers)
        }
        .overlay {
            // Drop zone indicator
            if isDragOver {
                RoundedRectangle(cornerRadius: 8)
                    .stroke(Color.accentColor, style: StrokeStyle(lineWidth: 3, dash: [10]))
                    .background(Color.accentColor.opacity(0.1))
                    .padding(4)
            }
        }
        .sheet(isPresented: $showGoToPage) {
            GoToPageView(
                pageNumber: $goToPageNumber,
                totalPages: viewModel.pageCount,
                isPresented: $showGoToPage,
                onGoToPage: { page in
                    viewModel.goToPage(page)
                }
            )
        }
        .alert("Unsaved Corrections", isPresented: $viewModel.showUnsavedCorrectionsAlert) {
            Button("Discard", role: .destructive) {
                viewModel.confirmNavigationDiscardEdits()
            }
            Button("Save First") {
                viewModel.saveCorrections()
                viewModel.cancelPendingNavigation()
            }
            Button("Cancel", role: .cancel) {
                viewModel.cancelPendingNavigation()
            }
        } message: {
            Text("You have \(viewModel.editedElements.count) unsaved correction(s) on this page. Would you like to save them before navigating?")
        }
        .sheet(isPresented: $viewModel.showLabelPicker) {
            NewElementLabelPicker(
                isPresented: $viewModel.showLabelPicker,
                onSelectLabel: { label in
                    viewModel.createNewElement(with: label)
                },
                onCancel: {
                    viewModel.cancelNewElement()
                }
            )
        }
        .sheet(isPresented: $viewModel.showSplitDialog) {
            SplitElementDialog(
                viewModel: viewModel,
                isPresented: $viewModel.showSplitDialog
            )
        }
        .sheet(isPresented: $showKeyboardShortcuts) {
            KeyboardShortcutsView(isPresented: $showKeyboardShortcuts)
        }
        .onReceive(NotificationCenter.default.publisher(for: .showKeyboardShortcuts)) { _ in
            showKeyboardShortcuts = true
        }
        .onReceive(NotificationCenter.default.publisher(for: .goToPage)) { _ in
            if viewModel.pageCount > 0 {
                goToPageNumber = "\(viewModel.currentPage + 1)"
                showGoToPage = true
            }
        }
        .onChange(of: viewModel.zoomLevel) { _, newZoom in
            // Show zoom indicator when zoom changes
            if abs(newZoom - lastZoomLevel) > 0.001 {
                lastZoomLevel = newZoom
                withAnimation(.easeOut(duration: 0.2)) {
                    showZoomIndicator = true
                }
                // Hide after delay
                DispatchQueue.main.asyncAfter(deadline: .now() + 1.0) {
                    withAnimation(.easeOut(duration: 0.3)) {
                        showZoomIndicator = false
                    }
                }
            }
        }
    }

    /// Handle dropped PDF files
    private func handleDrop(providers: [NSItemProvider]) -> Bool {
        guard let provider = providers.first else { return false }

        // Try to load as PDF first
        if provider.hasItemConformingToTypeIdentifier("com.adobe.pdf") {
            provider.loadDataRepresentation(forTypeIdentifier: "com.adobe.pdf") { data, error in
                if let data = data {
                    // Save to temp file and load
                    let tempURL = FileManager.default.temporaryDirectory
                        .appendingPathComponent(UUID().uuidString)
                        .appendingPathExtension("pdf")
                    do {
                        try data.write(to: tempURL)
                        Task { @MainActor in
                            viewModel.loadPDF(from: tempURL)
                        }
                    } catch {
                        Task { @MainActor in
                            viewModel.error = "Failed to save dropped PDF: \(error)"
                        }
                    }
                }
            }
            return true
        }

        // Try to load as file URL
        if provider.hasItemConformingToTypeIdentifier("public.file-url") {
            provider.loadItem(forTypeIdentifier: "public.file-url", options: nil) { item, error in
                if let data = item as? Data,
                   let url = URL(dataRepresentation: data, relativeTo: nil),
                   url.pathExtension.lowercased() == "pdf" {
                    Task { @MainActor in
                        viewModel.loadPDF(from: url)
                    }
                }
            }
            return true
        }

        return false
    }
}

// MARK: - Status Bar View

/// Status bar showing errors, progress, and corrections count
struct StatusBarView: View {
    @ObservedObject var viewModel: DocumentViewModel

    var body: some View {
        if let error = viewModel.error {
            HStack {
                Image(systemName: "exclamationmark.triangle.fill")
                    .foregroundColor(.orange)
                Text(error)
                    .font(.caption)
                    .foregroundColor(.secondary)
                Spacer()
                Button("Dismiss") {
                    viewModel.error = nil
                }
                .buttonStyle(.borderless)
                .font(.caption)
            }
            .padding(.horizontal, 8)
            .padding(.vertical, 4)
            .background(Color.orange.opacity(0.1))
        } else if viewModel.isProcessing {
            HStack {
                ProgressView()
                    .scaleEffect(0.7)
                Text(viewModel.processingProgress)
                    .font(.caption)
                    .foregroundColor(.secondary)
                Spacer()
                // Corrections indicator (even during processing)
                if viewModel.hasUnsavedEdits {
                    CorrectionsIndicatorView(editCount: viewModel.editedElements.count)
                }
            }
            .padding(.horizontal, 8)
            .padding(.vertical, 4)
            .background(Color.blue.opacity(0.1))
        } else if !viewModel.processingProgress.isEmpty {
            HStack {
                Image(systemName: "info.circle")
                    .foregroundColor(.blue)
                Text(viewModel.processingProgress)
                    .font(.caption)
                    .foregroundColor(.secondary)
                Spacer()
                // Corrections indicator
                if viewModel.hasUnsavedEdits {
                    CorrectionsIndicatorView(editCount: viewModel.editedElements.count)
                }
            }
            .padding(.horizontal, 8)
            .padding(.vertical, 4)
            .background(Color.blue.opacity(0.05))
        } else if viewModel.hasUnsavedEdits {
            // Show corrections indicator when there are unsaved edits
            HStack {
                CorrectionsIndicatorView(editCount: viewModel.editedElements.count)
                Spacer()
            }
            .padding(.horizontal, 8)
            .padding(.vertical, 4)
            .background(Color.orange.opacity(0.1))
        }
    }
}

// MARK: - Empty State View

/// Empty state when no PDF is loaded
struct EmptyStateView: View {
    var body: some View {
        VStack(spacing: 16) {
            Image(systemName: "doc.fill")
                .font(.system(size: 64))
                .foregroundColor(.secondary.opacity(0.5))
            Text("No PDF Loaded")
                .font(.title2)
                .foregroundColor(.secondary)
            Text("Open a PDF file (Cmd+O) or drag one here")
                .font(.caption)
                .foregroundColor(.secondary.opacity(0.8))
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(Color(nsColor: .windowBackgroundColor))
    }
}

// MARK: - Main Toolbar Items

/// Toolbar items for the main window
struct MainToolbarItems: ToolbarContent {
    @ObservedObject var viewModel: DocumentViewModel
    let appState: AppState

    var body: some SwiftUI.ToolbarContent {
        ToolbarItemGroup(placement: .navigation) {
            Button(action: { viewModel.previousPage() }) {
                Image(systemName: "chevron.left")
            }
            .disabled(!viewModel.canGoBack)

            Text("Page \(viewModel.currentPage + 1) / \(max(1, viewModel.pageCount))")
                .monospacedDigit()
                .frame(width: 100)

            Button(action: { viewModel.nextPage() }) {
                Image(systemName: "chevron.right")
            }
            .disabled(!viewModel.canGoForward)
        }

        ToolbarItemGroup(placement: .primaryAction) {
            // Draw tool toggle
            Toggle(isOn: Binding(
                get: { viewModel.editTool == .draw },
                set: { viewModel.editTool = $0 ? .draw : .select }
            )) {
                Image(systemName: "pencil.and.outline")
            }
            .help("Draw new element (D)")
            .disabled(!appState.hasPdfMl || viewModel.pdfDocument == nil)

            // Lasso tool toggle
            Toggle(isOn: Binding(
                get: { viewModel.editTool == .lasso },
                set: { viewModel.editTool = $0 ? .lasso : .select }
            )) {
                Image(systemName: "lasso")
            }
            .help("Lasso selection (L)")
            .disabled(!appState.hasPdfMl || viewModel.pdfDocument == nil)

            // Marquee tool toggle
            Toggle(isOn: Binding(
                get: { viewModel.editTool == .marquee },
                set: { viewModel.editTool = $0 ? .marquee : .select }
            )) {
                Image(systemName: "rectangle.dashed.badge.record")
            }
            .help("Marquee selection (M)")
            .disabled(!appState.hasPdfMl || viewModel.pdfDocument == nil)

            Divider()

            // Zoom controls
            Button(action: { viewModel.zoomOut() }) {
                Image(systemName: "minus.magnifyingglass")
            }
            .disabled(!viewModel.canZoomOut)
            .help("Zoom Out")

            Text("\(viewModel.zoomPercentage)%")
                .monospacedDigit()
                .frame(width: 50)

            Button(action: { viewModel.zoomIn() }) {
                Image(systemName: "plus.magnifyingglass")
            }
            .disabled(!viewModel.canZoomIn)
            .help("Zoom In")

            Divider()

            Toggle(isOn: $viewModel.showTextCells) {
                Image(systemName: "character.cursor.ibeam")
            }
            .help("Show text cells")
            .disabled(!appState.hasPdfMl)

            Toggle(isOn: $viewModel.showBoundingBoxes) {
                Image(systemName: "rectangle.dashed")
            }
            .help("Show bounding boxes")
            .disabled(!appState.hasPdfMl)

            // Feature status indicator
            if !appState.hasPdfMl {
                Image(systemName: "exclamationmark.triangle")
                    .foregroundColor(.orange)
                    .help(appState.featureStatusMessage)
            }

            Divider()

            // Keyboard shortcuts help
            Button(action: {}) {
                Image(systemName: "keyboard")
            }
            .help("""
                Keyboard Shortcuts:
                ───────────────────
                Space: Play/Pause stages
                ←/→: Previous/Next stage
                ↑/↓: Previous/Next page
                Cmd+G: Go to page
                ───────────────────
                Quick Labels (number keys):
                1=Text  2=Title  3=Section
                4=Table 5=Picture 6=List
                7=Caption 8=Footnote 9=Code
                ───────────────────
                D: Toggle draw mode
                L: Toggle lasso selection
                M: Toggle marquee selection
                Cmd+D: Duplicate element
                Delete: Delete element
                Escape: Deselect/Cancel
                Cmd+A: Select all
                ───────────────────
                Cmd+C: Copy element(s)
                Cmd+X: Cut element(s)
                Cmd+V: Paste element(s)
                Cmd+Shift+M: Merge elements
                ───────────────────
                Alignment (multi-select):
                Cmd+Opt+[: Align left
                Cmd+Opt+]: Align right
                Cmd+Shift+Opt+[: Align top
                Cmd+Shift+Opt+]: Align bottom
                Cmd+Opt+\\: Distribute H
                Cmd+Shift+Opt+\\: Distribute V
                ───────────────────
                Option+↑/↓/←/→: Nudge element
                Cmd+Z: Undo
                Cmd+Shift+Z: Redo
                ───────────────────
                Cmd+': Toggle snap to grid
                Cmd+;: Toggle alignment guides
                ───────────────────
                Cmd+Shift+S: Save corrections
                Cmd+Shift+L: Load corrections
                """)
            .buttonStyle(.borderless)
        }
    }
}
