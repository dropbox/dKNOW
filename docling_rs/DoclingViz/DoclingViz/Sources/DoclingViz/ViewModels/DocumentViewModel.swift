// DocumentViewModel - State management for PDF visualization
// Manages PDF document, pipeline processing, and UI state

import SwiftUI
import PDFKit
import Combine
import AppKit
import UniformTypeIdentifiers
import DoclingBridge

// MARK: - Edit Tool

/// Available editing tools
public enum EditTool: String, CaseIterable {
    case select = "Select"
    case move = "Move"
    case resize = "Resize"
    case draw = "Draw"
    case lasso = "Lasso"
    case marquee = "Marquee"  // Box selection
}

// MARK: - Drag Handle

/// Resize handle position
public enum DragHandle: Int, CaseIterable {
    case topLeft = 0
    case topCenter = 1
    case topRight = 2
    case middleLeft = 3
    case middleRight = 4
    case bottomLeft = 5
    case bottomCenter = 6
    case bottomRight = 7
    case center = 8  // For moving

    /// Cursor for this handle type
    var cursor: NSCursor {
        switch self {
        case .topLeft, .bottomRight: return NSCursor.crosshair
        case .topRight, .bottomLeft: return NSCursor.crosshair
        case .topCenter, .bottomCenter: return NSCursor.resizeUpDown
        case .middleLeft, .middleRight: return NSCursor.resizeLeftRight
        case .center: return NSCursor.openHand
        }
    }
}

// MARK: - BBox Edit

/// A single bounding box edit operation
public struct BBoxEdit: Identifiable, Equatable {
    public let id = UUID()
    public let elementId: UInt32
    public let originalBbox: BoundingBox
    public var newBbox: BoundingBox
    public let timestamp: Date

    public init(elementId: UInt32, originalBbox: BoundingBox, newBbox: BoundingBox) {
        self.elementId = elementId
        self.originalBbox = originalBbox
        self.newBbox = newBbox
        self.timestamp = Date()
    }
}

// MARK: - Label Edit

/// A label change operation
public struct LabelEdit: Identifiable, Equatable {
    public let id = UUID()
    public let elementId: UInt32
    public let originalLabel: DocItemLabel
    public let newLabel: DocItemLabel
    public let timestamp: Date

    public init(elementId: UInt32, originalLabel: DocItemLabel, newLabel: DocItemLabel) {
        self.elementId = elementId
        self.originalLabel = originalLabel
        self.newLabel = newLabel
        self.timestamp = Date()
    }
}

// MARK: - Alignment Guides

/// Alignment guides for element snapping
public struct AlignmentGuides {
    public var horizontalGuides: [Float] = []
    public var verticalGuides: [Float] = []

    public var isEmpty: Bool {
        horizontalGuides.isEmpty && verticalGuides.isEmpty
    }
}

// MARK: - Snap Indicators

/// Snap indicators showing where an element is snapping to the grid
public struct SnapIndicators {
    /// Vertical grid lines the element is snapping to (x coordinates)
    public var verticalLines: [Float] = []
    /// Horizontal grid lines the element is snapping to (y coordinates)
    public var horizontalLines: [Float] = []
    /// Whether left edge is snapped
    public var leftSnapped: Bool = false
    /// Whether right edge is snapped
    public var rightSnapped: Bool = false
    /// Whether top edge is snapped
    public var topSnapped: Bool = false
    /// Whether bottom edge is snapped
    public var bottomSnapped: Bool = false

    public var isEmpty: Bool {
        verticalLines.isEmpty && horizontalLines.isEmpty
    }

    public mutating func clear() {
        verticalLines.removeAll()
        horizontalLines.removeAll()
        leftSnapped = false
        rightSnapped = false
        topSnapped = false
        bottomSnapped = false
    }
}

// MARK: - Merge Edit

/// A merge operation that combines multiple elements into one
public struct MergeEdit: Identifiable, Equatable {
    public let id = UUID()
    public let mergedElementId: UInt32  // ID of the newly created merged element
    public let originalElementIds: Set<UInt32>  // IDs of elements that were merged
    public let originalElements: [Element]  // Full element data for undo
    public let timestamp: Date

    public init(mergedElementId: UInt32, originalElementIds: Set<UInt32>, originalElements: [Element]) {
        self.mergedElementId = mergedElementId
        self.originalElementIds = originalElementIds
        self.originalElements = originalElements
        self.timestamp = Date()
    }
}

// MARK: - Split Edit

/// A split operation that divides one element into multiple elements
public struct SplitEdit: Identifiable, Equatable {
    public let id = UUID()
    public let originalElementId: UInt32  // ID of the element that was split
    public let originalElement: Element   // Full element data for undo
    public let resultElementIds: [UInt32]  // IDs of the elements created from split
    public let timestamp: Date

    public init(originalElementId: UInt32, originalElement: Element, resultElementIds: [UInt32]) {
        self.originalElementId = originalElementId
        self.originalElement = originalElement
        self.resultElementIds = resultElementIds
        self.timestamp = Date()
    }
}

// MARK: - Split Direction

/// Direction for splitting an element
public enum SplitDirection: String, CaseIterable, Identifiable {
    case horizontal = "Horizontal"  // Split into top and bottom
    case vertical = "Vertical"      // Split into left and right

    public var id: String { rawValue }

    public var systemImage: String {
        switch self {
        case .horizontal: return "rectangle.split.1x2"
        case .vertical: return "rectangle.split.2x1"
        }
    }

    public var description: String {
        switch self {
        case .horizontal: return "Top/Bottom"
        case .vertical: return "Left/Right"
        }
    }
}

// MARK: - Edit Action

/// Combined edit action for undo/redo
public enum EditAction: Identifiable, Equatable {
    case bbox(BBoxEdit)
    case label(LabelEdit)
    case delete(Element)
    case create(Element)
    case merge(MergeEdit)
    case split(SplitEdit)

    public var id: UUID {
        switch self {
        case .bbox(let edit): return edit.id
        case .label(let edit): return edit.id
        case .delete(_): return UUID()
        case .create(_): return UUID()
        case .merge(let edit): return edit.id
        case .split(let edit): return edit.id
        }
    }
}

// MARK: - Recent Files Manager

@MainActor
class RecentFilesManager: ObservableObject {
    static let shared = RecentFilesManager()

    @Published private(set) var recentFiles: [URL] = []
    private let maxRecentFiles = 10
    private let userDefaultsKey = "DoclingViz.RecentFiles"

    private init() {
        loadRecentFiles()
    }

    func addRecentFile(_ url: URL) {
        // Remove if already exists (to move it to front)
        recentFiles.removeAll { $0 == url }
        // Add to front
        recentFiles.insert(url, at: 0)
        // Trim to max size
        if recentFiles.count > maxRecentFiles {
            recentFiles = Array(recentFiles.prefix(maxRecentFiles))
        }
        saveRecentFiles()
    }

    func clearRecentFiles() {
        recentFiles = []
        saveRecentFiles()
    }

    private func loadRecentFiles() {
        guard let data = UserDefaults.standard.data(forKey: userDefaultsKey),
              let bookmarks = try? JSONDecoder().decode([Data].self, from: data) else {
            return
        }

        recentFiles = bookmarks.compactMap { bookmark -> URL? in
            var stale = false
            guard let url = try? URL(resolvingBookmarkData: bookmark,
                                     options: .withSecurityScope,
                                     relativeTo: nil,
                                     bookmarkDataIsStale: &stale) else {
                return nil
            }
            // Check file still exists
            if FileManager.default.fileExists(atPath: url.path) {
                return url
            }
            return nil
        }
    }

    private func saveRecentFiles() {
        let bookmarks = recentFiles.compactMap { url -> Data? in
            try? url.bookmarkData(options: .withSecurityScope,
                                  includingResourceValuesForKeys: nil,
                                  relativeTo: nil)
        }

        if let data = try? JSONEncoder().encode(bookmarks) {
            UserDefaults.standard.set(data, forKey: userDefaultsKey)
        }
    }
}

// MARK: - Document View Model

@MainActor
class DocumentViewModel: ObservableObject {
    // MARK: - Document State

    @Published var pdfDocument: PDFDocument?
    @Published var pdfURL: URL?
    @Published var currentPage: Int = 0
    @Published private(set) var pageThumbnails: [Int: NSImage] = [:]

    /// Thumbnail size for sidebar
    static let thumbnailSize = CGSize(width: 80, height: 104)

    // MARK: - Pipeline State

    @Published var pipeline: DoclingPipeline?
    @Published var currentStage: PipelineStage = .rawPdf
    @Published private(set) var stageSnapshots: [PipelineStage: StageSnapshot] = [:]

    // MARK: - UI State

    @Published var selectedElementId: UInt32?  // Primary selection (last clicked)
    @Published var selectedElementIds: Set<UInt32> = []  // Multi-selection support
    @Published var showTextCells: Bool = false
    @Published var showBoundingBoxes: Bool = true
    @Published var isPlaying: Bool = false
    @Published var playbackSpeed: Double = 1.0
    @Published var zoomLevel: Double = 1.0  // 1.0 = 100%
    @Published var zoomTargetRect: CGRect?  // Target rect for zoom-to-selection (PDF coordinates)
    @Published var confidenceThreshold: Double = 0.0  // 0.0-1.0, filter elements below
    @Published var colorByConfidence: Bool = false  // Color elements by confidence level
    @Published var selectedLabels: Set<DocItemLabel> = []  // Empty = show all labels

    // MARK: - Editing State

    @Published var editTool: EditTool = .select
    @Published var isEditing: Bool = false
    @Published var editedElements: [UInt32: Element] = [:]  // Modified elements by ID
    @Published var activeDragHandle: DragHandle?
    @Published var isDragging: Bool = false
    @Published var dragStartPoint: CGPoint?
    @Published var dragStartBbox: BoundingBox?

    // MARK: - Element Lock State

    @Published var lockedElementIds: Set<UInt32> = []  // Locked elements cannot be edited

    /// Check if element is locked
    func isElementLocked(_ elementId: UInt32) -> Bool {
        lockedElementIds.contains(elementId)
    }

    /// Check if selected element is locked
    var isSelectedElementLocked: Bool {
        guard let elementId = selectedElementId else { return false }
        return isElementLocked(elementId)
    }

    /// Check if any selected element is locked
    var hasLockedSelection: Bool {
        !selectedElementIds.intersection(lockedElementIds).isEmpty
    }

    /// Lock the selected element(s)
    func lockSelectedElements() {
        lockedElementIds.formUnion(selectedElementIds)
    }

    /// Unlock the selected element(s)
    func unlockSelectedElements() {
        lockedElementIds.subtract(selectedElementIds)
    }

    /// Toggle lock state of selected element(s)
    func toggleLockSelectedElements() {
        if hasLockedSelection {
            unlockSelectedElements()
        } else {
            lockSelectedElements()
        }
    }

    /// Lock all elements on current page
    func lockAllElements() {
        guard let snapshot = currentStageSnapshot else { return }
        lockedElementIds.formUnion(snapshot.elements.map { $0.id })
    }

    /// Unlock all elements
    func unlockAllElements() {
        lockedElementIds.removeAll()
    }

    // MARK: - Snap Settings

    @Published var snapToGrid: Bool = false  // Enable snap-to-grid
    @Published var gridSize: Float = 10.0    // Grid size in points
    @Published var showAlignmentGuides: Bool = true  // Show alignment guides when dragging
    @Published var showGridOverlay: Bool = false  // Show visual grid when snap-to-grid is enabled

    /// Snap a value to the grid
    func snapToGridValue(_ value: Float) -> Float {
        guard snapToGrid && gridSize > 0 else { return value }
        return round(value / gridSize) * gridSize
    }

    /// Snap a bounding box to the grid and update snap indicators
    func snapBboxToGrid(_ bbox: BoundingBox) -> BoundingBox {
        guard snapToGrid else {
            currentSnapIndicators.clear()
            return bbox
        }

        var indicators = SnapIndicators()

        // Snap each edge
        let snappedX = snapToGridValue(bbox.x)
        let snappedY = snapToGridValue(bbox.y)
        let snappedRight = snapToGridValue(bbox.x + bbox.width)
        let snappedTop = snapToGridValue(bbox.y + bbox.height)

        // Check which edges snapped
        if abs(snappedX - bbox.x) < 0.01 {
            indicators.leftSnapped = true
            indicators.verticalLines.append(snappedX)
        }
        if abs(snappedY - bbox.y) < 0.01 {
            indicators.bottomSnapped = true
            indicators.horizontalLines.append(snappedY)
        }
        if abs(snappedRight - (bbox.x + bbox.width)) < 0.01 {
            indicators.rightSnapped = true
            indicators.verticalLines.append(snappedRight)
        }
        if abs(snappedTop - (bbox.y + bbox.height)) < 0.01 {
            indicators.topSnapped = true
            indicators.horizontalLines.append(snappedTop)
        }

        currentSnapIndicators = indicators

        return BoundingBox(
            x: snappedX,
            y: snappedY,
            width: max(gridSize, snappedRight - snappedX),
            height: max(gridSize, snappedTop - snappedY)
        )
    }

    /// Find alignment guides for the current element
    func alignmentGuides(for element: Element) -> AlignmentGuides {
        guard showAlignmentGuides, let snapshot = currentStageSnapshot else {
            return AlignmentGuides()
        }

        var guides = AlignmentGuides()
        let tolerance: Float = 5.0  // 5 points snap tolerance

        for other in snapshot.elements where other.id != element.id {
            let otherBbox = editedElements[other.id]?.bbox ?? other.bbox

            // Horizontal alignment (left edges)
            if abs(element.bbox.x - otherBbox.x) < tolerance {
                guides.verticalGuides.append(otherBbox.x)
            }
            // Horizontal alignment (right edges)
            if abs(element.bbox.right - otherBbox.right) < tolerance {
                guides.verticalGuides.append(otherBbox.right)
            }
            // Horizontal alignment (centers)
            let elementCenterX = element.bbox.x + element.bbox.width / 2
            let otherCenterX = otherBbox.x + otherBbox.width / 2
            if abs(elementCenterX - otherCenterX) < tolerance {
                guides.verticalGuides.append(otherCenterX)
            }

            // Vertical alignment (top edges)
            if abs(element.bbox.top - otherBbox.top) < tolerance {
                guides.horizontalGuides.append(otherBbox.top)
            }
            // Vertical alignment (bottom edges)
            if abs(element.bbox.y - otherBbox.y) < tolerance {
                guides.horizontalGuides.append(otherBbox.y)
            }
            // Vertical alignment (centers)
            let elementCenterY = element.bbox.y + element.bbox.height / 2
            let otherCenterY = otherBbox.y + otherBbox.height / 2
            if abs(elementCenterY - otherCenterY) < tolerance {
                guides.horizontalGuides.append(otherCenterY)
            }
        }

        return guides
    }

    /// Current alignment guides (computed during drag)
    @Published var currentAlignmentGuides: AlignmentGuides = AlignmentGuides()

    /// Current snap indicators (grid lines the element is snapping to)
    @Published var currentSnapIndicators: SnapIndicators = SnapIndicators()

    // MARK: - Drawing State

    @Published var isDrawing: Bool = false
    @Published var drawStartPoint: CGPoint?
    @Published var drawCurrentPoint: CGPoint?
    @Published var showLabelPicker: Bool = false
    @Published var pendingNewElementBbox: BoundingBox?

    // MARK: - Lasso Selection State

    @Published var isLassoDrawing: Bool = false
    @Published var lassoPath: [CGPoint] = []
    @Published var lassoViewSize: CGSize = .zero  // View size when lasso was drawn

    // MARK: - Marquee Selection State

    @Published var isMarqueeDrawing: Bool = false
    @Published var marqueeStartPoint: CGPoint?
    @Published var marqueeCurrentPoint: CGPoint?
    @Published var marqueeViewSize: CGSize = .zero  // View size when marquee was drawn

    // MARK: - Split Mode State

    @Published var isSplitMode: Bool = false
    @Published var splitDirection: SplitDirection = .horizontal
    @Published var splitPosition: Float = 0.5  // 0.0 to 1.0 (percentage across element)
    @Published var showSplitDialog: Bool = false  // Show split configuration dialog

    // MARK: - Clipboard State

    /// Clipboard for copy/paste operations
    /// Stores elements with their bounding boxes relative to their original position
    @Published private(set) var clipboard: [Element] = []

    /// Source page index for clipboard (used for paste offset calculation)
    private var clipboardSourcePage: Int = 0

    /// Whether paste is available
    var canPaste: Bool { !clipboard.isEmpty }

    /// Next available element ID for new elements
    private var nextElementId: UInt32 = 1_000_000

    /// Undo/Redo stacks
    @Published private(set) var undoStack: [EditAction] = []
    @Published private(set) var redoStack: [EditAction] = []

    /// Whether undo is available
    var canUndo: Bool { !undoStack.isEmpty }

    /// Whether redo is available
    var canRedo: Bool { !redoStack.isEmpty }

    // MARK: - Zoom Constants
    static let minZoom: Double = 0.25
    static let maxZoom: Double = 4.0
    static let zoomSteps: [Double] = [0.25, 0.5, 0.75, 1.0, 1.25, 1.5, 2.0, 3.0, 4.0]

    // MARK: - Processing State

    @Published var isProcessing: Bool = false
    @Published var processingProgress: String = ""
    @Published var error: String?

    private var playbackTimer: Timer?
    private var cancellables = Set<AnyCancellable>()

    // MARK: - Computed Properties

    var pageCount: Int {
        pdfDocument?.pageCount ?? 0
    }

    var canGoBack: Bool {
        currentPage > 0
    }

    var canGoForward: Bool {
        currentPage < pageCount - 1
    }

    var currentStageSnapshot: StageSnapshot? {
        stageSnapshots[currentStage]
    }

    /// Elements filtered by confidence threshold and label selection
    /// Uses edited elements if available
    var filteredElements: [Element] {
        guard let snapshot = currentStageSnapshot else { return [] }

        // Start with snapshot elements (with edits applied)
        var elements = snapshot.elements.compactMap { element -> Element? in
            // Exclude deleted elements
            if deletedElementIds.contains(element.id) {
                return nil
            }
            // Use edited version if available
            return editedElements[element.id] ?? element
        }

        // Add user-created elements (not in snapshot)
        let snapshotIds = Set(snapshot.elements.map { $0.id })
        for (id, element) in editedElements {
            if !snapshotIds.contains(id) && !deletedElementIds.contains(id) {
                elements.append(element)
            }
        }

        // Apply filters
        return elements.filter { element in
            // Filter by confidence threshold
            if confidenceThreshold > 0 && element.confidence < Float(confidenceThreshold) {
                return false
            }
            // Filter by selected labels (empty = show all)
            if !selectedLabels.isEmpty && !selectedLabels.contains(element.label) {
                return false
            }
            return true
        }
    }

    /// Get the currently selected element (with edits applied)
    var selectedElement: Element? {
        guard let id = selectedElementId else { return nil }
        // First check edited elements
        if let edited = editedElements[id] {
            return edited
        }
        // Otherwise find in snapshot
        return currentStageSnapshot?.elements.first { $0.id == id }
    }

    /// Get all selected elements (with edits applied)
    var selectedElements: [Element] {
        guard !selectedElementIds.isEmpty else {
            // Fall back to single selection for backward compatibility
            if let element = selectedElement {
                return [element]
            }
            return []
        }
        var elements: [Element] = []
        for id in selectedElementIds {
            if let edited = editedElements[id] {
                elements.append(edited)
            } else if let element = currentStageSnapshot?.elements.first(where: { $0.id == id }) {
                elements.append(element)
            }
        }
        return elements
    }

    /// Check if an element is selected
    func isElementSelected(_ elementId: UInt32) -> Bool {
        selectedElementIds.contains(elementId) || selectedElementId == elementId
    }

    /// Count of selected elements
    var selectedElementCount: Int {
        if !selectedElementIds.isEmpty {
            return selectedElementIds.count
        }
        return selectedElementId != nil ? 1 : 0
    }

    /// Count of elements hidden by filters
    var hiddenElementCount: Int {
        guard let snapshot = currentStageSnapshot else { return 0 }
        return snapshot.elements.count - filteredElements.count
    }

    /// Check if a specific label is currently visible
    func isLabelVisible(_ label: DocItemLabel) -> Bool {
        selectedLabels.isEmpty || selectedLabels.contains(label)
    }

    /// Toggle label visibility
    func toggleLabel(_ label: DocItemLabel) {
        if selectedLabels.contains(label) {
            selectedLabels.remove(label)
        } else {
            selectedLabels.insert(label)
        }
    }

    /// Clear label filter (show all)
    func clearLabelFilter() {
        selectedLabels = []
    }

    /// Set label filter to show only specific label
    func showOnlyLabel(_ label: DocItemLabel) {
        selectedLabels = [label]
    }

    var pageSize: (width: Float, height: Float) {
        pipeline?.pageSize(at: currentPage) ?? (612, 792) // Default to letter size
    }

    /// Window title showing current document
    var windowTitle: String {
        if let url = pdfURL {
            return url.lastPathComponent
        }
        return "DoclingViz"
    }

    // MARK: - Initialization

    init() {
        // Observe page changes to reload snapshots
        $currentPage
            .removeDuplicates()
            .sink { [weak self] _ in
                Task { await self?.loadPageSnapshots() }
            }
            .store(in: &cancellables)

        // Observe stage changes
        $currentStage
            .removeDuplicates()
            .sink { [weak self] stage in
                self?.runToStage(stage)
            }
            .store(in: &cancellables)

        // Observe playback speed changes to update timer
        $playbackSpeed
            .removeDuplicates()
            .dropFirst() // Ignore initial value
            .sink { [weak self] _ in
                self?.updatePlaybackIfNeeded()
            }
            .store(in: &cancellables)
    }

    // MARK: - Document Operations

    func openPDF() {
        let panel = NSOpenPanel()
        panel.allowedContentTypes = [.pdf]
        panel.allowsMultipleSelection = false

        if panel.runModal() == .OK, let url = panel.url {
            loadPDF(from: url)
        }
    }

    func loadPDF(from url: URL) {
        // Reset state
        pdfURL = url
        pdfDocument = PDFDocument(url: url)
        currentPage = 0
        currentStage = .rawPdf
        stageSnapshots = [:]
        pageThumbnails = [:]
        selectedElementId = nil
        error = nil

        // Even without pipeline, we can still view the PDF via PDFKit
        guard let _ = pdfDocument else {
            error = "Failed to open PDF file"
            return
        }

        // Add to recent files
        RecentFilesManager.shared.addRecentFile(url)

        // Generate page thumbnails asynchronously
        Task {
            await generateThumbnails()
        }

        // Try to load into pipeline for ML processing (optional)
        guard let pipeline = pipeline else {
            // No pipeline - view-only mode
            processingProgress = "View-only mode (no pipeline)"
            return
        }

        Task {
            isProcessing = true
            processingProgress = "Loading PDF..."

            do {
                try pipeline.loadPDF(at: url)
                processingProgress = "Processing..."
                await loadPageSnapshots()
                processingProgress = ""
            } catch {
                // Pipeline failed but we can still view the PDF
                self.error = "Pipeline unavailable: \(error). View-only mode active."
            }

            isProcessing = false
        }
    }

    // MARK: - Page Navigation

    /// Pending page navigation (for unsaved corrections dialog)
    @Published var pendingPageNavigation: Int?
    @Published var showUnsavedCorrectionsAlert: Bool = false

    func nextPage() {
        guard canGoForward else { return }
        if hasUnsavedEdits {
            pendingPageNavigation = currentPage + 1
            showUnsavedCorrectionsAlert = true
        } else {
            navigateToPage(currentPage + 1)
        }
    }

    func previousPage() {
        guard canGoBack else { return }
        if hasUnsavedEdits {
            pendingPageNavigation = currentPage - 1
            showUnsavedCorrectionsAlert = true
        } else {
            navigateToPage(currentPage - 1)
        }
    }

    func goToPage(_ page: Int) {
        guard page >= 0 && page < pageCount else { return }
        if hasUnsavedEdits && page != currentPage {
            pendingPageNavigation = page
            showUnsavedCorrectionsAlert = true
        } else {
            navigateToPage(page)
        }
    }

    /// Navigate without checking for unsaved changes (called after confirmation)
    func navigateToPage(_ page: Int) {
        currentPage = page
        stageSnapshots = [:]
    }

    /// Confirm navigation and discard edits
    func confirmNavigationDiscardEdits() {
        guard let targetPage = pendingPageNavigation else { return }
        discardEdits()
        navigateToPage(targetPage)
        pendingPageNavigation = nil
    }

    /// Cancel pending navigation
    func cancelPendingNavigation() {
        pendingPageNavigation = nil
    }

    // MARK: - Stage Navigation

    func nextStage() {
        let allStages = PipelineStage.allCases
        if let currentIndex = allStages.firstIndex(of: currentStage),
           currentIndex + 1 < allStages.count {
            currentStage = allStages[currentIndex + 1]
        }
    }

    func previousStage() {
        let allStages = PipelineStage.allCases
        if let currentIndex = allStages.firstIndex(of: currentStage),
           currentIndex > 0 {
            currentStage = allStages[currentIndex - 1]
        }
    }

    // MARK: - Zoom

    func zoomIn() {
        let nextStep = Self.zoomSteps.first { $0 > zoomLevel } ?? Self.maxZoom
        zoomLevel = min(nextStep, Self.maxZoom)
    }

    func zoomOut() {
        let prevStep = Self.zoomSteps.last { $0 < zoomLevel } ?? Self.minZoom
        zoomLevel = max(prevStep, Self.minZoom)
    }

    func zoomToFit() {
        zoomLevel = 1.0
    }

    func zoomToActualSize() {
        zoomLevel = 1.0
    }

    /// Set zoom level directly with clamping (for pinch-to-zoom)
    func setZoom(_ level: Double) {
        zoomLevel = min(Self.maxZoom, max(Self.minZoom, level))
    }

    /// Adjust zoom by a magnification delta (for pinch gestures)
    func adjustZoom(by magnification: Double) {
        // Magnification is centered around 1.0 (no change)
        // Apply it as a multiplier to current zoom
        let newZoom = zoomLevel * magnification
        setZoom(newZoom)
    }

    var zoomPercentage: Int {
        Int(zoomLevel * 100)
    }

    var canZoomIn: Bool {
        zoomLevel < Self.maxZoom
    }

    var canZoomOut: Bool {
        zoomLevel > Self.minZoom
    }

    /// Check if zoom to selection is possible (need selected elements)
    var canZoomToSelection: Bool {
        !selectedElementIds.isEmpty
    }

    /// Zoom and scroll to fit selected elements in the viewport
    func zoomToSelection() {
        let elements = selectedElements
        guard !elements.isEmpty else { return }

        // Calculate combined bounding box of all selected elements
        var minX = Float.infinity
        var minY = Float.infinity
        var maxX = -Float.infinity
        var maxY = -Float.infinity

        for element in elements {
            minX = min(minX, element.bbox.x)
            minY = min(minY, element.bbox.y)
            maxX = max(maxX, element.bbox.x + element.bbox.width)
            maxY = max(maxY, element.bbox.y + element.bbox.height)
        }

        // Add padding (10% on each side)
        let paddingX = (maxX - minX) * 0.1
        let paddingY = (maxY - minY) * 0.1
        minX -= paddingX
        minY -= paddingY
        maxX += paddingX
        maxY += paddingY

        // Convert to CGRect (in PDF coordinates, origin at bottom-left)
        let rect = CGRect(
            x: CGFloat(minX),
            y: CGFloat(minY),
            width: CGFloat(maxX - minX),
            height: CGFloat(maxY - minY)
        )

        // Set the target rect - the view will react to this
        zoomTargetRect = rect
    }

    /// Clear the zoom target after scrolling is complete
    func clearZoomTarget() {
        zoomTargetRect = nil
    }

    // MARK: - Playback

    func togglePlayback() {
        isPlaying.toggle()
        if isPlaying {
            startPlayback()
        } else {
            stopPlayback()
        }
    }

    private func startPlayback() {
        // Base interval is 1 second, divided by speed
        let interval = 1.0 / playbackSpeed
        playbackTimer = Timer.scheduledTimer(withTimeInterval: interval, repeats: true) { [weak self] _ in
            Task { @MainActor in
                guard let self = self else { return }
                let allStages = PipelineStage.allCases
                if let currentIndex = allStages.firstIndex(of: self.currentStage),
                   currentIndex + 1 < allStages.count {
                    self.currentStage = allStages[currentIndex + 1]
                } else {
                    self.stopPlayback()
                }
            }
        }
    }

    /// Restart playback with new speed if playing
    private func updatePlaybackIfNeeded() {
        if isPlaying {
            stopPlayback()
            isPlaying = true
            startPlayback()
        }
    }

    private func stopPlayback() {
        isPlaying = false
        playbackTimer?.invalidate()
        playbackTimer = nil
    }

    // MARK: - Pipeline Operations

    private func runToStage(_ stage: PipelineStage) {
        guard let pipeline = pipeline, pdfDocument != nil else { return }

        // Check if we already have this snapshot
        if stageSnapshots[stage] != nil { return }

        Task {
            do {
                try pipeline.runToStage(stage, pageIndex: currentPage)
                if let snapshot = pipeline.snapshot(at: stage, pageIndex: currentPage) {
                    stageSnapshots[stage] = snapshot
                }
            } catch {
                self.error = "Pipeline error: \(error)"
            }
        }
    }

    private func loadPageSnapshots() async {
        guard let pipeline = pipeline else { return }

        stageSnapshots = [:]

        // Load all stage snapshots for current page
        for stage in PipelineStage.allCases {
            do {
                try pipeline.runToStage(stage, pageIndex: currentPage)
                if let snapshot = pipeline.snapshot(at: stage, pageIndex: currentPage) {
                    stageSnapshots[stage] = snapshot
                }
            } catch {
                // Some stages might not be available
                break
            }
        }
    }

    // MARK: - Thumbnail Generation

    /// Generate thumbnails for all pages asynchronously
    private func generateThumbnails() async {
        guard let document = pdfDocument else { return }

        // Generate thumbnails in batches, keeping on main thread since PDFPage isn't thread-safe
        // but yielding periodically to keep UI responsive
        for pageIndex in 0..<document.pageCount {
            guard let page = document.page(at: pageIndex) else { continue }

            // Generate thumbnail (PDFPage.thumbnail is fast, ~1-2ms per page)
            let thumbnail = page.thumbnail(of: Self.thumbnailSize, for: .cropBox)
            pageThumbnails[pageIndex] = thumbnail

            // Yield periodically to keep UI responsive
            if pageIndex % 10 == 0 {
                await Task.yield()
            }
        }
    }

    /// Get thumbnail for a specific page (generates on-demand if not cached)
    func thumbnail(for pageIndex: Int) -> NSImage? {
        // Return cached thumbnail if available
        if let cached = pageThumbnails[pageIndex] {
            return cached
        }

        // Generate on-demand if not yet available
        guard let document = pdfDocument,
              let page = document.page(at: pageIndex) else { return nil }

        let thumbnail = page.thumbnail(of: Self.thumbnailSize, for: .cropBox)
        pageThumbnails[pageIndex] = thumbnail
        return thumbnail
    }

    // MARK: - Interaction

    /// Handle tap with modifier key support
    /// - Parameters:
    ///   - point: Tap location in view coordinates
    ///   - size: View size
    ///   - modifiers: Keyboard modifiers (shift, command)
    func handleTap(at point: CGPoint, in size: CGSize, modifiers: EventModifiers = []) {
        guard let snapshot = currentStageSnapshot else { return }

        let pageSize = self.pageSize
        let scale = min(
            size.width / CGFloat(pageSize.width),
            size.height / CGFloat(pageSize.height)
        )

        // Find clicked element
        var hitElement: Element?
        for element in snapshot.elements {
            let x = CGFloat(element.bbox.x) * scale
            let y = (CGFloat(pageSize.height) - CGFloat(element.bbox.y) - CGFloat(element.bbox.height)) * scale
            let width = CGFloat(element.bbox.width) * scale
            let height = CGFloat(element.bbox.height) * scale
            let rect = CGRect(x: x, y: y, width: width, height: height)

            if rect.contains(point) {
                hitElement = element
                break
            }
        }

        if let element = hitElement {
            handleElementSelection(element.id, modifiers: modifiers)
        } else {
            // Click in empty space - clear selection unless shift/command is held
            if !modifiers.contains(.shift) && !modifiers.contains(.command) {
                clearSelection()
            }
        }
    }

    /// Handle element selection with modifier support
    func handleElementSelection(_ elementId: UInt32, modifiers: EventModifiers = []) {
        if modifiers.contains(.shift) {
            // Shift+click: Range selection (add to selection)
            // For simplicity, we just toggle the element in the selection set
            toggleElementInSelection(elementId)
        } else if modifiers.contains(.command) {
            // Command+click: Toggle individual element in selection
            toggleElementInSelection(elementId)
        } else {
            // Regular click: Select only this element
            selectSingleElement(elementId)
        }
    }

    /// Select a single element (clear multi-selection)
    func selectSingleElement(_ elementId: UInt32) {
        selectedElementIds.removeAll()
        selectedElementId = elementId
    }

    /// Toggle an element in the multi-selection
    func toggleElementInSelection(_ elementId: UInt32) {
        if selectedElementIds.contains(elementId) {
            selectedElementIds.remove(elementId)
            // If removing the primary selection, update it
            if selectedElementId == elementId {
                selectedElementId = selectedElementIds.first
            }
        } else {
            // Add to multi-selection
            if let currentId = selectedElementId {
                // Move current selection to set if not already there
                selectedElementIds.insert(currentId)
            }
            selectedElementIds.insert(elementId)
            selectedElementId = elementId
        }
    }

    /// Clear all selections
    func clearSelection() {
        selectedElementId = nil
        selectedElementIds.removeAll()
    }

    /// Legacy compatibility wrapper
    func handleTap(at point: CGPoint, in size: CGSize) {
        handleTap(at: point, in: size, modifiers: [])
    }

    // MARK: - Export Operations

    /// Export all pages to COCO format
    func exportCOCOAllPages() {
        guard let pipeline = pipeline, let url = pdfURL else {
            error = "No document loaded or pipeline unavailable"
            return
        }

        let panel = NSSavePanel()
        panel.allowedContentTypes = [.json]
        panel.nameFieldStringValue = url.deletingPathExtension().lastPathComponent + "_coco.json"
        panel.title = "Export COCO Dataset (All Pages)"

        if panel.runModal() == .OK, let saveURL = panel.url {
            Task {
                isProcessing = true
                processingProgress = "Exporting COCO dataset..."

                do {
                    let sourceFile = url.deletingPathExtension().lastPathComponent
                    let dataset = try pipeline.exportCOCO(
                        stage: currentStage,
                        sourceFile: sourceFile
                    )
                    let jsonData = try dataset.toJSONData()
                    try jsonData.write(to: saveURL)
                    processingProgress = "Exported to \(saveURL.lastPathComponent)"
                } catch {
                    self.error = "Export failed: \(error)"
                }

                isProcessing = false
            }
        }
    }

    /// Export current page to COCO format
    func exportCOCOCurrentPage() {
        guard let pipeline = pipeline, let url = pdfURL else {
            error = "No document loaded or pipeline unavailable"
            return
        }

        let panel = NSSavePanel()
        panel.allowedContentTypes = [.json]
        panel.nameFieldStringValue = url.deletingPathExtension().lastPathComponent + "_page\(currentPage + 1)_coco.json"
        panel.title = "Export COCO Dataset (Current Page)"

        if panel.runModal() == .OK, let saveURL = panel.url {
            Task {
                isProcessing = true
                processingProgress = "Exporting COCO dataset..."

                do {
                    let sourceFile = url.deletingPathExtension().lastPathComponent
                    let dataset = try pipeline.exportPageCOCO(
                        pageIndex: currentPage,
                        stage: currentStage,
                        sourceFile: sourceFile
                    )
                    let jsonData = try dataset.toJSONData()
                    try jsonData.write(to: saveURL)
                    processingProgress = "Exported to \(saveURL.lastPathComponent)"
                } catch {
                    self.error = "Export failed: \(error)"
                }

                isProcessing = false
            }
        }
    }

    /// Export current page to JSON format
    func exportJSON() {
        guard let pipeline = pipeline, let url = pdfURL else {
            error = "No document loaded or pipeline unavailable"
            return
        }

        let panel = NSSavePanel()
        panel.allowedContentTypes = [.json]
        panel.nameFieldStringValue = url.deletingPathExtension().lastPathComponent + "_page\(currentPage + 1).json"
        panel.title = "Export JSON"

        if panel.runModal() == .OK, let saveURL = panel.url {
            Task {
                isProcessing = true
                processingProgress = "Exporting JSON..."

                if let json = pipeline.exportJSON(pageIndex: currentPage) {
                    do {
                        try json.write(to: saveURL, atomically: true, encoding: .utf8)
                        processingProgress = "Exported to \(saveURL.lastPathComponent)"
                    } catch {
                        self.error = "Export failed: \(error)"
                    }
                } else {
                    self.error = "Failed to generate JSON"
                }

                isProcessing = false
            }
        }
    }

    /// Export COCO dataset with images to a folder
    func exportCOCOWithImages() {
        guard let pipeline = pipeline, let url = pdfURL else {
            error = "No document loaded or pipeline unavailable"
            return
        }

        let panel = NSOpenPanel()
        panel.canChooseFiles = false
        panel.canChooseDirectories = true
        panel.canCreateDirectories = true
        panel.allowsMultipleSelection = false
        panel.prompt = "Export"
        panel.message = "Choose a folder to export COCO dataset with images"
        panel.directoryURL = url.deletingLastPathComponent()

        if panel.runModal() == .OK, let folderURL = panel.url {
            // Create a subfolder with document name
            let datasetFolder = folderURL.appendingPathComponent(
                url.deletingPathExtension().lastPathComponent + "_coco"
            )

            Task {
                isProcessing = true
                let totalPages = pipeline.pageCount

                do {
                    try pipeline.exportCOCOWithImages(
                        to: datasetFolder,
                        stage: currentStage,
                        imageFormat: .png,
                        imageScale: 2.0
                    ) { [weak self] pageIndex, total in
                        Task { @MainActor in
                            self?.processingProgress = "Exporting page \(pageIndex + 1) of \(total)..."
                        }
                    }
                    processingProgress = "Exported \(totalPages) pages to \(datasetFolder.lastPathComponent)"
                } catch {
                    self.error = "Export failed: \(error)"
                }

                isProcessing = false
            }
        }
    }

    /// Export validation report as Markdown
    func exportValidationReportMarkdown() {
        guard let url = pdfURL else {
            error = "No document loaded"
            return
        }

        guard let snapshot = currentStageSnapshot else {
            error = "No stage data available"
            return
        }

        let panel = NSSavePanel()
        panel.allowedContentTypes = [UTType(filenameExtension: "md") ?? .plainText]
        panel.nameFieldStringValue = url.deletingPathExtension().lastPathComponent + "_validation_report.md"
        panel.title = "Export Validation Report (Markdown)"

        if panel.runModal() == .OK, let saveURL = panel.url {
            let report = generateValidationReportMarkdown(
                elements: snapshot.elements,
                sourceFile: url.lastPathComponent
            )
            do {
                try report.write(to: saveURL, atomically: true, encoding: .utf8)
                processingProgress = "Validation report exported to \(saveURL.lastPathComponent)"
            } catch {
                self.error = "Export failed: \(error)"
            }
        }
    }

    /// Export validation report as JSON
    func exportValidationReportJSON() {
        guard let url = pdfURL else {
            error = "No document loaded"
            return
        }

        guard let snapshot = currentStageSnapshot else {
            error = "No stage data available"
            return
        }

        let panel = NSSavePanel()
        panel.allowedContentTypes = [.json]
        panel.nameFieldStringValue = url.deletingPathExtension().lastPathComponent + "_validation_report.json"
        panel.title = "Export Validation Report (JSON)"

        if panel.runModal() == .OK, let saveURL = panel.url {
            let report = generateValidationReportJSON(
                elements: snapshot.elements,
                sourceFile: url.lastPathComponent
            )
            do {
                let jsonData = try JSONSerialization.data(withJSONObject: report, options: [.prettyPrinted, .sortedKeys])
                try jsonData.write(to: saveURL)
                processingProgress = "Validation report exported to \(saveURL.lastPathComponent)"
            } catch {
                self.error = "Export failed: \(error)"
            }
        }
    }

    /// Export all pages to YOLO format (one .txt file per page)
    func exportYOLO() {
        guard let pipeline = pipeline, let url = pdfURL else {
            error = "No document loaded or pipeline unavailable"
            return
        }

        let panel = NSOpenPanel()
        panel.canChooseFiles = false
        panel.canChooseDirectories = true
        panel.canCreateDirectories = true
        panel.allowsMultipleSelection = false
        panel.prompt = "Export"
        panel.message = "Choose a folder to export YOLO dataset"
        panel.directoryURL = url.deletingLastPathComponent()

        if panel.runModal() == .OK, let folderURL = panel.url {
            // Create a subfolder with document name
            let datasetFolder = folderURL.appendingPathComponent(
                url.deletingPathExtension().lastPathComponent + "_yolo"
            )

            Task {
                isProcessing = true
                processingProgress = "Exporting YOLO dataset..."

                do {
                    // Create folder if needed
                    try FileManager.default.createDirectory(at: datasetFolder, withIntermediateDirectories: true)

                    // Export classes.txt (label names file)
                    let classesPath = datasetFolder.appendingPathComponent("classes.txt")
                    let classNames = DocItemLabel.allCases.map { $0.cocoName }.joined(separator: "\n")
                    try classNames.write(to: classesPath, atomically: true, encoding: .utf8)

                    // Export annotation files for each page
                    let totalPages = pipeline.pageCount
                    var totalAnnotations = 0

                    for pageIndex in 0..<totalPages {
                        processingProgress = "Exporting page \(pageIndex + 1) of \(totalPages)..."

                        // Get elements for this page at current stage
                        guard let snapshot = pipeline.snapshot(at: currentStage, pageIndex: pageIndex) else {
                            continue
                        }

                        // Get page size from pipeline
                        let pageDims = pipeline.pageSize(at: pageIndex)
                        let pageSize = CGSize(
                            width: CGFloat(pageDims?.width ?? 612),
                            height: CGFloat(pageDims?.height ?? 792)
                        )

                        // Generate YOLO annotation lines
                        var yoloLines: [String] = []
                        for element in snapshot.elements {
                            // YOLO format: class_id x_center y_center width height (all normalized 0-1)
                            let classId = Int(element.label.rawValue)  // 0-indexed
                            let xCenter = (Double(element.bbox.x) + Double(element.bbox.width) / 2.0) / pageSize.width
                            let yCenter = (Double(element.bbox.y) + Double(element.bbox.height) / 2.0) / pageSize.height
                            let normalizedWidth = Double(element.bbox.width) / pageSize.width
                            let normalizedHeight = Double(element.bbox.height) / pageSize.height

                            // Clamp values to [0, 1] range
                            let cx = max(0, min(1, xCenter))
                            let cy = max(0, min(1, yCenter))
                            let w = max(0, min(1, normalizedWidth))
                            let h = max(0, min(1, normalizedHeight))

                            let line = String(format: "%d %.6f %.6f %.6f %.6f", classId, cx, cy, w, h)
                            yoloLines.append(line)
                        }

                        totalAnnotations += yoloLines.count

                        // Write annotation file (page_001.txt, etc.)
                        let filename = String(format: "page_%03d.txt", pageIndex + 1)
                        let annotationPath = datasetFolder.appendingPathComponent(filename)
                        let annotationContent = yoloLines.joined(separator: "\n")
                        try annotationContent.write(to: annotationPath, atomically: true, encoding: .utf8)
                    }

                    processingProgress = "Exported \(totalAnnotations) annotations across \(totalPages) pages to \(datasetFolder.lastPathComponent)"
                } catch {
                    self.error = "Export failed: \(error)"
                }

                isProcessing = false
            }
        }
    }

    // MARK: - Import Operations

    /// Import COCO annotations from JSON file
    func importCOCOAnnotations() {
        let panel = NSOpenPanel()
        panel.allowedContentTypes = [.json]
        panel.allowsMultipleSelection = false
        panel.canChooseDirectories = false
        panel.title = "Import COCO Annotations"
        panel.message = "Select a COCO format JSON file to import annotations"

        if panel.runModal() == .OK, let fileURL = panel.url {
            Task {
                isProcessing = true
                processingProgress = "Importing COCO annotations..."

                do {
                    let importedCount = try await importCOCOFromFile(fileURL)
                    processingProgress = "Imported \(importedCount) annotations"
                } catch {
                    self.error = "Import failed: \(error.localizedDescription)"
                }

                isProcessing = false
            }
        }
    }

    /// Parse COCO JSON file and import annotations
    private func importCOCOFromFile(_ url: URL) async throws -> Int {
        let data = try Data(contentsOf: url)
        guard let json = try JSONSerialization.jsonObject(with: data) as? [String: Any] else {
            throw ImportError.invalidFormat("Could not parse JSON")
        }

        // Extract categories for label mapping
        guard let categories = json["categories"] as? [[String: Any]] else {
            throw ImportError.invalidFormat("Missing 'categories' array")
        }

        // Build category ID to label mapping
        var categoryMap: [Int: DocItemLabel] = [:]
        for category in categories {
            guard let id = category["id"] as? Int,
                  let name = category["name"] as? String else { continue }
            categoryMap[id] = cocoLabelToDocItemLabel(name)
        }

        // Extract annotations
        guard let annotations = json["annotations"] as? [[String: Any]] else {
            throw ImportError.invalidFormat("Missing 'annotations' array")
        }

        // Extract images for page/dimension mapping (optional)
        let images = json["images"] as? [[String: Any]] ?? []
        var imagePageMap: [Int: Int] = [:]  // image_id -> page index
        var imageDimensions: [Int: (width: Float, height: Float)] = [:]  // image_id -> (w, h)

        for (index, image) in images.enumerated() {
            guard let imageId = image["id"] as? Int else { continue }
            imagePageMap[imageId] = index
            if let width = image["width"] as? Int,
               let height = image["height"] as? Int {
                imageDimensions[imageId] = (Float(width), Float(height))
            }
        }

        // Clear existing edits if importing fresh
        if !annotations.isEmpty {
            discardEdits()
        }

        // Import annotations as edited elements
        var importedCount = 0
        for annotation in annotations {
            guard let annotationId = annotation["id"] as? Int,
                  let categoryId = annotation["category_id"] as? Int,
                  let bbox = annotation["bbox"] as? [Double],
                  bbox.count >= 4 else { continue }

            // COCO bbox format: [x, y, width, height] (origin at top-left)
            // We need to convert to our format (origin at bottom-left in PDF coordinates)
            let x = Float(bbox[0])
            let y = Float(bbox[1])
            let width = Float(bbox[2])
            let height = Float(bbox[3])

            // Determine page height for coordinate conversion
            let imageId = annotation["image_id"] as? Int ?? 0
            // Note: pageIdx could be used in future to navigate to specific pages
            _ = imagePageMap[imageId] ?? currentPage
            let dims = imageDimensions[imageId] ?? (Float(pageSize.width), Float(pageSize.height))
            let pageHeight = dims.height

            // Convert from top-left origin to bottom-left origin
            let convertedY = pageHeight - y - height

            let label = categoryMap[categoryId] ?? .text
            let confidence = Float((annotation["score"] as? Double) ?? 1.0)

            // Create element with unique ID
            let elementId = UInt32(2_000_000 + annotationId)
            let boundingBox = BoundingBox(x: x, y: convertedY, width: width, height: height)
            let element = Element(
                id: elementId,
                bbox: boundingBox,
                label: label,
                confidence: confidence
            )

            // Add to edited elements (overrides snapshot)
            editedElements[elementId] = element
            importedCount += 1
        }

        return importedCount
    }

    /// Convert COCO category name to DocItemLabel
    private func cocoLabelToDocItemLabel(_ name: String) -> DocItemLabel {
        let lowercased = name.lowercased()
        switch lowercased {
        case "text", "paragraph": return .text
        case "title", "document_title": return .title
        case "section_header", "section-header", "sectionheader", "heading": return .sectionHeader
        case "page_header", "page-header", "pageheader", "header": return .pageHeader
        case "page_footer", "page-footer", "pagefooter", "footer": return .pageFooter
        case "caption", "figure_caption": return .caption
        case "footnote": return .footnote
        case "table": return .table
        case "picture", "image", "figure": return .picture
        case "list_item", "list-item", "listitem": return .listItem
        case "code", "code_block": return .code
        case "formula", "equation", "math": return .formula
        case "checkbox_selected": return .checkboxSelected
        case "checkbox_unselected": return .checkboxUnselected
        case "form": return .form
        case "key_value_region": return .keyValueRegion
        case "document_index": return .documentIndex
        default: return .text  // Default to text for unknown labels
        }
    }

    /// Import error types
    enum ImportError: LocalizedError {
        case invalidFormat(String)
        case missingData(String)

        var errorDescription: String? {
            switch self {
            case .invalidFormat(let msg): return "Invalid format: \(msg)"
            case .missingData(let msg): return "Missing data: \(msg)"
            }
        }
    }

    /// Generate markdown validation report
    private func generateValidationReportMarkdown(elements: [Element], sourceFile: String) -> String {
        var lines: [String] = []
        let issues = computeValidationIssues(elements: elements)

        // Header
        lines.append("# Validation Report")
        lines.append("")
        lines.append("**Source:** \(sourceFile)")
        lines.append("**Page:** \(currentPage + 1) of \(pageCount)")
        lines.append("**Stage:** \(currentStage.description)")
        lines.append("**Generated:** \(ISO8601DateFormatter().string(from: Date()))")
        lines.append("")

        // Summary
        lines.append("## Summary")
        lines.append("")
        lines.append("| Metric | Value |")
        lines.append("|--------|-------|")
        lines.append("| Total Elements | \(elements.count) |")
        lines.append("| Total Issues | \(issues.count) |")

        // Count issues by type
        var typeCounts: [String: Int] = [:]
        for issue in issues {
            let typeName = issue.typeName
            typeCounts[typeName, default: 0] += 1
        }
        for (typeName, count) in typeCounts.sorted(by: { $0.value > $1.value }) {
            lines.append("| \(typeName) | \(count) |")
        }
        lines.append("")

        // Details
        if issues.isEmpty {
            lines.append("## Status")
            lines.append("")
            lines.append("No validation issues found.")
        } else {
            lines.append("## Issues")
            lines.append("")
            for (index, issue) in issues.enumerated() {
                lines.append("\(index + 1). **\(issue.typeName)**: \(issue.description)")
            }
        }
        lines.append("")

        // Element Statistics
        lines.append("## Element Statistics")
        lines.append("")
        var labelCounts: [String: Int] = [:]
        var totalConfidence: Float = 0
        for element in elements {
            labelCounts[element.label.description, default: 0] += 1
            totalConfidence += element.confidence
        }
        let avgConfidence = elements.isEmpty ? 0 : totalConfidence / Float(elements.count)

        lines.append("| Label | Count |")
        lines.append("|-------|-------|")
        for (label, count) in labelCounts.sorted(by: { $0.value > $1.value }) {
            lines.append("| \(label) | \(count) |")
        }
        lines.append("")
        lines.append("**Average Confidence:** \(String(format: "%.1f%%", avgConfidence * 100))")
        lines.append("")

        return lines.joined(separator: "\n")
    }

    /// Generate JSON validation report
    private func generateValidationReportJSON(elements: [Element], sourceFile: String) -> [String: Any] {
        let issues = computeValidationIssues(elements: elements)

        // Issue details
        let issueDetails: [[String: Any]] = issues.map { issue in
            [
                "type": issue.typeName,
                "description": issue.description,
                "element_ids": issue.elementIds.map { Int($0) }
            ]
        }

        // Label counts
        var labelCounts: [String: Int] = [:]
        var totalConfidence: Float = 0
        for element in elements {
            labelCounts[element.label.description, default: 0] += 1
            totalConfidence += element.confidence
        }
        let avgConfidence = elements.isEmpty ? 0 : totalConfidence / Float(elements.count)

        return [
            "source_file": sourceFile,
            "page": currentPage + 1,
            "total_pages": pageCount,
            "stage": currentStage.description,
            "generated_at": ISO8601DateFormatter().string(from: Date()),
            "summary": [
                "total_elements": elements.count,
                "total_issues": issues.count,
                "average_confidence": Double(avgConfidence),
                "label_counts": labelCounts
            ],
            "issues": issueDetails
        ]
    }

    /// Compute validation issues for elements
    private func computeValidationIssues(elements: [Element]) -> [ValidationIssue] {
        var result: [ValidationIssue] = []
        let pageSize = self.pageSize

        for element in elements {
            // Tiny boxes
            if element.bbox.width < 10 || element.bbox.height < 10 {
                result.append(.tinyBox(elementId: element.id))
            }
            // Out of bounds
            if element.bbox.x < 0 || element.bbox.y < 0 ||
               element.bbox.x + element.bbox.width > pageSize.width ||
               element.bbox.y + element.bbox.height > pageSize.height {
                result.append(.outOfBounds(elementId: element.id))
            }
            // Low confidence
            if element.confidence < 0.5 {
                result.append(.lowConfidence(elementId: element.id))
            }
            // Extreme aspect ratio
            let aspectRatio = element.bbox.width / max(element.bbox.height, 0.001)
            if aspectRatio < 0.05 || aspectRatio > 20 {
                result.append(.extremeAspectRatio(elementId: element.id))
            }
        }

        // Overlaps (limited to first 100 elements)
        let checkElements = Array(elements.prefix(100))
        for i in 0..<checkElements.count {
            for j in (i+1)..<checkElements.count {
                let b1 = checkElements[i].bbox
                let b2 = checkElements[j].bbox
                let x1 = max(b1.x, b2.x)
                let y1 = max(b1.y, b2.y)
                let x2 = min(b1.x + b1.width, b2.x + b2.width)
                let y2 = min(b1.y + b1.height, b2.y + b2.height)
                if x1 < x2 && y1 < y2 {
                    let overlapArea = (x2 - x1) * (y2 - y1)
                    let area1 = b1.width * b1.height
                    let area2 = b2.width * b2.height
                    if area1 > 0 && area2 > 0 {
                        let ratio = max(overlapArea / area1, overlapArea / area2)
                        if ratio > 0.5 {
                            result.append(.significantOverlap(element1: checkElements[i].id, element2: checkElements[j].id))
                        }
                    }
                }
            }
        }

        // Reading order validation
        let withOrder = elements.filter { $0.hasReadingOrder }
        let withoutOrder = elements.filter { !$0.hasReadingOrder }

        if !withOrder.isEmpty && !withoutOrder.isEmpty {
            for element in withoutOrder {
                result.append(.missingReadingOrder(elementId: element.id))
            }
        }

        var orderMap: [Int32: [UInt32]] = [:]
        for element in withOrder {
            orderMap[element.readingOrder, default: []].append(element.id)
        }
        for (order, elementIds) in orderMap where elementIds.count > 1 {
            result.append(.duplicateReadingOrder(order: order, elements: elementIds))
        }

        if !withOrder.isEmpty {
            let orders = Set(withOrder.map { $0.readingOrder })
            if let minOrder = orders.min(), let maxOrder = orders.max() {
                for expected in minOrder...maxOrder {
                    if !orders.contains(expected) {
                        result.append(.readingOrderGap(expectedOrder: expected))
                    }
                }
            }
        }

        return result
    }

    // MARK: - Editing Operations

    /// Start dragging a bounding box handle
    func startDrag(at point: CGPoint, in size: CGSize) {
        guard let element = selectedElement else { return }

        // Check if element is locked
        guard !isElementLocked(element.id) else {
            error = "Element is locked"
            return
        }

        let pageSize = self.pageSize
        let scale = min(
            size.width / CGFloat(pageSize.width),
            size.height / CGFloat(pageSize.height)
        )

        // Check if we hit a resize handle
        let handles = resizeHandleRects(for: element.bbox, scale: CGFloat(scale), pageHeight: pageSize.height)

        for (handle, rect) in handles {
            if rect.contains(point) {
                activeDragHandle = handle
                isDragging = true
                dragStartPoint = point
                dragStartBbox = element.bbox
                isEditing = true
                return
            }
        }

        // Check if we hit the element itself (for moving)
        let elementRect = scaledRect(for: element.bbox, scale: CGFloat(scale), pageHeight: pageSize.height)
        if elementRect.contains(point) {
            activeDragHandle = .center
            isDragging = true
            dragStartPoint = point
            dragStartBbox = element.bbox
            isEditing = true
        }
    }

    /// Update drag position
    func updateDrag(at point: CGPoint, in size: CGSize) {
        guard isDragging,
              let handle = activeDragHandle,
              let startPoint = dragStartPoint,
              let startBbox = dragStartBbox,
              let elementId = selectedElementId else { return }

        let pageSize = self.pageSize
        let scale = min(
            size.width / CGFloat(pageSize.width),
            size.height / CGFloat(pageSize.height)
        )

        // Calculate delta in PDF coordinates
        let deltaX = Float((point.x - startPoint.x) / CGFloat(scale))
        let deltaY = Float(-(point.y - startPoint.y) / CGFloat(scale)) // Flip Y

        // Apply the delta based on handle type
        var newBbox = startBbox

        switch handle {
        case .center:
            // Move the entire box
            newBbox.x = startBbox.x + deltaX
            newBbox.y = startBbox.y + deltaY

        case .topLeft:
            newBbox.x = startBbox.x + deltaX
            newBbox.width = startBbox.width - deltaX
            newBbox.height = startBbox.height + deltaY

        case .topCenter:
            newBbox.height = startBbox.height + deltaY

        case .topRight:
            newBbox.width = startBbox.width + deltaX
            newBbox.height = startBbox.height + deltaY

        case .middleLeft:
            newBbox.x = startBbox.x + deltaX
            newBbox.width = startBbox.width - deltaX

        case .middleRight:
            newBbox.width = startBbox.width + deltaX

        case .bottomLeft:
            newBbox.x = startBbox.x + deltaX
            newBbox.y = startBbox.y + deltaY
            newBbox.width = startBbox.width - deltaX
            newBbox.height = startBbox.height - deltaY

        case .bottomCenter:
            newBbox.y = startBbox.y + deltaY
            newBbox.height = startBbox.height - deltaY

        case .bottomRight:
            newBbox.y = startBbox.y + deltaY
            newBbox.width = startBbox.width + deltaX
            newBbox.height = startBbox.height - deltaY
        }

        // Enforce minimum size
        let minSize: Float = 10.0
        if newBbox.width < minSize { newBbox.width = minSize }
        if newBbox.height < minSize { newBbox.height = minSize }

        // Apply snap-to-grid if enabled
        newBbox = snapBboxToGrid(newBbox)

        // Update the element
        if var element = editedElements[elementId] ?? currentStageSnapshot?.elements.first(where: { $0.id == elementId }) {
            element.bbox = newBbox
            editedElements[elementId] = element

            // Compute alignment guides
            currentAlignmentGuides = alignmentGuides(for: element)
        }
    }

    /// End dragging and commit the edit
    func endDrag() {
        guard isDragging,
              let elementId = selectedElementId,
              let startBbox = dragStartBbox,
              let element = editedElements[elementId] else {
            resetDragState()
            return
        }

        // Create edit action for undo
        let edit = BBoxEdit(elementId: elementId, originalBbox: startBbox, newBbox: element.bbox)
        pushUndo(.bbox(edit))

        resetDragState()
    }

    /// Cancel the current drag without saving
    func cancelDrag() {
        if let elementId = selectedElementId, let startBbox = dragStartBbox {
            // Restore original bbox
            if var element = editedElements[elementId] {
                element.bbox = startBbox
                editedElements[elementId] = element
            } else {
                editedElements.removeValue(forKey: elementId)
            }
        }
        resetDragState()
    }

    private func resetDragState() {
        isDragging = false
        activeDragHandle = nil
        dragStartPoint = nil
        dragStartBbox = nil
        currentAlignmentGuides = AlignmentGuides()  // Clear alignment guides
        currentSnapIndicators.clear()  // Clear snap indicators
    }

    /// Get resize handle rectangles for an element
    func resizeHandleRects(for bbox: BoundingBox, scale: CGFloat, pageHeight: Float) -> [(DragHandle, CGRect)] {
        let handleSize: CGFloat = 8.0
        let halfHandle = handleSize / 2

        // Convert bbox to screen coordinates
        let rect = scaledRect(for: bbox, scale: scale, pageHeight: pageHeight)

        var handles: [(DragHandle, CGRect)] = []

        // Corner handles
        handles.append((.topLeft, CGRect(x: rect.minX - halfHandle, y: rect.minY - halfHandle, width: handleSize, height: handleSize)))
        handles.append((.topRight, CGRect(x: rect.maxX - halfHandle, y: rect.minY - halfHandle, width: handleSize, height: handleSize)))
        handles.append((.bottomLeft, CGRect(x: rect.minX - halfHandle, y: rect.maxY - halfHandle, width: handleSize, height: handleSize)))
        handles.append((.bottomRight, CGRect(x: rect.maxX - halfHandle, y: rect.maxY - halfHandle, width: handleSize, height: handleSize)))

        // Edge handles
        handles.append((.topCenter, CGRect(x: rect.midX - halfHandle, y: rect.minY - halfHandle, width: handleSize, height: handleSize)))
        handles.append((.bottomCenter, CGRect(x: rect.midX - halfHandle, y: rect.maxY - halfHandle, width: handleSize, height: handleSize)))
        handles.append((.middleLeft, CGRect(x: rect.minX - halfHandle, y: rect.midY - halfHandle, width: handleSize, height: handleSize)))
        handles.append((.middleRight, CGRect(x: rect.maxX - halfHandle, y: rect.midY - halfHandle, width: handleSize, height: handleSize)))

        return handles
    }

    /// Convert bbox to screen rect
    func scaledRect(for bbox: BoundingBox, scale: CGFloat, pageHeight: Float) -> CGRect {
        let x = CGFloat(bbox.x) * scale
        let y = (CGFloat(pageHeight) - CGFloat(bbox.y) - CGFloat(bbox.height)) * scale
        let width = CGFloat(bbox.width) * scale
        let height = CGFloat(bbox.height) * scale
        return CGRect(x: x, y: y, width: width, height: height)
    }

    // MARK: - Undo/Redo

    /// Push an action onto the undo stack
    private func pushUndo(_ action: EditAction) {
        undoStack.append(action)
        redoStack.removeAll() // Clear redo stack on new action
    }

    /// Undo the last edit
    func undo() {
        guard let action = undoStack.popLast() else { return }

        switch action {
        case .bbox(let edit):
            // Restore original bbox
            if var element = editedElements[edit.elementId] ?? currentStageSnapshot?.elements.first(where: { $0.id == edit.elementId }) {
                // Store current state for redo
                let redoEdit = BBoxEdit(elementId: edit.elementId, originalBbox: edit.newBbox, newBbox: edit.originalBbox)
                redoStack.append(.bbox(redoEdit))

                // Apply undo
                element.bbox = edit.originalBbox
                editedElements[edit.elementId] = element
            }

        case .label(let edit):
            // Restore original label
            if var element = editedElements[edit.elementId] ?? currentStageSnapshot?.elements.first(where: { $0.id == edit.elementId }) {
                let redoEdit = LabelEdit(elementId: edit.elementId, originalLabel: edit.newLabel, newLabel: edit.originalLabel)
                redoStack.append(.label(redoEdit))

                element.label = edit.originalLabel
                editedElements[edit.elementId] = element
            }

        case .delete(let element):
            // Restore deleted element
            editedElements[element.id] = element
            redoStack.append(.delete(element))

        case .create(let element):
            // Remove created element
            editedElements.removeValue(forKey: element.id)
            redoStack.append(.create(element))

        case .merge(let edit):
            // Undo merge: remove merged element, restore originals
            editedElements.removeValue(forKey: edit.mergedElementId)
            deletedElementIds.insert(edit.mergedElementId)  // Mark merged as deleted too

            // Restore original elements
            for element in edit.originalElements {
                deletedElementIds.remove(element.id)
                // Only add to editedElements if it was edited before merge
                // (snapshot elements will reappear automatically)
            }

            redoStack.append(.merge(edit))

        case .split(let edit):
            // Undo split: remove split elements, restore original
            for resultId in edit.resultElementIds {
                editedElements.removeValue(forKey: resultId)
                deletedElementIds.insert(resultId)  // Mark split elements as deleted
            }

            // Restore original element
            deletedElementIds.remove(edit.originalElementId)
            // If the original was edited, restore it
            editedElements[edit.originalElementId] = edit.originalElement

            redoStack.append(.split(edit))
        }
    }

    /// Redo the last undone edit
    func redo() {
        guard let action = redoStack.popLast() else { return }

        switch action {
        case .bbox(let edit):
            if var element = editedElements[edit.elementId] ?? currentStageSnapshot?.elements.first(where: { $0.id == edit.elementId }) {
                let undoEdit = BBoxEdit(elementId: edit.elementId, originalBbox: edit.newBbox, newBbox: edit.originalBbox)
                undoStack.append(.bbox(undoEdit))

                element.bbox = edit.originalBbox
                editedElements[edit.elementId] = element
            }

        case .label(let edit):
            if var element = editedElements[edit.elementId] ?? currentStageSnapshot?.elements.first(where: { $0.id == edit.elementId }) {
                let undoEdit = LabelEdit(elementId: edit.elementId, originalLabel: edit.newLabel, newLabel: edit.originalLabel)
                undoStack.append(.label(undoEdit))

                element.label = edit.originalLabel
                editedElements[edit.elementId] = element
            }

        case .delete(let element):
            editedElements.removeValue(forKey: element.id)
            undoStack.append(.delete(element))

        case .create(let element):
            editedElements[element.id] = element
            undoStack.append(.create(element))

        case .merge(let edit):
            // Redo merge: recreate merged element, delete originals
            // First, recreate the merged element by computing union bbox again
            var minX = Float.infinity
            var minY = Float.infinity
            var maxX = -Float.infinity
            var maxY = -Float.infinity

            for element in edit.originalElements {
                minX = min(minX, element.bbox.x)
                minY = min(minY, element.bbox.y)
                maxX = max(maxX, element.bbox.x + element.bbox.width)
                maxY = max(maxY, element.bbox.y + element.bbox.height)
            }

            let mostCommonLabel = edit.originalElements.first?.label ?? .text
            let avgConfidence = edit.originalElements.map { $0.confidence }.reduce(0, +) / Float(edit.originalElements.count)

            let mergedElement = Element(
                id: edit.mergedElementId,
                bbox: BoundingBox(
                    x: minX,
                    y: minY,
                    width: maxX - minX,
                    height: maxY - minY
                ),
                label: mostCommonLabel,
                confidence: avgConfidence
            )

            // Add merged element back
            editedElements[edit.mergedElementId] = mergedElement
            deletedElementIds.remove(edit.mergedElementId)

            // Delete originals again
            for id in edit.originalElementIds {
                deletedElementIds.insert(id)
            }

            undoStack.append(.merge(edit))

        case .split(let edit):
            // Redo split: recreate split elements from original
            let bbox = edit.originalElement.bbox

            // Recreate split elements (same logic as splitSelectedElement)
            // We use the stored IDs and create new bboxes at center split
            // Note: This is a simplified redo - it recreates at 50% position
            let bbox1: BoundingBox
            let bbox2: BoundingBox

            // Assume horizontal split for redo (since we don't store direction)
            // Actually, we should determine from the stored elements
            // For now, just split horizontally at center
            let splitY = bbox.y + bbox.height * 0.5
            bbox1 = BoundingBox(
                x: bbox.x,
                y: bbox.y,
                width: bbox.width,
                height: splitY - bbox.y
            )
            bbox2 = BoundingBox(
                x: bbox.x,
                y: splitY,
                width: bbox.width,
                height: bbox.height - (splitY - bbox.y)
            )

            // Recreate the split elements
            let element1 = Element(
                id: edit.resultElementIds[0],
                bbox: bbox1,
                label: edit.originalElement.label,
                confidence: edit.originalElement.confidence
            )
            let element2 = Element(
                id: edit.resultElementIds[1],
                bbox: bbox2,
                label: edit.originalElement.label,
                confidence: edit.originalElement.confidence
            )

            // Add split elements back
            editedElements[element1.id] = element1
            editedElements[element2.id] = element2
            deletedElementIds.remove(element1.id)
            deletedElementIds.remove(element2.id)

            // Delete original again
            deletedElementIds.insert(edit.originalElementId)
            editedElements.removeValue(forKey: edit.originalElementId)

            undoStack.append(.split(edit))
        }
    }

    /// Check if there are unsaved edits
    var hasUnsavedEdits: Bool {
        !editedElements.isEmpty
    }

    /// Clear all edits and reset to original state
    func discardEdits() {
        editedElements.removeAll()
        deletedElementIds.removeAll()
        undoStack.removeAll()
        redoStack.removeAll()
        clearSelection()
        isEditing = false
    }

    // MARK: - Label Editing

    /// Change the label of an element
    func changeLabel(elementId: UInt32, to newLabel: DocItemLabel) {
        // Check if element is locked
        guard !isElementLocked(elementId) else {
            error = "Element is locked"
            return
        }

        guard let element = editedElements[elementId] ?? currentStageSnapshot?.elements.first(where: { $0.id == elementId }) else {
            return
        }

        let oldLabel = element.label
        guard oldLabel != newLabel else { return }

        // Create the edit
        var updatedElement = element
        updatedElement.label = newLabel
        editedElements[elementId] = updatedElement

        // Push to undo stack
        let edit = LabelEdit(elementId: elementId, originalLabel: oldLabel, newLabel: newLabel)
        pushUndo(.label(edit))

        isEditing = true
    }

    /// Change the label of all selected elements (batch operation)
    func changeSelectedLabels(to newLabel: DocItemLabel) {
        let elements = selectedElements
        guard !elements.isEmpty else { return }

        var changedCount = 0
        var skippedLocked = 0
        for element in elements {
            // Skip locked elements
            if isElementLocked(element.id) {
                skippedLocked += 1
                continue
            }

            let oldLabel = element.label
            guard oldLabel != newLabel else { continue }

            // Create the edit
            var updatedElement = element
            updatedElement.label = newLabel
            editedElements[element.id] = updatedElement

            // Push to undo stack (each label change is separately undoable)
            let edit = LabelEdit(elementId: element.id, originalLabel: oldLabel, newLabel: newLabel)
            pushUndo(.label(edit))

            changedCount += 1
        }

        if changedCount > 0 {
            isEditing = true
            var message = "Changed label of \(changedCount) element(s) to \(newLabel.description)"
            if skippedLocked > 0 {
                message += " (skipped \(skippedLocked) locked)"
            }
            processingProgress = message
        } else if skippedLocked > 0 {
            error = "All selected elements are locked"
        }
    }

    // MARK: - Save/Load Corrections

    /// Corrections file format
    struct CorrectionsFile: Codable {
        let version: Int
        let pdfPath: String?
        let pageIndex: Int
        let stage: Int32
        let timestamp: Date
        let corrections: [ElementCorrection]

        struct ElementCorrection: Codable {
            let elementId: UInt32
            let originalBbox: CodableBBox?
            let newBbox: CodableBBox
            let originalLabel: Int32?
            let newLabel: Int32
        }

        struct CodableBBox: Codable {
            let x: Float
            let y: Float
            let width: Float
            let height: Float

            init(from bbox: BoundingBox) {
                self.x = bbox.x
                self.y = bbox.y
                self.width = bbox.width
                self.height = bbox.height
            }

            func toBoundingBox() -> BoundingBox {
                BoundingBox(x: x, y: y, width: width, height: height)
            }
        }
    }

    /// Save current corrections to a JSON file
    func saveCorrections() {
        guard !editedElements.isEmpty else {
            error = "No corrections to save"
            return
        }

        let panel = NSSavePanel()
        panel.allowedContentTypes = [.json]
        panel.nameFieldStringValue = (pdfURL?.deletingPathExtension().lastPathComponent ?? "document") + "_corrections.json"
        panel.title = "Save Corrections"

        if panel.runModal() == .OK, let saveURL = panel.url {
            do {
                let correctionsFile = buildCorrectionsFile()
                let encoder = JSONEncoder()
                encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
                encoder.dateEncodingStrategy = .iso8601
                let data = try encoder.encode(correctionsFile)
                try data.write(to: saveURL)
                processingProgress = "Saved \(editedElements.count) correction(s) to \(saveURL.lastPathComponent)"
            } catch {
                self.error = "Failed to save corrections: \(error)"
            }
        }
    }

    /// Load corrections from a JSON file
    func loadCorrections() {
        let panel = NSOpenPanel()
        panel.allowedContentTypes = [.json]
        panel.allowsMultipleSelection = false
        panel.title = "Load Corrections"

        if panel.runModal() == .OK, let url = panel.url {
            do {
                let data = try Data(contentsOf: url)
                let decoder = JSONDecoder()
                decoder.dateDecodingStrategy = .iso8601
                let correctionsFile = try decoder.decode(CorrectionsFile.self, from: data)

                // Validate version
                guard correctionsFile.version == 1 else {
                    error = "Unsupported corrections file version: \(correctionsFile.version)"
                    return
                }

                // Apply corrections
                applyCorrections(from: correctionsFile)
                processingProgress = "Loaded \(correctionsFile.corrections.count) correction(s) from \(url.lastPathComponent)"
            } catch {
                self.error = "Failed to load corrections: \(error)"
            }
        }
    }

    /// Build corrections file from current edits
    private func buildCorrectionsFile() -> CorrectionsFile {
        var corrections: [CorrectionsFile.ElementCorrection] = []

        for (elementId, editedElement) in editedElements {
            // Find original element to get original values
            let originalElement = currentStageSnapshot?.elements.first { $0.id == elementId }

            let correction = CorrectionsFile.ElementCorrection(
                elementId: elementId,
                originalBbox: originalElement.map { CorrectionsFile.CodableBBox(from: $0.bbox) },
                newBbox: CorrectionsFile.CodableBBox(from: editedElement.bbox),
                originalLabel: originalElement?.label.rawValue,
                newLabel: editedElement.label.rawValue
            )
            corrections.append(correction)
        }

        return CorrectionsFile(
            version: 1,
            pdfPath: pdfURL?.path,
            pageIndex: currentPage,
            stage: currentStage.rawValue,
            timestamp: Date(),
            corrections: corrections
        )
    }

    /// Apply corrections from a loaded file
    private func applyCorrections(from file: CorrectionsFile) {
        // Clear existing edits
        editedElements.removeAll()
        undoStack.removeAll()
        redoStack.removeAll()

        for correction in file.corrections {
            // Try to find the element in current snapshot
            guard var element = currentStageSnapshot?.elements.first(where: { $0.id == correction.elementId }) else {
                // Element not found, create a placeholder
                let label = DocItemLabel(rawValue: correction.newLabel) ?? .text
                let element = Element(
                    id: correction.elementId,
                    bbox: correction.newBbox.toBoundingBox(),
                    label: label,
                    confidence: 1.0
                )
                editedElements[correction.elementId] = element
                continue
            }

            // Apply corrections
            element.bbox = correction.newBbox.toBoundingBox()
            if let newLabel = DocItemLabel(rawValue: correction.newLabel) {
                element.label = newLabel
            }
            editedElements[correction.elementId] = element
        }

        isEditing = !editedElements.isEmpty
    }

    /// Check if there are unsaved corrections for the current page
    var hasCorrectionsForCurrentPage: Bool {
        !editedElements.isEmpty
    }

    /// Delete the selected element(s)
    /// Supports both single and multi-selection
    func deleteSelectedElement() {
        let elements = selectedElements
        guard !elements.isEmpty else { return }

        var deletedCount = 0
        var skippedLocked = 0
        for element in elements {
            // Skip locked elements
            if isElementLocked(element.id) {
                skippedLocked += 1
                continue
            }

            // Store for undo
            pushUndo(.delete(element))

            // Mark as deleted using deletedElementIds set
            deletedElementIds.insert(element.id)
            deletedCount += 1
        }

        // Clear selection
        clearSelection()

        if deletedCount > 0 {
            isEditing = true
            var message = "Deleted \(deletedCount) element(s)"
            if skippedLocked > 0 {
                message += " (skipped \(skippedLocked) locked)"
            }
            processingProgress = message
        } else if skippedLocked > 0 {
            error = "All selected elements are locked"
        }
    }

    // MARK: - Selection Operations

    /// Deselect all elements and cancel any ongoing operations
    func deselect() {
        clearSelection()
        if isDragging {
            cancelDrag()
        }
        if isDrawing {
            resetDrawingState()
        }
        // Exit draw mode when pressing escape
        if editTool == .draw {
            editTool = .select
        }
    }

    /// Select all visible elements on the current page
    func selectAll() {
        guard let snapshot = currentStageSnapshot, !snapshot.elements.isEmpty else { return }

        // Select all filtered (visible) elements
        let visibleElements = filteredElements
        guard !visibleElements.isEmpty else { return }

        selectedElementIds = Set(visibleElements.map { $0.id })
        selectedElementId = visibleElements.first?.id
    }

    /// Cycle to next element (Tab key behavior)
    func selectNextElement() {
        guard let snapshot = currentStageSnapshot, !snapshot.elements.isEmpty else { return }

        let elements = filteredElements
        guard !elements.isEmpty else { return }

        if let currentId = selectedElementId,
           let currentIndex = elements.firstIndex(where: { $0.id == currentId }) {
            let nextIndex = (currentIndex + 1) % elements.count
            selectSingleElement(elements[nextIndex].id)
        } else {
            selectSingleElement(elements[0].id)
        }
    }

    /// Cycle to previous element (Shift+Tab key behavior)
    func selectPreviousElement() {
        guard let snapshot = currentStageSnapshot, !snapshot.elements.isEmpty else { return }

        let elements = filteredElements
        guard !elements.isEmpty else { return }

        if let currentId = selectedElementId,
           let currentIndex = elements.firstIndex(where: { $0.id == currentId }) {
            let prevIndex = currentIndex > 0 ? currentIndex - 1 : elements.count - 1
            selectSingleElement(elements[prevIndex].id)
        } else {
            selectSingleElement(elements.last!.id)
        }
    }

    /// Duplicate the selected element with a slight offset
    func duplicateSelectedElement() {
        guard let element = selectedElement else { return }

        // Create a copy with a new ID and offset position
        var duplicatedElement = element
        duplicatedElement = Element(
            id: nextElementId,
            bbox: BoundingBox(
                x: element.bbox.x + 10,  // Offset by 10 points
                y: element.bbox.y - 10,
                width: element.bbox.width,
                height: element.bbox.height
            ),
            label: element.label,
            confidence: 1.0  // User-created elements have 100% confidence
        )

        nextElementId += 1

        // Add to edited elements
        editedElements[duplicatedElement.id] = duplicatedElement

        // Push to undo stack
        pushUndo(.create(duplicatedElement))

        // Select the new element
        selectedElementId = duplicatedElement.id

        isEditing = true
    }

    /// Nudge the selected element by a small amount
    /// - Parameters:
    ///   - dx: Horizontal offset in points
    ///   - dy: Vertical offset in points
    func nudgeSelectedElement(dx: Float, dy: Float) {
        guard let elementId = selectedElementId,
              var element = editedElements[elementId] ?? currentStageSnapshot?.elements.first(where: { $0.id == elementId }) else {
            return
        }

        // Check if element is locked
        guard !isElementLocked(elementId) else {
            error = "Element is locked"
            return
        }

        let nudgeAmount: Float = 2.0  // 2 points per nudge
        let originalBbox = element.bbox

        // Apply the nudge
        element.bbox.x += dx * nudgeAmount
        element.bbox.y += dy * nudgeAmount

        // Store the edited element
        editedElements[elementId] = element

        // Push to undo stack
        let edit = BBoxEdit(elementId: elementId, originalBbox: originalBbox, newBbox: element.bbox)
        pushUndo(.bbox(edit))

        isEditing = true
    }

    // MARK: - Multi-Selection Operations

    /// Check if merging is possible (need 2+ selected elements)
    var canMergeSelection: Bool {
        selectedElementCount >= 2
    }

    /// Merge all selected elements into a single element
    /// Creates a new element with a bounding box containing all selected elements
    func mergeSelectedElements() {
        let elements = selectedElements
        guard elements.count >= 2 else {
            error = "Select at least 2 elements to merge"
            return
        }

        // Check if any selected elements are locked
        let lockedCount = elements.filter { isElementLocked($0.id) }.count
        if lockedCount > 0 {
            error = "\(lockedCount) element(s) are locked - cannot merge"
            return
        }

        // Compute the union bounding box
        var minX = Float.infinity
        var minY = Float.infinity
        var maxX = -Float.infinity
        var maxY = -Float.infinity

        for element in elements {
            minX = min(minX, element.bbox.x)
            minY = min(minY, element.bbox.y)
            maxX = max(maxX, element.bbox.x + element.bbox.width)
            maxY = max(maxY, element.bbox.y + element.bbox.height)
        }

        // Find the most common label (for the merged element)
        var labelCounts: [DocItemLabel: Int] = [:]
        for element in elements {
            labelCounts[element.label, default: 0] += 1
        }
        let mostCommonLabel = labelCounts.max(by: { $0.value < $1.value })?.key ?? elements.first!.label

        // Calculate average confidence
        let avgConfidence = elements.map { $0.confidence }.reduce(0, +) / Float(elements.count)

        // Create the merged element
        let mergedElement = Element(
            id: nextElementId,
            bbox: BoundingBox(
                x: minX,
                y: minY,
                width: maxX - minX,
                height: maxY - minY
            ),
            label: mostCommonLabel,
            confidence: avgConfidence
        )
        nextElementId += 1

        // Store the original elements for undo
        let originalElementIds = Set(elements.map { $0.id })

        // Add merged element to edited elements
        editedElements[mergedElement.id] = mergedElement

        // Push compound operation to undo stack
        pushUndo(.merge(MergeEdit(
            mergedElementId: mergedElement.id,
            originalElementIds: originalElementIds,
            originalElements: elements
        )))

        // Select the merged element
        clearSelection()
        selectedElementId = mergedElement.id

        // Mark deleted elements (they won't appear in filteredElements)
        for id in originalElementIds {
            deletedElementIds.insert(id)
        }

        isEditing = true
    }

    /// Set of deleted element IDs (hidden from view, restored on undo)
    @Published var deletedElementIds: Set<UInt32> = []

    // MARK: - Split Operations

    /// Check if splitting is possible (need exactly 1 selected element)
    var canSplitSelection: Bool {
        selectedElement != nil && selectedElementCount == 1
    }

    /// Enter split mode for the selected element
    func enterSplitMode() {
        guard canSplitSelection else {
            error = "Select exactly 1 element to split"
            return
        }
        isSplitMode = true
        splitPosition = 0.5  // Reset to center
        showSplitDialog = true
    }

    /// Exit split mode without splitting
    func cancelSplit() {
        isSplitMode = false
        showSplitDialog = false
        splitPosition = 0.5
    }

    /// Split the selected element at the current position
    /// - Parameter direction: Horizontal (top/bottom) or Vertical (left/right)
    func splitSelectedElement(direction: SplitDirection? = nil) {
        guard let element = selectedElement else {
            error = "No element selected to split"
            return
        }

        // Check if element is locked
        guard !isElementLocked(element.id) else {
            error = "Element is locked"
            return
        }

        let splitDir = direction ?? splitDirection
        let pos = splitPosition

        // Ensure position is valid
        guard pos > 0.05 && pos < 0.95 else {
            error = "Split position must be between 5% and 95%"
            return
        }

        // Calculate the two new bounding boxes
        let bbox = element.bbox
        var bbox1: BoundingBox
        var bbox2: BoundingBox

        switch splitDir {
        case .horizontal:
            // Split horizontally: top and bottom
            let splitY = bbox.y + bbox.height * (1.0 - pos)  // pos from top
            // Bottom element (lower y in PDF coordinates)
            bbox1 = BoundingBox(
                x: bbox.x,
                y: bbox.y,
                width: bbox.width,
                height: splitY - bbox.y
            )
            // Top element (higher y in PDF coordinates)
            bbox2 = BoundingBox(
                x: bbox.x,
                y: splitY,
                width: bbox.width,
                height: bbox.height - (splitY - bbox.y)
            )

        case .vertical:
            // Split vertically: left and right
            let splitX = bbox.x + bbox.width * pos
            // Left element
            bbox1 = BoundingBox(
                x: bbox.x,
                y: bbox.y,
                width: splitX - bbox.x,
                height: bbox.height
            )
            // Right element
            bbox2 = BoundingBox(
                x: splitX,
                y: bbox.y,
                width: bbox.width - (splitX - bbox.x),
                height: bbox.height
            )
        }

        // Create the two new elements
        let element1 = Element(
            id: nextElementId,
            bbox: bbox1,
            label: element.label,
            confidence: element.confidence
        )
        let element2Id = nextElementId + 1
        let element2 = Element(
            id: element2Id,
            bbox: bbox2,
            label: element.label,
            confidence: element.confidence
        )
        nextElementId += 2

        // Add new elements to edited elements
        editedElements[element1.id] = element1
        editedElements[element2.id] = element2

        // Mark original element as deleted
        deletedElementIds.insert(element.id)

        // Push to undo stack
        pushUndo(.split(SplitEdit(
            originalElementId: element.id,
            originalElement: element,
            resultElementIds: [element1.id, element2.id]
        )))

        // Select the first new element
        clearSelection()
        selectedElementIds.insert(element1.id)
        selectedElementIds.insert(element2.id)
        selectedElementId = element1.id

        // Exit split mode
        cancelSplit()
        isEditing = true
    }

    /// Quick split: split at center without entering split mode
    func quickSplit(direction: SplitDirection) {
        splitDirection = direction
        splitPosition = 0.5
        splitSelectedElement(direction: direction)
    }

    // MARK: - Drawing Operations

    /// Start drawing a new bounding box
    func startDrawing(at point: CGPoint, in size: CGSize) {
        guard editTool == .draw else { return }
        isDrawing = true
        drawStartPoint = point
        drawCurrentPoint = point
    }

    /// Update the drawing rectangle
    func updateDrawing(at point: CGPoint, in size: CGSize) {
        guard isDrawing else { return }
        drawCurrentPoint = point
    }

    /// End drawing and show label picker
    func endDrawing(in size: CGSize) {
        guard isDrawing,
              let startPoint = drawStartPoint,
              let endPoint = drawCurrentPoint else {
            resetDrawingState()
            return
        }

        let pageSize = self.pageSize
        let scale = min(
            size.width / CGFloat(pageSize.width),
            size.height / CGFloat(pageSize.height)
        )

        // Calculate bbox in PDF coordinates
        let minX = min(startPoint.x, endPoint.x)
        let maxX = max(startPoint.x, endPoint.x)
        let minY = min(startPoint.y, endPoint.y)
        let maxY = max(startPoint.y, endPoint.y)

        // Convert screen coordinates to PDF coordinates
        let x = Float(minX / scale)
        let width = Float((maxX - minX) / scale)
        let height = Float((maxY - minY) / scale)
        let y = Float(pageSize.height) - Float(minY / scale) - height

        // Minimum size check
        if width < 10 || height < 10 {
            resetDrawingState()
            return
        }

        pendingNewElementBbox = BoundingBox(x: x, y: y, width: width, height: height)
        showLabelPicker = true
        resetDrawingState()
    }

    /// Create a new element with the selected label
    func createNewElement(with label: DocItemLabel) {
        guard let bbox = pendingNewElementBbox else { return }

        let newElement = Element(
            id: nextElementId,
            bbox: bbox,
            label: label,
            confidence: 1.0  // User-created elements have 100% confidence
        )

        nextElementId += 1

        // Add to edited elements
        editedElements[newElement.id] = newElement

        // Push to undo stack
        pushUndo(.create(newElement))

        // Select the new element
        selectedElementId = newElement.id

        // Clear pending state
        pendingNewElementBbox = nil
        showLabelPicker = false
        isEditing = true
    }

    /// Cancel new element creation
    func cancelNewElement() {
        pendingNewElementBbox = nil
        showLabelPicker = false
    }

    /// Reset drawing state
    private func resetDrawingState() {
        isDrawing = false
        drawStartPoint = nil
        drawCurrentPoint = nil
    }

    /// Get the current drawing rectangle in screen coordinates
    func drawingRect(in size: CGSize) -> CGRect? {
        guard isDrawing,
              let startPoint = drawStartPoint,
              let endPoint = drawCurrentPoint else { return nil }

        let minX = min(startPoint.x, endPoint.x)
        let maxX = max(startPoint.x, endPoint.x)
        let minY = min(startPoint.y, endPoint.y)
        let maxY = max(startPoint.y, endPoint.y)

        return CGRect(x: minX, y: minY, width: maxX - minX, height: maxY - minY)
    }

    // MARK: - Lasso Selection Operations

    /// Start drawing a lasso selection path
    func startLasso(at point: CGPoint, in size: CGSize) {
        guard editTool == .lasso else { return }
        isLassoDrawing = true
        lassoPath = [point]
        lassoViewSize = size
    }

    /// Continue drawing the lasso path
    func updateLasso(at point: CGPoint) {
        guard isLassoDrawing else { return }
        // Only add point if it's sufficiently far from the last point (reduces path complexity)
        if let lastPoint = lassoPath.last {
            let distance = hypot(point.x - lastPoint.x, point.y - lastPoint.y)
            if distance > 3.0 {  // Minimum distance between points
                lassoPath.append(point)
            }
        } else {
            lassoPath.append(point)
        }
    }

    /// End lasso drawing and select all elements within the path
    func endLasso(addToSelection: Bool = false) {
        guard isLassoDrawing, lassoPath.count >= 3 else {
            resetLassoState()
            return
        }

        // Close the path
        if let firstPoint = lassoPath.first {
            lassoPath.append(firstPoint)
        }

        // Find all elements within the lasso
        let selectedIds = elementsWithinLasso()

        if selectedIds.isEmpty {
            // No elements found - just deselect
            if !addToSelection {
                deselect()
            }
        } else if addToSelection {
            // Add to existing selection (Shift key held)
            for id in selectedIds {
                selectedElementIds.insert(id)
            }
            // Update primary selection to most recently added
            if let lastId = selectedIds.first {
                selectedElementId = lastId
            }
        } else {
            // Replace selection
            selectedElementIds = selectedIds
            selectedElementId = selectedIds.first
        }

        resetLassoState()
    }

    /// Reset lasso drawing state
    func resetLassoState() {
        isLassoDrawing = false
        lassoPath = []
        lassoViewSize = .zero
    }

    /// Get the current lasso path for rendering
    func lassoSwiftUIPath() -> Path? {
        guard !lassoPath.isEmpty else { return nil }

        var path = Path()
        path.move(to: lassoPath[0])
        for point in lassoPath.dropFirst() {
            path.addLine(to: point)
        }
        return path
    }

    /// Find all element IDs whose centers are within the lasso path
    private func elementsWithinLasso() -> Set<UInt32> {
        guard lassoPath.count >= 3 else { return [] }

        let pageSize = self.pageSize
        let scale = min(
            lassoViewSize.width / CGFloat(pageSize.width),
            lassoViewSize.height / CGFloat(pageSize.height)
        )

        var result = Set<UInt32>()

        for element in filteredElements {
            // Get element center in screen coordinates
            let centerX = CGFloat(element.bbox.x + element.bbox.width / 2) * scale
            let centerY = (CGFloat(pageSize.height) - CGFloat(element.bbox.y + element.bbox.height / 2)) * scale
            let center = CGPoint(x: centerX, y: centerY)

            // Check if center is within lasso polygon
            if isPointInLasso(center) {
                result.insert(element.id)
            }
        }

        return result
    }

    /// Check if a point is inside the lasso polygon using ray casting algorithm
    private func isPointInLasso(_ point: CGPoint) -> Bool {
        guard lassoPath.count >= 3 else { return false }

        var inside = false
        var j = lassoPath.count - 1

        for i in 0..<lassoPath.count {
            let xi = lassoPath[i].x
            let yi = lassoPath[i].y
            let xj = lassoPath[j].x
            let yj = lassoPath[j].y

            if ((yi > point.y) != (yj > point.y)) &&
               (point.x < (xj - xi) * (point.y - yi) / (yj - yi) + xi) {
                inside = !inside
            }
            j = i
        }

        return inside
    }

    // MARK: - Marquee Selection Operations

    /// Start drawing a marquee (rectangular) selection
    func startMarquee(at point: CGPoint, in size: CGSize) {
        guard editTool == .marquee else { return }
        isMarqueeDrawing = true
        marqueeStartPoint = point
        marqueeCurrentPoint = point
        marqueeViewSize = size
    }

    /// Continue drawing the marquee rectangle
    func updateMarquee(at point: CGPoint) {
        guard isMarqueeDrawing else { return }
        marqueeCurrentPoint = point
    }

    /// End marquee drawing and select all elements within the rectangle
    func endMarquee(addToSelection: Bool = false) {
        guard isMarqueeDrawing,
              let startPoint = marqueeStartPoint,
              let endPoint = marqueeCurrentPoint else {
            resetMarqueeState()
            return
        }

        // Calculate the rectangle
        let minX = min(startPoint.x, endPoint.x)
        let maxX = max(startPoint.x, endPoint.x)
        let minY = min(startPoint.y, endPoint.y)
        let maxY = max(startPoint.y, endPoint.y)

        // Minimum size check (to prevent accidental micro-selections)
        let width = maxX - minX
        let height = maxY - minY
        guard width > 5 && height > 5 else {
            // Treat as a click rather than a drag selection
            if !addToSelection {
                deselect()
            }
            resetMarqueeState()
            return
        }

        // Find all elements within the marquee rectangle
        let selectedIds = elementsWithinMarquee()

        if selectedIds.isEmpty {
            // No elements found - just deselect
            if !addToSelection {
                deselect()
            }
        } else if addToSelection {
            // Add to existing selection (Shift key held)
            for id in selectedIds {
                selectedElementIds.insert(id)
            }
            // Update primary selection to most recently added
            if let lastId = selectedIds.first {
                selectedElementId = lastId
            }
        } else {
            // Replace selection
            selectedElementIds = selectedIds
            selectedElementId = selectedIds.first
        }

        resetMarqueeState()
    }

    /// Reset marquee drawing state
    func resetMarqueeState() {
        isMarqueeDrawing = false
        marqueeStartPoint = nil
        marqueeCurrentPoint = nil
        marqueeViewSize = .zero
    }

    /// Get the current marquee rectangle in screen coordinates for rendering
    func marqueeRect() -> CGRect? {
        guard isMarqueeDrawing,
              let startPoint = marqueeStartPoint,
              let endPoint = marqueeCurrentPoint else { return nil }

        let minX = min(startPoint.x, endPoint.x)
        let maxX = max(startPoint.x, endPoint.x)
        let minY = min(startPoint.y, endPoint.y)
        let maxY = max(startPoint.y, endPoint.y)

        return CGRect(x: minX, y: minY, width: maxX - minX, height: maxY - minY)
    }

    /// Find all element IDs whose centers are within the marquee rectangle
    private func elementsWithinMarquee() -> Set<UInt32> {
        guard let startPoint = marqueeStartPoint,
              let endPoint = marqueeCurrentPoint else { return [] }

        let pageSize = self.pageSize
        let scale = min(
            marqueeViewSize.width / CGFloat(pageSize.width),
            marqueeViewSize.height / CGFloat(pageSize.height)
        )

        // Calculate marquee bounds in screen coordinates
        let minX = min(startPoint.x, endPoint.x)
        let maxX = max(startPoint.x, endPoint.x)
        let minY = min(startPoint.y, endPoint.y)
        let maxY = max(startPoint.y, endPoint.y)
        let marqueeRect = CGRect(x: minX, y: minY, width: maxX - minX, height: maxY - minY)

        var result = Set<UInt32>()

        for element in filteredElements {
            // Get element center in screen coordinates
            let centerX = CGFloat(element.bbox.x + element.bbox.width / 2) * scale
            let centerY = (CGFloat(pageSize.height) - CGFloat(element.bbox.y + element.bbox.height / 2)) * scale
            let center = CGPoint(x: centerX, y: centerY)

            // Check if center is within marquee rectangle
            if marqueeRect.contains(center) {
                result.insert(element.id)
            }
        }

        return result
    }

    // MARK: - Clipboard Operations

    /// Copy the selected elements to clipboard
    func copySelectedElements() {
        let elements = selectedElements
        guard !elements.isEmpty else {
            error = "No elements selected to copy"
            return
        }

        // Store elements in clipboard
        clipboard = elements
        clipboardSourcePage = currentPage

        processingProgress = "Copied \(elements.count) element(s)"
    }

    /// Cut the selected elements (copy to clipboard and delete)
    func cutSelectedElements() {
        let elements = selectedElements
        guard !elements.isEmpty else {
            error = "No elements selected to cut"
            return
        }

        // Copy to clipboard
        clipboard = elements
        clipboardSourcePage = currentPage

        // Delete all selected elements
        for element in elements {
            // Mark as deleted
            deletedElementIds.insert(element.id)
            // Push to undo stack (so each deletion is undoable)
            pushUndo(.delete(element))
        }

        // Clear selection
        clearSelection()
        isEditing = true

        processingProgress = "Cut \(elements.count) element(s)"
    }

    /// Paste elements from clipboard
    /// - Parameter offset: Optional offset to apply (default: slight offset if pasting on same page)
    func pasteElements() {
        guard !clipboard.isEmpty else {
            error = "Clipboard is empty"
            return
        }

        // Calculate offset: slight offset if same page, no offset if different page
        let isSamePage = (clipboardSourcePage == currentPage)
        let offsetX: Float = isSamePage ? 15.0 : 0.0
        let offsetY: Float = isSamePage ? -15.0 : 0.0

        var pastedElements: [Element] = []

        for originalElement in clipboard {
            // Create new element with new ID
            let newElement = Element(
                id: nextElementId,
                bbox: BoundingBox(
                    x: originalElement.bbox.x + offsetX,
                    y: originalElement.bbox.y + offsetY,
                    width: originalElement.bbox.width,
                    height: originalElement.bbox.height
                ),
                label: originalElement.label,
                confidence: 1.0  // User-pasted elements have 100% confidence
            )

            nextElementId += 1

            // Add to edited elements
            editedElements[newElement.id] = newElement
            pastedElements.append(newElement)

            // Push to undo stack
            pushUndo(.create(newElement))
        }

        // Select the pasted elements
        clearSelection()
        if pastedElements.count == 1 {
            selectedElementId = pastedElements.first?.id
        } else {
            selectedElementIds = Set(pastedElements.map { $0.id })
            selectedElementId = pastedElements.first?.id
        }

        isEditing = true
        processingProgress = "Pasted \(pastedElements.count) element(s)"
    }

    /// Clear the clipboard
    func clearClipboard() {
        clipboard = []
        clipboardSourcePage = 0
    }

    // MARK: - Alignment Operations

    /// Alignment type for multi-selection alignment
    public enum AlignmentType: String, CaseIterable, Identifiable {
        case left = "Left"
        case right = "Right"
        case top = "Top"
        case bottom = "Bottom"
        case centerHorizontal = "Center H"
        case centerVertical = "Center V"

        public var id: String { rawValue }

        public var systemImage: String {
            switch self {
            case .left: return "align.horizontal.left"
            case .right: return "align.horizontal.right"
            case .top: return "align.vertical.top"
            case .bottom: return "align.vertical.bottom"
            case .centerHorizontal: return "align.horizontal.center"
            case .centerVertical: return "align.vertical.center"
            }
        }

        public var description: String {
            switch self {
            case .left: return "Align left edges"
            case .right: return "Align right edges"
            case .top: return "Align top edges"
            case .bottom: return "Align bottom edges"
            case .centerHorizontal: return "Align horizontal centers"
            case .centerVertical: return "Align vertical centers"
            }
        }
    }

    /// Distribution type for multi-selection distribution
    public enum DistributionType: String, CaseIterable, Identifiable {
        case horizontal = "Horizontal"
        case vertical = "Vertical"

        public var id: String { rawValue }

        public var systemImage: String {
            switch self {
            case .horizontal: return "distribute.horizontal.center"
            case .vertical: return "distribute.vertical.center"
            }
        }

        public var description: String {
            switch self {
            case .horizontal: return "Distribute horizontally"
            case .vertical: return "Distribute vertically"
            }
        }
    }

    /// Check if alignment/distribution is possible (need 2+ selected elements)
    var canAlign: Bool {
        selectedElementCount >= 2
    }

    /// Check if distribution is possible (need 3+ selected elements)
    var canDistribute: Bool {
        selectedElementCount >= 3
    }

    /// Align selected elements to a specified alignment type
    func alignSelectedElements(_ alignmentType: AlignmentType) {
        let elements = selectedElements
        guard elements.count >= 2 else {
            error = "Select at least 2 elements to align"
            return
        }

        // Calculate the target position based on alignment type
        let targetValue: Float
        switch alignmentType {
        case .left:
            targetValue = elements.map { $0.bbox.x }.min() ?? 0
        case .right:
            targetValue = elements.map { $0.bbox.x + $0.bbox.width }.max() ?? 0
        case .top:
            targetValue = elements.map { $0.bbox.y + $0.bbox.height }.max() ?? 0
        case .bottom:
            targetValue = elements.map { $0.bbox.y }.min() ?? 0
        case .centerHorizontal:
            // Average of horizontal centers
            let centers = elements.map { $0.bbox.x + $0.bbox.width / 2 }
            targetValue = centers.reduce(0, +) / Float(centers.count)
        case .centerVertical:
            // Average of vertical centers
            let centers = elements.map { $0.bbox.y + $0.bbox.height / 2 }
            targetValue = centers.reduce(0, +) / Float(centers.count)
        }

        // Apply alignment to each element
        for element in elements {
            var newBbox = element.bbox
            let originalBbox = element.bbox

            switch alignmentType {
            case .left:
                newBbox.x = targetValue
            case .right:
                newBbox.x = targetValue - element.bbox.width
            case .top:
                newBbox.y = targetValue - element.bbox.height
            case .bottom:
                newBbox.y = targetValue
            case .centerHorizontal:
                newBbox.x = targetValue - element.bbox.width / 2
            case .centerVertical:
                newBbox.y = targetValue - element.bbox.height / 2
            }

            // Only update if changed
            if newBbox.x != originalBbox.x || newBbox.y != originalBbox.y {
                var updatedElement = element
                updatedElement.bbox = newBbox
                editedElements[element.id] = updatedElement

                // Push to undo stack
                let edit = BBoxEdit(elementId: element.id, originalBbox: originalBbox, newBbox: newBbox)
                pushUndo(.bbox(edit))
            }
        }

        isEditing = true
        processingProgress = "Aligned \(elements.count) element(s) \(alignmentType.rawValue.lowercased())"
    }

    /// Distribute selected elements evenly
    func distributeSelectedElements(_ distributionType: DistributionType) {
        let elements = selectedElements
        guard elements.count >= 3 else {
            error = "Select at least 3 elements to distribute"
            return
        }

        // Sort elements by position
        let sortedElements: [Element]
        let totalSpan: Float
        let totalElementSize: Float

        switch distributionType {
        case .horizontal:
            sortedElements = elements.sorted { $0.bbox.x < $1.bbox.x }
            let minX = sortedElements.first!.bbox.x
            let maxX = sortedElements.last!.bbox.x + sortedElements.last!.bbox.width
            totalSpan = maxX - minX
            totalElementSize = elements.map { $0.bbox.width }.reduce(0, +)
        case .vertical:
            sortedElements = elements.sorted { $0.bbox.y < $1.bbox.y }
            let minY = sortedElements.first!.bbox.y
            let maxY = sortedElements.last!.bbox.y + sortedElements.last!.bbox.height
            totalSpan = maxY - minY
            totalElementSize = elements.map { $0.bbox.height }.reduce(0, +)
        }

        // Calculate spacing between elements
        let spacing = (totalSpan - totalElementSize) / Float(elements.count - 1)

        // Apply distribution
        var currentPosition: Float
        switch distributionType {
        case .horizontal:
            currentPosition = sortedElements.first!.bbox.x
        case .vertical:
            currentPosition = sortedElements.first!.bbox.y
        }

        for (index, element) in sortedElements.enumerated() {
            // Skip first and last elements (they stay in place to define the span)
            if index == 0 || index == sortedElements.count - 1 {
                switch distributionType {
                case .horizontal:
                    currentPosition = element.bbox.x + element.bbox.width + spacing
                case .vertical:
                    currentPosition = element.bbox.y + element.bbox.height + spacing
                }
                continue
            }

            var newBbox = element.bbox
            let originalBbox = element.bbox

            switch distributionType {
            case .horizontal:
                newBbox.x = currentPosition
                currentPosition = newBbox.x + newBbox.width + spacing
            case .vertical:
                newBbox.y = currentPosition
                currentPosition = newBbox.y + newBbox.height + spacing
            }

            // Only update if changed
            if newBbox.x != originalBbox.x || newBbox.y != originalBbox.y {
                var updatedElement = element
                updatedElement.bbox = newBbox
                editedElements[element.id] = updatedElement

                // Push to undo stack
                let edit = BBoxEdit(elementId: element.id, originalBbox: originalBbox, newBbox: newBbox)
                pushUndo(.bbox(edit))
            }
        }

        isEditing = true
        processingProgress = "Distributed \(elements.count) element(s) \(distributionType.rawValue.lowercased())"
    }

    /// Convenience methods for common alignments
    func alignLeft() { alignSelectedElements(.left) }
    func alignRight() { alignSelectedElements(.right) }
    func alignTop() { alignSelectedElements(.top) }
    func alignBottom() { alignSelectedElements(.bottom) }
    func alignCenterHorizontal() { alignSelectedElements(.centerHorizontal) }
    func alignCenterVertical() { alignSelectedElements(.centerVertical) }
    func distributeHorizontally() { distributeSelectedElements(.horizontal) }
    func distributeVertically() { distributeSelectedElements(.vertical) }

    // MARK: - Match Size Operations

    /// Match size type for multi-selection size matching
    public enum MatchSizeType: String, CaseIterable, Identifiable {
        case width = "Width"
        case height = "Height"
        case both = "Both"

        public var id: String { rawValue }

        public var systemImage: String {
            switch self {
            case .width: return "arrow.left.and.right"
            case .height: return "arrow.up.and.down"
            case .both: return "arrow.up.left.and.arrow.down.right"
            }
        }

        public var description: String {
            switch self {
            case .width: return "Make same width"
            case .height: return "Make same height"
            case .both: return "Make same size"
            }
        }
    }

    /// Reference for size matching
    public enum SizeReference: String, CaseIterable, Identifiable {
        case smallest = "Smallest"
        case largest = "Largest"
        case average = "Average"
        case first = "First Selected"

        public var id: String { rawValue }
    }

    /// Check if match size is possible (need 2+ selected elements)
    var canMatchSize: Bool {
        selectedElementCount >= 2
    }

    /// Match size of selected elements
    func matchSizeOfSelectedElements(_ sizeType: MatchSizeType, reference: SizeReference = .largest) {
        let elements = selectedElements
        guard elements.count >= 2 else {
            error = "Select at least 2 elements to match size"
            return
        }

        // Calculate target dimensions based on reference
        let targetWidth: Float
        let targetHeight: Float

        switch reference {
        case .smallest:
            targetWidth = elements.map { $0.bbox.width }.min() ?? 0
            targetHeight = elements.map { $0.bbox.height }.min() ?? 0
        case .largest:
            targetWidth = elements.map { $0.bbox.width }.max() ?? 0
            targetHeight = elements.map { $0.bbox.height }.max() ?? 0
        case .average:
            targetWidth = elements.map { $0.bbox.width }.reduce(0, +) / Float(elements.count)
            targetHeight = elements.map { $0.bbox.height }.reduce(0, +) / Float(elements.count)
        case .first:
            // Use the primary selected element (last clicked)
            if let primaryId = selectedElementId,
               let primaryElement = elements.first(where: { $0.id == primaryId }) {
                targetWidth = primaryElement.bbox.width
                targetHeight = primaryElement.bbox.height
            } else {
                targetWidth = elements.first!.bbox.width
                targetHeight = elements.first!.bbox.height
            }
        }

        // Apply size matching to each element
        for element in elements {
            var newBbox = element.bbox
            let originalBbox = element.bbox

            // Calculate center point to resize around
            let centerX = element.bbox.x + element.bbox.width / 2
            let centerY = element.bbox.y + element.bbox.height / 2

            switch sizeType {
            case .width:
                newBbox.width = targetWidth
                newBbox.x = centerX - targetWidth / 2
            case .height:
                newBbox.height = targetHeight
                newBbox.y = centerY - targetHeight / 2
            case .both:
                newBbox.width = targetWidth
                newBbox.height = targetHeight
                newBbox.x = centerX - targetWidth / 2
                newBbox.y = centerY - targetHeight / 2
            }

            // Only update if changed
            if newBbox != originalBbox {
                var updatedElement = element
                updatedElement.bbox = newBbox
                editedElements[element.id] = updatedElement

                // Push to undo stack
                let edit = BBoxEdit(elementId: element.id, originalBbox: originalBbox, newBbox: newBbox)
                pushUndo(.bbox(edit))
            }
        }

        isEditing = true
        let refText = reference == .largest ? "largest" : reference == .smallest ? "smallest" : reference == .average ? "average" : "first"
        processingProgress = "Matched \(sizeType.rawValue.lowercased()) of \(elements.count) element(s) to \(refText)"
    }

    /// Convenience methods for common match size operations
    func matchWidth() { matchSizeOfSelectedElements(.width) }
    func matchHeight() { matchSizeOfSelectedElements(.height) }
    func matchBothDimensions() { matchSizeOfSelectedElements(.both) }
    func matchWidthToSmallest() { matchSizeOfSelectedElements(.width, reference: .smallest) }
    func matchHeightToSmallest() { matchSizeOfSelectedElements(.height, reference: .smallest) }
}
