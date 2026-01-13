# Plan: LLM Ensemble Baselines for PDF Verification

**Created:** 2025-12-04
**Status:** Planning
**Branch:** feature/38-of-38-plus-pdf-verification

## Objective

Create extremely reliable ground truth baselines for PDFs using an ensemble of top SOTA LLMs, producing both DocItem (JSON) and Markdown outputs.

## Current State Analysis

### Existing Infrastructure (`docling-llm-verify` crate)

1. **PDF Rendering** (`pdf/mod.rs`)
   - Renders PDF pages to PNG using pdfium
   - Configurable DPI (default 150)
   - Works correctly

2. **OpenAI Client** (`models/openai.rs`)
   - Supports GPT-4o and o1 models
   - Vision API integration
   - Cost tracking

3. **Ensemble Algorithm** (`ensemble/mod.rs`)
   - Element alignment via bbox IoU + text similarity
   - Majority voting for labels
   - Consensus text selection
   - Average bounding boxes

4. **Output** (`output/mod.rs`)
   - JSON export (DocItem format)
   - Markdown generation
   - Comparison utilities

### Current Limitations

1. **Only OpenAI models** - Missing Claude Opus 4.5 (excellent for document understanding)
2. **Anchor bias** - Uses first model as anchor, may miss elements unique to other models
3. **No Gemini** - Missing Google's Gemini Pro Vision
4. **Simple consensus** - Could use more sophisticated merging for tables

## Proposed Solution

### Phase 1: Add Claude Anthropic Support

Create `models/anthropic.rs`:

```rust
pub enum AnthropicModel {
    ClaudeOpus45,      // claude-opus-4-5-20251101
    ClaudeSonnet37,    // claude-3-5-sonnet-20241022
}
```

**API Details:**
- Endpoint: `https://api.anthropic.com/v1/messages`
- Vision: Base64 image in `content` array with `type: "image"`
- Cost: Opus 4.5 ~$15/1M input, $75/1M output

### Phase 2: Add Google Gemini Support (Optional)

Create `models/gemini.rs`:

```rust
pub enum GeminiModel {
    GeminiPro15,       // gemini-1.5-pro
    Gemini20Flash,     // gemini-2.0-flash
}
```

### Phase 3: Improve Ensemble Algorithm

**Current Issue:** First model anchors everything, potentially missing elements.

**Solution: Multi-pass alignment**
1. First pass: Collect ALL elements from ALL models
2. Second pass: Cluster by bbox/text similarity
3. Third pass: Vote on each cluster
4. Fourth pass: Resolve conflicts with confidence weighting

**Enhanced table merging:**
- Compare cell-by-cell across models
- Use OCR confidence for cell selection
- Preserve structure from model with most complete extraction

### Phase 4: Baseline Generation Pipeline

**Command-line interface:**
```bash
# Generate baselines for all test PDFs
docling-llm-verify generate-baselines \
    --input-dir test-corpus/pdf/ \
    --output-dir test-corpus/groundtruth/llm-ensemble/ \
    --models claude-opus-4.5,gpt-4o,o1 \
    --min-agreement 0.66 \
    --dpi 150
```

**Output structure:**
```
test-corpus/groundtruth/llm-ensemble/
├── 2305.03393v1/
│   ├── docitems.json       # Full DocItem format
│   ├── document.md         # Markdown rendering
│   ├── confidence.json     # Per-element agreement scores
│   ├── metadata.json       # Which models agreed on what
│   └── pages/
│       ├── page_1.json
│       └── page_1.png      # Annotated with bboxes
```

### Phase 5: Integration with Test Framework

Add new test suite:
```rust
#[test]
fn test_pdf_against_llm_ensemble_baseline() {
    let baseline = load_llm_baseline("2305.03393v1");
    let rust_output = rust_pdf_backend.convert("2305.03393v1.pdf");
    assert_eq!(baseline.compare(&rust_output).accuracy, 100.0);
}
```

## Implementation Tasks

### Task 1: Claude/Anthropic Client
- [ ] Create `models/anthropic.rs`
- [ ] Implement `AnthropicClient` with vision API
- [ ] Add cost tracking for Anthropic models
- [ ] Test with single PDF page

### Task 2: Update Ensemble Module
- [ ] Implement multi-pass alignment
- [ ] Add weighted voting based on model confidence
- [ ] Improve table cell merging
- [ ] Add minimum agreement threshold filtering

### Task 3: CLI Enhancements
- [ ] Add `--models` flag for model selection
- [ ] Add `generate-baselines` command
- [ ] Add `--min-agreement` threshold
- [ ] Add progress reporting for batch processing

### Task 4: Generate Baselines
- [ ] Run on all 14 test PDFs
- [ ] Verify output quality manually (spot check)
- [ ] Document any PDFs with low agreement

### Task 5: Test Integration
- [ ] Create test harness for baseline comparison
- [ ] Add `test_llm_baseline_*` tests
- [ ] Document expected accuracy thresholds

## Models to Use (Ranked by Document Understanding)

1. **Claude Opus 4.5** - Best overall document understanding
2. **GPT-4o** - Strong vision, good structure preservation
3. **o1** - Reasoning for complex layouts
4. **Gemini 1.5 Pro** (optional) - Good OCR, different perspective

## Cost Estimate

For 14 test PDFs (estimated 50 total pages):

| Model | Input Cost | Output Cost | Per PDF | Total |
|-------|-----------|-------------|---------|-------|
| Claude Opus 4.5 | $15/1M | $75/1M | ~$0.15 | ~$2.10 |
| GPT-4o | $2.50/1M | $10/1M | ~$0.03 | ~$0.42 |
| o1 | $15/1M | $60/1M | ~$0.12 | ~$1.68 |

**Total estimated cost: ~$4.20**

## Success Criteria

1. All 14 PDFs have LLM ensemble baselines
2. Minimum 2/3 model agreement on each element
3. Both DocItem JSON and Markdown outputs generated
4. Integration tests compare Rust output against baselines
5. Documentation of any elements with low agreement

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| API rate limits | Sequential processing with delays |
| Cost overrun | Use GPT-4o only mode for iteration |
| Model disagreement on tables | Manual review, use best extraction |
| Bounding box drift | Normalize coordinates, use relative |

## Environment Requirements

```bash
# Required API keys in .env
OPENAI_API_KEY=sk-proj-...
ANTHROPIC_API_KEY=sk-ant-...
# Optional
GOOGLE_API_KEY=...
```

## References

- Existing crate: `crates/docling-llm-verify/`
- Test PDFs: `test-corpus/pdf/`
- Current groundtruth: `test-corpus/groundtruth/docling_v2/`
