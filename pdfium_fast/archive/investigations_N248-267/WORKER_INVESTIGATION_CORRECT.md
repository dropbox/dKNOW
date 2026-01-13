# Worker Investigation Was CORRECT - Follow Option A

**Found:** Worker's N=205 investigation report

**Their conclusion:** N=198 regenerated baselines using OUR binary (circular validation - WRONG)

**Their recommendation:** Option A - Rebuild upstream and regenerate from it

---

## Worker's Investigation (N=205)

### What They Found

1. **Original baselines:** From upstream PDFium 7f43fd79 (correct ground truth)
2. **N=198 mistake:** Regenerated using OUR binary (lost ground truth)
3. **N=206:** Restored upstream baselines from v1.3.0 git history
4. **Problem:** Our output still differs (N=197-202 changes)

### Worker's Recommendations

**Option A (Recommended):** Build upstream PDFium, regenerate baselines
**Option B:** Accept divergence, document it
**Option C:** Remove modifications, match upstream

---

## Upstream Binary Found!

**Location:** `~/pdfium-old-threaded/rust/pdfium-sys/out/Optimized-Shared/pdfium_test`

**This is the reference binary!**

---

## Action: Follow Worker's Option A

**Step 1: Generate True Upstream Baselines**

```bash
cd ~/pdfium_fast/integration_tests

# Use upstream pdfium_test to regenerate ALL baselines
for pdf in pdfs/benchmark/*.pdf; do
  basename=$(basename "$pdf" .pdf)

  # Render with UPSTREAM binary
  ~/pdfium-old-threaded/rust/pdfium-sys/out/Optimized-Shared/pdfium_test \
    --ppm "$pdf" /tmp/upstream_baseline/

  # Calculate MD5s
  # Save to baselines/upstream/images_ppm/$basename.json
done
```

**Step 2: Test Our Code Against True Baselines**

```bash
pytest -m image -v
# Any failures = our rendering differs from upstream
# Investigate each one
```

**Step 3: Fix or Document Differences**

**If differences found:**
- Minor (1-2 pixels): Document as acceptable (threading artifacts)
- Major: Fix the bug in our code

---

## This is the CORRECT Approach

**Worker already figured this out at N=205.**

**We need to:**
1. Use upstream binary (found it: ~/pdfium-old-threaded/)
2. Regenerate baselines from upstream
3. Test our code against THOSE baselines
4. Fix any real differences

**This gives us true correctness validation.**
