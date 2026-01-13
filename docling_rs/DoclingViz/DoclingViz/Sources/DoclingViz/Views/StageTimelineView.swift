// StageTimelineView - Pipeline stage controls and timeline
// Contains stage selection and playback controls

import SwiftUI
import DoclingBridge

// MARK: - Stage Timeline View

struct StageTimelineView: View {
    @EnvironmentObject var appState: AppState
    @ObservedObject var viewModel: DocumentViewModel

    var body: some View {
        VStack(spacing: 8) {
            if !appState.hasPdfMl {
                // Show message when ML pipeline isn't available
                HStack {
                    Image(systemName: "info.circle")
                        .foregroundColor(.orange)
                    Text("ML pipeline not available. Viewing mode only.")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                .padding(.vertical, 4)
            } else {
                // Stage buttons
                HStack(spacing: 4) {
                    ForEach(PipelineStage.allCases) { stage in
                        Button(action: { viewModel.currentStage = stage }) {
                            VStack(spacing: 2) {
                                Text("\(stage.rawValue)")
                                    .font(.caption2.bold())
                                Text(stage.shortName)
                                    .font(.system(size: 8))
                            }
                            .frame(width: 50, height: 40)
                            .background(stage == viewModel.currentStage ? Color.accentColor : Color.secondary.opacity(0.2))
                            .foregroundColor(stage == viewModel.currentStage ? .white : .primary)
                            .cornerRadius(6)
                        }
                        .buttonStyle(.plain)
                    }
                }

                // Playback controls
                HStack {
                    Button(action: { viewModel.currentStage = .rawPdf }) {
                        Image(systemName: "backward.end.fill")
                    }
                    Button(action: { viewModel.previousStage() }) {
                        Image(systemName: "backward.fill")
                    }
                    Button(action: { viewModel.togglePlayback() }) {
                        Image(systemName: viewModel.isPlaying ? "pause.fill" : "play.fill")
                    }
                    Button(action: { viewModel.nextStage() }) {
                        Image(systemName: "forward.fill")
                    }
                    Button(action: { viewModel.currentStage = .readingOrder }) {
                        Image(systemName: "forward.end.fill")
                    }

                    Spacer()

                    // Playback speed control
                    Picker("Speed", selection: $viewModel.playbackSpeed) {
                        Text("0.5x").tag(0.5)
                        Text("1x").tag(1.0)
                        Text("2x").tag(2.0)
                        Text("4x").tag(4.0)
                    }
                    .pickerStyle(.segmented)
                    .frame(width: 160)

                    Spacer()

                    if let snapshot = viewModel.currentStageSnapshot {
                        Text("Elements: \(snapshot.elements.count)")
                            .font(.caption)
                            .foregroundColor(.secondary)
                        Text("Cells: \(snapshot.textCells.count)")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                }
                .padding(.horizontal)
            }
        }
        .padding(.vertical, 8)
        .background(Color(nsColor: .controlBackgroundColor))
    }
}
