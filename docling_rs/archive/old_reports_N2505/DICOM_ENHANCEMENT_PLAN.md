# DICOM Advanced Metadata Enhancement Plan (N=1694+)

**Status:** Ready for implementation
**Priority:** Priority 3 (last remaining format at 90%)
**Estimated effort:** 2-3 commits
**Expected quality improvement:** 90% → 95%+

## Current State (N=1693)

### What's Already Implemented

**Parser** (`crates/docling-medical/src/dicom.rs`):
- ✅ Patient info (name, ID, birth date, sex)
- ✅ Study info (UID, date, time, description, ID, physician)
- ✅ Series info (UID, modality, number, description)
- ✅ Image info (SOP UIDs, instance number, dimensions, frames, image type)

**Backend** (`crates/docling-backend/src/dicom.rs`):
- ✅ Markdown generation from metadata
- ✅ DocItem creation (4 sections)
- ✅ Comprehensive test suite (75+ tests including advanced scenarios)

### What's Missing (Gaps)

According to PRIORITY_FORMATS_2025-11-20.md, DICOM needs "advanced medical metadata":

1. **Equipment/Technical Info** (HIGH VALUE)
   - Manufacturer (0008,0070)
   - Model Name / Manufacturer Model Name (0008,1090)
   - Station Name (0008,1010)
   - Software Version (0018,1020)

2. **Acquisition Parameters** (HIGH VALUE - Critical for medical imaging)
   - Pixel Spacing (0028,0030) - mm per pixel
   - Slice Thickness (0018,0050) - mm
   - Image Position (Patient) (0020,0032) - x,y,z coordinates
   - Window Center / Width (0028,1050, 0028,1051) - display settings
   - KVP (0018,0060) - X-ray tube voltage
   - Exposure (0018,1152) - mAs

3. **Anatomical Context** (MEDIUM VALUE)
   - Body Part Examined (0018,0015)
   - Patient Position (0018,5100) - HFS, HFP, FFS, etc.

4. **Advanced Features** (LOWER PRIORITY - Only if time permits)
   - Contrast agent info
   - MRI sequence parameters (TE, TR, flip angle)
   - CT dose information (CTDI, DLP)

## Recommended Implementation (N=1694-1696)

### Phase 1: Equipment & Acquisition (N=1694) - 1-2 hours
**File:** `crates/docling-medical/src/dicom.rs`

1. **Extend structs:**
   ```rust
   pub struct EquipmentInfo {
       pub manufacturer: Option<String>,
       pub model_name: Option<String>,
       pub station_name: Option<String>,
       pub software_version: Option<String>,
   }

   pub struct AcquisitionInfo {
       pub pixel_spacing: Option<String>,  // "1.5 × 1.5 mm"
       pub slice_thickness: Option<String>, // "5.0 mm"
       pub image_position: Option<String>,  // "x, y, z"
       pub kvp: Option<String>,
       pub exposure: Option<String>,
   }

   // Add to DicomMetadata struct:
   pub struct DicomMetadata {
       // ... existing fields ...
       pub equipment: Option<EquipmentInfo>,
       pub acquisition: Option<AcquisitionInfo>,
   }
   ```

2. **Add extraction functions:**
   ```rust
   fn extract_equipment_info(obj: &DefaultDicomObject) -> Option<EquipmentInfo>
   fn extract_acquisition_info(obj: &DefaultDicomObject) -> Option<AcquisitionInfo>
   ```

3. **Update `parse_dicom()` to call new extractors**

**File:** `crates/docling-backend/src/dicom.rs`

4. **Update markdown generation** (lines 105-174):
   - Add "## Equipment" section after Image Information
   - Add "## Acquisition Parameters" section
   - Display new fields if present

### Phase 2: Anatomical Context (N=1695) - 30 min
**File:** `crates/docling-medical/src/dicom.rs`

1. **Add to ImageInfo struct:**
   ```rust
   pub struct ImageInfo {
       // ... existing fields ...
       pub body_part_examined: Option<String>,
       pub patient_position: Option<String>,
   }
   ```

2. **Extract from DICOM tags:**
   - Body Part Examined (0018,0015)
   - Patient Position (0018,5100)

3. **Display in Image Information section**

### Phase 3: Testing & Polish (N=1696) - 30 min

1. **Run tests:**
   ```bash
   cargo test --package docling-backend --lib dicom::tests
   cargo test --package docling-medical --lib
   ```

2. **Test with real DICOM file:**
   ```bash
   cargo run --release -p docling-cli -- convert test-corpus/medical/dicom/*.dcm
   ```

3. **Verify output includes new sections:**
   - Equipment section with manufacturer, model
   - Acquisition section with pixel spacing, slice thickness
   - Body part in Image section

## DICOM Tag Reference (Standard Tags)

### Equipment Tags
- `(0008,0070)` Manufacturer - "Siemens", "GE", "Philips"
- `(0008,1090)` Manufacturer's Model Name - "SOMATOM Definition", "Discovery CT750"
- `(0008,1010)` Station Name - "CT01", "MRI_3T"
- `(0018,1020)` Software Versions - "syngo CT 2012B"

### Acquisition Tags
- `(0028,0030)` Pixel Spacing - [1.5, 1.5] → "1.5 × 1.5 mm"
- `(0018,0050)` Slice Thickness - "5.0" → "5.0 mm"
- `(0020,0032)` Image Position (Patient) - [-125.0, -125.0, 100.0]
- `(0028,1050)` Window Center - "40"
- `(0028,1051)` Window Width - "400"
- `(0018,0060)` KVP - "120"
- `(0018,1152)` Exposure - "250"

### Anatomical Tags
- `(0018,0015)` Body Part Examined - "CHEST", "HEAD", "ABDOMEN"
- `(0018,5100)` Patient Position - "HFS" (Head First Supine), "FFS" (Feet First Supine)

## Implementation Notes

1. **All fields should be Optional** - Not all DICOM files have all tags
2. **Format values nicely:**
   - Pixel spacing: "1.5 × 1.5 mm" (not raw array)
   - Position: "x, y, z" format
   - Units: Always include units (mm, kV, mAs)

3. **Use existing helper functions:**
   - `get_string_tag(obj, group, element)` for strings
   - `get_u16_tag(obj, group, element)` for numbers
   - May need `get_float_tag()` for pixel spacing, slice thickness

4. **Maintain test coverage:**
   - Existing 75+ tests should still pass
   - Add tests for new sections
   - Test optional field handling

5. **Backend update minimal:**
   - Just add new markdown sections
   - Use existing patterns from Patient/Study/Series/Image sections

## Success Criteria

After N=1696, DICOM output should include:
- ✅ Equipment section (if tags present)
- ✅ Acquisition parameters section (if tags present)
- ✅ Body part in Image section (if tag present)
- ✅ All existing tests passing
- ✅ Real DICOM file test shows new metadata

Expected quality score: **95%+** (up from 90%)

## Files to Modify

1. `crates/docling-medical/src/dicom.rs` - Add extraction logic
2. `crates/docling-backend/src/dicom.rs` - Update markdown generation
3. No test changes needed (optional: add specific equipment/acquisition tests)

## Time Estimate

- **Total:** 2-3 hours for Next AI
- **N=1694:** Equipment & acquisition extraction (1-2 hours)
- **N=1695:** Anatomical context (30 min)
- **N=1696:** Testing & verification (30 min)

## After DICOM

✅ **Priority 3 COMPLETE!** All formats 80-89% will be at 90%+

**Next Steps:**
- Move to Priority 2 formats (50-79%): VSDX, TEX, AVIF, HEIF, KEY
- Or move to Priority 4 polish (90-94%): DOCX, DXF, ICS, etc.

---

📋 Created by Claude Code (N=1693)
For Next AI to implement DICOM advanced metadata (N=1694-1696)
