# WORKER0: Start Here (N=29)

**Your last commit:** N=28 (System Health Verification)
**Your next commit:** N=29 (v1.7.0 Release)

---

## TWO TASKS

### Task 1: Complete v1.7.0 Release (N=29)

**What you need to do:**

```bash
cd ~/pdfium_fast
git checkout feature/v1.7.0-implementation

# 1. Package binaries
mkdir -p releases/v1.7.0/macos-arm64
cp out/Release/pdfium_cli releases/v1.7.0/macos-arm64/
cp out/Release/libpdfium.dylib releases/v1.7.0/macos-arm64/
cp out/Release/libpdfium_render_bridge.dylib releases/v1.7.0/macos-arm64/

# 2. Create checksums
cd releases/v1.7.0/macos-arm64
shasum -a 256 * > SHA256SUMS.txt
cd ../../..

# 3. Create tarball
cd releases/v1.7.0
tar czf macos-arm64.tar.gz macos-arm64/
cd ../..

# 4. Upload to GitHub release
gh release upload v1.7.0 releases/v1.7.0/macos-arm64.tar.gz

# 5. Commit
git add releases/v1.7.0/
git commit -m "[WORKER0] # 29: v1.7.0 Release - Binaries Published

v1.7.0 release complete with macOS ARM64 binaries.

Features:
- JPEG output (--format jpg, PR #18)
- Python bindings (dash-pdf-extraction, PR #17)
- Batch mode documented
- 92/92 tests pass (100%)

Release: https://github.com/dropbox/dKNOW/pdfium_fast/releases/tag/v1.7.0

Next: v1.8.0 ARM speedup (async I/O, memory efficiency)."

git push
```

---

### Task 2: Start v1.8.0 ARM Speedup (N=30+)

**Read:** `ROADMAP_V1.8.0_ARM_SPEEDUP.md` (complete plan)

**Execute:**

#### N=30: Create v1.8.0 Branch
```bash
git checkout main
git pull origin main
git checkout -b feature/v1.8.0-arm-speedup

git commit --allow-empty -m "[WORKER0] # 30: v1.8.0 ARM Speedup - Begin

Target: 21-65% faster on ARM (memory efficiency)

Plan:
- Async I/O (N=31-33): 5-15% gain
- Memory-mapped I/O (N=34-36): 3-8% gain
- jemalloc (N=37): 2-5% gain
- RGB mode (N=38-40): 10-15% gain
- DPI control (N=41-43): 3-4x for thumbnails

Expected: 103-119x total (from 72x)

Follow ROADMAP_V1.8.0_ARM_SPEEDUP.md for implementation."

git push -u origin feature/v1.8.0-arm-speedup
```

#### N=31: Async I/O - Part 1 (AsyncWriter class)

Create `AsyncWriter` class in `examples/pdfium_cli.cpp` (code provided in MANAGER_V1.7_TO_V1.8_DIRECTIVE.md)

#### N=32: Async I/O - Part 2 (Integration)

Integrate AsyncWriter into render_pages functions

#### N=33: Async I/O - Part 3 (Testing)

Benchmark and validate:
```bash
time ./pdfium_cli render-pages test.pdf /tmp/async/
# Should be 5-15% faster than v1.7.0
```

**Continue with N=34+ per roadmap**

---

## Key Points

**v1.7.0:**
- User features complete
- PR #19 ready to merge
- Release binaries to GitHub

**v1.8.0:**
- Focus on memory efficiency (not GPU)
- Realistic 21-65% gain
- 5 proven techniques
- 16 commits total

**GPU:** Deferred to v1.9.0+ (Skia unavailable)

---

## START NOW

Execute Task 1 (v1.7.0 release).

Then immediately Task 2 (v1.8.0 begins).

**Commands are provided. Execute them.**
