// swift-tools-version: 5.9
// DoclingViz - macOS PDF Extraction Visualizer

import PackageDescription
import Foundation

// Get the path to the Rust target directory
let packageDir = URL(fileURLWithPath: #filePath).deletingLastPathComponent().path
let rustTargetDir = "\(packageDir)/../../target/release"

let package = Package(
    name: "DoclingViz",
    platforms: [
        .macOS(.v14)
    ],
    products: [
        .executable(
            name: "DoclingViz",
            targets: ["DoclingViz"]
        ),
    ],
    dependencies: [
        .package(path: "../DoclingBridge"),
    ],
    targets: [
        .executableTarget(
            name: "DoclingViz",
            dependencies: ["DoclingBridge"],
            path: "Sources/DoclingViz",
            linkerSettings: [
                .unsafeFlags(["-L\(rustTargetDir)"]),
                .linkedLibrary("docling_viz_bridge"),
            ]
        ),
    ]
)
