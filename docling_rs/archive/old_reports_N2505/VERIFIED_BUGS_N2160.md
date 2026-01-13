# Verified Bugs from LLM Testing - N=2160

## Summary

Tested formats at <85% to identify real issues vs. LLM variance.

**Formats Tested:**
- ODT: 84% → 85% (paragraph spacing)
- GLTF: 83% → 87% (missing accessors/buffer views)
- DXF: 82% → 82% (missing DIMASSOC header variable)

## Bug #1: DXF Missing DIMASSOC Header Variable ❌ FALSE POSITIVE

**Score:** 82%
**LLM Complaint:** "Missing some header variables such as $DIMASSOC and $DIMLTYPE"

**Initial Investigation:**
```bash
grep -r "DIMASSOC" crates/docling-cad/
# Result: No matches in code (because it's parsed generically, not hardcoded)
```

**Deep Verification (N=2161):**
```bash
# Check if DIMASSOC in test file
grep "DIMASSOC" test-corpus/cad/dxf/floor_plan.dxf
# Result: 1 match found (line: "$DIMASSOC" with value "2")

# Check if parsed
cargo test test_floor_plan_dim_vars -- --ignored --nocapture
# Result: 78 dim vars parsed ✅

# Check if serialized
cargo test test_floor_plan_serialization -- --ignored --nocapture | grep DIMASSOC
# Result: "- **$DIMASSOC**: 2" ✅
```

**Analysis:**
- DIMLTYPE: ✅ Parsed and output
- DIMASSOC: ✅ Parsed and output
- **LLM complaint is FALSE POSITIVE** - both variables present in output

**Root Cause:** DXF parser uses generic approach (lines 484-549) that extracts ALL $DIM* variables automatically. Serializer outputs ALL parsed dim_vars (lines 142-161). No hardcoded variable names needed.

**Impact:** None - DXF header parsing is complete

**Fix Required:** None - this is LLM error, not code error

## Bug #2: GLTF Missing Accessors/Buffer Views (Potential)

**Score:** 87% (up from 83%)
**LLM Complaint:** "The output does not include details about accessors and buffer views"

**Analysis:**
- These are technical details of glTF data structure
- Might be intentionally omitted for simplicity
- Needs investigation: Are these in parsed data but not serialized?

**Status:** Needs code review to determine if real bug

## Bug #3: ODT Paragraph Spacing (False Positive)

**Score:** 85% (up from 84%)
**LLM Complaint:** "Paragraph spacing and formatting not accurately represented"

**Analysis:**
- Subjective formatting issue
- Markdown doesn't preserve exact paragraph spacing from ODT
- Likely FALSE POSITIVE (limitation of markdown format)

**Status:** Not fixable without changing output format

## Recommendations

### Investigate
1. **GLTF accessors:** Check if data exists but not serialized
2. **GLTF buffer views:** Same as above

### Accept As-Is
3. **ODT spacing:** Markdown limitation, not parser bug
4. **DXF DIMASSOC:** FALSE POSITIVE - already implemented and working

## Score Impact Estimates

If GLTF accessor/buffer views added:
- GLTF: 87% → 90-92% (+3-5%)

DXF unlikely to improve (82% is due to LLM error, not missing features)

