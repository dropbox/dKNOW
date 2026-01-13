// InspectorView - Right panel showing element details and statistics
// Contains inspector, element details, statistics, and filter views

import SwiftUI
import DoclingBridge

// MARK: - Inspector View

struct InspectorView: View {
    @ObservedObject var viewModel: DocumentViewModel
    @State private var searchText: String = ""

    /// Elements filtered by search text
    private var searchFilteredElements: [Element] {
        guard !searchText.isEmpty else { return viewModel.filteredElements }
        let lowercased = searchText.lowercased()

        // Search by ID (number) or label name
        return viewModel.filteredElements.filter { element in
            // Match by ID
            if let searchId = UInt32(searchText), element.id == searchId {
                return true
            }
            // Match by ID as string contains
            if String(element.id).contains(searchText) {
                return true
            }
            // Match by label name
            if element.label.description.lowercased().contains(lowercased) {
                return true
            }
            // Match by COCO label name
            if element.label.cocoName.lowercased().contains(lowercased) {
                return true
            }
            return false
        }
    }

    var body: some View {
        List {
            // Document info section (always shown when PDF loaded)
            if viewModel.pdfDocument != nil {
                Section("Document") {
                    if let url = viewModel.pdfURL {
                        LabeledContent("File", value: url.lastPathComponent)
                    }
                    LabeledContent("Pages", value: "\(viewModel.pageCount)")
                    LabeledContent("Current Page", value: "\(viewModel.currentPage + 1)")
                    let size = viewModel.pageSize
                    LabeledContent("Page Size", value: String(format: "%.0f × %.0f pt", size.width, size.height))
                }
            }

            if let snapshot = viewModel.currentStageSnapshot {
                Section("Stage: \(viewModel.currentStage.description)") {
                    LabeledContent("Processing Time", value: String(format: "%.2f ms", snapshot.processingTimeMs))
                    LabeledContent("Elements", value: "\(snapshot.elements.count)")
                    LabeledContent("Text Cells", value: "\(snapshot.textCells.count)")
                }

                // Element statistics
                Section("Statistics") {
                    ElementStatisticsView(elements: snapshot.elements)
                }

                // Validation section (collapsible, shows issue count in header)
                Section {
                    DisclosureGroup {
                        ValidationView(
                            elements: snapshot.elements,
                            pageSize: viewModel.pageSize,
                            onSelectElement: { elementId in
                                viewModel.selectSingleElement(elementId)
                            }
                        )
                    } label: {
                        ValidationSummaryLabel(
                            issueCount: ValidationView.computeIssueCount(
                                elements: snapshot.elements,
                                pageSize: viewModel.pageSize
                            )
                        )
                    }
                }

                // Edit settings section
                Section("Edit Settings") {
                    Toggle("Snap to Grid", isOn: $viewModel.snapToGrid)
                        .toggleStyle(.switch)

                    if viewModel.snapToGrid {
                        HStack {
                            Text("Grid Size")
                            Spacer()
                            Text("\(Int(viewModel.gridSize)) pt")
                                .foregroundColor(.secondary)
                                .monospacedDigit()
                        }
                        Slider(value: Binding(
                            get: { Double(viewModel.gridSize) },
                            set: { viewModel.gridSize = Float($0) }
                        ), in: 5...50, step: 5)

                        Toggle("Show Grid", isOn: $viewModel.showGridOverlay)
                            .toggleStyle(.switch)
                    }

                    Toggle("Show Alignment Guides", isOn: $viewModel.showAlignmentGuides)
                        .toggleStyle(.switch)
                }

                // Filters section
                Section("Filters") {
                    // Confidence threshold
                    VStack(alignment: .leading, spacing: 8) {
                        HStack {
                            Text("Confidence")
                            Spacer()
                            Text("\(Int(viewModel.confidenceThreshold * 100))%")
                                .foregroundColor(.secondary)
                                .monospacedDigit()
                        }
                        Slider(value: $viewModel.confidenceThreshold, in: 0...1, step: 0.05)
                    }

                    Toggle("Color by confidence", isOn: $viewModel.colorByConfidence)
                        .toggleStyle(.switch)

                    Divider()

                    // Label filter
                    DisclosureGroup("Label Filter") {
                        LabelFilterView(viewModel: viewModel)
                    }

                    if viewModel.hiddenElementCount > 0 {
                        HStack {
                            Image(systemName: "eye.slash")
                                .foregroundColor(.orange)
                            Text("\(viewModel.hiddenElementCount) element(s) hidden")
                                .font(.caption)
                                .foregroundColor(.orange)
                        }
                    }
                }

                // Multi-selection info and actions
                if viewModel.selectedElementCount > 1 {
                    Section("Selection (\(viewModel.selectedElementCount) elements)") {
                        MultiSelectionView(viewModel: viewModel)
                    }
                }
                // Selected element details (single selection)
                else if let selectedElement = viewModel.selectedElement {
                    Section("Selected Element") {
                        SelectedElementDetailsView(
                            element: selectedElement,
                            viewModel: viewModel
                        )
                    }
                }

                Section("Elements (\(searchFilteredElements.count)/\(viewModel.filteredElements.count))") {
                    // Search field
                    ElementSearchField(
                        searchText: $searchText,
                        totalCount: viewModel.filteredElements.count,
                        matchCount: searchFilteredElements.count
                    )

                    // Element list
                    ForEach(searchFilteredElements) { element in
                        ElementRow(
                            element: element,
                            isSelected: viewModel.isElementSelected(element.id),
                            colorByConfidence: viewModel.colorByConfidence
                        )
                            .onTapGesture {
                                // Regular click in list - single selection
                                let modifiers = NSEvent.modifierFlags
                                if modifiers.contains(.command) || modifiers.contains(.shift) {
                                    viewModel.toggleElementInSelection(element.id)
                                } else {
                                    viewModel.selectSingleElement(element.id)
                                }
                            }
                    }
                }
            } else if viewModel.pdfDocument == nil {
                // No document loaded
                VStack(spacing: 8) {
                    Image(systemName: "doc.text.magnifyingglass")
                        .font(.title)
                        .foregroundColor(.secondary)
                    Text("Open a PDF to inspect")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                .frame(maxWidth: .infinity)
                .padding()
            } else {
                // Document loaded but no pipeline data
                Text("Pipeline data not available")
                    .foregroundColor(.secondary)
            }
        }
        .listStyle(.inset)
        .navigationTitle("Inspector")
    }
}

// MARK: - Selected Element Details View

struct SelectedElementDetailsView: View {
    let element: Element
    @ObservedObject var viewModel: DocumentViewModel

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            // Element ID, label, and lock status
            HStack {
                Circle()
                    .fill(element.label.swiftUIColor)
                    .frame(width: 12, height: 12)
                Text(element.label.description)
                    .font(.headline)
                Spacer()
                // Lock button
                Button(action: { viewModel.toggleLockSelectedElements() }) {
                    Image(systemName: viewModel.isElementLocked(element.id) ? "lock.fill" : "lock.open")
                        .foregroundColor(viewModel.isElementLocked(element.id) ? .orange : .secondary)
                }
                .buttonStyle(.borderless)
                .help(viewModel.isElementLocked(element.id) ? "Unlock element (⌘L)" : "Lock element (⌘L)")
                Text("ID: \(element.id)")
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .monospacedDigit()
            }

            // Show lock warning if locked
            if viewModel.isElementLocked(element.id) {
                HStack {
                    Image(systemName: "lock.fill")
                        .foregroundColor(.orange)
                    Text("Element is locked")
                        .font(.caption)
                        .foregroundColor(.orange)
                }
            }

            // Label picker for editing
            Picker("Label", selection: Binding(
                get: { element.label },
                set: { newLabel in
                    viewModel.changeLabel(elementId: element.id, to: newLabel)
                }
            )) {
                ForEach(DocItemLabel.allCases) { label in
                    Text(label.description).tag(label)
                }
            }
            .pickerStyle(.menu)

            Divider()

            // Confidence
            HStack {
                Text("Confidence")
                Spacer()
                Text(String(format: "%.1f%%", element.confidence * 100))
                    .foregroundColor(confidenceColor(for: element.confidence))
                    .monospacedDigit()
            }

            // Reading order
            if element.hasReadingOrder {
                HStack {
                    Text("Reading Order")
                    Spacer()
                    Text("\(element.readingOrder)")
                        .monospacedDigit()
                }
            }

            Divider()

            // Bounding box coordinates
            Text("Bounding Box")
                .font(.caption.bold())

            Grid(alignment: .leading, horizontalSpacing: 12, verticalSpacing: 4) {
                GridRow {
                    Text("X:")
                        .foregroundColor(.secondary)
                    Text(String(format: "%.1f", element.bbox.x))
                        .monospacedDigit()
                    Text("Y:")
                        .foregroundColor(.secondary)
                    Text(String(format: "%.1f", element.bbox.y))
                        .monospacedDigit()
                }
                GridRow {
                    Text("W:")
                        .foregroundColor(.secondary)
                    Text(String(format: "%.1f", element.bbox.width))
                        .monospacedDigit()
                    Text("H:")
                        .foregroundColor(.secondary)
                    Text(String(format: "%.1f", element.bbox.height))
                        .monospacedDigit()
                }
            }
            .font(.caption)

            // Area
            HStack {
                Text("Area")
                Spacer()
                Text(String(format: "%.0f pt²", element.bbox.width * element.bbox.height))
                    .monospacedDigit()
                    .foregroundColor(.secondary)
            }
            .font(.caption)

            Divider()

            // Split controls
            VStack(alignment: .leading, spacing: 8) {
                Text("Split Element")
                    .font(.caption.bold())

                HStack(spacing: 8) {
                    // Horizontal split button
                    Button(action: { viewModel.quickSplit(direction: .horizontal) }) {
                        Label("H", systemImage: "rectangle.split.1x2")
                    }
                    .buttonStyle(.bordered)
                    .help("Split horizontally (top/bottom)")

                    // Vertical split button
                    Button(action: { viewModel.quickSplit(direction: .vertical) }) {
                        Label("V", systemImage: "rectangle.split.2x1")
                    }
                    .buttonStyle(.bordered)
                    .help("Split vertically (left/right)")

                    Spacer()

                    // Advanced split with position control
                    Button(action: { viewModel.enterSplitMode() }) {
                        Image(systemName: "slider.horizontal.3")
                    }
                    .buttonStyle(.borderless)
                    .help("Advanced split with position control")
                }
            }

            Divider()

            // Action buttons
            HStack(spacing: 12) {
                Button(action: { viewModel.duplicateSelectedElement() }) {
                    Label("Duplicate", systemImage: "doc.on.doc")
                }
                .buttonStyle(.borderless)

                Button(role: .destructive, action: { viewModel.deleteSelectedElement() }) {
                    Label("Delete", systemImage: "trash")
                }
                .buttonStyle(.borderless)
            }
            .font(.caption)

            Divider()

            // JSON Preview
            DisclosureGroup("JSON Preview") {
                JSONPreviewView(element: element)
            }
        }
    }
}

// MARK: - JSON Preview View

/// View showing JSON representation of an element
struct JSONPreviewView: View {
    let element: Element
    @State private var isCopied = false

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            // JSON content
            ScrollView(.horizontal, showsIndicators: false) {
                Text(jsonString)
                    .font(.system(.caption, design: .monospaced))
                    .foregroundColor(.primary)
                    .textSelection(.enabled)
            }
            .frame(maxHeight: 150)
            .padding(8)
            .background(Color(NSColor.textBackgroundColor))
            .cornerRadius(6)
            .overlay(
                RoundedRectangle(cornerRadius: 6)
                    .stroke(Color(NSColor.separatorColor), lineWidth: 0.5)
            )

            // Copy button
            HStack {
                Spacer()
                Button(action: copyToClipboard) {
                    Label(isCopied ? "Copied!" : "Copy JSON", systemImage: isCopied ? "checkmark" : "doc.on.clipboard")
                }
                .buttonStyle(.borderless)
                .font(.caption)
            }
        }
    }

    private var jsonString: String {
        let jsonDict: [String: Any] = [
            "id": element.id,
            "label": element.label.cocoName,
            "confidence": Double(element.confidence),
            "reading_order": element.readingOrder,
            "bbox": [
                "x": Double(element.bbox.x),
                "y": Double(element.bbox.y),
                "width": Double(element.bbox.width),
                "height": Double(element.bbox.height)
            ]
        ]

        do {
            let data = try JSONSerialization.data(withJSONObject: jsonDict, options: [.prettyPrinted, .sortedKeys])
            return String(data: data, encoding: .utf8) ?? "{}"
        } catch {
            return "{\"error\": \"Failed to serialize\"}"
        }
    }

    private func copyToClipboard() {
        let pasteboard = NSPasteboard.general
        pasteboard.clearContents()
        pasteboard.setString(jsonString, forType: .string)

        isCopied = true
        DispatchQueue.main.asyncAfter(deadline: .now() + 1.5) {
            isCopied = false
        }
    }
}

// MARK: - Multi-Element JSON Preview View

/// View showing JSON representation of multiple elements
struct MultiElementJSONPreviewView: View {
    let elements: [Element]
    @State private var isCopied = false

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            // JSON content
            ScrollView(.horizontal, showsIndicators: false) {
                Text(jsonString)
                    .font(.system(.caption, design: .monospaced))
                    .foregroundColor(.primary)
                    .textSelection(.enabled)
            }
            .frame(maxHeight: 200)
            .padding(8)
            .background(Color(NSColor.textBackgroundColor))
            .cornerRadius(6)
            .overlay(
                RoundedRectangle(cornerRadius: 6)
                    .stroke(Color(NSColor.separatorColor), lineWidth: 0.5)
            )

            // Copy button
            HStack {
                Text("\(elements.count) elements")
                    .font(.caption)
                    .foregroundColor(.secondary)
                Spacer()
                Button(action: copyToClipboard) {
                    Label(isCopied ? "Copied!" : "Copy JSON", systemImage: isCopied ? "checkmark" : "doc.on.clipboard")
                }
                .buttonStyle(.borderless)
                .font(.caption)
            }
        }
    }

    private var jsonString: String {
        let annotationsArray: [[String: Any]] = elements.map { element in
            [
                "id": element.id,
                "label": element.label.cocoName,
                "confidence": Double(element.confidence),
                "reading_order": element.readingOrder,
                "bbox": [
                    "x": Double(element.bbox.x),
                    "y": Double(element.bbox.y),
                    "width": Double(element.bbox.width),
                    "height": Double(element.bbox.height)
                ]
            ]
        }

        let jsonDict: [String: Any] = [
            "count": elements.count,
            "annotations": annotationsArray
        ]

        do {
            let data = try JSONSerialization.data(withJSONObject: jsonDict, options: [.prettyPrinted, .sortedKeys])
            return String(data: data, encoding: .utf8) ?? "{}"
        } catch {
            return "{\"error\": \"Failed to serialize\"}"
        }
    }

    private func copyToClipboard() {
        let pasteboard = NSPasteboard.general
        pasteboard.clearContents()
        pasteboard.setString(jsonString, forType: .string)

        isCopied = true
        DispatchQueue.main.asyncAfter(deadline: .now() + 1.5) {
            isCopied = false
        }
    }
}

// MARK: - Multi-Selection View

/// View shown when multiple elements are selected
struct MultiSelectionView: View {
    @ObservedObject var viewModel: DocumentViewModel

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            // Selection summary
            let elements = viewModel.selectedElements
            let labelCounts = Dictionary(grouping: elements, by: { $0.label })
                .mapValues { $0.count }
                .sorted { $0.value > $1.value }

            // Label breakdown
            VStack(alignment: .leading, spacing: 4) {
                ForEach(labelCounts.prefix(3), id: \.key) { label, count in
                    HStack {
                        Circle()
                            .fill(label.swiftUIColor)
                            .frame(width: 8, height: 8)
                        Text(label.description)
                            .font(.caption)
                        Spacer()
                        Text("\(count)")
                            .font(.caption)
                            .foregroundColor(.secondary)
                            .monospacedDigit()
                    }
                }
                if labelCounts.count > 3 {
                    Text("+ \(labelCounts.count - 3) more types")
                        .font(.caption2)
                        .foregroundColor(.secondary)
                }
            }

            // Bounding box summary
            if !elements.isEmpty {
                let (minX, minY, maxX, maxY) = computeBoundingBox(elements)
                Divider()
                VStack(alignment: .leading, spacing: 4) {
                    Text("Union Bounding Box")
                        .font(.caption.bold())
                    Text(String(format: "%.0f × %.0f pt", maxX - minX, maxY - minY))
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .monospacedDigit()
                }
            }

            // Lock status and controls
            Divider()
            HStack {
                Text("Lock")
                    .font(.caption.bold())
                Spacer()
                let lockedCount = viewModel.selectedElementIds.intersection(viewModel.lockedElementIds).count
                if lockedCount > 0 {
                    Text("\(lockedCount)/\(elements.count) locked")
                        .font(.caption)
                        .foregroundColor(.orange)
                }
                Button(action: { viewModel.toggleLockSelectedElements() }) {
                    Image(systemName: viewModel.hasLockedSelection ? "lock.fill" : "lock.open")
                        .foregroundColor(viewModel.hasLockedSelection ? .orange : .secondary)
                }
                .buttonStyle(.borderless)
                .help(viewModel.hasLockedSelection ? "Unlock selected (⌘L)" : "Lock selected (⌘L)")
            }

            Divider()

            // Alignment controls
            VStack(alignment: .leading, spacing: 8) {
                Text("Align")
                    .font(.caption.bold())

                // Alignment buttons - row 1
                HStack(spacing: 4) {
                    ForEach([DocumentViewModel.AlignmentType.left,
                             .centerHorizontal,
                             .right], id: \.self) { alignType in
                        Button(action: { viewModel.alignSelectedElements(alignType) }) {
                            Image(systemName: alignType.systemImage)
                                .frame(width: 24, height: 24)
                        }
                        .buttonStyle(.bordered)
                        .help(alignType.description)
                        .disabled(!viewModel.canAlign)
                    }
                }

                // Alignment buttons - row 2
                HStack(spacing: 4) {
                    ForEach([DocumentViewModel.AlignmentType.top,
                             .centerVertical,
                             .bottom], id: \.self) { alignType in
                        Button(action: { viewModel.alignSelectedElements(alignType) }) {
                            Image(systemName: alignType.systemImage)
                                .frame(width: 24, height: 24)
                        }
                        .buttonStyle(.bordered)
                        .help(alignType.description)
                        .disabled(!viewModel.canAlign)
                    }
                }
            }

            // Distribution controls (only show when 3+ elements)
            if viewModel.selectedElementCount >= 3 {
                VStack(alignment: .leading, spacing: 8) {
                    Text("Distribute")
                        .font(.caption.bold())

                    HStack(spacing: 4) {
                        ForEach(DocumentViewModel.DistributionType.allCases) { distType in
                            Button(action: { viewModel.distributeSelectedElements(distType) }) {
                                Image(systemName: distType.systemImage)
                                    .frame(width: 24, height: 24)
                            }
                            .buttonStyle(.bordered)
                            .help(distType.description)
                            .disabled(!viewModel.canDistribute)
                        }
                    }
                }
            }

            // Match Size controls
            VStack(alignment: .leading, spacing: 8) {
                Text("Match Size")
                    .font(.caption.bold())

                HStack(spacing: 4) {
                    ForEach(DocumentViewModel.MatchSizeType.allCases) { sizeType in
                        Button(action: { viewModel.matchSizeOfSelectedElements(sizeType) }) {
                            Image(systemName: sizeType.systemImage)
                                .frame(width: 24, height: 24)
                        }
                        .buttonStyle(.bordered)
                        .help(sizeType.description)
                        .disabled(!viewModel.canMatchSize)
                    }
                }
            }

            Divider()

            // Action buttons
            HStack(spacing: 12) {
                // Merge button
                Button(action: { viewModel.mergeSelectedElements() }) {
                    Label("Merge", systemImage: "arrow.triangle.merge")
                }
                .buttonStyle(.borderedProminent)
                .disabled(!viewModel.canMergeSelection)

                // Clear selection button
                Button(action: { viewModel.clearSelection() }) {
                    Label("Clear", systemImage: "xmark")
                }
                .buttonStyle(.bordered)
            }
            .font(.caption)

            Divider()

            // JSON Preview for multiple elements
            DisclosureGroup("JSON Preview (\(viewModel.selectedElementCount) elements)") {
                MultiElementJSONPreviewView(elements: viewModel.selectedElements)
            }
        }
    }

    private func computeBoundingBox(_ elements: [Element]) -> (Float, Float, Float, Float) {
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

        return (minX, minY, maxX, maxY)
    }
}

// MARK: - Element Row

struct ElementRow: View {
    let element: Element
    let isSelected: Bool
    var colorByConfidence: Bool = false

    var body: some View {
        HStack {
            Circle()
                .fill(displayColor)
                .frame(width: 10, height: 10)
            VStack(alignment: .leading) {
                Text(element.label.description)
                    .font(.headline)
                HStack(spacing: 4) {
                    Text(String(format: "%.0f%%", element.confidence * 100))
                        .font(.caption)
                        .foregroundColor(confidenceColor(for: element.confidence))
                    if element.confidence < 0.5 {
                        Image(systemName: "exclamationmark.triangle.fill")
                            .font(.caption2)
                            .foregroundColor(.orange)
                    }
                }
            }
            Spacer()
            if element.hasReadingOrder {
                Text("#\(element.readingOrder)")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
        }
        .padding(.vertical, 2)
        .background(isSelected ? Color.accentColor.opacity(0.2) : Color.clear)
        .cornerRadius(4)
    }

    /// Display color: either label-based or confidence-based
    var displayColor: Color {
        if colorByConfidence {
            return confidenceColor(for: element.confidence)
        }
        return element.label.swiftUIColor
    }
}

// MARK: - Element Search Field

/// Search field for filtering elements by ID or label
struct ElementSearchField: View {
    @Binding var searchText: String
    let totalCount: Int
    let matchCount: Int

    var body: some View {
        HStack(spacing: 8) {
            Image(systemName: "magnifyingglass")
                .foregroundColor(.secondary)
            TextField("Search by ID or label...", text: $searchText)
                .textFieldStyle(.plain)
            if !searchText.isEmpty {
                // Show match count
                Text("\(matchCount)/\(totalCount)")
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .monospacedDigit()
                // Clear button
                Button(action: { searchText = "" }) {
                    Image(systemName: "xmark.circle.fill")
                        .foregroundColor(.secondary)
                }
                .buttonStyle(.plain)
            }
        }
        .padding(.horizontal, 8)
        .padding(.vertical, 4)
        .background(Color(NSColor.controlBackgroundColor))
        .cornerRadius(6)
    }
}
