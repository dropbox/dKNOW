// Dialogs - Modal dialog views for DoclingViz
// Contains label picker, split dialog, and go-to-page dialogs

import SwiftUI
import DoclingBridge

// MARK: - New Element Label Picker

/// Dialog for selecting a label when creating a new element
struct NewElementLabelPicker: View {
    @Binding var isPresented: Bool
    let onSelectLabel: (DocItemLabel) -> Void
    let onCancel: () -> Void

    var body: some View {
        VStack(spacing: 16) {
            Text("Select Label")
                .font(.headline)

            Text("Choose a label for the new element")
                .font(.subheadline)
                .foregroundColor(.secondary)

            // Label grid
            LazyVGrid(columns: [
                GridItem(.flexible()),
                GridItem(.flexible()),
                GridItem(.flexible())
            ], spacing: 8) {
                ForEach(DocItemLabel.allCases) { label in
                    LabelButton(label: label) {
                        onSelectLabel(label)
                        isPresented = false
                    }
                }
            }
            .padding(.vertical, 8)

            Divider()

            Button("Cancel") {
                onCancel()
                isPresented = false
            }
            .keyboardShortcut(.cancelAction)
        }
        .padding(20)
        .frame(width: 400)
    }
}

/// Button for selecting a label
struct LabelButton: View {
    let label: DocItemLabel
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack(spacing: 6) {
                Circle()
                    .fill(label.swiftUIColor)
                    .frame(width: 12, height: 12)
                Text(label.shortDescription)
                    .font(.caption)
                    .lineLimit(1)
            }
            .frame(maxWidth: .infinity)
            .padding(.horizontal, 8)
            .padding(.vertical, 6)
            .background(label.swiftUIColor.opacity(0.1))
            .cornerRadius(6)
            .overlay(
                RoundedRectangle(cornerRadius: 6)
                    .stroke(label.swiftUIColor.opacity(0.3), lineWidth: 1)
            )
        }
        .buttonStyle(.plain)
    }
}

// MARK: - Split Element Dialog

/// Dialog for configuring element split with position slider
struct SplitElementDialog: View {
    @ObservedObject var viewModel: DocumentViewModel
    @Binding var isPresented: Bool

    var body: some View {
        VStack(spacing: 20) {
            Text("Split Element")
                .font(.headline)

            if let element = viewModel.selectedElement {
                // Element info
                HStack {
                    Circle()
                        .fill(element.label.swiftUIColor)
                        .frame(width: 12, height: 12)
                    Text(element.label.description)
                        .font(.subheadline)
                    Spacer()
                    Text("ID: \(element.id)")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                .padding(.horizontal)

                Divider()

                // Direction picker
                VStack(alignment: .leading, spacing: 8) {
                    Text("Split Direction")
                        .font(.caption.bold())

                    Picker("Direction", selection: $viewModel.splitDirection) {
                        ForEach(SplitDirection.allCases) { direction in
                            Label(direction.description, systemImage: direction.systemImage)
                                .tag(direction)
                        }
                    }
                    .pickerStyle(.segmented)
                }
                .padding(.horizontal)

                // Position slider
                VStack(alignment: .leading, spacing: 8) {
                    HStack {
                        Text("Split Position")
                            .font(.caption.bold())
                        Spacer()
                        Text("\(Int(viewModel.splitPosition * 100))%")
                            .font(.caption)
                            .foregroundColor(.secondary)
                            .monospacedDigit()
                    }

                    Slider(value: $viewModel.splitPosition, in: 0.1...0.9, step: 0.05)

                    // Position labels
                    HStack {
                        Text(viewModel.splitDirection == .horizontal ? "Top" : "Left")
                            .font(.caption2)
                            .foregroundColor(.secondary)
                        Spacer()
                        Text(viewModel.splitDirection == .horizontal ? "Bottom" : "Right")
                            .font(.caption2)
                            .foregroundColor(.secondary)
                    }
                }
                .padding(.horizontal)

                // Preview info
                VStack(alignment: .leading, spacing: 4) {
                    Text("Preview")
                        .font(.caption.bold())

                    let (size1, size2) = computeSplitSizes(element: element)
                    HStack(spacing: 16) {
                        VStack {
                            Text("Part 1")
                                .font(.caption2)
                            Text(size1)
                                .font(.caption)
                                .monospacedDigit()
                        }
                        .frame(maxWidth: .infinity)
                        .padding(8)
                        .background(Color.blue.opacity(0.1))
                        .cornerRadius(4)

                        VStack {
                            Text("Part 2")
                                .font(.caption2)
                            Text(size2)
                                .font(.caption)
                                .monospacedDigit()
                        }
                        .frame(maxWidth: .infinity)
                        .padding(8)
                        .background(Color.green.opacity(0.1))
                        .cornerRadius(4)
                    }
                }
                .padding(.horizontal)

                Divider()

                // Action buttons
                HStack(spacing: 16) {
                    Button("Cancel") {
                        viewModel.cancelSplit()
                        isPresented = false
                    }
                    .keyboardShortcut(.cancelAction)

                    Button("Split") {
                        viewModel.splitSelectedElement()
                        isPresented = false
                    }
                    .keyboardShortcut(.defaultAction)
                    .buttonStyle(.borderedProminent)
                }
            } else {
                Text("No element selected")
                    .foregroundColor(.secondary)

                Button("Close") {
                    isPresented = false
                }
            }
        }
        .padding(24)
        .frame(width: 320)
    }

    private func computeSplitSizes(element: Element) -> (String, String) {
        let bbox = element.bbox
        let pos = viewModel.splitPosition

        switch viewModel.splitDirection {
        case .horizontal:
            let height1 = bbox.height * pos
            let height2 = bbox.height * (1 - pos)
            return (
                String(format: "%.0f × %.0f", bbox.width, height1),
                String(format: "%.0f × %.0f", bbox.width, height2)
            )
        case .vertical:
            let width1 = bbox.width * pos
            let width2 = bbox.width * (1 - pos)
            return (
                String(format: "%.0f × %.0f", width1, bbox.height),
                String(format: "%.0f × %.0f", width2, bbox.height)
            )
        }
    }
}

// MARK: - Go To Page View

struct GoToPageView: View {
    @Binding var pageNumber: String
    let totalPages: Int
    @Binding var isPresented: Bool
    let onGoToPage: (Int) -> Void

    @FocusState private var isTextFieldFocused: Bool

    var body: some View {
        VStack(spacing: 16) {
            Text("Go to Page")
                .font(.headline)

            HStack {
                TextField("Page", text: $pageNumber)
                    .textFieldStyle(.roundedBorder)
                    .frame(width: 80)
                    .focused($isTextFieldFocused)
                    .onSubmit {
                        navigateToPage()
                    }

                Text("of \(totalPages)")
                    .foregroundColor(.secondary)
            }

            HStack {
                Button("Cancel") {
                    isPresented = false
                }
                .keyboardShortcut(.cancelAction)

                Button("Go") {
                    navigateToPage()
                }
                .keyboardShortcut(.defaultAction)
                .disabled(!isValidPageNumber)
            }
        }
        .padding(24)
        .frame(width: 200)
        .onAppear {
            isTextFieldFocused = true
        }
    }

    private var isValidPageNumber: Bool {
        guard let page = Int(pageNumber) else { return false }
        return page >= 1 && page <= totalPages
    }

    private func navigateToPage() {
        if let page = Int(pageNumber), page >= 1, page <= totalPages {
            onGoToPage(page - 1)  // Convert to 0-indexed
            isPresented = false
        }
    }
}

// MARK: - Keyboard Shortcuts Help View

/// Model for a keyboard shortcut entry
struct KeyboardShortcutItem: Identifiable {
    let id = UUID()
    let name: String
    let shortcut: String
    let description: String?

    init(_ name: String, _ shortcut: String, description: String? = nil) {
        self.name = name
        self.shortcut = shortcut
        self.description = description
    }
}

/// Category of keyboard shortcuts
struct ShortcutCategory: Identifiable {
    let id = UUID()
    let name: String
    let icon: String
    let shortcuts: [KeyboardShortcutItem]
}

/// Dialog showing all keyboard shortcuts organized by category
struct KeyboardShortcutsView: View {
    @Binding var isPresented: Bool
    @State private var searchText: String = ""

    // All shortcut categories
    private let categories: [ShortcutCategory] = [
        ShortcutCategory(
            name: "File",
            icon: "doc",
            shortcuts: [
                KeyboardShortcutItem("Open PDF", "Cmd+O"),
                KeyboardShortcutItem("Export COCO (All Pages)", "Cmd+Shift+E"),
                KeyboardShortcutItem("Export COCO (Current Page)", "Cmd+E"),
                KeyboardShortcutItem("Export COCO with Images", "Cmd+Option+E"),
                KeyboardShortcutItem("Export JSON", "Cmd+Shift+J"),
            ]
        ),
        ShortcutCategory(
            name: "View",
            icon: "eye",
            shortcuts: [
                KeyboardShortcutItem("Toggle Bounding Boxes", "Cmd+B"),
                KeyboardShortcutItem("Toggle Text Cells", "Cmd+T"),
                KeyboardShortcutItem("Toggle Snap to Grid", "Cmd+'"),
                KeyboardShortcutItem("Toggle Alignment Guides", "Cmd+;"),
                KeyboardShortcutItem("Zoom In", "Cmd++"),
                KeyboardShortcutItem("Zoom Out", "Cmd+-"),
                KeyboardShortcutItem("Actual Size (100%)", "Cmd+0"),
                KeyboardShortcutItem("Fit to Window", "Cmd+9"),
                KeyboardShortcutItem("Zoom to Selection", "Cmd+3"),
            ]
        ),
        ShortcutCategory(
            name: "Edit",
            icon: "pencil",
            shortcuts: [
                KeyboardShortcutItem("Undo", "Cmd+Z"),
                KeyboardShortcutItem("Redo", "Cmd+Shift+Z"),
                KeyboardShortcutItem("Cut", "Cmd+X"),
                KeyboardShortcutItem("Copy", "Cmd+C"),
                KeyboardShortcutItem("Paste", "Cmd+V"),
                KeyboardShortcutItem("Duplicate", "Cmd+D"),
                KeyboardShortcutItem("Delete", "Delete"),
            ]
        ),
        ShortcutCategory(
            name: "Selection",
            icon: "cursorarrow.click.2",
            shortcuts: [
                KeyboardShortcutItem("Select All", "Cmd+A"),
                KeyboardShortcutItem("Deselect", "Escape"),
                KeyboardShortcutItem("Select Next Element", "Tab"),
                KeyboardShortcutItem("Select Previous Element", "Shift+Tab"),
                KeyboardShortcutItem("Draw Mode", "D", description: "Draw new elements"),
                KeyboardShortcutItem("Lasso Selection", "L", description: "Freeform selection"),
                KeyboardShortcutItem("Marquee Selection", "M", description: "Rectangle selection"),
            ]
        ),
        ShortcutCategory(
            name: "Element Operations",
            icon: "rectangle.on.rectangle",
            shortcuts: [
                KeyboardShortcutItem("Merge Elements", "Cmd+Shift+M", description: "Combine selected elements"),
                KeyboardShortcutItem("Split Element Dialog", "Option+S"),
                KeyboardShortcutItem("Split Horizontally", "Cmd+Shift+H"),
                KeyboardShortcutItem("Split Vertically", "Cmd+Shift+V"),
                KeyboardShortcutItem("Nudge Up", "Option+Up"),
                KeyboardShortcutItem("Nudge Down", "Option+Down"),
                KeyboardShortcutItem("Nudge Left", "Option+Left"),
                KeyboardShortcutItem("Nudge Right", "Option+Right"),
            ]
        ),
        ShortcutCategory(
            name: "Lock",
            icon: "lock",
            shortcuts: [
                KeyboardShortcutItem("Toggle Lock", "Cmd+L"),
                KeyboardShortcutItem("Lock All Elements", "Cmd+Option+L"),
                KeyboardShortcutItem("Unlock All Elements", "Cmd+Shift+L"),
            ]
        ),
        ShortcutCategory(
            name: "Alignment",
            icon: "align.horizontal.left",
            shortcuts: [
                KeyboardShortcutItem("Align Left", "Cmd+Option+["),
                KeyboardShortcutItem("Align Right", "Cmd+Option+]"),
                KeyboardShortcutItem("Align Top", "Cmd+Shift+Option+["),
                KeyboardShortcutItem("Align Bottom", "Cmd+Shift+Option+]"),
                KeyboardShortcutItem("Distribute Horizontally", "Cmd+Option+\\"),
                KeyboardShortcutItem("Distribute Vertically", "Cmd+Shift+Option+\\"),
            ]
        ),
        ShortcutCategory(
            name: "Match Size",
            icon: "aspectratio",
            shortcuts: [
                KeyboardShortcutItem("Match Width", "Cmd+Ctrl+W"),
                KeyboardShortcutItem("Match Height", "Cmd+Ctrl+H"),
                KeyboardShortcutItem("Match Both", "Cmd+Ctrl+B"),
            ]
        ),
        ShortcutCategory(
            name: "Quick Labels",
            icon: "tag",
            shortcuts: [
                KeyboardShortcutItem("Text", "1"),
                KeyboardShortcutItem("Title", "2"),
                KeyboardShortcutItem("Section Header", "3"),
                KeyboardShortcutItem("Table", "4"),
                KeyboardShortcutItem("Picture", "5"),
                KeyboardShortcutItem("List Item", "6"),
                KeyboardShortcutItem("Caption", "7"),
                KeyboardShortcutItem("Footnote", "8"),
                KeyboardShortcutItem("Code", "9"),
            ]
        ),
        ShortcutCategory(
            name: "Playback",
            icon: "play",
            shortcuts: [
                KeyboardShortcutItem("Play/Pause", "Space"),
                KeyboardShortcutItem("Previous Stage", "Left Arrow"),
                KeyboardShortcutItem("Next Stage", "Right Arrow"),
                KeyboardShortcutItem("First Stage", "Cmd+Left Arrow"),
                KeyboardShortcutItem("Last Stage", "Cmd+Right Arrow"),
            ]
        ),
        ShortcutCategory(
            name: "Navigation",
            icon: "arrow.up.arrow.down",
            shortcuts: [
                KeyboardShortcutItem("Previous Page", "Up Arrow"),
                KeyboardShortcutItem("Next Page", "Down Arrow"),
                KeyboardShortcutItem("Go to Page", "Cmd+G"),
            ]
        ),
        ShortcutCategory(
            name: "Corrections",
            icon: "checkmark.circle",
            shortcuts: [
                KeyboardShortcutItem("Save Corrections", "Cmd+Shift+S"),
                KeyboardShortcutItem("Load Corrections", "Cmd+Shift+L"),
            ]
        ),
    ]

    /// Filtered categories based on search text
    private var filteredCategories: [ShortcutCategory] {
        guard !searchText.isEmpty else { return categories }
        let lowercasedSearch = searchText.lowercased()

        return categories.compactMap { category in
            let filteredShortcuts = category.shortcuts.filter { shortcut in
                shortcut.name.lowercased().contains(lowercasedSearch) ||
                shortcut.shortcut.lowercased().contains(lowercasedSearch) ||
                (shortcut.description?.lowercased().contains(lowercasedSearch) ?? false)
            }
            if filteredShortcuts.isEmpty {
                return nil
            }
            return ShortcutCategory(name: category.name, icon: category.icon, shortcuts: filteredShortcuts)
        }
    }

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                Image(systemName: "keyboard")
                    .font(.title2)
                    .foregroundColor(.accentColor)
                Text("Keyboard Shortcuts")
                    .font(.title2)
                    .fontWeight(.semibold)
                Spacer()
                Button(action: { isPresented = false }) {
                    Image(systemName: "xmark.circle.fill")
                        .font(.title2)
                        .foregroundColor(.secondary)
                }
                .buttonStyle(.plain)
            }
            .padding()

            // Search field
            HStack {
                Image(systemName: "magnifyingglass")
                    .foregroundColor(.secondary)
                TextField("Search shortcuts...", text: $searchText)
                    .textFieldStyle(.plain)
                if !searchText.isEmpty {
                    Button(action: { searchText = "" }) {
                        Image(systemName: "xmark.circle.fill")
                            .foregroundColor(.secondary)
                    }
                    .buttonStyle(.plain)
                }
            }
            .padding(8)
            .background(Color(NSColor.controlBackgroundColor))
            .cornerRadius(8)
            .padding(.horizontal)
            .padding(.bottom, 8)

            Divider()

            // Shortcuts list
            ScrollView {
                LazyVStack(alignment: .leading, spacing: 16) {
                    ForEach(filteredCategories) { category in
                        ShortcutCategoryView(category: category)
                    }
                }
                .padding()
            }

            Divider()

            // Footer with tips
            HStack {
                Image(systemName: "lightbulb")
                    .foregroundColor(.orange)
                Text("Tip: Hold Cmd or Shift while clicking to multi-select elements")
                    .font(.caption)
                    .foregroundColor(.secondary)
                Spacer()
                Button("Close") {
                    isPresented = false
                }
                .keyboardShortcut(.cancelAction)
            }
            .padding()
        }
        .frame(width: 520, height: 600)
    }
}

/// View for a category of shortcuts
struct ShortcutCategoryView: View {
    let category: ShortcutCategory

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            // Category header
            HStack(spacing: 6) {
                Image(systemName: category.icon)
                    .foregroundColor(.accentColor)
                    .frame(width: 20)
                Text(category.name)
                    .font(.headline)
            }

            // Shortcuts grid
            LazyVGrid(columns: [
                GridItem(.flexible()),
                GridItem(.flexible())
            ], alignment: .leading, spacing: 6) {
                ForEach(category.shortcuts) { shortcut in
                    ShortcutRowView(shortcut: shortcut)
                }
            }
        }
        .padding(.vertical, 4)
    }
}

/// View for a single shortcut row
struct ShortcutRowView: View {
    let shortcut: KeyboardShortcutItem

    var body: some View {
        HStack(spacing: 8) {
            Text(shortcut.name)
                .font(.subheadline)
                .lineLimit(1)
            Spacer()
            Text(shortcut.shortcut)
                .font(.system(.caption, design: .monospaced))
                .padding(.horizontal, 6)
                .padding(.vertical, 2)
                .background(Color(NSColor.controlBackgroundColor))
                .cornerRadius(4)
                .overlay(
                    RoundedRectangle(cornerRadius: 4)
                        .stroke(Color(NSColor.separatorColor), lineWidth: 0.5)
                )
        }
        .help(shortcut.description ?? shortcut.name)
    }
}
