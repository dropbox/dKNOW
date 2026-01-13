// DoclingPipeline - Swift wrapper for Rust FFI pipeline
// Provides safe, idiomatic Swift interface to DoclingViz FFI

import Foundation
import CDoclingBridge

/// Main interface to the Docling PDF extraction pipeline
public final class DoclingPipeline {
    private var handle: OpaquePointer?

    /// Library version
    public static var version: String {
        guard let ptr = dlviz_version() else { return "unknown" }
        return String(cString: ptr)
    }

    /// Number of pipeline stages
    public static var stageCount: Int {
        Int(dlviz_stage_count())
    }

    /// Whether PDF rendering is available
    public static var hasPdfRender: Bool {
        dlviz_has_pdf_render()
    }

    /// Whether ML pipeline is available
    public static var hasPdfMl: Bool {
        dlviz_has_pdf_ml()
    }

    /// Get name of a pipeline stage
    public static func stageName(_ stage: PipelineStage) -> String {
        guard let ptr = dlviz_stage_name(stage.ffiValue) else { return "Unknown" }
        return String(cString: ptr)
    }

    // MARK: - Initialization

    /// Create a new pipeline instance
    public init() throws {
        handle = dlviz_pipeline_new()
        guard handle != nil else {
            throw DoclingResult.internalError
        }
    }

    deinit {
        if let handle = handle {
            dlviz_pipeline_free(handle)
        }
    }

    // MARK: - Document Loading

    /// Load a PDF document
    /// - Parameter path: Path to the PDF file
    /// - Throws: DoclingResult error on failure
    public func loadPDF(at path: String) throws {
        let result = path.withCString { pathPtr in
            dlviz_load_pdf(handle, pathPtr)
        }
        let docResult = DoclingResult(from: result)
        if docResult != .success {
            throw docResult
        }
    }

    /// Load a PDF document from a URL
    /// - Parameter url: URL to the PDF file
    /// - Throws: DoclingResult error on failure
    public func loadPDF(at url: URL) throws {
        try loadPDF(at: url.path)
    }

    // MARK: - Document Properties

    /// Number of pages in the loaded document
    public var pageCount: Int {
        Int(dlviz_get_page_count(handle))
    }

    /// Get dimensions of a page in points
    /// - Parameter pageIndex: Zero-based page index
    /// - Returns: Tuple of (width, height) in points, or nil on error
    public func pageSize(at pageIndex: Int) -> (width: Float, height: Float)? {
        var width: Float = 0
        var height: Float = 0
        let result = dlviz_get_page_size(handle, UInt(pageIndex), &width, &height)
        if DoclingResult(from: result) == .success {
            return (width, height)
        }
        return nil
    }

    // MARK: - Page Rendering

    /// Render a page to an RGBA image buffer
    /// - Parameters:
    ///   - pageIndex: Zero-based page index
    ///   - scale: Scale factor (1.0 = native resolution)
    /// - Returns: Tuple of (width, height, rgbaData), or nil on error
    public func renderPage(at pageIndex: Int, scale: Float = 1.0) -> (width: Int, height: Int, data: Data)? {
        var width: UInt32 = 0
        var height: UInt32 = 0

        // First call to get dimensions
        var result = dlviz_render_page(handle, UInt(pageIndex), scale, &width, &height, nil, 0)

        // If we got dimensions, allocate buffer and render
        if result == dlviz_DlvizResult(INVALID_ARGUMENT.rawValue) && width > 0 && height > 0 {
            let bufferSize = Int(width * height * 4)
            var buffer = Data(count: bufferSize)

            result = buffer.withUnsafeMutableBytes { bufferPtr in
                dlviz_render_page(
                    handle,
                    UInt(pageIndex),
                    scale,
                    &width,
                    &height,
                    bufferPtr.baseAddress?.assumingMemoryBound(to: UInt8.self),
                    UInt(bufferSize)
                )
            }

            if DoclingResult(from: result) == .success {
                return (Int(width), Int(height), buffer)
            }
        }

        return nil
    }

    // MARK: - Pipeline Execution

    /// Run pipeline up to and including the specified stage
    /// - Parameters:
    ///   - pageIndex: Zero-based page index
    ///   - stage: Target stage to run to
    /// - Throws: DoclingResult error on failure
    public func runToStage(_ stage: PipelineStage, pageIndex: Int) throws {
        let result = dlviz_run_to_stage(handle, UInt(pageIndex), stage.ffiValue)
        let docResult = DoclingResult(from: result)
        if docResult != .success {
            throw docResult
        }
    }

    /// Get snapshot of pipeline state at a stage
    /// - Parameters:
    ///   - stage: Stage to get snapshot for
    ///   - pageIndex: Zero-based page index
    /// - Returns: StageSnapshot, or nil if not available
    public func snapshot(at stage: PipelineStage, pageIndex: Int) -> StageSnapshot? {
        var snapshot = dlviz_DlvizStageSnapshot()
        let result = dlviz_get_stage_snapshot(handle, UInt(pageIndex), stage.ffiValue, &snapshot)
        if DoclingResult(from: result) == .success {
            return StageSnapshot(ffi: snapshot)
        }
        return nil
    }

    // MARK: - Text Access

    /// Get text content of an element
    /// - Parameters:
    ///   - elementId: Element ID from snapshot
    ///   - pageIndex: Zero-based page index
    /// - Returns: Text content, or nil if not available
    public func elementText(elementId: UInt32, pageIndex: Int) -> String? {
        var actualSize: UInt = 0
        let bufferSize: UInt = 4096
        var buffer = [CChar](repeating: 0, count: Int(bufferSize))

        let result = dlviz_get_element_text(
            handle,
            UInt(pageIndex),
            elementId,
            &buffer,
            bufferSize,
            &actualSize
        )

        if DoclingResult(from: result) == .success && actualSize > 0 {
            return String(cString: buffer)
        }
        return nil
    }

    /// Get text content of a cell
    /// - Parameters:
    ///   - cellId: Cell ID from snapshot
    ///   - pageIndex: Zero-based page index
    /// - Returns: Text content, or nil if not available
    public func cellText(cellId: UInt32, pageIndex: Int) -> String? {
        var actualSize: UInt = 0
        let bufferSize: UInt = 1024
        var buffer = [CChar](repeating: 0, count: Int(bufferSize))

        let result = dlviz_get_cell_text(
            handle,
            UInt(pageIndex),
            cellId,
            &buffer,
            bufferSize,
            &actualSize
        )

        if DoclingResult(from: result) == .success && actualSize > 0 {
            return String(cString: buffer)
        }
        return nil
    }

    // MARK: - Export

    /// Export current pipeline state as JSON
    /// - Parameter pageIndex: Zero-based page index
    /// - Returns: JSON string, or nil on error
    public func exportJSON(pageIndex: Int) -> String? {
        guard let ptr = dlviz_export_json(handle, UInt(pageIndex)) else {
            return nil
        }
        defer { dlviz_string_free(ptr) }
        return String(cString: ptr)
    }
}

// MARK: - Convenience Extensions

extension DoclingPipeline {
    /// Process all stages for a page
    /// - Parameter pageIndex: Zero-based page index
    /// - Throws: DoclingResult error on failure
    public func processPage(_ pageIndex: Int) throws {
        try runToStage(.readingOrder, pageIndex: pageIndex)
    }

    /// Get all snapshots for a page
    /// - Parameter pageIndex: Zero-based page index
    /// - Returns: Array of snapshots for each stage
    public func allSnapshots(pageIndex: Int) -> [StageSnapshot] {
        PipelineStage.allCases.compactMap { stage in
            snapshot(at: stage, pageIndex: pageIndex)
        }
    }
}

// MARK: - Image Export Extensions

extension DoclingPipeline {
    /// Export a page as PNG data
    /// - Parameters:
    ///   - pageIndex: Zero-based page index
    ///   - scale: Scale factor (default: 2.0 for high resolution)
    /// - Returns: PNG image data, or nil on error
    public func exportPageAsPNG(pageIndex: Int, scale: Float = 2.0) -> Data? {
        guard let (width, height, rgbaData) = renderPage(at: pageIndex, scale: scale) else {
            return nil
        }
        return createPNG(from: rgbaData, width: width, height: height)
    }

    /// Export a page as JPEG data
    /// - Parameters:
    ///   - pageIndex: Zero-based page index
    ///   - scale: Scale factor (default: 2.0 for high resolution)
    ///   - quality: JPEG quality (0.0-1.0, default: 0.9)
    /// - Returns: JPEG image data, or nil on error
    public func exportPageAsJPEG(pageIndex: Int, scale: Float = 2.0, quality: Double = 0.9) -> Data? {
        guard let (width, height, rgbaData) = renderPage(at: pageIndex, scale: scale) else {
            return nil
        }
        return createJPEG(from: rgbaData, width: width, height: height, quality: quality)
    }

    /// Create PNG data from RGBA buffer
    private func createPNG(from rgbaData: Data, width: Int, height: Int) -> Data? {
        #if canImport(AppKit)
        let bytesPerRow = width * 4
        let bitmapInfo = CGBitmapInfo(rawValue: CGImageAlphaInfo.premultipliedLast.rawValue)

        guard let providerRef = CGDataProvider(data: rgbaData as CFData),
              let cgImage = CGImage(
                width: width,
                height: height,
                bitsPerComponent: 8,
                bitsPerPixel: 32,
                bytesPerRow: bytesPerRow,
                space: CGColorSpaceCreateDeviceRGB(),
                bitmapInfo: bitmapInfo,
                provider: providerRef,
                decode: nil,
                shouldInterpolate: false,
                intent: .defaultIntent
              ) else {
            return nil
        }

        let nsImage = NSImage(cgImage: cgImage, size: NSSize(width: width, height: height))
        guard let tiffData = nsImage.tiffRepresentation,
              let bitmapRep = NSBitmapImageRep(data: tiffData) else {
            return nil
        }

        return bitmapRep.representation(using: .png, properties: [:])
        #else
        return nil
        #endif
    }

    /// Create JPEG data from RGBA buffer
    private func createJPEG(from rgbaData: Data, width: Int, height: Int, quality: Double) -> Data? {
        #if canImport(AppKit)
        let bytesPerRow = width * 4
        let bitmapInfo = CGBitmapInfo(rawValue: CGImageAlphaInfo.premultipliedLast.rawValue)

        guard let providerRef = CGDataProvider(data: rgbaData as CFData),
              let cgImage = CGImage(
                width: width,
                height: height,
                bitsPerComponent: 8,
                bitsPerPixel: 32,
                bytesPerRow: bytesPerRow,
                space: CGColorSpaceCreateDeviceRGB(),
                bitmapInfo: bitmapInfo,
                provider: providerRef,
                decode: nil,
                shouldInterpolate: false,
                intent: .defaultIntent
              ) else {
            return nil
        }

        let nsImage = NSImage(cgImage: cgImage, size: NSSize(width: width, height: height))
        guard let tiffData = nsImage.tiffRepresentation,
              let bitmapRep = NSBitmapImageRep(data: tiffData) else {
            return nil
        }

        return bitmapRep.representation(using: .jpeg, properties: [.compressionFactor: quality])
        #else
        return nil
        #endif
    }
}

#if canImport(AppKit)
import AppKit
#endif

// MARK: - COCO Export Extensions

extension DoclingPipeline {
    /// Export current document to COCO format
    /// - Parameters:
    ///   - stage: The stage to export (default: readingOrder for final results)
    ///   - pages: Optional list of page indices to include (nil = all pages)
    ///   - sourceFile: Source file name for the dataset
    /// - Returns: COCODataset ready for JSON export
    /// - Throws: DoclingResult error on failure
    public func exportCOCO(
        stage: PipelineStage = .readingOrder,
        pages: [Int]? = nil,
        sourceFile: String = "document"
    ) throws -> COCODataset {
        let pagesToExport = pages ?? Array(0..<pageCount)
        var snapshots: [(page: Int, snapshot: StageSnapshot, pageWidth: Float, pageHeight: Float)] = []

        for pageIndex in pagesToExport {
            // Ensure pipeline has run to the requested stage
            try runToStage(stage, pageIndex: pageIndex)

            guard let snap = snapshot(at: stage, pageIndex: pageIndex),
                  let size = pageSize(at: pageIndex) else {
                continue
            }

            snapshots.append((page: pageIndex, snapshot: snap, pageWidth: size.width, pageHeight: size.height))
        }

        return COCODataset.from(snapshots: snapshots, sourceFile: sourceFile)
    }

    /// Export current page to COCO format
    /// - Parameters:
    ///   - pageIndex: Zero-based page index
    ///   - stage: The stage to export (default: readingOrder)
    ///   - sourceFile: Source file name for the dataset
    /// - Returns: COCODataset for single page
    /// - Throws: DoclingResult error on failure
    public func exportPageCOCO(
        pageIndex: Int,
        stage: PipelineStage = .readingOrder,
        sourceFile: String = "document"
    ) throws -> COCODataset {
        try exportCOCO(stage: stage, pages: [pageIndex], sourceFile: sourceFile)
    }

    /// Image format for COCO export
    public enum ImageFormat {
        case png
        case jpeg(quality: Double)

        var fileExtension: String {
            switch self {
            case .png: return "png"
            case .jpeg: return "jpg"
            }
        }
    }

    /// Export COCO dataset with images to a folder
    /// - Parameters:
    ///   - folderURL: Destination folder URL
    ///   - stage: The stage to export
    ///   - imageFormat: Format for exported images
    ///   - imageScale: Scale factor for images (default: 2.0 for training)
    ///   - progress: Optional progress callback (pageIndex, totalPages)
    /// - Throws: Error on failure
    public func exportCOCOWithImages(
        to folderURL: URL,
        stage: PipelineStage = .readingOrder,
        imageFormat: ImageFormat = .png,
        imageScale: Float = 2.0,
        progress: ((Int, Int) -> Void)? = nil
    ) throws {
        let fileManager = FileManager.default

        // Create folder structure
        let imagesFolder = folderURL.appendingPathComponent("images")
        let annotationsFolder = folderURL.appendingPathComponent("annotations")

        try fileManager.createDirectory(at: imagesFolder, withIntermediateDirectories: true)
        try fileManager.createDirectory(at: annotationsFolder, withIntermediateDirectories: true)

        // Export each page image and build COCO dataset
        var snapshots: [(page: Int, snapshot: StageSnapshot, pageWidth: Float, pageHeight: Float)] = []
        var cocoImages: [COCOImage] = []

        let totalPages = pageCount
        for pageIndex in 0..<totalPages {
            progress?(pageIndex, totalPages)

            // Export image
            let imageName = String(format: "page_%04d.\(imageFormat.fileExtension)", pageIndex + 1)
            let imageURL = imagesFolder.appendingPathComponent(imageName)

            let imageData: Data?
            switch imageFormat {
            case .png:
                imageData = exportPageAsPNG(pageIndex: pageIndex, scale: imageScale)
            case .jpeg(let quality):
                imageData = exportPageAsJPEG(pageIndex: pageIndex, scale: imageScale, quality: quality)
            }

            if let data = imageData {
                try data.write(to: imageURL)
            }

            // Get page size and snapshot
            guard let size = pageSize(at: pageIndex) else { continue }

            // Run pipeline and get snapshot
            try runToStage(stage, pageIndex: pageIndex)

            if let snap = snapshot(at: stage, pageIndex: pageIndex) {
                snapshots.append((page: pageIndex, snapshot: snap, pageWidth: size.width, pageHeight: size.height))

                // Create COCO image entry (use scaled dimensions)
                let scaledWidth = Int(size.width * imageScale)
                let scaledHeight = Int(size.height * imageScale)
                let cocoImage = COCOImage(
                    id: pageIndex + 1,
                    fileName: imageName,
                    width: scaledWidth,
                    height: scaledHeight
                )
                cocoImages.append(cocoImage)
            }
        }

        progress?(totalPages, totalPages)

        // Create COCO dataset with correct image references
        let dataset = COCODataset.fromWithImages(
            snapshots: snapshots,
            images: cocoImages,
            imageScale: imageScale,
            sourceFile: folderURL.lastPathComponent
        )

        // Export annotations JSON
        let annotationsURL = annotationsFolder.appendingPathComponent("instances_default.json")
        let jsonData = try dataset.toJSONData()
        try jsonData.write(to: annotationsURL)
    }
}
