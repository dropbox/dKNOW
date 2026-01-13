// DoclingBridge - Swift types for DoclingViz FFI
// Auto-generated wrapper around Rust FFI

import Foundation
import CoreGraphics
import CDoclingBridge

// MARK: - Result Code

/// Result code from FFI operations
public enum DoclingResult: Int32, Error, CustomStringConvertible {
    case success = 0
    case invalidArgument = -1
    case fileNotFound = -2
    case parseError = -3
    case inferenceError = -4
    case outOfMemory = -5
    case internalError = -99

    public var description: String {
        switch self {
        case .success: return "Success"
        case .invalidArgument: return "Invalid argument"
        case .fileNotFound: return "File not found"
        case .parseError: return "Parse error"
        case .inferenceError: return "Inference error"
        case .outOfMemory: return "Out of memory"
        case .internalError: return "Internal error"
        }
    }

    static func fromFFI(_ result: Int32) -> DoclingResult {
        DoclingResult(rawValue: result) ?? .internalError
    }

    /// Initialize from FFI result code
    init(from result: Int32) {
        self = DoclingResult(rawValue: result) ?? .internalError
    }
}

// MARK: - Document Item Label

/// Document element label/class
public enum DocItemLabel: Int32, CaseIterable, Identifiable, CustomStringConvertible {
    case caption = 0
    case footnote = 1
    case formula = 2
    case listItem = 3
    case pageFooter = 4
    case pageHeader = 5
    case picture = 6
    case sectionHeader = 7
    case table = 8
    case text = 9
    case title = 10
    case code = 11
    case checkboxSelected = 12
    case checkboxUnselected = 13
    case documentIndex = 14
    case form = 15
    case keyValueRegion = 16

    public var id: Int32 { rawValue }

    public var description: String {
        switch self {
        case .caption: return "Caption"
        case .footnote: return "Footnote"
        case .formula: return "Formula"
        case .listItem: return "List Item"
        case .pageFooter: return "Page Footer"
        case .pageHeader: return "Page Header"
        case .picture: return "Picture"
        case .sectionHeader: return "Section Header"
        case .table: return "Table"
        case .text: return "Text"
        case .title: return "Title"
        case .code: return "Code"
        case .checkboxSelected: return "Checkbox (Selected)"
        case .checkboxUnselected: return "Checkbox (Unselected)"
        case .documentIndex: return "Document Index"
        case .form: return "Form"
        case .keyValueRegion: return "Key-Value Region"
        }
    }

    /// Color for visualization
    public var color: (r: UInt8, g: UInt8, b: UInt8) {
        switch self {
        case .caption: return (255, 165, 0)    // Orange
        case .footnote: return (128, 128, 128) // Gray
        case .formula: return (0, 255, 255)    // Cyan
        case .listItem: return (144, 238, 144) // Light green
        case .pageFooter: return (192, 192, 192) // Silver
        case .pageHeader: return (192, 192, 192) // Silver
        case .picture: return (255, 0, 255)    // Magenta
        case .sectionHeader: return (0, 0, 255) // Blue
        case .table: return (0, 255, 0)        // Green
        case .text: return (255, 255, 0)       // Yellow
        case .title: return (255, 0, 0)        // Red
        case .code: return (128, 0, 128)       // Purple
        case .checkboxSelected: return (0, 128, 0) // Dark green
        case .checkboxUnselected: return (128, 0, 0) // Dark red
        case .documentIndex: return (0, 128, 128) // Teal
        case .form: return (255, 192, 203)     // Pink
        case .keyValueRegion: return (255, 215, 0) // Gold
        }
    }

    static func fromFFI(_ label: Int32) -> DocItemLabel {
        DocItemLabel(rawValue: label) ?? .text
    }
}

// MARK: - Pipeline Stage

/// Pipeline processing stage
public enum PipelineStage: Int32, CaseIterable, Identifiable, CustomStringConvertible {
    case rawPdf = 0
    case ocrDetection = 1
    case ocrRecognition = 2
    case layoutDetection = 3
    case cellAssignment = 4
    case emptyClusterRemoval = 5
    case orphanDetection = 6
    case bboxAdjust1 = 7
    case bboxAdjust2 = 8
    case finalAssembly = 9
    case readingOrder = 10

    public var id: Int32 { rawValue }

    public var description: String {
        switch self {
        case .rawPdf: return "Raw PDF"
        case .ocrDetection: return "OCR Detection"
        case .ocrRecognition: return "OCR Recognition"
        case .layoutDetection: return "Layout Detection"
        case .cellAssignment: return "Cell Assignment"
        case .emptyClusterRemoval: return "Empty Cluster Removal"
        case .orphanDetection: return "Orphan Detection"
        case .bboxAdjust1: return "BBox Adjust 1"
        case .bboxAdjust2: return "BBox Adjust 2"
        case .finalAssembly: return "Final Assembly"
        case .readingOrder: return "Reading Order"
        }
    }

    /// Short name for compact display
    public var shortName: String {
        switch self {
        case .rawPdf: return "Raw"
        case .ocrDetection: return "OCR"
        case .ocrRecognition: return "Recog"
        case .layoutDetection: return "Layout"
        case .cellAssignment: return "Assign"
        case .emptyClusterRemoval: return "Empty"
        case .orphanDetection: return "Orphan"
        case .bboxAdjust1: return "Adj1"
        case .bboxAdjust2: return "Adj2"
        case .finalAssembly: return "Final"
        case .readingOrder: return "Order"
        }
    }

    public static let count = 11

    var ffiValue: Int32 {
        rawValue
    }
}

// MARK: - Bounding Box

/// Bounding box in PDF coordinates
public struct BoundingBox: Codable, Equatable {
    public var x: Float
    public var y: Float
    public var width: Float
    public var height: Float

    public var left: Float { x }
    public var bottom: Float { y }
    public var right: Float { x + width }
    public var top: Float { y + height }

    public var center: CGPoint {
        CGPoint(x: CGFloat(x + width / 2), y: CGFloat(y + height / 2))
    }

    public var cgRect: CGRect {
        CGRect(x: CGFloat(x), y: CGFloat(y), width: CGFloat(width), height: CGFloat(height))
    }

    public init(x: Float, y: Float, width: Float, height: Float) {
        self.x = x
        self.y = y
        self.width = width
        self.height = height
    }

    init(ffi bbox: dlviz_DlvizBBox) {
        self.x = bbox.x
        self.y = bbox.y
        self.width = bbox.width
        self.height = bbox.height
    }
}

// MARK: - Element

/// A detected layout element
public struct Element: Identifiable, Equatable {
    public let id: UInt32
    public var bbox: BoundingBox
    public var label: DocItemLabel
    public var confidence: Float
    public var readingOrder: Int32

    public var hasReadingOrder: Bool {
        readingOrder >= 0
    }

    public init(id: UInt32, bbox: BoundingBox, label: DocItemLabel, confidence: Float, readingOrder: Int32 = -1) {
        self.id = id
        self.bbox = bbox
        self.label = label
        self.confidence = confidence
        self.readingOrder = readingOrder
    }

    init(ffi element: dlviz_DlvizElement) {
        self.id = element.id
        self.bbox = BoundingBox(ffi: element.bbox)
        self.label = DocItemLabel.fromFFI(element.label)
        self.confidence = element.confidence
        self.readingOrder = element.reading_order
    }

    public static func == (lhs: Element, rhs: Element) -> Bool {
        lhs.id == rhs.id &&
        lhs.bbox == rhs.bbox &&
        lhs.label == rhs.label &&
        lhs.confidence == rhs.confidence &&
        lhs.readingOrder == rhs.readingOrder
    }
}

// MARK: - Text Cell

/// An OCR text cell
public struct TextCell: Identifiable {
    public let id: UInt32
    public var bbox: BoundingBox
    public var confidence: Float
    public var elementId: Int32

    public var isOrphan: Bool {
        elementId < 0
    }

    init(ffi cell: dlviz_DlvizTextCell) {
        self.id = cell.id
        self.bbox = BoundingBox(ffi: cell.bbox)
        self.confidence = cell.confidence
        self.elementId = cell.element_id
    }
}

// MARK: - Stage Snapshot

/// Snapshot of pipeline state at a specific stage
public struct StageSnapshot {
    public let stage: PipelineStage
    public let elements: [Element]
    public let textCells: [TextCell]
    public let processingTimeMs: Double

    init(ffi snapshot: dlviz_DlvizStageSnapshot) {
        self.stage = PipelineStage(rawValue: snapshot.stage) ?? .rawPdf
        self.processingTimeMs = snapshot.processing_time_ms

        // Convert elements
        var elements: [Element] = []
        if let elementsPtr = snapshot.elements, snapshot.element_count > 0 {
            for i in 0..<Int(snapshot.element_count) {
                elements.append(Element(ffi: elementsPtr[i]))
            }
        }
        self.elements = elements

        // Convert text cells
        var cells: [TextCell] = []
        if let cellsPtr = snapshot.cells, snapshot.cell_count > 0 {
            for i in 0..<Int(snapshot.cell_count) {
                cells.append(TextCell(ffi: cellsPtr[i]))
            }
        }
        self.textCells = cells
    }
}

// MARK: - COCO Format Export

/// COCO format dataset for object detection
public struct COCODataset: Codable {
    public var info: COCOInfo
    public var images: [COCOImage]
    public var annotations: [COCOAnnotation]
    public var categories: [COCOCategory]

    public init(
        info: COCOInfo = COCOInfo(),
        images: [COCOImage] = [],
        annotations: [COCOAnnotation] = [],
        categories: [COCOCategory] = COCOCategory.documentLayoutCategories
    ) {
        self.info = info
        self.images = images
        self.annotations = annotations
        self.categories = categories
    }

    /// Create from a document with all page snapshots
    public static func from(
        snapshots: [(page: Int, snapshot: StageSnapshot, pageWidth: Float, pageHeight: Float)],
        sourceFile: String
    ) -> COCODataset {
        var dataset = COCODataset()
        var annotationId = 1

        for (pageIndex, (page, snapshot, width, height)) in snapshots.enumerated() {
            let imageId = pageIndex + 1

            // Add image entry for this page
            let image = COCOImage(
                id: imageId,
                fileName: "\(sourceFile)_page_\(page + 1).png",
                width: Int(width),
                height: Int(height)
            )
            dataset.images.append(image)

            // Add annotations for each element
            for element in snapshot.elements {
                let annotation = COCOAnnotation(
                    id: annotationId,
                    imageId: imageId,
                    categoryId: Int(element.label.rawValue + 1), // COCO uses 1-indexed
                    bbox: [
                        Double(element.bbox.x),
                        Double(element.bbox.y),
                        Double(element.bbox.width),
                        Double(element.bbox.height)
                    ],
                    area: Double(element.bbox.width * element.bbox.height),
                    score: Double(element.confidence)
                )
                dataset.annotations.append(annotation)
                annotationId += 1
            }
        }

        return dataset
    }

    /// Create from document with pre-built image entries and scaled bounding boxes
    /// Used when exporting images at a different scale than the page coordinates
    public static func fromWithImages(
        snapshots: [(page: Int, snapshot: StageSnapshot, pageWidth: Float, pageHeight: Float)],
        images: [COCOImage],
        imageScale: Float,
        sourceFile: String
    ) -> COCODataset {
        var dataset = COCODataset()
        dataset.images = images
        var annotationId = 1

        for (pageIndex, (_, snapshot, _, _)) in snapshots.enumerated() {
            let imageId = pageIndex + 1

            // Add annotations for each element, scaling bbox to match image dimensions
            for element in snapshot.elements {
                // Scale bounding box coordinates to match exported image dimensions
                let scaledX = Double(element.bbox.x * imageScale)
                let scaledY = Double(element.bbox.y * imageScale)
                let scaledWidth = Double(element.bbox.width * imageScale)
                let scaledHeight = Double(element.bbox.height * imageScale)

                let annotation = COCOAnnotation(
                    id: annotationId,
                    imageId: imageId,
                    categoryId: Int(element.label.rawValue + 1), // COCO uses 1-indexed
                    bbox: [scaledX, scaledY, scaledWidth, scaledHeight],
                    area: scaledWidth * scaledHeight,
                    score: Double(element.confidence)
                )
                dataset.annotations.append(annotation)
                annotationId += 1
            }
        }

        return dataset
    }

    /// Encode to JSON data
    public func toJSONData(prettyPrint: Bool = true) throws -> Data {
        let encoder = JSONEncoder()
        if prettyPrint {
            encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        }
        return try encoder.encode(self)
    }

    /// Encode to JSON string
    public func toJSONString(prettyPrint: Bool = true) throws -> String {
        let data = try toJSONData(prettyPrint: prettyPrint)
        return String(data: data, encoding: .utf8) ?? ""
    }
}

/// COCO dataset info
public struct COCOInfo: Codable {
    public var description: String
    public var version: String
    public var year: Int
    public var contributor: String
    public var dateCreated: String

    enum CodingKeys: String, CodingKey {
        case description, version, year, contributor
        case dateCreated = "date_created"
    }

    public init(
        description: String = "DoclingViz Document Layout Dataset",
        version: String = "1.0",
        year: Int = Calendar.current.component(.year, from: Date()),
        contributor: String = "DoclingViz",
        dateCreated: String = ISO8601DateFormatter().string(from: Date())
    ) {
        self.description = description
        self.version = version
        self.year = year
        self.contributor = contributor
        self.dateCreated = dateCreated
    }
}

/// COCO image entry
public struct COCOImage: Codable {
    public var id: Int
    public var fileName: String
    public var width: Int
    public var height: Int

    enum CodingKeys: String, CodingKey {
        case id, width, height
        case fileName = "file_name"
    }

    public init(id: Int, fileName: String, width: Int, height: Int) {
        self.id = id
        self.fileName = fileName
        self.width = width
        self.height = height
    }
}

/// COCO annotation entry
public struct COCOAnnotation: Codable {
    public var id: Int
    public var imageId: Int
    public var categoryId: Int
    public var bbox: [Double]  // [x, y, width, height]
    public var area: Double
    public var iscrowd: Int
    public var score: Double?  // Optional confidence score

    enum CodingKeys: String, CodingKey {
        case id, bbox, area, iscrowd, score
        case imageId = "image_id"
        case categoryId = "category_id"
    }

    public init(
        id: Int,
        imageId: Int,
        categoryId: Int,
        bbox: [Double],
        area: Double,
        iscrowd: Int = 0,
        score: Double? = nil
    ) {
        self.id = id
        self.imageId = imageId
        self.categoryId = categoryId
        self.bbox = bbox
        self.area = area
        self.iscrowd = iscrowd
        self.score = score
    }
}

/// COCO category entry
public struct COCOCategory: Codable {
    public var id: Int
    public var name: String
    public var supercategory: String

    public init(id: Int, name: String, supercategory: String = "document") {
        self.id = id
        self.name = name
        self.supercategory = supercategory
    }

    /// Standard document layout categories matching DocItemLabel
    public static var documentLayoutCategories: [COCOCategory] {
        DocItemLabel.allCases.map { label in
            COCOCategory(
                id: Int(label.rawValue + 1), // COCO uses 1-indexed
                name: label.cocoName,
                supercategory: label.cocoSupercategory
            )
        }
    }
}

// MARK: - Batch Processing Types

/// Status of a document in batch processing
public enum BatchDocStatus: Int32, Codable, CustomStringConvertible {
    case queued = 0
    case processing = 1
    case completed = 2
    case failed = 3

    public var description: String {
        switch self {
        case .queued: return "Queued"
        case .processing: return "Processing"
        case .completed: return "Completed"
        case .failed: return "Failed"
        }
    }
}

/// Progress update from batch processor
public struct BatchProgress: Codable {
    public let docIndex: Int
    public let docName: String
    public let pageNo: Int
    public let totalPages: Int
    public let stage: Int32
    public let status: BatchDocStatus
    public let errorMessage: String?
    public let processingTimeMs: Double
    public let elementsDetected: Int

    enum CodingKeys: String, CodingKey {
        case docIndex = "doc_index"
        case docName = "doc_name"
        case pageNo = "page_no"
        case totalPages = "total_pages"
        case stage
        case status
        case errorMessage = "error_message"
        case processingTimeMs = "processing_time_ms"
        case elementsDetected = "elements_detected"
    }

    public var pipelineStage: PipelineStage {
        PipelineStage(rawValue: stage) ?? .rawPdf
    }
}

/// Batch processing statistics
public struct BatchStats {
    public let totalDocs: Int
    public let completedDocs: Int
    public let failedDocs: Int
    public let isRunning: Bool
    public let isPaused: Bool
    public let speed: Double

    public var pendingDocs: Int {
        totalDocs - completedDocs - failedDocs
    }

    public var progress: Double {
        guard totalDocs > 0 else { return 0 }
        return Double(completedDocs) / Double(totalDocs)
    }

    init(ffi stats: dlviz_DlvizBatchStats) {
        self.totalDocs = Int(stats.total_docs)
        self.completedDocs = Int(stats.completed_docs)
        self.failedDocs = Int(stats.failed_docs)
        self.isRunning = stats.is_running
        self.isPaused = stats.is_paused
        self.speed = stats.speed
    }
}

/// Document in batch queue
public struct BatchDocument: Identifiable {
    public let id: Int
    public let name: String
    public let path: URL
    public var status: BatchDocStatus
    public var currentPage: Int
    public var totalPages: Int
    public var currentStage: PipelineStage
    public var errorMessage: String?
    public var processingTimeMs: Double
    public var elementsDetected: Int

    public init(
        id: Int,
        name: String,
        path: URL,
        status: BatchDocStatus = .queued,
        currentPage: Int = 0,
        totalPages: Int = 0,
        currentStage: PipelineStage = .rawPdf,
        errorMessage: String? = nil,
        processingTimeMs: Double = 0,
        elementsDetected: Int = 0
    ) {
        self.id = id
        self.name = name
        self.path = path
        self.status = status
        self.currentPage = currentPage
        self.totalPages = totalPages
        self.currentStage = currentStage
        self.errorMessage = errorMessage
        self.processingTimeMs = processingTimeMs
        self.elementsDetected = elementsDetected
    }
}

// MARK: - DocItemLabel COCO Extensions

extension DocItemLabel {
    /// COCO-compatible category name
    public var cocoName: String {
        switch self {
        case .caption: return "caption"
        case .footnote: return "footnote"
        case .formula: return "formula"
        case .listItem: return "list_item"
        case .pageFooter: return "page_footer"
        case .pageHeader: return "page_header"
        case .picture: return "picture"
        case .sectionHeader: return "section_header"
        case .table: return "table"
        case .text: return "text"
        case .title: return "title"
        case .code: return "code"
        case .checkboxSelected: return "checkbox_selected"
        case .checkboxUnselected: return "checkbox_unselected"
        case .documentIndex: return "document_index"
        case .form: return "form"
        case .keyValueRegion: return "key_value_region"
        }
    }

    /// COCO supercategory
    public var cocoSupercategory: String {
        switch self {
        case .title, .sectionHeader, .text, .listItem:
            return "text_block"
        case .table, .form, .keyValueRegion:
            return "structured"
        case .picture, .formula, .code:
            return "media"
        case .caption, .footnote:
            return "annotation"
        case .pageHeader, .pageFooter, .documentIndex:
            return "navigation"
        case .checkboxSelected, .checkboxUnselected:
            return "interactive"
        }
    }
}
