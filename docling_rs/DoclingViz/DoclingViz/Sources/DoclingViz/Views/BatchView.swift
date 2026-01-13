// BatchView.swift
// Grid-based batch processing visualization

import SwiftUI
import DoclingBridge
import AppKit

/// Main batch processing view with document grid
public struct BatchView: View {
    @StateObject private var viewModel = BatchViewModel()

    /// Grid columns for document tiles
    private let columns = Array(repeating: GridItem(.flexible(), spacing: 8), count: 8)

    public init() {}

    public var body: some View {
        VStack(spacing: 0) {
            // Header with progress and controls
            BatchHeaderView(viewModel: viewModel)

            Divider()

            HSplitView {
                // Grid of document thumbnails
                ScrollView {
                    LazyVGrid(columns: columns, spacing: 8) {
                        ForEach(viewModel.documents) { doc in
                            DocumentTileView(
                                document: doc,
                                isActive: doc.id == viewModel.activeDocId
                            )
                            .onTapGesture {
                                viewModel.selectDocument(doc.id)
                            }
                        }
                    }
                    .padding()
                }
                .frame(minWidth: 450)

                // Live processing detail view
                if let activeDoc = viewModel.activeDocument {
                    LiveProcessingDetailView(
                        document: activeDoc,
                        viewModel: viewModel
                    )
                    .frame(minWidth: 400)
                } else {
                    emptyDetailView
                        .frame(minWidth: 400)
                }
            }
        }
        .toolbar {
            ToolbarItemGroup {
                Button(action: viewModel.selectInputFolder) {
                    Label("Open Folder", systemImage: "folder")
                }

                Divider()

                Button(action: viewModel.togglePlayPause) {
                    if viewModel.isRunning && !viewModel.isPaused {
                        Image(systemName: "pause.fill")
                    } else {
                        Image(systemName: "play.fill")
                    }
                }
                .disabled(viewModel.documents.isEmpty)
                .help(viewModel.isRunning && !viewModel.isPaused ? "Pause" : "Play")

                Button(action: viewModel.stop) {
                    Image(systemName: "stop.fill")
                }
                .disabled(!viewModel.isRunning)
                .help("Stop")

                Divider()

                Picker("Speed", selection: Binding(
                    get: { viewModel.playbackSpeed },
                    set: { viewModel.setSpeed($0) }
                )) {
                    ForEach(PlaybackSpeed.allCases) { speed in
                        Text(speed.label).tag(speed)
                    }
                }
                .pickerStyle(.menu)
                .frame(width: 80)
                .help("Playback speed")
            }
        }
    }

    /// Empty detail view placeholder
    private var emptyDetailView: some View {
        VStack(spacing: 12) {
            Image(systemName: "doc.on.doc")
                .font(.system(size: 48))
                .foregroundColor(.secondary)

            Text("Select a document")
                .font(.headline)
                .foregroundColor(.secondary)

            Text("Click on a document in the grid to see processing details")
                .font(.caption)
                .foregroundColor(.secondary)
                .multilineTextAlignment(.center)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(Color(nsColor: .windowBackgroundColor))
    }
}

// MARK: - Header View

/// Header with progress bar and statistics
struct BatchHeaderView: View {
    @ObservedObject var viewModel: BatchViewModel

    var body: some View {
        VStack(spacing: 8) {
            // Directory info
            if let inputDir = viewModel.inputDirectory {
                HStack {
                    Image(systemName: "folder.fill")
                        .foregroundColor(.accentColor)
                    Text(inputDir.path)
                        .lineLimit(1)
                        .truncationMode(.middle)
                    Spacer()
                    Text("\(viewModel.totalCount) PDFs")
                        .foregroundColor(.secondary)
                }
                .font(.caption)
            }

            // Progress bar
            HStack(spacing: 12) {
                ProgressView(value: viewModel.overallProgress)
                    .progressViewStyle(.linear)

                Text(viewModel.progressText)
                    .font(.caption)
                    .monospacedDigit()
                    .foregroundColor(.secondary)

                if viewModel.estimatedTimeRemaining > 0 {
                    Text("ETA: \(formatTime(viewModel.estimatedTimeRemaining))")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }

            // Statistics row
            HStack(spacing: 24) {
                Label("\(viewModel.completedCount) completed", systemImage: "checkmark.circle.fill")
                    .foregroundColor(.green)

                Label("\(viewModel.failedCount) failed", systemImage: "xmark.circle.fill")
                    .foregroundColor(.red)

                Label(String(format: "%.1f pages/sec", viewModel.pagesPerSecond),
                      systemImage: "speedometer")
                    .foregroundColor(.secondary)

                Label("\(viewModel.totalElementsDetected) elements", systemImage: "rectangle.3.group")
                    .foregroundColor(.secondary)

                Spacer()

                // Status indicator
                if viewModel.isRunning {
                    HStack(spacing: 4) {
                        Circle()
                            .fill(viewModel.isPaused ? Color.orange : Color.green)
                            .frame(width: 8, height: 8)
                        Text(viewModel.isPaused ? "Paused" : "Processing")
                            .font(.caption)
                    }
                }
            }
            .font(.caption)
        }
        .padding(.horizontal)
        .padding(.vertical, 8)
        .background(Color(nsColor: .controlBackgroundColor))
    }

    /// Format time interval as mm:ss or hh:mm:ss
    private func formatTime(_ interval: TimeInterval) -> String {
        let hours = Int(interval) / 3600
        let minutes = (Int(interval) % 3600) / 60
        let seconds = Int(interval) % 60

        if hours > 0 {
            return String(format: "%d:%02d:%02d", hours, minutes, seconds)
        } else {
            return String(format: "%d:%02d", minutes, seconds)
        }
    }
}

// MARK: - Document Tile View

/// Individual document tile in the grid
struct DocumentTileView: View {
    let document: BatchDocument
    let isActive: Bool

    var body: some View {
        VStack(spacing: 4) {
            // Thumbnail with status overlay
            ZStack {
                // Background
                RoundedRectangle(cornerRadius: 4)
                    .fill(backgroundColor)
                    .frame(width: 70, height: 90)

                // Document icon
                Image(systemName: "doc.fill")
                    .font(.system(size: 24))
                    .foregroundColor(.secondary)

                // Status overlay
                statusOverlay
            }
            .frame(width: 70, height: 90)
            .overlay(
                RoundedRectangle(cornerRadius: 4)
                    .stroke(isActive ? Color.accentColor : Color.clear, lineWidth: 2)
            )

            // Document name
            Text(document.name)
                .font(.caption2)
                .lineLimit(1)
                .truncationMode(.middle)
                .frame(width: 70)
        }
    }

    /// Background color based on status
    private var backgroundColor: Color {
        switch document.status {
        case .queued:
            return Color.secondary.opacity(0.1)
        case .processing:
            return Color.blue.opacity(0.2)
        case .completed:
            return Color.green.opacity(0.15)
        case .failed:
            return Color.red.opacity(0.15)
        }
    }

    /// Status overlay
    @ViewBuilder
    private var statusOverlay: some View {
        switch document.status {
        case .queued:
            EmptyView()

        case .processing:
            VStack {
                Spacer()
                ProgressView()
                    .scaleEffect(0.6)
                Spacer()
                HStack {
                    Spacer()
                    Text("p\(document.currentPage + 1)")
                        .font(.system(size: 9))
                        .padding(2)
                        .background(Color.blue.opacity(0.8))
                        .foregroundColor(.white)
                        .cornerRadius(2)
                }
                .padding(4)
            }

        case .completed:
            VStack {
                Spacer()
                HStack {
                    Spacer()
                    Image(systemName: "checkmark.circle.fill")
                        .foregroundColor(.green)
                        .font(.system(size: 16))
                        .padding(4)
                }
            }

        case .failed:
            VStack {
                Spacer()
                HStack {
                    Spacer()
                    Image(systemName: "xmark.circle.fill")
                        .foregroundColor(.red)
                        .font(.system(size: 16))
                        .padding(4)
                }
            }
        }
    }
}

// MARK: - Live Processing Detail View

/// Detailed view of currently processing document
struct LiveProcessingDetailView: View {
    let document: BatchDocument
    @ObservedObject var viewModel: BatchViewModel

    var body: some View {
        VStack(spacing: 8) {
            // Document header
            HStack {
                VStack(alignment: .leading, spacing: 2) {
                    Text(document.name)
                        .font(.headline)

                    Text(document.path.path)
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .lineLimit(1)
                        .truncationMode(.middle)
                }

                Spacer()

                // Page indicator
                if document.totalPages > 0 {
                    Text("Page \(document.currentPage + 1)/\(document.totalPages)")
                        .font(.subheadline)
                        .monospacedDigit()
                        .foregroundColor(.secondary)
                }
            }
            .padding(.horizontal)

            // Status badge
            HStack {
                StatusBadge(status: document.status)

                if document.status == .processing {
                    Text(document.currentStage.description)
                        .font(.caption)
                        .foregroundColor(.secondary)
                }

                Spacer()

                if document.processingTimeMs > 0 {
                    Text(String(format: "%.0fms", document.processingTimeMs))
                        .font(.caption)
                        .monospacedDigit()
                        .foregroundColor(.secondary)
                }
            }
            .padding(.horizontal)

            Divider()

            // Processing stages visualization
            StageProgressView(currentStage: document.currentStage)
                .padding(.horizontal)

            // Stats
            HStack(spacing: 16) {
                Label("\(document.elementsDetected) elements", systemImage: "rectangle.3.group")

                if let error = document.errorMessage {
                    Label(error, systemImage: "exclamationmark.triangle.fill")
                        .foregroundColor(.red)
                        .lineLimit(1)
                }

                Spacer()
            }
            .font(.caption)
            .foregroundColor(.secondary)
            .padding(.horizontal)

            Spacer()
        }
        .padding(.vertical)
        .background(Color(nsColor: .windowBackgroundColor))
    }
}

// MARK: - Status Badge

/// Colored status badge
struct StatusBadge: View {
    let status: BatchDocStatus

    var body: some View {
        HStack(spacing: 4) {
            Circle()
                .fill(statusColor)
                .frame(width: 8, height: 8)

            Text(status.description)
                .font(.caption)
        }
        .padding(.horizontal, 8)
        .padding(.vertical, 4)
        .background(statusColor.opacity(0.2))
        .cornerRadius(4)
    }

    private var statusColor: Color {
        switch status {
        case .queued: return .secondary
        case .processing: return .blue
        case .completed: return .green
        case .failed: return .red
        }
    }
}

// MARK: - Stage Progress View

/// Visual progress through processing stages
struct StageProgressView: View {
    let currentStage: PipelineStage

    var body: some View {
        HStack(spacing: 2) {
            ForEach(0..<PipelineStage.count, id: \.self) { index in
                let stage = PipelineStage(rawValue: Int32(index)) ?? .rawPdf
                let isCurrent = stage == currentStage
                let isCompleted = stage.rawValue < currentStage.rawValue

                VStack(spacing: 2) {
                    Rectangle()
                        .fill(stageColor(isCompleted: isCompleted, isCurrent: isCurrent))
                        .frame(height: 4)

                    if isCurrent {
                        Text(stage.shortName)
                            .font(.system(size: 8))
                            .foregroundColor(.accentColor)
                    }
                }
            }
        }
    }

    private func stageColor(isCompleted: Bool, isCurrent: Bool) -> Color {
        if isCurrent {
            return .accentColor
        } else if isCompleted {
            return .accentColor.opacity(0.5)
        } else {
            return .secondary.opacity(0.2)
        }
    }
}

// MARK: - Preview

#if DEBUG
struct BatchView_Previews: PreviewProvider {
    static var previews: some View {
        BatchView()
            .frame(width: 1000, height: 600)
    }
}
#endif
