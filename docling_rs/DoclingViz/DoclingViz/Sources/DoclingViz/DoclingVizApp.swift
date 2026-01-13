// DoclingViz - PDF Extraction Visualization App
// Native macOS application for ML pipeline debugging

import SwiftUI
import DoclingBridge

@main
struct DoclingVizApp: App {
    @StateObject private var appState = AppState()
    @StateObject private var recentFilesManager = RecentFilesManager.shared
    @State private var showAbout = false

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environmentObject(appState)
                .environmentObject(recentFilesManager)
                .sheet(isPresented: $showAbout) {
                    AboutView(appState: appState)
                }
        }
        .commands {
            // About command
            CommandGroup(replacing: .appInfo) {
                Button("About DoclingViz") {
                    showAbout = true
                }
            }
            CommandGroup(replacing: .newItem) {
                Button("Open PDF...") {
                    NotificationCenter.default.post(name: .openPDF, object: nil)
                }
                .keyboardShortcut("o", modifiers: .command)

                // Recent Files submenu
                Menu("Open Recent") {
                    if recentFilesManager.recentFiles.isEmpty {
                        Text("No Recent Documents")
                            .foregroundColor(.secondary)
                    } else {
                        ForEach(recentFilesManager.recentFiles, id: \.self) { url in
                            Button(url.lastPathComponent) {
                                NotificationCenter.default.post(name: .openRecentPDF, object: url)
                            }
                        }
                        Divider()
                        Button("Clear Recent") {
                            recentFilesManager.clearRecentFiles()
                        }
                    }
                }

                Divider()

                // Export submenu
                Menu("Export") {
                    Button("COCO Format (All Pages)...") {
                        NotificationCenter.default.post(name: .exportCOCOAll, object: nil)
                    }
                    .keyboardShortcut("e", modifiers: [.command, .shift])

                    Button("COCO Format (Current Page)...") {
                        NotificationCenter.default.post(name: .exportCOCOPage, object: nil)
                    }
                    .keyboardShortcut("e", modifiers: .command)

                    Divider()

                    Button("COCO with Images...") {
                        NotificationCenter.default.post(name: .exportCOCOWithImages, object: nil)
                    }
                    .keyboardShortcut("e", modifiers: [.command, .option])

                    Divider()

                    Button("JSON Format...") {
                        NotificationCenter.default.post(name: .exportJSON, object: nil)
                    }
                    .keyboardShortcut("j", modifiers: [.command, .shift])

                    Button("YOLO Format...") {
                        NotificationCenter.default.post(name: .exportYOLO, object: nil)
                    }
                    .keyboardShortcut("y", modifiers: [.command, .shift])

                    Divider()

                    Button("Validation Report (Markdown)...") {
                        NotificationCenter.default.post(name: .exportValidationMarkdown, object: nil)
                    }

                    Button("Validation Report (JSON)...") {
                        NotificationCenter.default.post(name: .exportValidationJSON, object: nil)
                    }
                }

                // Import submenu
                Menu("Import") {
                    Button("COCO Annotations...") {
                        NotificationCenter.default.post(name: .importCOCO, object: nil)
                    }
                    .keyboardShortcut("i", modifiers: [.command, .shift])
                }
            }

            // View commands
            CommandGroup(after: .sidebar) {
                Button("Toggle Bounding Boxes") {
                    NotificationCenter.default.post(name: .toggleBoundingBoxes, object: nil)
                }
                .keyboardShortcut("b", modifiers: .command)

                Button("Toggle Text Cells") {
                    NotificationCenter.default.post(name: .toggleTextCells, object: nil)
                }
                .keyboardShortcut("t", modifiers: .command)

                Button("Toggle Snap to Grid") {
                    NotificationCenter.default.post(name: .toggleSnapToGrid, object: nil)
                }
                .keyboardShortcut("'", modifiers: .command)

                Button("Toggle Alignment Guides") {
                    NotificationCenter.default.post(name: .toggleAlignmentGuides, object: nil)
                }
                .keyboardShortcut(";", modifiers: .command)

                Divider()

                Button("Zoom In") {
                    NotificationCenter.default.post(name: .zoomIn, object: nil)
                }
                .keyboardShortcut("+", modifiers: .command)

                Button("Zoom Out") {
                    NotificationCenter.default.post(name: .zoomOut, object: nil)
                }
                .keyboardShortcut("-", modifiers: .command)

                Button("Actual Size") {
                    NotificationCenter.default.post(name: .zoomActualSize, object: nil)
                }
                .keyboardShortcut("0", modifiers: .command)

                Button("Fit to Window") {
                    NotificationCenter.default.post(name: .zoomFitToWindow, object: nil)
                }
                .keyboardShortcut("9", modifiers: .command)

                Button("Zoom to Selection") {
                    NotificationCenter.default.post(name: .zoomToSelection, object: nil)
                }
                .keyboardShortcut("3", modifiers: .command)
            }

            // Edit commands (Undo/Redo)
            CommandGroup(replacing: .undoRedo) {
                Button("Undo") {
                    NotificationCenter.default.post(name: .undoEdit, object: nil)
                }
                .keyboardShortcut("z", modifiers: .command)

                Button("Redo") {
                    NotificationCenter.default.post(name: .redoEdit, object: nil)
                }
                .keyboardShortcut("z", modifiers: [.command, .shift])

                Divider()

                Button("Cut") {
                    NotificationCenter.default.post(name: .cutElements, object: nil)
                }
                .keyboardShortcut("x", modifiers: .command)

                Button("Copy") {
                    NotificationCenter.default.post(name: .copyElements, object: nil)
                }
                .keyboardShortcut("c", modifiers: .command)

                Button("Paste") {
                    NotificationCenter.default.post(name: .pasteElements, object: nil)
                }
                .keyboardShortcut("v", modifiers: .command)

                Divider()

                Button("Draw New Element") {
                    NotificationCenter.default.post(name: .toggleDrawMode, object: nil)
                }
                .keyboardShortcut("d", modifiers: [])

                Button("Lasso Selection") {
                    NotificationCenter.default.post(name: .toggleLassoMode, object: nil)
                }
                .keyboardShortcut("l", modifiers: [])

                Button("Marquee Selection") {
                    NotificationCenter.default.post(name: .toggleMarqueeMode, object: nil)
                }
                .keyboardShortcut("m", modifiers: [])

                Button("Delete Selected Element") {
                    NotificationCenter.default.post(name: .deleteElement, object: nil)
                }
                .keyboardShortcut(.delete, modifiers: [])

                Button("Duplicate Selected Element") {
                    NotificationCenter.default.post(name: .duplicateElement, object: nil)
                }
                .keyboardShortcut("d", modifiers: .command)

                Button("Merge Selected Elements") {
                    NotificationCenter.default.post(name: .mergeElements, object: nil)
                }
                .keyboardShortcut("m", modifiers: [.command, .shift])

                Divider()

                Button("Toggle Lock") {
                    NotificationCenter.default.post(name: .toggleLock, object: nil)
                }
                .keyboardShortcut("l", modifiers: .command)

                Button("Lock All Elements") {
                    NotificationCenter.default.post(name: .lockAll, object: nil)
                }
                .keyboardShortcut("l", modifiers: [.command, .option])

                Button("Unlock All Elements") {
                    NotificationCenter.default.post(name: .unlockAll, object: nil)
                }
                .keyboardShortcut("l", modifiers: [.command, .shift])

                Divider()

                // Alignment submenu
                Menu("Align") {
                    Button("Align Left") {
                        NotificationCenter.default.post(name: .alignLeft, object: nil)
                    }
                    .keyboardShortcut("[", modifiers: [.command, .option])

                    Button("Align Center Horizontal") {
                        NotificationCenter.default.post(name: .alignCenterH, object: nil)
                    }

                    Button("Align Right") {
                        NotificationCenter.default.post(name: .alignRight, object: nil)
                    }
                    .keyboardShortcut("]", modifiers: [.command, .option])

                    Divider()

                    Button("Align Top") {
                        NotificationCenter.default.post(name: .alignTop, object: nil)
                    }
                    .keyboardShortcut("[", modifiers: [.command, .shift, .option])

                    Button("Align Center Vertical") {
                        NotificationCenter.default.post(name: .alignCenterV, object: nil)
                    }

                    Button("Align Bottom") {
                        NotificationCenter.default.post(name: .alignBottom, object: nil)
                    }
                    .keyboardShortcut("]", modifiers: [.command, .shift, .option])

                    Divider()

                    Button("Distribute Horizontally") {
                        NotificationCenter.default.post(name: .distributeH, object: nil)
                    }
                    .keyboardShortcut("\\", modifiers: [.command, .option])

                    Button("Distribute Vertically") {
                        NotificationCenter.default.post(name: .distributeV, object: nil)
                    }
                    .keyboardShortcut("\\", modifiers: [.command, .shift, .option])
                }

                // Match Size submenu
                Menu("Match Size") {
                    Button("Match Width") {
                        NotificationCenter.default.post(name: .matchWidth, object: nil)
                    }
                    .keyboardShortcut("w", modifiers: [.command, .control])

                    Button("Match Height") {
                        NotificationCenter.default.post(name: .matchHeight, object: nil)
                    }
                    .keyboardShortcut("h", modifiers: [.command, .control])

                    Button("Match Both") {
                        NotificationCenter.default.post(name: .matchBoth, object: nil)
                    }
                    .keyboardShortcut("b", modifiers: [.command, .control])

                    Divider()

                    Button("Match Width (Smallest)") {
                        NotificationCenter.default.post(name: .matchWidthSmallest, object: nil)
                    }

                    Button("Match Height (Smallest)") {
                        NotificationCenter.default.post(name: .matchHeightSmallest, object: nil)
                    }
                }

                Divider()

                Button("Split Horizontally") {
                    NotificationCenter.default.post(name: .splitHorizontal, object: nil)
                }
                .keyboardShortcut("h", modifiers: [.command, .shift])

                Button("Split Vertically") {
                    NotificationCenter.default.post(name: .splitVertical, object: nil)
                }
                .keyboardShortcut("v", modifiers: [.command, .shift])

                Button("Split Element...") {
                    NotificationCenter.default.post(name: .splitElement, object: nil)
                }
                .keyboardShortcut("s", modifiers: .option)

                Divider()

                // Nudge element with arrow keys
                Button("Nudge Up") {
                    NotificationCenter.default.post(name: .nudgeUp, object: nil)
                }
                .keyboardShortcut(.upArrow, modifiers: .option)

                Button("Nudge Down") {
                    NotificationCenter.default.post(name: .nudgeDown, object: nil)
                }
                .keyboardShortcut(.downArrow, modifiers: .option)

                Button("Nudge Left") {
                    NotificationCenter.default.post(name: .nudgeLeft, object: nil)
                }
                .keyboardShortcut(.leftArrow, modifiers: .option)

                Button("Nudge Right") {
                    NotificationCenter.default.post(name: .nudgeRight, object: nil)
                }
                .keyboardShortcut(.rightArrow, modifiers: .option)

                Divider()

                Button("Deselect") {
                    NotificationCenter.default.post(name: .deselect, object: nil)
                }
                .keyboardShortcut(.escape, modifiers: [])

                Button("Select All") {
                    NotificationCenter.default.post(name: .selectAll, object: nil)
                }
                .keyboardShortcut("a", modifiers: .command)

                Button("Select Next Element") {
                    NotificationCenter.default.post(name: .selectNext, object: nil)
                }
                .keyboardShortcut(.tab, modifiers: [])

                Button("Select Previous Element") {
                    NotificationCenter.default.post(name: .selectPrevious, object: nil)
                }
                .keyboardShortcut(.tab, modifiers: .shift)

                Divider()

                // Quick Label submenu with number keys
                Menu("Quick Labels") {
                    Button("1: Text") {
                        NotificationCenter.default.post(name: .quickLabel1, object: nil)
                    }
                    .keyboardShortcut("1", modifiers: [])

                    Button("2: Title") {
                        NotificationCenter.default.post(name: .quickLabel2, object: nil)
                    }
                    .keyboardShortcut("2", modifiers: [])

                    Button("3: Section Header") {
                        NotificationCenter.default.post(name: .quickLabel3, object: nil)
                    }
                    .keyboardShortcut("3", modifiers: [])

                    Button("4: Table") {
                        NotificationCenter.default.post(name: .quickLabel4, object: nil)
                    }
                    .keyboardShortcut("4", modifiers: [])

                    Button("5: Picture") {
                        NotificationCenter.default.post(name: .quickLabel5, object: nil)
                    }
                    .keyboardShortcut("5", modifiers: [])

                    Button("6: List Item") {
                        NotificationCenter.default.post(name: .quickLabel6, object: nil)
                    }
                    .keyboardShortcut("6", modifiers: [])

                    Button("7: Caption") {
                        NotificationCenter.default.post(name: .quickLabel7, object: nil)
                    }
                    .keyboardShortcut("7", modifiers: [])

                    Button("8: Footnote") {
                        NotificationCenter.default.post(name: .quickLabel8, object: nil)
                    }
                    .keyboardShortcut("8", modifiers: [])

                    Button("9: Code") {
                        NotificationCenter.default.post(name: .quickLabel9, object: nil)
                    }
                    .keyboardShortcut("9", modifiers: [])
                }

                Divider()

                Button("Discard All Edits") {
                    NotificationCenter.default.post(name: .discardEdits, object: nil)
                }
            }

            // Corrections commands
            CommandMenu("Corrections") {
                Button("Save Corrections...") {
                    NotificationCenter.default.post(name: .saveCorrections, object: nil)
                }
                .keyboardShortcut("s", modifiers: [.command, .shift])

                Button("Load Corrections...") {
                    NotificationCenter.default.post(name: .loadCorrections, object: nil)
                }
                .keyboardShortcut("l", modifiers: [.command, .shift])
            }

            // Playback commands
            CommandMenu("Playback") {
                Button("Play/Pause") {
                    NotificationCenter.default.post(name: .togglePlayback, object: nil)
                }
                .keyboardShortcut(" ", modifiers: [])

                Divider()

                Button("Previous Stage") {
                    NotificationCenter.default.post(name: .previousStage, object: nil)
                }
                .keyboardShortcut(.leftArrow, modifiers: [])

                Button("Next Stage") {
                    NotificationCenter.default.post(name: .nextStage, object: nil)
                }
                .keyboardShortcut(.rightArrow, modifiers: [])

                Button("First Stage") {
                    NotificationCenter.default.post(name: .firstStage, object: nil)
                }
                .keyboardShortcut(.leftArrow, modifiers: .command)

                Button("Last Stage") {
                    NotificationCenter.default.post(name: .lastStage, object: nil)
                }
                .keyboardShortcut(.rightArrow, modifiers: .command)

                Divider()

                Button("Previous Page") {
                    NotificationCenter.default.post(name: .previousPage, object: nil)
                }
                .keyboardShortcut(.upArrow, modifiers: [])

                Button("Next Page") {
                    NotificationCenter.default.post(name: .nextPage, object: nil)
                }
                .keyboardShortcut(.downArrow, modifiers: [])

                Divider()

                Button("Go to Page...") {
                    NotificationCenter.default.post(name: .goToPage, object: nil)
                }
                .keyboardShortcut("g", modifiers: .command)
            }

            // Help commands
            CommandGroup(replacing: .help) {
                Button("Keyboard Shortcuts") {
                    NotificationCenter.default.post(name: .showKeyboardShortcuts, object: nil)
                }
                .keyboardShortcut("/", modifiers: .command)
            }
        }
    }
}

// MARK: - App State

@MainActor
class AppState: ObservableObject {
    @Published var pipeline: DoclingPipeline?
    @Published var error: String?
    @Published var hasPdfRender: Bool = false
    @Published var hasPdfMl: Bool = false

    init() {
        // Check feature availability
        hasPdfRender = DoclingPipeline.hasPdfRender
        hasPdfMl = DoclingPipeline.hasPdfMl

        do {
            pipeline = try DoclingPipeline()
        } catch {
            self.error = "Failed to initialize pipeline: \(error)"
        }
    }

    /// User-friendly message about available features
    var featureStatusMessage: String {
        if hasPdfMl {
            return "Full ML pipeline available"
        } else if hasPdfRender {
            return "PDF rendering only (ML not available)"
        } else {
            return "Viewing mode only (pipeline not available)"
        }
    }
}

// MARK: - About View

struct AboutView: View {
    let appState: AppState
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        VStack(spacing: 16) {
            Image(systemName: "doc.viewfinder")
                .font(.system(size: 64))
                .foregroundColor(.accentColor)

            Text("DoclingViz")
                .font(.title)
                .bold()

            Text("PDF Extraction Visualizer")
                .font(.subheadline)
                .foregroundColor(.secondary)

            Divider()
                .frame(width: 200)

            VStack(alignment: .leading, spacing: 8) {
                LabeledContent("Bridge Version") {
                    Text(DoclingPipeline.version)
                        .font(.system(.body, design: .monospaced))
                }

                LabeledContent("Pipeline Stages") {
                    Text("\(DoclingPipeline.stageCount)")
                }

                LabeledContent("PDF Rendering") {
                    HStack {
                        Circle()
                            .fill(appState.hasPdfRender ? Color.green : Color.red)
                            .frame(width: 8, height: 8)
                        Text(appState.hasPdfRender ? "Available" : "Not Available")
                    }
                }

                LabeledContent("ML Pipeline") {
                    HStack {
                        Circle()
                            .fill(appState.hasPdfMl ? Color.green : Color.red)
                            .frame(width: 8, height: 8)
                        Text(appState.hasPdfMl ? "Available" : "Not Available")
                    }
                }
            }
            .padding(.horizontal, 20)

            Spacer()

            Button("Close") {
                dismiss()
            }
            .keyboardShortcut(.defaultAction)
        }
        .padding(24)
        .frame(width: 320, height: 360)
    }
}

// MARK: - Notifications

extension Notification.Name {
    // File operations
    static let openPDF = Notification.Name("openPDF")
    static let openRecentPDF = Notification.Name("openRecentPDF")

    // Edit operations
    static let undoEdit = Notification.Name("undoEdit")
    static let redoEdit = Notification.Name("redoEdit")
    static let discardEdits = Notification.Name("discardEdits")
    static let deleteElement = Notification.Name("deleteElement")
    static let duplicateElement = Notification.Name("duplicateElement")
    static let toggleDrawMode = Notification.Name("toggleDrawMode")
    static let toggleLassoMode = Notification.Name("toggleLassoMode")
    static let toggleMarqueeMode = Notification.Name("toggleMarqueeMode")
    static let deselect = Notification.Name("deselect")
    static let selectAll = Notification.Name("selectAll")
    static let selectNext = Notification.Name("selectNext")
    static let selectPrevious = Notification.Name("selectPrevious")

    // Clipboard operations
    static let copyElements = Notification.Name("copyElements")
    static let cutElements = Notification.Name("cutElements")
    static let pasteElements = Notification.Name("pasteElements")

    // Merge operations
    static let mergeElements = Notification.Name("mergeElements")

    // Lock operations
    static let toggleLock = Notification.Name("toggleLock")
    static let lockAll = Notification.Name("lockAll")
    static let unlockAll = Notification.Name("unlockAll")

    // Alignment operations
    static let alignLeft = Notification.Name("alignLeft")
    static let alignRight = Notification.Name("alignRight")
    static let alignTop = Notification.Name("alignTop")
    static let alignBottom = Notification.Name("alignBottom")
    static let alignCenterH = Notification.Name("alignCenterH")
    static let alignCenterV = Notification.Name("alignCenterV")
    static let distributeH = Notification.Name("distributeH")
    static let distributeV = Notification.Name("distributeV")

    // Match size operations
    static let matchWidth = Notification.Name("matchWidth")
    static let matchHeight = Notification.Name("matchHeight")
    static let matchBoth = Notification.Name("matchBoth")
    static let matchWidthSmallest = Notification.Name("matchWidthSmallest")
    static let matchHeightSmallest = Notification.Name("matchHeightSmallest")

    // Quick label shortcuts (number keys)
    static let quickLabel1 = Notification.Name("quickLabel1")  // Text
    static let quickLabel2 = Notification.Name("quickLabel2")  // Title
    static let quickLabel3 = Notification.Name("quickLabel3")  // Section Header
    static let quickLabel4 = Notification.Name("quickLabel4")  // Table
    static let quickLabel5 = Notification.Name("quickLabel5")  // Picture
    static let quickLabel6 = Notification.Name("quickLabel6")  // List Item
    static let quickLabel7 = Notification.Name("quickLabel7")  // Caption
    static let quickLabel8 = Notification.Name("quickLabel8")  // Footnote
    static let quickLabel9 = Notification.Name("quickLabel9")  // Code

    // Split operations
    static let splitElement = Notification.Name("splitElement")
    static let splitHorizontal = Notification.Name("splitHorizontal")
    static let splitVertical = Notification.Name("splitVertical")

    // Nudge operations (fine positioning)
    static let nudgeUp = Notification.Name("nudgeUp")
    static let nudgeDown = Notification.Name("nudgeDown")
    static let nudgeLeft = Notification.Name("nudgeLeft")
    static let nudgeRight = Notification.Name("nudgeRight")

    // Corrections operations
    static let saveCorrections = Notification.Name("saveCorrections")
    static let loadCorrections = Notification.Name("loadCorrections")

    // Export operations
    static let exportCOCOAll = Notification.Name("exportCOCOAll")
    static let exportCOCOPage = Notification.Name("exportCOCOPage")
    static let exportCOCOWithImages = Notification.Name("exportCOCOWithImages")
    static let exportJSON = Notification.Name("exportJSON")
    static let exportValidationMarkdown = Notification.Name("exportValidationMarkdown")
    static let exportValidationJSON = Notification.Name("exportValidationJSON")
    static let exportYOLO = Notification.Name("exportYOLO")

    // Import operations
    static let importCOCO = Notification.Name("importCOCO")

    // View toggles
    static let toggleBoundingBoxes = Notification.Name("toggleBoundingBoxes")
    static let toggleTextCells = Notification.Name("toggleTextCells")
    static let toggleSnapToGrid = Notification.Name("toggleSnapToGrid")
    static let toggleAlignmentGuides = Notification.Name("toggleAlignmentGuides")

    // Playback controls
    static let togglePlayback = Notification.Name("togglePlayback")
    static let previousStage = Notification.Name("previousStage")
    static let nextStage = Notification.Name("nextStage")
    static let firstStage = Notification.Name("firstStage")
    static let lastStage = Notification.Name("lastStage")

    // Page navigation
    static let previousPage = Notification.Name("previousPage")
    static let nextPage = Notification.Name("nextPage")
    static let goToPage = Notification.Name("goToPage")

    // Zoom controls
    static let zoomIn = Notification.Name("zoomIn")
    static let zoomOut = Notification.Name("zoomOut")
    static let zoomActualSize = Notification.Name("zoomActualSize")
    static let zoomFitToWindow = Notification.Name("zoomFitToWindow")
    static let zoomToSelection = Notification.Name("zoomToSelection")

    // Help
    static let showKeyboardShortcuts = Notification.Name("showKeyboardShortcuts")
}
