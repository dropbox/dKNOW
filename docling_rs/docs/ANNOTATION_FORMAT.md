# PDF Annotation Format

## Overview

This document describes the annotation format used for reviewing and correcting PDF document element detections.

## Element Structure

Each detected element has:

| Field | Type | Description |
|-------|------|-------------|
| `id` | u32 | Unique element identifier (stable across sessions) |
| `label` | string | Element type (see Labels below) |
| `confidence` | f32 | ML model confidence (0.0 - 1.0) |
| `bbox` | object | Bounding box coordinates |
| `text` | string? | Extracted text content (optional) |
| `reading_order` | i32 | Reading sequence (-1 if unassigned) |

### Bounding Box

```json
{
  "x": 72.0,      // Left edge (points from page left)
  "y": 100.5,    // Top edge (points from page top)
  "width": 468.0,
  "height": 24.0
}
```

Coordinates are in PDF points (1/72 inch), origin at **top-left** of page.

## Labels

### Current ML Labels (17)

| ID | Label | Description | Color |
|----|-------|-------------|-------|
| 0 | `caption` | Figure/table caption | Orange |
| 1 | `footnote` | Footnotes | Brown |
| 2 | `formula` | Math equations | Red |
| 3 | `list_item` | Bullet/numbered items | Cyan |
| 4 | `page_footer` | Page footers | Light Gray |
| 5 | `page_header` | Page headers | Light Gray |
| 6 | `picture` | Images/figures | Yellow |
| 7 | `section_header` | Section headings | Blue |
| 8 | `table` | Tables | Green |
| 9 | `text` | Regular paragraphs | Gray |
| 10 | `title` | Document title | Purple |
| 11 | `code` | Code blocks | Blue-Gray |
| 12 | `checkbox_selected` | Checked checkbox | Gray |
| 13 | `checkbox_unselected` | Unchecked checkbox | Gray |
| 14 | `document_index` | TOC entries | Indigo |
| 15 | `form` | Form elements | Pink |
| 16 | `key_value_region` | Key-value pairs | Deep Orange |

### Extended Labels (for manual annotation)

These labels can be assigned during human/AI review:

| Label | Description | Typically found in |
|-------|-------------|-------------------|
| `author` | Author names | Below title |
| `affiliation` | Institution/organization | Below authors |
| `abstract` | Abstract text | After authors, before body |
| `keywords` | Keyword list | After abstract |
| `date` | Publication date | Near title/header |
| `doi` | DOI identifier | Header/footer |
| `email` | Email addresses | With affiliations |
| `reference` | Bibliography entry | End of document |
| `page_number` | Page number | Header/footer |
| `equation_number` | Equation reference number | Right of formula |

## Correction Format

Corrections are stored in `corrections.json`:

```json
{
  "version": "1.0",
  "pdf": "paper.pdf",
  "created": "2024-01-03T12:00:00Z",
  "corrections": [
    {
      "type": "relabel",
      "page": 0,
      "element_id": 5,
      "old_label": "text",
      "new_label": "author"
    },
    {
      "type": "bbox",
      "page": 0,
      "element_id": 12,
      "old_bbox": {"x": 72, "y": 100, "width": 200, "height": 20},
      "new_bbox": {"x": 72, "y": 98, "width": 210, "height": 24}
    },
    {
      "type": "delete",
      "page": 1,
      "element_id": 42,
      "reason": "false_positive"
    },
    {
      "type": "add",
      "page": 1,
      "label": "page_number",
      "bbox": {"x": 300, "y": 750, "width": 20, "height": 12},
      "text": "42"
    },
    {
      "type": "split",
      "page": 2,
      "element_id": 88,
      "into": [
        {"label": "author", "bbox": {"x": 72, "y": 100, "width": 150, "height": 20}},
        {"label": "affiliation", "bbox": {"x": 72, "y": 122, "width": 200, "height": 18}}
      ]
    },
    {
      "type": "merge",
      "page": 3,
      "element_ids": [10, 11, 12],
      "into_label": "abstract"
    }
  ]
}
```

## Workflow

### 1. Generate Visualizations

```bash
# Generate numbered visualization for all pages
dlviz-screenshot paper.pdf --all --show-ids --output-dir ./review/
```

Output:
- `review/paper_p0.png` - Page 0 with numbered boxes
- `review/paper_p0.json` - Element metadata (sidecar)
- `review/paper_p1.png` - Page 1...

### 2. Review & Annotate

**Option A: Manual JSON editing**
Create `corrections.json` based on visual inspection.

**Option B: Interactive tool** (planned)
```bash
dlviz-annotate ./review/
# Opens web UI at http://localhost:8080
```

**Option C: AI review**
```bash
# Send images + JSON to Claude/GPT-4V for review
dlviz-ai-review ./review/ --model claude-3-opus
```

### 3. Apply Corrections

```bash
dlviz-apply-corrections ./review/ --output ./golden/
```

Output:
- `golden/coco_annotations.json` - COCO format for training
- `golden/yolo/` - YOLO format labels
- `golden/corrected_sidecars/` - Updated JSON sidecars

### 4. Export Training Data

The corrected annotations can be used to fine-tune the layout model:

```bash
# COCO format (detectron2, mmdetection)
golden/coco_annotations.json

# YOLO format (ultralytics)
golden/yolo/
  images/
  labels/
  data.yaml
```

## JSON Sidecar Example

Each visualization PNG has a companion JSON:

```json
{
  "pdf": "2305.03393v1.pdf",
  "page": 0,
  "page_size": {"width": 612.0, "height": 792.0},
  "stage": "layout_detection",
  "render_time_ms": 153.2,
  "element_count": 12,
  "elements": [
    {
      "id": 0,
      "label": "title",
      "confidence": 0.96,
      "bbox": {"x": 108, "y": 89, "width": 396, "height": 48},
      "text": "Optimized Table Tokenization for Table Structure Recognition",
      "reading_order": 0
    },
    {
      "id": 1,
      "label": "text",
      "confidence": 0.79,
      "bbox": {"x": 126, "y": 147, "width": 360, "height": 36},
      "text": "Maksym Lysak, Ahmed Nassar, Nikolaos Livathinos...",
      "reading_order": 1
    }
  ],
  "statistics": {
    "by_label": {"title": 1, "text": 8, "section_header": 2, "page_header": 1},
    "avg_confidence": 0.89,
    "low_confidence_count": 2
  }
}
```

## Best Practices

1. **Review low-confidence elements first** - Elements with confidence < 0.8 are more likely to be wrong
2. **Check page boundaries** - Headers/footers often misclassified as text
3. **Verify reading order** - Especially for multi-column layouts
4. **Split compound elements** - Author + affiliation often detected as single text block
5. **Add missing elements** - Page numbers, equation labels often missed
