// BatchViewModel.swift
// Manages batch processing state and FFI interaction

import SwiftUI
import DoclingBridge
import Combine
import CDoclingBridge
import AppKit

/// Playback speed options for batch visualization
public enum PlaybackSpeed: Double, CaseIterable, Identifiable {
    case slow10x = 0.1    // 10x slower - presentation mode
    case slow5x = 0.2     // 5x slower - demo mode
    case slow2x = 0.5     // 2x slower - see details
    case realtime = 1.0   // Actual processing speed
    case fast2x = 2.0     // Skip frames
    case fast10x = 10.0   // Heavy frame skip

    public var id: Double { rawValue }

    public var label: String {
        switch self {
        case .slow10x: return "0.1x"
        case .slow5x: return "0.2x"
        case .slow2x: return "0.5x"
        case .realtime: return "1x"
        case .fast2x: return "2x"
        case .fast10x: return "10x"
        }
    }
}

/// ViewModel for batch processing view
@MainActor
public class BatchViewModel: ObservableObject {
    // MARK: - Published State

    /// All documents in the batch
    @Published public var documents: [BatchDocument] = []

    /// Currently active (processing or selected) document ID
    @Published public var activeDocId: Int?

    /// Is batch processing running
    @Published public var isRunning = false

    /// Is batch processing paused
    @Published public var isPaused = false

    /// Current playback speed
    @Published public var playbackSpeed: PlaybackSpeed = .realtime

    /// Input directory path
    @Published public var inputDirectory: URL?

    /// Output directory path
    @Published public var outputDirectory: URL?

    // MARK: - Statistics

    /// Total documents in batch
    @Published public var totalCount = 0

    /// Completed documents
    @Published public var completedCount = 0

    /// Failed documents
    @Published public var failedCount = 0

    /// Current pages per second
    @Published public var pagesPerSecond: Double = 0

    /// Estimated time remaining
    @Published public var estimatedTimeRemaining: TimeInterval = 0

    // MARK: - Live Processing State

    /// Current page image being processed
    @Published public var currentPageImage: NSImage?

    /// Elements detected on current page
    @Published public var currentElements: [Element] = []

    /// Total elements detected so far
    @Published public var totalElementsDetected: Int = 0

    /// Processing time for current page
    @Published public var processingTimeMs: Double = 0

    // MARK: - Private State

    /// FFI batch processor handle
    private var batchProcessor: OpaquePointer?

    /// Timer for polling progress updates
    private var pollTimer: Timer?

    /// Start time for ETA calculation
    private var startTime: Date?

    /// Pages processed counter
    private var pagesProcessed: Int = 0

    // MARK: - Computed Properties

    /// Currently active document
    public var activeDocument: BatchDocument? {
        documents.first { $0.id == activeDocId }
    }

    /// Overall progress (0.0 to 1.0)
    public var overallProgress: Double {
        guard totalCount > 0 else { return 0 }
        return Double(completedCount) / Double(totalCount)
    }

    /// Progress text
    public var progressText: String {
        "\(completedCount)/\(totalCount)"
    }

    // MARK: - Lifecycle

    public init() {}

    deinit {
        // Clean up FFI resources synchronously
        // Note: pollTimer will be invalidated by the system when the object is deallocated
        if let batch = batchProcessor {
            _ = dlviz_batch_stop(batch)
            dlviz_batch_free(batch)
        }
    }

    // MARK: - Actions

    /// Open folder picker to select input directory
    public func selectInputFolder() {
        let panel = NSOpenPanel()
        panel.canChooseDirectories = true
        panel.canChooseFiles = false
        panel.allowsMultipleSelection = false
        panel.message = "Select a folder containing PDF files"
        panel.prompt = "Select"

        if panel.runModal() == .OK, let url = panel.url {
            loadDocuments(from: url)
        }
    }

    /// Load documents from a directory
    public func loadDocuments(from directory: URL) {
        inputDirectory = directory
        outputDirectory = FileManager.default.temporaryDirectory
            .appendingPathComponent("doclingviz_batch_\(UUID().uuidString)")

        // Find all PDFs recursively
        var docs: [BatchDocument] = []
        var index = 0

        if let enumerator = FileManager.default.enumerator(
            at: directory,
            includingPropertiesForKeys: [.isRegularFileKey],
            options: [.skipsHiddenFiles]
        ) {
            while let fileURL = enumerator.nextObject() as? URL {
                if fileURL.pathExtension.lowercased() == "pdf" {
                    docs.append(BatchDocument(
                        id: index,
                        name: fileURL.lastPathComponent,
                        path: fileURL,
                        status: .queued
                    ))
                    index += 1
                }
            }
        }

        // Sort by name
        docs.sort { $0.name.localizedStandardCompare($1.name) == .orderedAscending }

        // Update state
        documents = docs
        totalCount = docs.count
        completedCount = 0
        failedCount = 0
        activeDocId = nil
        totalElementsDetected = 0
        pagesProcessed = 0
    }

    /// Toggle play/pause
    public func togglePlayPause() {
        if isRunning {
            if isPaused {
                resume()
            } else {
                pause()
            }
        } else {
            start()
        }
    }

    /// Start batch processing
    public func start() {
        guard !documents.isEmpty else { return }
        guard let inputDir = inputDirectory, let outputDir = outputDirectory else { return }

        // Create output directory
        try? FileManager.default.createDirectory(at: outputDir, withIntermediateDirectories: true)

        // Create batch processor
        batchProcessor = dlviz_batch_new()

        // Start processing
        inputDir.path.withCString { input in
            outputDir.path.withCString { output in
                _ = dlviz_batch_start(batchProcessor, input, output)
            }
        }

        isRunning = true
        isPaused = false
        startTime = Date()

        // Start polling timer (30 Hz)
        pollTimer = Timer.scheduledTimer(withTimeInterval: 1.0 / 30.0, repeats: true) { [weak self] _ in
            Task { @MainActor in
                self?.pollProgress()
            }
        }
    }

    /// Pause processing
    public func pause() {
        guard let batch = batchProcessor else { return }
        _ = dlviz_batch_pause(batch)
        isPaused = true
    }

    /// Resume processing
    public func resume() {
        guard let batch = batchProcessor else { return }
        _ = dlviz_batch_resume(batch)
        isPaused = false
    }

    /// Stop processing completely
    public func stop() {
        pollTimer?.invalidate()
        pollTimer = nil

        if let batch = batchProcessor {
            _ = dlviz_batch_stop(batch)
            dlviz_batch_free(batch)
            batchProcessor = nil
        }

        isRunning = false
        isPaused = false
    }

    /// Set playback speed
    public func setSpeed(_ speed: PlaybackSpeed) {
        playbackSpeed = speed
        if let batch = batchProcessor {
            _ = dlviz_batch_set_speed(batch, speed.rawValue)
        }
    }

    /// Select a document to view details
    public func selectDocument(_ id: Int) {
        activeDocId = id
    }

    // MARK: - Private Methods

    /// Poll for progress updates from the batch processor
    private func pollProgress() {
        guard let batch = batchProcessor else { return }

        // Process all available updates
        while let jsonPtr = dlviz_batch_poll(batch) {
            defer { dlviz_string_free(jsonPtr) }

            let json = String(cString: jsonPtr)
            guard let data = json.data(using: .utf8),
                  let progress = try? JSONDecoder().decode(BatchProgress.self, from: data) else {
                continue
            }

            updateProgress(progress)
        }

        // Update running/paused state
        isRunning = dlviz_batch_is_running(batch)
        isPaused = dlviz_batch_is_paused(batch)

        // Stop polling if no longer running
        if !isRunning {
            pollTimer?.invalidate()
            pollTimer = nil
        }
    }

    /// Update state from a progress event
    private func updateProgress(_ progress: BatchProgress) {
        guard progress.docIndex < documents.count else { return }

        // Update document status
        documents[progress.docIndex].status = progress.status
        documents[progress.docIndex].currentPage = progress.pageNo
        documents[progress.docIndex].totalPages = progress.totalPages
        documents[progress.docIndex].currentStage = progress.pipelineStage
        documents[progress.docIndex].processingTimeMs = progress.processingTimeMs
        documents[progress.docIndex].elementsDetected = progress.elementsDetected

        if let errorMsg = progress.errorMessage {
            documents[progress.docIndex].errorMessage = errorMsg
        }

        // Track active document
        if progress.status == .processing {
            activeDocId = progress.docIndex
            processingTimeMs = progress.processingTimeMs
            pagesProcessed += 1
            totalElementsDetected += progress.elementsDetected
        }

        // Update counters
        switch progress.status {
        case .completed:
            // Only increment if this is the final completion message
            if documents[progress.docIndex].status == .completed {
                let previousCompleted = documents.filter { $0.status == .completed }.count - 1
                if completedCount <= previousCompleted {
                    completedCount += 1
                }
            }
        case .failed:
            let previousFailed = documents.filter { $0.status == .failed }.count - 1
            if failedCount <= previousFailed {
                failedCount += 1
            }
        default:
            break
        }

        // Recalculate stats
        completedCount = documents.filter { $0.status == .completed }.count
        failedCount = documents.filter { $0.status == .failed }.count

        // Update pages per second
        if let start = startTime, pagesProcessed > 0 {
            let elapsed = Date().timeIntervalSince(start)
            if elapsed > 0 {
                pagesPerSecond = Double(pagesProcessed) / elapsed
            }
        }

        // Estimate time remaining
        if pagesPerSecond > 0 {
            let remainingDocs = totalCount - completedCount - failedCount
            // Rough estimate: 5 pages per document average
            let estimatedPages = Double(remainingDocs) * 5.0
            estimatedTimeRemaining = estimatedPages / pagesPerSecond
        }
    }
}
