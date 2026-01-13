# Visualization Tools Backlog

## HIGH PRIORITY

### 1. Element ID Numbers on All Boxes
**Status:** NOT IMPLEMENTED
**Problem:** Currently boxes only show reading order numbers in `--stage reading-order`. Need unique IDs visible on ALL stages.
**Solution:**
- Add `--show-ids` flag to dlviz-screenshot
- Display element ID (not reading order) as primary identifier
- Format: `[ID:42] text 98%`

### 2. Interactive Annotation Review Tool
**Status:** NOT IMPLEMENTED
**Problem:** No way for human/AI to interactively review and correct detections
**Requirements:**
- View PDF page with detection overlays
- Click element to select it
- Change label (dropdown: text → author, abstract, etc.)
- Adjust bounding box (drag corners/edges)
- Add new element (draw box, assign label)
- Delete false positive (click, press delete)
- Split element (one box → two)
- Merge elements (select multiple → combine)
- Save corrections to JSON (for dlviz-apply-corrections)

**Implementation Options:**
1. **Web UI** (HTML/JS + Rust backend via WebSocket)
   - Pro: Cross-platform, easy to iterate
   - Con: More complex architecture
2. **Native macOS app** (Swift + Metal)
   - Pro: Fast, native feel
   - Con: macOS only
3. **Terminal UI** (ratatui)
   - Pro: Simple, works in SSH
   - Con: Limited interaction

**Recommended:** Web UI with local server

### 3. Fine-Grained Labels
**Status:** NOT IMPLEMENTED
**Problem:** ML model only has 17 coarse labels. Missing:
- Author
- Abstract
- Keywords
- Affiliation
- Page number
- References/Bibliography
- Date
- DOI
- Email

**Solution Options:**
1. Heuristic post-processing (position + regex patterns)
2. Train new model with fine-grained labels
3. LLM classification of detected text elements

---

## MEDIUM PRIORITY

### 4. Batch Annotation Workflow
- Process directory of PDFs
- AI pre-annotates with confidence scores
- Human reviews only low-confidence or flagged pages
- Export corrections as training data

### 5. Keyboard Shortcuts for Annotation
- `1-9` = assign label by number
- `d` = delete element
- `n` = new element mode
- `s` = save
- `←/→` = prev/next page
- `Tab` = next element

### 6. Annotation History/Undo
- Track all changes
- Undo/redo stack
- Export change log

---

## LOW PRIORITY

### 7. Multi-User Annotation
- Consensus from multiple annotators
- Inter-annotator agreement metrics

### 8. Active Learning Integration
- Model suggests elements for review
- Prioritize uncertain predictions
