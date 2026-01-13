import XCTest
@testable import DoclingBridge

final class DoclingBridgeTests: XCTestCase {

    func testVersion() {
        let version = DoclingPipeline.version
        XCTAssertFalse(version.isEmpty)
        XCTAssertEqual(version, "0.1.0")
    }

    func testStageCount() {
        XCTAssertEqual(DoclingPipeline.stageCount, 11)
    }

    func testStageName() {
        XCTAssertEqual(DoclingPipeline.stageName(.rawPdf), "Raw PDF")
        XCTAssertEqual(DoclingPipeline.stageName(.readingOrder), "Reading Order")
    }

    func testFeatureFlags() {
        // These should compile and return boolean values
        _ = DoclingPipeline.hasPdfRender
        _ = DoclingPipeline.hasPdfMl
    }

    func testPipelineCreation() throws {
        let pipeline = try DoclingPipeline()
        XCTAssertEqual(pipeline.pageCount, 0)
    }

    func testLoadNonexistentPDF() throws {
        let pipeline = try DoclingPipeline()

        do {
            try pipeline.loadPDF(at: "/nonexistent/path/file.pdf")
            XCTFail("Should have thrown an error")
        } catch let error as DoclingResult {
            // Expected: either fileNotFound or internalError (if pdf-render not available)
            XCTAssertTrue(
                error == .fileNotFound || error == .internalError || error == .parseError,
                "Expected fileNotFound, internalError, or parseError, got \(error)"
            )
        }
    }

    func testDocItemLabelColors() {
        for label in DocItemLabel.allCases {
            let color = label.color
            // All colors should be valid RGB values
            XCTAssertLessThanOrEqual(color.r, 255)
            XCTAssertLessThanOrEqual(color.g, 255)
            XCTAssertLessThanOrEqual(color.b, 255)
        }
    }

    func testPipelineStages() {
        XCTAssertEqual(PipelineStage.allCases.count, 11)

        for stage in PipelineStage.allCases {
            // Each stage should have a description and short name
            XCTAssertFalse(stage.description.isEmpty)
            XCTAssertFalse(stage.shortName.isEmpty)
        }
    }

    func testBoundingBox() {
        let bbox = BoundingBox(x: 10, y: 20, width: 100, height: 50)

        XCTAssertEqual(bbox.left, 10)
        XCTAssertEqual(bbox.bottom, 20)
        XCTAssertEqual(bbox.right, 110)
        XCTAssertEqual(bbox.top, 70)
        XCTAssertEqual(bbox.center, CGPoint(x: 60, y: 45))
    }

    // MARK: - COCO Export Tests

    func testCOCODatasetCreation() {
        let dataset = COCODataset()

        XCTAssertEqual(dataset.info.description, "DoclingViz Document Layout Dataset")
        XCTAssertEqual(dataset.info.version, "1.0")
        XCTAssertEqual(dataset.info.contributor, "DoclingViz")
        XCTAssertTrue(dataset.images.isEmpty)
        XCTAssertTrue(dataset.annotations.isEmpty)
        XCTAssertEqual(dataset.categories.count, DocItemLabel.allCases.count)
    }

    func testCOCOCategories() {
        let categories = COCOCategory.documentLayoutCategories

        XCTAssertEqual(categories.count, 17)

        // Verify first category (caption)
        XCTAssertEqual(categories[0].id, 1)
        XCTAssertEqual(categories[0].name, "caption")
        XCTAssertEqual(categories[0].supercategory, "annotation")

        // Verify last category (keyValueRegion)
        XCTAssertEqual(categories[16].id, 17)
        XCTAssertEqual(categories[16].name, "key_value_region")
        XCTAssertEqual(categories[16].supercategory, "structured")
    }

    func testCOCOImage() {
        let image = COCOImage(id: 1, fileName: "test.png", width: 612, height: 792)

        XCTAssertEqual(image.id, 1)
        XCTAssertEqual(image.fileName, "test.png")
        XCTAssertEqual(image.width, 612)
        XCTAssertEqual(image.height, 792)
    }

    func testCOCOAnnotation() {
        let annotation = COCOAnnotation(
            id: 1,
            imageId: 1,
            categoryId: 9,
            bbox: [100.0, 200.0, 300.0, 400.0],
            area: 120000.0,
            score: 0.95
        )

        XCTAssertEqual(annotation.id, 1)
        XCTAssertEqual(annotation.imageId, 1)
        XCTAssertEqual(annotation.categoryId, 9)
        XCTAssertEqual(annotation.bbox, [100.0, 200.0, 300.0, 400.0])
        XCTAssertEqual(annotation.area, 120000.0)
        XCTAssertEqual(annotation.iscrowd, 0)
        XCTAssertEqual(annotation.score, 0.95)
    }

    func testCOCODatasetJSONExport() throws {
        var dataset = COCODataset()

        // Add an image
        dataset.images.append(COCOImage(id: 1, fileName: "doc_page_1.png", width: 612, height: 792))

        // Add an annotation
        dataset.annotations.append(COCOAnnotation(
            id: 1,
            imageId: 1,
            categoryId: 10, // text
            bbox: [50.0, 100.0, 200.0, 50.0],
            area: 10000.0,
            score: 0.99
        ))

        // Export to JSON
        let jsonData = try dataset.toJSONData()
        let jsonString = try dataset.toJSONString()

        XCTAssertFalse(jsonData.isEmpty)
        XCTAssertFalse(jsonString.isEmpty)

        // Verify it contains expected keys
        XCTAssertTrue(jsonString.contains("\"info\""))
        XCTAssertTrue(jsonString.contains("\"images\""))
        XCTAssertTrue(jsonString.contains("\"annotations\""))
        XCTAssertTrue(jsonString.contains("\"categories\""))
        XCTAssertTrue(jsonString.contains("\"doc_page_1.png\""))
    }

    func testDocItemLabelCOCONames() {
        // Verify all labels have COCO names
        for label in DocItemLabel.allCases {
            XCTAssertFalse(label.cocoName.isEmpty)
            XCTAssertFalse(label.cocoSupercategory.isEmpty)
        }

        // Check specific mappings
        XCTAssertEqual(DocItemLabel.text.cocoName, "text")
        XCTAssertEqual(DocItemLabel.text.cocoSupercategory, "text_block")
        XCTAssertEqual(DocItemLabel.table.cocoName, "table")
        XCTAssertEqual(DocItemLabel.table.cocoSupercategory, "structured")
        XCTAssertEqual(DocItemLabel.picture.cocoName, "picture")
        XCTAssertEqual(DocItemLabel.picture.cocoSupercategory, "media")
    }
}
