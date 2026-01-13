# Safe Optimization Development Workflow

**Date:** 2025-11-03
**Purpose:** How to develop PDFium optimizations without corrupting baseline

---

## Your Question

> "If I preserve the binary here, I don't need two copies of source code?"

**Answer: MOSTLY YES, with critical safety requirements.**

---

## The Safe Approach

### What Will Happen

**Baseline binary (SAFE):**
```
Location: out/Optimized-Shared/libpdfium.dylib
MD5: 00cd20f999bf60b1f779249dbec8ceaa
Status: Pre-compiled binary file
Effect of C++ changes: NONE (already compiled)
```

**When you modify C++ code:**
- Source files change
- Baseline binary stays UNCHANGED (it's already compiled)
- Need to build NEW binary to see changes

**This is safe UNLESS:**
- You accidentally run `ninja -C out/Optimized-Shared` (WILL OVERWRITE!)
- You run `gclient sync` (may trigger rebuild)
- You modify args.gn in out/Optimized-Shared/ (may trigger rebuild)

---

## CRITICAL: Protect Baseline Binary

### Step 1: Make Baseline Read-Only

```bash
# Protect baseline binary from accidental overwrite
chmod -R a-w out/Optimized-Shared/libpdfium.dylib
chmod -R a-w out/Optimized-Shared/pdfium_test

# Verify
ls -lh out/Optimized-Shared/libpdfium.dylib
# Should show: r-xr-xr-x (no 'w' flags)
```

Now if you accidentally try to rebuild, it will FAIL instead of overwriting.

### Step 2: Create Separate Build Directory for Optimizations

```bash
# Create new build config
mkdir -p out/Optimized-Custom

# Copy baseline config as starting point
cp out/Optimized-Shared/args.gn out/Optimized-Custom/args.gn

# Edit for your optimizations
vim out/Optimized-Custom/args.gn
# Add your optimization flags here

# Generate build files
./buildtools/mac/gn gen out/Optimized-Custom

# Build optimized version
ninja -C out/Optimized-Custom pdfium_test
```

**Result:**
- Baseline: `out/Optimized-Shared/libpdfium.dylib` (protected, unchanged)
- Optimized: `out/Optimized-Custom/libpdfium.dylib` (your changes)

---

## Development Workflow

### Making Changes

```bash
# 1. Modify C++ code
vim core/fpdftext/some_file.cpp

# 2. Build to NEW directory (NOT Optimized-Shared!)
ninja -C out/Optimized-Custom pdfium_test

# 3. Test optimized vs baseline
# Baseline:
out/Optimized-Shared/pdfium_test --ppm input.pdf

# Optimized:
out/Optimized-Custom/pdfium_test --ppm input.pdf

# 4. Compare outputs
diff baseline.ppm optimized.ppm
```

### Git Workflow

**Safe to commit:**
- ✅ C++ source changes (core/, fpdfsdk/, etc.)
- ✅ New build configs (out/Optimized-Custom/args.gn)
- ✅ Test results and documentation

**Never commit:**
- ❌ Binary files (*.dylib, pdfium_test executables)
- ❌ Build artifacts (*.o, *.a, gen/, obj/)
- ❌ Changes to out/Optimized-Shared/

---

## Testing Against Baseline

### Use Existing Baseline System

```bash
# Your baselines are already generated
ls integration_tests/baselines/upstream/images_ppm/*.json
# 451 baseline files

# Test your optimized binary
cd integration_tests
DYLD_LIBRARY_PATH=../out/Optimized-Custom pytest -m smoke

# Should pass if rendering matches baseline
# Failures indicate your changes affect output
```

### Comparison Testing

```python
# Test script example
baseline_binary = "out/Optimized-Shared/libpdfium.dylib"
optimized_binary = "out/Optimized-Custom/libpdfium.dylib"

# Render with both
render_with_binary(baseline_binary, pdf, out_baseline/)
render_with_binary(optimized_binary, pdf, out_optimized/)

# Compare
assert md5(out_baseline/) == md5(out_optimized/)  # Correctness
assert time(optimized) < time(baseline)  # Performance
```

---

## Safety Checklist

**Before starting development:**
- [ ] Baseline binary is read-only (`chmod -R a-w out/Optimized-Shared/lib*`)
- [ ] Baseline binary MD5 documented: 00cd20f999bf
- [ ] New build directory created: `out/Optimized-Custom/`
- [ ] .gitignore excludes binaries
- [ ] Baseline tests pass

**Before each build:**
- [ ] Building to NEW directory (not Optimized-Shared)
- [ ] Baseline binary still protected

**After each change:**
- [ ] Test against baseline (correctness)
- [ ] Measure performance difference
- [ ] Document changes in commit message

---

## Risks & Mitigation

### Risk 1: Accidental Rebuild

**Problem:** Running `ninja -C out/Optimized-Shared` overwrites baseline

**Mitigation:**
- Make binaries read-only
- Always build to out/Optimized-Custom/
- Add pre-commit hook to verify baseline MD5

### Risk 2: Source Sync Issues

**Problem:** `gclient sync` might trigger rebuilds

**Mitigation:**
- Don't run gclient sync unless intentional
- Use `gclient sync --nohooks` if needed
- Check baseline MD5 after any sync

### Risk 3: Lost Baseline

**Problem:** Baseline binary accidentally deleted or corrupted

**Mitigation:**
- Back up baseline binary:
  ```bash
  cp out/Optimized-Shared/libpdfium.dylib ~/baseline_backup/libpdfium_00cd20f999bf.dylib
  ```
- Document how to regenerate (checkout 7f43fd79, rebuild)

---

## Answer to Your Question

**YES, you can develop in this single repo:**
- Baseline binary won't change (already compiled)
- Modify C++ code freely
- Build to NEW directory (out/Optimized-Custom/)
- Test optimizations against preserved baseline

**NO separate source copy needed, IF:**
- You protect baseline binary (read-only)
- You always build to new directory
- You have backup of baseline binary

**Safest approach:** Make baseline read-only NOW before starting development.

---

**Recommendation:** Run this immediately:
```bash
# Protect baseline
chmod 444 out/Optimized-Shared/libpdfium.dylib
chmod 555 out/Optimized-Shared/pdfium_test

# Backup baseline
mkdir -p ~/pdfium_baseline_backup
cp out/Optimized-Shared/libpdfium.dylib ~/pdfium_baseline_backup/libpdfium_00cd20f999bf.dylib

# Verify protection
ls -lh out/Optimized-Shared/lib*
```

Then you're safe to start development.
