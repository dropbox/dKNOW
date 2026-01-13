// PageListView - Sidebar showing page thumbnails
// Contains page list and thumbnail rendering

import SwiftUI
import AppKit
import DoclingBridge

// MARK: - Page List View

struct PageListView: View {
    @ObservedObject var viewModel: DocumentViewModel

    var body: some View {
        ScrollViewReader { proxy in
            List(selection: $viewModel.currentPage) {
                ForEach(0..<viewModel.pageCount, id: \.self) { pageIndex in
                    PageThumbnailRow(
                        viewModel: viewModel,
                        pageIndex: pageIndex,
                        isSelected: pageIndex == viewModel.currentPage
                    )
                    .tag(pageIndex)
                    .id(pageIndex)
                }
            }
            .listStyle(.sidebar)
            .navigationTitle("Pages")
            .onChange(of: viewModel.currentPage) { _, newPage in
                withAnimation(.easeInOut(duration: 0.2)) {
                    proxy.scrollTo(newPage, anchor: .center)
                }
            }
        }
    }
}

// MARK: - Page Thumbnail Row

struct PageThumbnailRow: View {
    @ObservedObject var viewModel: DocumentViewModel
    let pageIndex: Int
    let isSelected: Bool

    var body: some View {
        VStack(spacing: 4) {
            // Thumbnail image
            ZStack {
                if let thumbnail = viewModel.pageThumbnails[pageIndex] {
                    Image(nsImage: thumbnail)
                        .resizable()
                        .aspectRatio(contentMode: .fit)
                        .frame(width: 70, height: 90)
                        .cornerRadius(4)
                        .shadow(color: .black.opacity(0.2), radius: 2, x: 0, y: 1)
                        .overlay(
                            RoundedRectangle(cornerRadius: 4)
                                .stroke(isSelected ? Color.accentColor : Color.clear, lineWidth: 2)
                        )
                } else {
                    // Placeholder while loading
                    RoundedRectangle(cornerRadius: 4)
                        .fill(Color.secondary.opacity(0.1))
                        .frame(width: 70, height: 90)
                        .overlay(
                            ProgressView()
                                .scaleEffect(0.5)
                        )
                }
            }

            // Page number
            Text("\(pageIndex + 1)")
                .font(.caption)
                .foregroundColor(isSelected ? .accentColor : .secondary)
        }
        .padding(.vertical, 4)
        .frame(maxWidth: .infinity)
    }
}
