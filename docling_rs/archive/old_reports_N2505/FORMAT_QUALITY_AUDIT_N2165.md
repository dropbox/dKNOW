# Format Quality Audit - N=2165

**Date:** 2025-11-24
**Goal:** Test formats mentioned in WORLD_BEST_PARSER.txt (90-94% range) to identify improvable issues

## Test Results

### ✅ Above 95% (PASSING)
- **IPYNB**: 96% - Minor structural issues with horizontal lines, but PASSES threshold

### 90-94% Range (Close to Passing)
- **ICS**: 92% - Event details not clearly separated from calendar metadata
- **GLB**: 92% - After fixes (was 93%, LLM variance)
- **OBJ**: 92% - Title formatting differs slightly
- **KML**: 92% - (not detailed yet)
- **GPX**: 92% - (not detailed yet)

### 85-89% Range (Needs More Work)
- **ODS**: 88% - Sheet name lacks distinction from content, table alignment complaint (FALSE POSITIVE - code already aligns)
- **SVG**: 88% - (not detailed yet)
- **VCF**: 88% - (not detailed yet)
- **TAR**: 87% - (not detailed yet)

## Common Patterns Identified

**Structural Separation Issues (Multiple Formats):**
- ODS: "Sheet name lacks clear distinction from metadata and sheet content"
- ICS: "Event details not clearly separated from calendar metadata"
- Similar complaints across multiple formats

**Possible Root Cause:**
- Section headers may need better visual separation (extra blank lines?)
- Metadata sections may need clearer formatting
- May be systematic issue in markdown serializer or backend header generation

**LLM Variance:**
- GLB: 93% → 92% despite fixing two real bugs
- New complaints emerged after fixes
- Original fixes were still correct improvements

## Improvements Made

**GLB (crates/docling-cad/src/gltf/serializer.rs):**
1. ✅ Line 64: Added bullet point to "Total Materials" count
2. ✅ Line 185: Removed extra newline in summary section

## Next Steps

1. **Investigate structural separation pattern:**
   - Check how section headers are formatted
   - Test if adding extra blank lines improves scores
   - Look at successful formats (96%+) to see what they do differently

2. **Focus on formats near threshold (92-93%):**
   - ICS, GLB, OBJ, KML, GPX
   - These are closest to 95% and may benefit from small fixes

3. **Verify "alignment" complaints:**
   - ODS complaint about table alignment appears to be FALSE POSITIVE
   - Code already does sophisticated column width calculation (lines 760-898 in markdown.rs)
   - May be LLM misunderstanding markdown table format

## Philosophy Adherence

Following WORLD_BEST_PARSER.txt:
- ✅ Testing systematically through formats
- ✅ Verifying LLM complaints against code (found FALSE POSITIVES)
- ✅ Making real improvements even when score doesn't reflect it
- ✅ Not getting stuck on variance - moving to next format
- ✅ Documenting findings for future work

## Cost

- ~$0.30 in API calls for testing 10+ formats
- Test time: ~45 minutes
- Real bugs found: 2 (GLB formatting issues)
- False positives identified: 2 (ODS table alignment, GLB texture details)
