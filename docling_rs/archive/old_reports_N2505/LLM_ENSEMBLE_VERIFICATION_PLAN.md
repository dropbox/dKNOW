# LLM Ensemble Verification Plan

**Date:** 2025-12-05
**Goal:** Generate ground-truth PDF outputs using ensemble of top 3 LLMs
**Target:** All 14 test PDFs, per-page verification

---

## Phase 1: Infrastructure Setup

### 1.1 API Keys Required
```bash
# Already have (in .env):
OPENAI_API_KEY=sk-proj-...

# Need to add:
ANTHROPIC_API_KEY=...  # For Opus 4.5 API calls
# (Note: We're running ON Opus 4.5, but need API for programmatic calls)
```

### 1.2 Dependencies
```toml
# Add to Cargo.toml
reqwest = { version = "0.11", features = ["json"] }
base64 = "0.21"
tokio = { version = "1", features = ["full"] }
serde_json = "1.0"
```

### 1.3 PDF to Image Rendering
- Use pdfium to render each page at 300 DPI
- Output: PNG images for LLM vision input
- Already have pdfium-render in the project

---

## Phase 2: LLM Prompt Engineering

### 2.1 Structured Extraction Prompt

```
You are an expert document analysis system. Extract ALL content from this document page image.

OUTPUT FORMAT (JSON):
{
  "page_number": 1,
  "elements": [
    {
      "type": "title|section_header|paragraph|list_item|table|figure|caption|footnote|page_header|page_footer",
      "text": "exact text content",
      "confidence": 0.0-1.0,
      "bounding_box": {"x": 0, "y": 0, "width": 100, "height": 20},
      "attributes": {
        "level": 1,  // for headers
        "list_type": "bullet|numbered",  // for lists
        "rows": [],  // for tables
        "alt_text": ""  // for figures
      }
    }
  ],
  "reading_order": [0, 1, 2, ...],  // indices into elements array
  "markdown": "## Full markdown representation\n\nWith all content..."
}

RULES:
1. Extract EVERY piece of text visible on the page
2. Preserve exact spelling, punctuation, and formatting
3. For tables, extract complete cell contents row by row
4. For figures, describe what you see and extract any embedded text
5. Maintain reading order (top-to-bottom, left-to-right for Western text)
6. For RTL text (Arabic, Hebrew), maintain proper reading direction
7. Include page headers/footers separately
8. Report confidence for each element (0.0 = uncertain, 1.0 = certain)
```

### 2.2 Model-Specific Prompts

**Claude Opus 4.5:**
- Best at reasoning about document structure
- Add: "Think step by step about the document layout before extracting"

**OpenAI o1:**
- Strong at complex reasoning
- Add: "Use your reasoning capabilities to understand table structures"

**GPT-4o:**
- Fast, good at vision
- Add: "Focus on accurate text extraction"

---

## Phase 3: Ensemble Algorithm

### 3.1 Text Consensus (Character-Level)

```python
def ensemble_text(texts: List[str]) -> Tuple[str, float]:
    """
    Given 3 text outputs, find consensus.
    Returns (consensus_text, confidence)
    """
    if texts[0] == texts[1] == texts[2]:
        return texts[0], 1.0  # Perfect agreement

    # Find majority (2/3 agreement)
    for i, t in enumerate(texts):
        matches = sum(1 for other in texts if other == t)
        if matches >= 2:
            return t, 0.9  # Majority agreement

    # No majority - use character-level voting
    # ... complex alignment algorithm
    return aligned_text, 0.7
```

### 3.2 Structure Consensus

```python
def ensemble_structure(structures: List[Dict]) -> Dict:
    """
    Merge 3 structural analyses.
    """
    # Vote on element types
    # Merge bounding boxes (average)
    # Combine confidence scores
    pass
```

### 3.3 Confidence Scoring

| Agreement | Confidence |
|-----------|------------|
| 3/3 exact match | 1.00 |
| 2/3 exact match | 0.90 |
| 3/3 similar (>95% overlap) | 0.85 |
| 2/3 similar | 0.75 |
| No consensus | 0.50 (flag for review) |

---

## Phase 4: Implementation

### 4.1 File Structure

```
crates/docling-llm-verify/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── models/
│   │   ├── mod.rs
│   │   ├── claude.rs      # Opus 4.5 API
│   │   ├── openai.rs      # o1 and GPT-4o
│   │   └── types.rs       # Shared types
│   ├── ensemble/
│   │   ├── mod.rs
│   │   ├── merger.rs      # Consensus algorithm
│   │   └── scorer.rs      # Confidence scoring
│   ├── pdf/
│   │   ├── mod.rs
│   │   └── renderer.rs    # PDF to PNG
│   └── output/
│       ├── mod.rs
│       ├── docitems.rs    # DocItems format
│       └── markdown.rs    # Markdown format
└── tests/
    └── integration.rs
```

### 4.2 CLI Interface

```bash
# Verify single PDF
docling-llm-verify verify test.pdf --output results/

# Verify all test PDFs
docling-llm-verify verify-all test-corpus/pdf/ --output results/

# Compare against Rust output
docling-llm-verify compare results/ground-truth/ results/rust-output/
```

---

## Phase 5: Execution Plan

### 5.1 Cost Estimation

| Model | Cost/1M tokens | Avg tokens/page | Cost/page |
|-------|----------------|-----------------|-----------|
| Opus 4.5 | $15 in / $75 out | ~2000 | ~$0.15 |
| o1 | $15 in / $60 out | ~2000 | ~$0.12 |
| GPT-4o | $2.5 in / $10 out | ~2000 | ~$0.02 |

**Total per page:** ~$0.30
**14 PDFs × avg 5 pages × 3 models:** ~$63

### 5.2 Timeline

| Phase | Task | Duration |
|-------|------|----------|
| 1 | Infrastructure setup | 2 hours |
| 2 | Prompt engineering & testing | 3 hours |
| 3 | Ensemble algorithm | 4 hours |
| 4 | Run on all 14 PDFs | 1 hour |
| 5 | Analysis & comparison | 2 hours |

**Total:** ~12 hours

### 5.3 Deliverables

1. **Ground Truth Baselines** (`test-corpus/groundtruth/llm_ensemble/`)
   - Per-page JSON with DocItems
   - Per-document Markdown
   - Confidence scores

2. **Comparison Report** (`reports/llm_verification_results.md`)
   - Rust output vs LLM ensemble
   - Per-file accuracy scores
   - Specific discrepancies identified

3. **Quality Metrics**
   - Character-level accuracy
   - Structure detection accuracy
   - Table extraction accuracy

---

## Phase 6: Immediate Actions

### For Current Worker (N=2335):

1. **DO NOT** assume current baselines are correct
2. **Implement** PDF-to-image renderer (if not exists)
3. **Create** API client for OpenAI (GPT-4o first, cheapest)
4. **Test** on 1 page of 1 PDF to validate approach
5. **Scale** to all 14 PDFs once validated

### Quick Start Script

```bash
# 1. Load API key
source .env

# 2. Test with single page
./target/release/docling-llm-verify \
  --pdf test-corpus/pdf/picture_classification.pdf \
  --page 1 \
  --model gpt-4o \
  --output /tmp/test_output.json

# 3. Compare
diff /tmp/test_output.json expected_output.json
```

---

## Decision Point

**QUESTION FOR USER:**

Before implementing, please confirm:

1. **Budget:** Approve ~$63 for full verification run?
2. **API Keys:** Do you have Anthropic API key for Opus 4.5 calls?
3. **Scope:** All 14 PDFs or subset?
4. **Priority:** Implement now or after other work?

---

## Alternative: Manual Spot-Check

If full LLM ensemble is too expensive/complex:

1. Pick 3 "problem" PDFs
2. Manually verify 1 page each using Claude vision
3. Update baselines based on findings
4. Extrapolate to others

This would cost ~$1 and take ~1 hour.
