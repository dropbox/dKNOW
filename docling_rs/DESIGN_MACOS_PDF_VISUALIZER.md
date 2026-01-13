# DoclingViz - Native macOS PDF Extraction Debugger & Visualizer

## Executive Summary

A native macOS application for interactive PDF extraction debugging, ML inference visualization, and golden training set creation. Built with Swift/SwiftUI + Metal for maximum performance on Apple Silicon.

---

## Priority 1: Single PDF Step-by-Step Debugger

### Core Requirements

1. **PDF Canvas with Overlay System**
   - Render PDF page at full resolution using PDFKit/CGPDFDocument
   - Semi-transparent overlay layer for ML inference visualization
   - Bounding box rendering with label colors and confidence scores
   - Text cell highlighting with OCR confidence heat mapping

2. **Pipeline Stage Scrubber**
   - Timeline/slider showing all 10 pipeline stages
   - Step forward/backward through stages
   - View intermediate outputs at each stage:
     - Stage 0: Raw PDF render
     - Stage 1-3: OCR text cells + Layout clusters
     - Stage 4: Cell assignment (cells colored by cluster)
     - Stage 5: Empty cluster removal
     - Stage 6: Orphan text cells highlighted
     - Stage 7-8: BBox adjustment iterations (animated)
     - Stage 9: Final assembled elements
     - Stage 10: Reading order (numbered sequence overlay)

3. **Live Correction Tools**
   - **Bounding Box Editor**: Drag corners/edges to resize, drag center to move
   - **Label Changer**: Right-click → change DocItemLabel (dropdown of 17 classes)
   - **Merge/Split**: Select multiple boxes → merge; draw line through box → split
   - **Delete**: Select → delete spurious detections
   - **Create**: Draw new bounding box → assign label
   - **Text Edit**: Double-click text cell → edit OCR text
   - **Table Cell Editor**: Edit row/col spans, header flags

4. **Correction Persistence**
   - Save corrections as JSON alongside PDF
   - Format: `{pdf_name}.corrections.json`
   - Schema matches DoclingDocument with `corrected: true` flag
   - Export as training data (COCO format for layout, OTSL for tables)

---

## Architecture

### Application Structure

```
DoclingViz.app/
├── DoclingViz (Swift executable)
├── Frameworks/
│   ├── DoclingBridge.framework (Rust FFI bridge)
│   └── PDFRenderer.framework (Metal-accelerated rendering)
└── Resources/
    ├── ML Models/ (CoreML models for on-device inference)
    └── Color Schemes/ (label color mappings)
```

### Module Breakdown

```
┌─────────────────────────────────────────────────────────────────┐
│                        DoclingViz.app                           │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐ │
│  │  PDF Canvas │  │  Stage      │  │  Inspector Panel        │ │
│  │  (Metal)    │  │  Timeline   │  │  - Selected item details│ │
│  │             │  │             │  │  - Confidence scores    │ │
│  │  - Page     │  │  [|◀ ▶|]    │  │  - Edit controls        │ │
│  │  - Overlays │  │  Stage 4/10 │  │  - JSON preview         │ │
│  │  - Tools    │  │             │  │                         │ │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘ │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │  Element List (NSOutlineView - hierarchical by reading order)││
│  │  ├── [1] SectionHeader: "Introduction" (conf: 0.94)         ││
│  │  ├── [2] Text: "This paper presents..." (conf: 0.89)        ││
│  │  ├── [3] Table: 5x3 (conf: 0.91)                            ││
│  │  │   ├── Cell [0,0]: "Header A"                             ││
│  │  │   └── ...                                                ││
│  │  └── [4] Figure: (conf: 0.87)                               ││
│  └─────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
```

### Data Flow

```
                     ┌──────────────────┐
                     │   PDF File       │
                     └────────┬─────────┘
                              │
              ┌───────────────┼───────────────┐
              ▼               ▼               ▼
     ┌────────────┐   ┌─────────────┐   ┌──────────────┐
     │ PDFKit     │   │ Rust Engine │   │ CoreML       │
     │ Rendering  │   │ (FFI)       │   │ (optional)   │
     └─────┬──────┘   └──────┬──────┘   └──────┬───────┘
           │                 │                  │
           │    ┌────────────┴────────────┐     │
           │    ▼                         ▼     │
           │  ┌───────────────────────────────┐ │
           │  │  Pipeline Stage Snapshots     │ │
           │  │  (in-memory cache)            │ │
           │  │  - stage_0_raw.json           │ │
           │  │  - stage_4_assigned.json      │ │
           │  │  - stage_10_ordered.json      │ │
           │  └───────────────┬───────────────┘ │
           │                  │                 │
           ▼                  ▼                 ▼
     ┌─────────────────────────────────────────────┐
     │           Metal Render Pipeline             │
     │  ┌─────────┐  ┌─────────┐  ┌─────────────┐  │
     │  │ PDF     │→ │ Overlay │→ │ Composition │  │
     │  │ Texture │  │ Layer   │  │ Output      │  │
     │  └─────────┘  └─────────┘  └─────────────┘  │
     └─────────────────────────────────────────────┘
                          │
                          ▼
                   ┌─────────────┐
                   │   Display   │
                   │   (60fps)   │
                   └─────────────┘
```

---

## UI Design Specification

### Main Window Layout

```
┌────────────────────────────────────────────────────────────────────────────┐
│ [◀ ▶] Page 3/47   │ DoclingViz - research_paper.pdf            │ [⚙] [?] │
├────────────────────┬───────────────────────────────────────┬───────────────┤
│                    │                                       │               │
│   PAGES            │         PDF CANVAS                    │  INSPECTOR    │
│   ┌──────────┐     │                                       │               │
│   │ Page 1   │     │    ┌─────────────────────────┐        │  Selected:    │
│   │ [thumb]  │     │    │ ┌───────────────────┐   │        │  SectionHeader│
│   └──────────┘     │    │ │ Introduction      │←──┼────────│               │
│   ┌──────────┐     │    │ └───────────────────┘   │        │  Confidence:  │
│   │ Page 2   │     │    │ ┌─────────────────────┐ │        │  [████████] 94%
│   │ [thumb]  │     │    │ │ This paper presents │ │        │               │
│   └──────────┘     │    │ │ a novel approach... │ │        │  BBox:        │
│   ┌──────────┐     │    │ └─────────────────────┘ │        │  L: 72.4      │
│   │ Page 3 ◀─┼─────│    │ ┌─────────────────────┐ │        │  T: 120.8     │
│   │ [thumb]  │     │    │ │     TABLE 1         │ │        │  R: 540.2     │
│   └──────────┘     │    │ │  ┌───┬───┬───┐      │ │        │  B: 145.3     │
│   ┌──────────┐     │    │ │  │ A │ B │ C │      │ │        │               │
│   │ Page 4   │     │    │ │  ├───┼───┼───┤      │ │        │  [Edit Label] │
│   │ [thumb]  │     │    │ │  │ 1 │ 2 │ 3 │      │ │        │  [Delete]     │
│   └──────────┘     │    │ │  └───┴───┴───┘      │ │        │  [Split]      │
│   ...              │    │ └─────────────────────┘ │        │               │
│                    │    └─────────────────────────┘        │  ─────────────│
│                    │                                       │  JSON Preview │
│                    │    [Pan] [Zoom] [Select] [Draw]       │  {            │
│                    │                                       │   "label":    │
├────────────────────┴───────────────────────────────────────┤   "section_  │
│  PIPELINE STAGES                                           │    header",  │
│  ┌────┬────┬────┬────┬────┬────┬────┬────┬────┬────┐       │   "conf":    │
│  │ 0  │ 1  │ 2  │ 3  │ 4● │ 5  │ 6  │ 7  │ 8  │ 9  │       │    0.94      │
│  │Raw │OCR │Lay │Det │Asgn│Empt│Orph│Adj1│Adj2│Ord │       │  }           │
│  └────┴────┴────┴────┴────┴────┴────┴────┴────┴────┘       │               │
│  [|◀]  [◀]  [▶]  [▶|]   ▶ Play   Speed: [1x ▼]             │               │
├────────────────────────────────────────────────────────────┴───────────────┤
│  ELEMENTS (Reading Order)                                                  │
│  ┌────┬─────────────────┬────────────┬───────────┬─────────┬──────────────┐│
│  │ #  │ Type            │ Content    │ Conf      │ Stage   │ Modified     ││
│  ├────┼─────────────────┼────────────┼───────────┼─────────┼──────────────┤│
│  │ 1  │ PageHeader      │ "arXiv:..." │ 0.92     │ 4       │              ││
│  │ 2  │ SectionHeader   │ "Intro..." │ 0.94      │ 4       │ ✎ (edited)  ││
│  │ 3  │ Text            │ "This pa...│ 0.89      │ 4       │              ││
│  │ 4  │ Table (5×3)     │ [view]     │ 0.91      │ 9       │              ││
│  │ 5  │ Figure          │ [image]    │ 0.87      │ 4       │ ✎ (bbox)    ││
│  └────┴─────────────────┴────────────┴───────────┴─────────┴──────────────┘│
└────────────────────────────────────────────────────────────────────────────┘
```

### Color Scheme (Label → Color)

```swift
enum DocItemLabelColor {
    static let colors: [String: NSColor] = [
        "text":               .systemGray,
        "section_header":     .systemBlue,
        "page_header":        .systemPurple.withAlphaComponent(0.5),
        "page_footer":        .systemPurple.withAlphaComponent(0.5),
        "title":              .systemBlue.darker(),
        "caption":            .systemOrange,
        "footnote":           .systemBrown,
        "table":              .systemGreen,
        "figure":             .systemYellow,
        "picture":            .systemYellow.darker(),
        "formula":            .systemRed,
        "list_item":          .systemTeal,
        "code":               .systemIndigo,
        "checkbox_selected":  .systemGreen,
        "checkbox_unselected":.systemGray,
        "form":               .systemPink,
        "key_value_region":   .systemCyan,
    ]
}
```

---

## Priority 2: Batch Processing Visualization

### Grid View for Corpus Processing

```
┌────────────────────────────────────────────────────────────────────────────┐
│ DoclingViz - Batch Mode: /corpus/research_papers/ (1,247 PDFs)            │
├────────────────────────────────────────────────────────────────────────────┤
│  Progress: [████████████████░░░░░░░░░░░░░░] 423/1247 (34%) - ETA: 12:34   │
│  Throughput: 8.3 pages/sec │ Errors: 2 │ [Pause] [Stop] [Speed: 0.5x ▼]   │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐  │
│  │ ░░░░░░░ │ │ ████▓▓░ │ │ ████████│ │ ████████│ │ ░░░░░░░ │ │ ░░░░░░░ │  │
│  │ doc_001 │ │ doc_002 │ │ doc_003 │ │ doc_004 │ │ doc_005 │ │ doc_006 │  │
│  │ p3/12   │ │ p7/8  ◀─┼─┼─ACTIVE  │ │ ✓ done  │ │ queued  │ │ queued  │  │
│  └─────────┘ └─────────┘ └─────────┘ └─────────┘ └─────────┘ └─────────┘  │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐  │
│  │ ████████│ │ ████████│ │ ████████│ │ ░░░░░░░ │ │ ░░░░░░░ │ │ ░░░░░░░ │  │
│  │ doc_007 │ │ doc_008 │ │ doc_009 │ │ doc_010 │ │ doc_011 │ │ doc_012 │  │
│  │ ✓ done  │ │ ✓ done  │ │ ✓ done  │ │ queued  │ │ queued  │ │ queued  │  │
│  └─────────┘ └─────────┘ └─────────┘ └─────────┘ └─────────┘ └─────────┘  │
│                                                                            │
│  [Active Processing - doc_002 page 7]                                      │
│  ┌──────────────────────────────────────────────────────────────────────┐  │
│  │                    LIVE INFERENCE VISUALIZATION                      │  │
│  │  ┌─────────────────────────────────────────────────────────────────┐ │  │
│  │  │                                                                 │ │  │
│  │  │  ┌──────────────────┐  ← Layout detection (0.92)               │ │  │
│  │  │  │ Section Header   │                                          │ │  │
│  │  │  └──────────────────┘                                          │ │  │
│  │  │  ┌─────────────────────────────────────────────┐               │ │  │
│  │  │  │                  TABLE                      │ ← TableFormer │ │  │
│  │  │  │  ┌─────┬─────┬─────┐   (processing...)     │               │ │  │
│  │  │  │  │     │     │     │                        │               │ │  │
│  │  │  │  └─────┴─────┴─────┘                        │               │ │  │
│  │  │  └─────────────────────────────────────────────┘               │ │  │
│  │  │                                                                 │ │  │
│  │  └─────────────────────────────────────────────────────────────────┘ │  │
│  │  Stage: [████████▓▓░░░░░░░░░░] TableFormer (4/10)                    │  │
│  └──────────────────────────────────────────────────────────────────────┘  │
│                                                                            │
└────────────────────────────────────────────────────────────────────────────┘
```

### Playback Speed Control

```swift
enum PlaybackSpeed: Double, CaseIterable {
    case realtime = 1.0      // Actual processing speed
    case slow_2x = 0.5       // 2x slower (see details)
    case slow_4x = 0.25      // 4x slower (demo mode)
    case slow_10x = 0.1      // 10x slower (presentation)
    case fast_2x = 2.0       // Skip frames, 2x faster
    case fast_10x = 10.0     // Heavy frame skipping
}
```

### Real-time Statistics Panel

```
┌─────────────────────────────────────────────┐
│  LIVE STATISTICS                            │
├─────────────────────────────────────────────┤
│  Current Document: paper_2023_neural.pdf    │
│  Page: 7 / 12                               │
│                                             │
│  Stage Timings (this page):                 │
│  ├── OCR:           45ms  [██░░░░░░░░]     │
│  ├── Layout:       142ms  [████████░░] ←   │
│  ├── Cell Assign:    3ms  [░░░░░░░░░░]     │
│  ├── TableFormer:   89ms  [█████░░░░░]     │
│  ├── Reading Ord:    8ms  [░░░░░░░░░░]     │
│  └── Total:        287ms                    │
│                                             │
│  Cascade Stats:                             │
│  ├── Heuristic: 234 pages (56%)            │
│  ├── YOLO:       89 pages (21%)            │
│  ├── RT-DETR:    98 pages (23%)            │
│                                             │
│  Memory: 4.2 GB / 16 GB                     │
│  GPU:    34% (M2 Max)                       │
└─────────────────────────────────────────────┘
```

---

## Priority 3: Golden Training Set Builder

### Correction Export Formats

#### Layout Detection (COCO Format)
```json
{
  "images": [
    {"id": 1, "file_name": "paper_p1.png", "width": 2480, "height": 3508}
  ],
  "annotations": [
    {
      "id": 1,
      "image_id": 1,
      "category_id": 7,
      "bbox": [72.4, 120.8, 467.8, 24.5],
      "area": 11461.1,
      "iscrowd": 0,
      "corrected": true,
      "original_bbox": [70.0, 118.0, 470.0, 28.0],
      "original_confidence": 0.94
    }
  ],
  "categories": [
    {"id": 0, "name": "caption"},
    {"id": 1, "name": "footnote"},
    {"id": 7, "name": "section_header"},
    ...
  ]
}
```

#### Table Structure (OTSL + Cell Annotations)
```json
{
  "table_id": "paper_p3_table_0",
  "source_pdf": "paper.pdf",
  "page": 3,
  "bbox": [100, 200, 500, 400],
  "num_rows": 5,
  "num_cols": 3,
  "otsl_sequence": ["<start>", "ched", "ched", "ched", "nl", "fcel", "fcel", "fcel", "nl", ...],
  "cells": [
    {
      "row": 0, "col": 0,
      "row_span": 1, "col_span": 1,
      "is_header": true,
      "text": "Method",
      "bbox": [100, 200, 200, 230],
      "corrected": true
    }
  ],
  "correction_history": [
    {"timestamp": "2025-01-15T10:30:00Z", "user": "annotator1", "action": "cell_text_edit", "cell": [0,0]}
  ]
}
```

#### OCR Ground Truth
```json
{
  "image_id": "paper_p1_region_42",
  "source_pdf": "paper.pdf",
  "page": 1,
  "bbox": [72, 500, 540, 520],
  "text": "The experimental results demonstrate...",
  "char_boxes": [
    {"char": "T", "bbox": [72, 500, 80, 520]},
    {"char": "h", "bbox": [80, 500, 88, 520]},
    ...
  ],
  "corrected": true,
  "original_ocr": "The experirnental resu1ts dernonstrate...",
  "original_confidence": 0.78
}
```

---

## Implementation Plan

### Phase 1: Core Infrastructure (Week 1-2)

```
1.1 Swift/Rust FFI Bridge
    ├── Create DoclingBridge.framework
    ├── Expose pipeline stages via C API
    ├── Implement async processing with callbacks
    └── Memory-mapped sharing for large data

1.2 PDF Canvas Foundation
    ├── PDFKit integration for page rendering
    ├── Metal overlay rendering pipeline
    ├── Coordinate system conversion (PDF ↔ screen)
    └── Zoom/pan with smooth 60fps

1.3 Data Model Layer
    ├── Swift structs mirroring Rust types
    ├── Codable for JSON serialization
    ├── Undo/Redo stack for corrections
    └── Autosave with conflict resolution
```

### Phase 2: Single PDF Debugger (Week 3-4)

```
2.1 Pipeline Stage Visualization
    ├── Stage snapshot capture in Rust
    ├── Stage timeline UI component
    ├── Animated transitions between stages
    └── Diff highlighting (what changed)

2.2 Overlay Rendering System
    ├── Bounding box rendering with labels
    ├── Text cell highlighting
    ├── Table grid visualization
    ├── Reading order flow lines
    └── Confidence heatmap mode

2.3 Editing Tools
    ├── Selection tool (click, drag-select, lasso)
    ├── BBox manipulation handles
    ├── Label picker dropdown
    ├── Text editing modal
    └── Table cell editor
```

### Phase 3: Batch Visualization (Week 5-6)

```
3.1 Grid View Component
    ├── Virtualized grid (handle 10K+ documents)
    ├── Thumbnail generation pipeline
    ├── Status indicators (queued/processing/done/error)
    └── Click to open in debugger

3.2 Live Processing Display
    ├── IPC channel for real-time updates
    ├── Frame rate control (playback speed)
    ├── Stage progress animation
    └── Statistics dashboard

3.3 Playback Controls
    ├── Play/pause/step controls
    ├── Speed slider (0.1x to 10x)
    ├── Jump to specific document
    └── Filter by status/error type
```

### Phase 4: Training Set Export (Week 7-8)

```
4.1 Correction Tracking
    ├── Diff computation (original vs corrected)
    ├── Correction history log
    ├── Multi-user annotation support
    └── Conflict detection

4.2 Export Pipelines
    ├── COCO format for layout
    ├── OTSL format for tables
    ├── OCR ground truth format
    └── Batch export with filtering

4.3 Quality Dashboard
    ├── Annotation coverage stats
    ├── Inter-annotator agreement
    ├── Model performance tracking
    └── Training/validation split tools
```

---

## Technical Specifications

### Rust FFI Interface

```rust
// crates/docling-viz-bridge/src/lib.rs

use std::ffi::{c_char, c_void};
use std::os::raw::c_int;

/// Opaque handle to pipeline instance
pub struct PipelineHandle(*mut c_void);

/// Stage snapshot for visualization
#[repr(C)]
pub struct StageSnapshot {
    pub stage_id: c_int,
    pub stage_name: *const c_char,
    pub clusters_json: *const c_char,      // JSON string
    pub text_cells_json: *const c_char,    // JSON string
    pub timing_ms: f64,
}

/// Callback for real-time updates
pub type ProgressCallback = extern "C" fn(
    user_data: *mut c_void,
    doc_id: *const c_char,
    page_no: c_int,
    stage_id: c_int,
    snapshot: *const StageSnapshot,
);

#[no_mangle]
pub extern "C" fn docling_pipeline_create(
    config_json: *const c_char,
) -> *mut PipelineHandle;

#[no_mangle]
pub extern "C" fn docling_pipeline_process_pdf(
    handle: *mut PipelineHandle,
    pdf_path: *const c_char,
    callback: ProgressCallback,
    user_data: *mut c_void,
) -> c_int;

#[no_mangle]
pub extern "C" fn docling_pipeline_get_stage_snapshot(
    handle: *mut PipelineHandle,
    page_no: c_int,
    stage_id: c_int,
) -> *mut StageSnapshot;

#[no_mangle]
pub extern "C" fn docling_pipeline_apply_correction(
    handle: *mut PipelineHandle,
    correction_json: *const c_char,
) -> c_int;

#[no_mangle]
pub extern "C" fn docling_pipeline_export_training_data(
    handle: *mut PipelineHandle,
    format: *const c_char,  // "coco", "otsl", "ocr"
    output_path: *const c_char,
) -> c_int;

#[no_mangle]
pub extern "C" fn docling_snapshot_free(snapshot: *mut StageSnapshot);

#[no_mangle]
pub extern "C" fn docling_pipeline_destroy(handle: *mut PipelineHandle);
```

### Swift Bridge Layer

```swift
// DoclingBridge/Sources/DoclingBridge.swift

import Foundation

public class DoclingPipeline {
    private var handle: OpaquePointer?

    public init(config: PipelineConfig) throws {
        let configJSON = try JSONEncoder().encode(config)
        let configStr = String(data: configJSON, encoding: .utf8)!
        handle = configStr.withCString { docling_pipeline_create($0) }
        guard handle != nil else {
            throw DoclingError.initializationFailed
        }
    }

    public func processPDF(
        at path: URL,
        progress: @escaping (DocumentProgress) -> Void
    ) async throws -> ProcessingResult {
        return try await withCheckedThrowingContinuation { continuation in
            let context = CallbackContext(progress: progress, continuation: continuation)
            let contextPtr = Unmanaged.passRetained(context).toOpaque()

            path.path.withCString { pathPtr in
                docling_pipeline_process_pdf(
                    handle,
                    pathPtr,
                    { userData, docId, pageNo, stageId, snapshot in
                        let ctx = Unmanaged<CallbackContext>.fromOpaque(userData!).takeUnretainedValue()
                        let progress = DocumentProgress(
                            documentId: String(cString: docId!),
                            pageNumber: Int(pageNo),
                            stageId: Int(stageId),
                            snapshot: StageSnapshot(from: snapshot!.pointee)
                        )
                        DispatchQueue.main.async {
                            ctx.progress(progress)
                        }
                    },
                    contextPtr
                )
            }
        }
    }

    public func getStageSnapshot(page: Int, stage: Int) -> StageSnapshot? {
        guard let ptr = docling_pipeline_get_stage_snapshot(handle, Int32(page), Int32(stage)) else {
            return nil
        }
        defer { docling_snapshot_free(ptr) }
        return StageSnapshot(from: ptr.pointee)
    }

    public func applyCorrection(_ correction: Correction) throws {
        let json = try JSONEncoder().encode(correction)
        let result = String(data: json, encoding: .utf8)!.withCString {
            docling_pipeline_apply_correction(handle, $0)
        }
        guard result == 0 else {
            throw DoclingError.correctionFailed(code: Int(result))
        }
    }

    deinit {
        if let handle = handle {
            docling_pipeline_destroy(handle)
        }
    }
}
```

### Metal Overlay Renderer

```swift
// PDFRenderer/Sources/OverlayRenderer.swift

import Metal
import MetalKit
import simd

struct BoundingBoxVertex {
    var position: SIMD2<Float>
    var color: SIMD4<Float>
}

class OverlayRenderer {
    private let device: MTLDevice
    private let commandQueue: MTLCommandQueue
    private let pipelineState: MTLRenderPipelineState

    private var boxVertices: [BoundingBoxVertex] = []
    private var vertexBuffer: MTLBuffer?

    init(device: MTLDevice) throws {
        self.device = device
        self.commandQueue = device.makeCommandQueue()!

        let library = device.makeDefaultLibrary()!
        let vertexFunc = library.makeFunction(name: "boxVertex")!
        let fragmentFunc = library.makeFunction(name: "boxFragment")!

        let descriptor = MTLRenderPipelineDescriptor()
        descriptor.vertexFunction = vertexFunc
        descriptor.fragmentFunction = fragmentFunc
        descriptor.colorAttachments[0].pixelFormat = .bgra8Unorm
        descriptor.colorAttachments[0].isBlendingEnabled = true
        descriptor.colorAttachments[0].sourceRGBBlendFactor = .sourceAlpha
        descriptor.colorAttachments[0].destinationRGBBlendFactor = .oneMinusSourceAlpha

        self.pipelineState = try device.makeRenderPipelineState(descriptor: descriptor)
    }

    func updateBoxes(_ clusters: [Cluster], pageSize: CGSize, viewport: CGRect) {
        boxVertices.removeAll(keepingCapacity: true)

        for cluster in clusters {
            let color = DocItemLabelColor.color(for: cluster.label)
                .withAlphaComponent(0.3)
            let rgba = color.toSIMD4()

            // Convert PDF coordinates to normalized device coordinates
            let bbox = cluster.bbox.normalized(pageSize: pageSize, viewport: viewport)

            // Box fill (two triangles)
            let corners = [
                SIMD2<Float>(Float(bbox.minX), Float(bbox.minY)),
                SIMD2<Float>(Float(bbox.maxX), Float(bbox.minY)),
                SIMD2<Float>(Float(bbox.maxX), Float(bbox.maxY)),
                SIMD2<Float>(Float(bbox.minX), Float(bbox.maxY)),
            ]

            // Triangle 1
            boxVertices.append(BoundingBoxVertex(position: corners[0], color: rgba))
            boxVertices.append(BoundingBoxVertex(position: corners[1], color: rgba))
            boxVertices.append(BoundingBoxVertex(position: corners[2], color: rgba))

            // Triangle 2
            boxVertices.append(BoundingBoxVertex(position: corners[0], color: rgba))
            boxVertices.append(BoundingBoxVertex(position: corners[2], color: rgba))
            boxVertices.append(BoundingBoxVertex(position: corners[3], color: rgba))

            // Border (line strip) - stronger color
            let borderColor = color.withAlphaComponent(0.8).toSIMD4()
            // ... add border vertices
        }

        vertexBuffer = device.makeBuffer(
            bytes: boxVertices,
            length: boxVertices.count * MemoryLayout<BoundingBoxVertex>.stride,
            options: .storageModeShared
        )
    }

    func render(to drawable: CAMetalDrawable, commandBuffer: MTLCommandBuffer) {
        guard let vertexBuffer = vertexBuffer, !boxVertices.isEmpty else { return }

        let descriptor = MTLRenderPassDescriptor()
        descriptor.colorAttachments[0].texture = drawable.texture
        descriptor.colorAttachments[0].loadAction = .load  // Preserve PDF underneath
        descriptor.colorAttachments[0].storeAction = .store

        let encoder = commandBuffer.makeRenderCommandEncoder(descriptor: descriptor)!
        encoder.setRenderPipelineState(pipelineState)
        encoder.setVertexBuffer(vertexBuffer, offset: 0, index: 0)
        encoder.drawPrimitives(type: .triangle, vertexStart: 0, vertexCount: boxVertices.count)
        encoder.endEncoding()
    }
}
```

---

## Correction Data Model

```swift
// DoclingViz/Models/Correction.swift

import Foundation

/// Represents a single correction to the extraction output
struct Correction: Codable, Identifiable {
    let id: UUID
    let timestamp: Date
    let pageNumber: Int
    let elementId: String  // JSON pointer: "#/texts/0"
    let type: CorrectionType
    let original: CorrectionValue
    let corrected: CorrectionValue

    enum CorrectionType: String, Codable {
        case boundingBox
        case label
        case text
        case tableCellText
        case tableCellSpan
        case tableCellHeader
        case merge
        case split
        case delete
        case create
    }

    enum CorrectionValue: Codable {
        case bbox(BoundingBox)
        case label(String)
        case text(String)
        case span(rowSpan: Int, colSpan: Int)
        case headerFlags(column: Bool, row: Bool)
        case elementIds([String])
        case none
    }
}

/// Document with all corrections
struct CorrectedDocument: Codable {
    let sourceFile: URL
    let processedAt: Date
    let corrections: [Correction]
    let finalDocument: DoclingDocument

    func exportCOCO() -> COCODataset { ... }
    func exportOTSL() -> [OTSLTable] { ... }
    func exportOCRGroundTruth() -> [OCRSample] { ... }
}
```

---

## File Structure

```
DoclingViz/
├── DoclingViz.xcodeproj
├── DoclingViz/
│   ├── App/
│   │   ├── DoclingVizApp.swift
│   │   └── AppDelegate.swift
│   ├── Views/
│   │   ├── MainWindow/
│   │   │   ├── MainWindowView.swift
│   │   │   ├── PDFCanvasView.swift
│   │   │   ├── PageListView.swift
│   │   │   ├── InspectorView.swift
│   │   │   ├── ElementListView.swift
│   │   │   └── StageTimelineView.swift
│   │   ├── BatchView/
│   │   │   ├── BatchWindowView.swift
│   │   │   ├── DocumentGridView.swift
│   │   │   ├── LiveProcessingView.swift
│   │   │   └── StatisticsView.swift
│   │   └── Components/
│   │       ├── BoundingBoxOverlay.swift
│   │       ├── LabelPicker.swift
│   │       ├── ConfidenceIndicator.swift
│   │       └── PlaybackControls.swift
│   ├── Models/
│   │   ├── DoclingTypes.swift       // Swift mirrors of Rust types
│   │   ├── Correction.swift
│   │   ├── CorrectedDocument.swift
│   │   └── ExportFormats.swift
│   ├── ViewModels/
│   │   ├── DocumentViewModel.swift
│   │   ├── BatchViewModel.swift
│   │   └── CorrectionViewModel.swift
│   ├── Services/
│   │   ├── DoclingBridge.swift      // Rust FFI wrapper
│   │   ├── PDFRenderService.swift
│   │   └── ExportService.swift
│   └── Resources/
│       ├── Assets.xcassets
│       ├── LabelColors.json
│       └── Localizable.strings
├── DoclingBridge/                    // Swift Package for Rust FFI
│   ├── Package.swift
│   ├── Sources/
│   │   └── DoclingBridge/
│   │       ├── DoclingBridge.swift
│   │       ├── Types.swift
│   │       └── docling_bridge.h
│   └── docling-viz-bridge/           // Rust crate
│       ├── Cargo.toml
│       ├── src/
│       │   ├── lib.rs
│       │   ├── ffi.rs
│       │   └── snapshot.rs
│       └── cbindgen.toml
└── PDFRenderer/                      // Metal rendering framework
    ├── Package.swift
    └── Sources/
        └── PDFRenderer/
            ├── OverlayRenderer.swift
            ├── Shaders.metal
            └── CoordinateTransform.swift
```

---

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Space` | Play/Pause stage animation |
| `←` / `→` | Previous/Next stage |
| `⇧←` / `⇧→` | Previous/Next page |
| `⌘←` / `⌘→` | First/Last stage |
| `1`-`9` | Jump to stage 1-9 |
| `0` | Jump to stage 10 (reading order) |
| `V` | Select tool |
| `B` | Bounding box draw tool |
| `M` | Merge selected |
| `S` | Split selected |
| `Delete` | Delete selected |
| `⌘Z` | Undo |
| `⌘⇧Z` | Redo |
| `⌘S` | Save corrections |
| `⌘E` | Export training data |
| `⌘+` / `⌘-` | Zoom in/out |
| `⌘0` | Fit to window |
| `Tab` | Toggle inspector |
| `⌘I` | Show element info |

---

## Performance Targets

| Metric | Target | Notes |
|--------|--------|-------|
| PDF page render | < 16ms | 60fps for smooth zoom/pan |
| Overlay render | < 8ms | Must composite with PDF render |
| Stage transition | < 100ms | Smooth animation feel |
| Correction save | < 50ms | Instant feedback |
| Batch grid update | < 16ms | 60fps even with 1000+ items |
| Memory (single PDF) | < 500MB | For 100-page document |
| Memory (batch 1K) | < 2GB | Virtualized grid |

---

## Next Steps

1. **Create Xcode project** with Swift Package Manager dependencies
2. **Implement DoclingBridge** Rust crate with FFI exports
3. **Build PDF canvas** with Metal overlay system
4. **Implement stage timeline** with snapshot capture
5. **Add correction tools** one at a time (bbox first)
6. **Build batch view** with grid and live display
7. **Implement export pipelines** for training data

---

## Questions for User

1. **Multi-user annotation**: Do you need support for multiple annotators working on the same corpus with conflict resolution?

2. **Remote processing**: Should the batch view support connecting to a remote Rust processing server, or always local?

3. **Model retraining**: Do you want a "retrain model" button that kicks off fine-tuning with the corrected data?

4. **Versioning**: Should corrections be versioned (git-style history) or just latest state?

5. **Table editor complexity**: Full spreadsheet-style table editor, or simpler cell-by-cell editing?
