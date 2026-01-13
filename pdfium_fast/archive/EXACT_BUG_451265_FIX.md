# EXACT bug_451265 Fix from Old Version - Line-by-Line Guide

**Worker**: You're debugging this at #231. Here's the EXACT working fix.

**Source**: ~/pdfium-old-threaded commit 266010fb6

---

## Change 1: cpdf_renderstatus.h

**Add to class CPDF_RenderStatus (around line 104)**:

```cpp
// Add public getter for pattern cache
std::vector<RetainPtr<const CPDF_TilingPattern>>& GetTilingPatternCache() {
  return tiling_pattern_cache_;
}
```

**Add to private members (around line 204)**:

```cpp
// Pattern cache to detect circular references (models Type3 font cache pattern)
std::vector<RetainPtr<const CPDF_TilingPattern>> tiling_pattern_cache_;
```

**Add friend declaration (around line 136)**:

```cpp
friend class CPDF_RenderTiling;
```

---

## Change 2: cpdf_renderstatus.cpp

**In Initialize() method (around line 191-194)**:

```cpp
// ADD AFTER: page_resource_.Reset(context_->GetPageResources());

// Automatically propagate tiling pattern cache from parent to detect circular references
if (pParentStatus) {
  tiling_pattern_cache_ = pParentStatus->tiling_pattern_cache_;
}
```

**In DrawTilingPattern() method (around line 1217, BEFORE pPatternForm = pattern->Load())**:

```cpp
// ADD AT START OF FUNCTION:

// Protect against circular pattern references (similar to Type3 font handling)
if (pdfium::Contains(tiling_pattern_cache_, pattern)) {
  return;
}

// Protect against infinite recursion from circular pattern references
AutoRestorer<int> recursion_restorer(&g_CurrentRecursionDepth);
if (++g_CurrentRecursionDepth > kRenderMaxRecursionDepth) {
  return;
}

// Add pattern to cache before loading/rendering to prevent circular references
tiling_pattern_cache_.emplace_back(pattern);
```

**Also in DrawTilingPattern()** - change variable name to avoid conflict:

```cpp
// OLD:
CFX_RenderDevice::StateRestorer restorer(device_);

// NEW:
CFX_RenderDevice::StateRestorer device_restorer(device_);
```

---

## Change 3: cpdf_rendertiling.cpp

**In DrawPatternBitmap() method (around line 67)**:

```cpp
// OLD:
bitmap_status.Initialize(nullptr, nullptr, ...);

// NEW:
bitmap_status.Initialize(pRenderStatus, nullptr, ...);
```

**In Draw() method (around lines 219-235, ADD pathological tile check)**:

```cpp
// ADD BEFORE: for (int col = min_col; col <= max_col; col++)

// Sanity check on tile counts to prevent pathological loops
int64_t col_count = static_cast<int64_t>(max_col) - static_cast<int64_t>(min_col) + 1;
int64_t row_count = static_cast<int64_t>(max_row) - static_cast<int64_t>(min_row) + 1;

// Reasonable limits for tiling patterns (check each dimension separately to avoid overflow)
const int64_t kMaxReasonableDimension = 1000;

// Check for negative, zero, or pathological counts
if (col_count <= 0 || row_count <= 0) {
  return nullptr;
}

// Check each dimension independently to avoid overflow in multiplication
if (col_count > kMaxReasonableDimension || row_count > kMaxReasonableDimension) {
  return nullptr;
}
```

---

## Summary of Changes

**3 files, 4 locations**:

1. **cpdf_renderstatus.h**: Add tiling_pattern_cache_ member + getter + friend
2. **cpdf_renderstatus.cpp**:
   - Initialize(): Inherit cache from parent
   - DrawTilingPattern(): Check cache, add recursion guards, add to cache
3. **cpdf_rendertiling.cpp**:
   - DrawPatternBitmap(): Pass pRenderStatus (not nullptr)
   - Draw(): Add tile dimension checks

---

## Test After Applying

```bash
cd ~/pdfium_fast

# Build
ninja -C out/Release pdfium_cli

# Test (should complete in <1 second, not hang)
time out/Release/pdfium_cli render-pages testing/resources/bug_451265.pdf /tmp/test_451265/

# Expected: ~0.01-0.02 seconds
# If hangs: Something is wrong with the port
```

---

## If Still Doesn't Work

**Compare line-by-line**:

```bash
diff ~/pdfium_fast/core/fpdfapi/render/cpdf_renderstatus.cpp \
     ~/pdfium-old-threaded/core/fpdfapi/render/cpdf_renderstatus.cpp

# Find any differences in Initialize() or DrawTilingPattern()
# Copy the old version's implementation exactly
```

---

**This is the complete, working fix from old version.** Just copy it exactly!
