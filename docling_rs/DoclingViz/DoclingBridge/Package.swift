// swift-tools-version: 5.9
// The swift-tools-version declares the minimum version of Swift required to build this package.

import PackageDescription
import Foundation

// Get the path to the Rust target directory using the package's location
// Package is at DoclingViz/DoclingBridge, Rust target is at target/release
let packageDir = URL(fileURLWithPath: #filePath).deletingLastPathComponent().path
let rustTargetDir = "\(packageDir)/../../target/release"

let package = Package(
    name: "DoclingBridge",
    platforms: [
        .macOS(.v14)
    ],
    products: [
        .library(
            name: "DoclingBridge",
            targets: ["DoclingBridge"]
        ),
    ],
    targets: [
        // The Rust FFI bridge (C header)
        .target(
            name: "CDoclingBridge",
            path: "Sources/CDoclingBridge",
            publicHeadersPath: "include",
            linkerSettings: [
                .unsafeFlags(["-L\(rustTargetDir)"]),
            ]
        ),
        // Swift wrapper around the C FFI
        .target(
            name: "DoclingBridge",
            dependencies: ["CDoclingBridge"],
            path: "Sources/DoclingBridge",
            linkerSettings: [
                .unsafeFlags(["-L\(rustTargetDir)"]),
            ]
        ),
        .testTarget(
            name: "DoclingBridgeTests",
            dependencies: ["DoclingBridge"],
            path: "Tests/DoclingBridgeTests",
            linkerSettings: [
                .unsafeFlags(["-L\(rustTargetDir)"]),
            ]
        ),
    ]
)
