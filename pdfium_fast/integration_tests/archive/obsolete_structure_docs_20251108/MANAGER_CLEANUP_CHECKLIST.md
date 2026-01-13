# MANAGER CLEANUP CHECKLIST - Professional Assessment

**Role:** Professional C++ engineer at Google
**Task:** Clean up repository to simple v1.2/v1.3 interface
**Approach:** Skeptical, rigorous, delete unnecessary complexity

---

## FINDINGS - Current Repository Structure

### What We Have (Verified):

**Root directory (`/Users/ayates/pdfium/`):**
- **Purpose:** GitHub fork repo, INCOMPLETE PDFium source
- **Git remote:** origin = pdfium.googlesource.com/pdfium/
- **Has:** Core PDFium source (core/, fpdfsdk/, public/, fxbarcode/, fxjs/)
- **MISSING:** Complete dependencies (third_party/googletest/, many others)
- **MISSING:** skia/ at root (has third_party/skia/ but BUILD.gn references //skia)
- **Status:** CANNOT build standalone with `gn gen out/Release`
- **Custom additions:** rust/, integration_tests/, docs

**pdfium/ subdirectory (`/Users/ayates/pdfium/pdfium/`):**
- **Purpose:** Complete upstream PDFium checkout (13GB)
- **Git remote:** origin = pdfium.googlesource.com/pdfium.git
- **Has:** EVERYTHING including skia/, all deps, complete source
- **Managed by:** .gclient file
- **Status:** CAN build with `gn gen out/Profile`
- **Custom additions:** examples/ (copied from root by worker)

**Key insight:** pdfium/ is a DUPLICATE complete checkout added to work around incomplete root

---

## THE ACTUAL PROBLEM

**v1.3.0 claimed:**
> git clone, cd pdfium, gn gen out/Release, ninja pdfium_cli

**Reality:**
- Root cannot build (missing deps)
- Worker added pdfium/ subdirectory to get buildable version
- Now we have 13GB duplicate

**This is NOT how Google structures PDFium repositories!**

---

## PROPER STRUCTURE (How It Should Be)

**Option A: Single Root (Google Style)**
```
pdfium/                    # ← Root of repository
  core/                    # PDFium source
  fpdfsdk/                 # PDFium source
  examples/                # Custom code
    pdfium_cli.cpp         # Our tool
    BUILD.gn               # Our targets
  BUILD.gn                 # Main build file
  out/Profile/             # Build output
  integration_tests/       # Our tests
  rust/                    # Our Rust tools
```

Build: `gn gen out/Profile && ninja -C out/Profile pdfium_cli`

**Option B: Submodule/Subtree (Current Accidental State)**
```
pdfium_fast/               # ← Root of our repo
  pdfium/                  # ← Upstream as subdir
    examples/              # Our custom code here
    out/Profile/           # Build here
  integration_tests/       # Our tests here
  rust/                    # Our Rust here
```

Build: `cd pdfium && gn gen out/Profile && ninja pdfium_cli`

---

## CHECKLIST FOR CLEANUP

### [ ] Investigation Phase

- [x] **1.1** Verify root has PDFium source (core/, fpdfsdk/, public/) ✓
- [x] **1.2** Verify root MISSING complete deps (skia/, googletest/) ✓
- [x] **1.3** Verify pdfium/ is complete PDFium checkout (13GB) ✓
- [x] **1.4** Check git remotes (both point to upstream) ✓
- [x] **1.5** Find working binary location (pdfium/out/Profile/pdfium_cli) ✓
- [x] **1.6** Check test suite references (tests use pdfium/out/Profile/) ✓

### [ ] Decision Phase

- [ ] **2.1** DECISION: Keep pdfium/ subdir structure OR fix root to be complete?
- [ ] **2.2** If keep pdfium/: Delete PDFium source at root (keep only custom code)
- [ ] **2.3** If fix root: Delete pdfium/ and properly fetch all deps

### [ ] Cleanup Phase (Assuming pdfium/ structure)

- [ ] **3.1** Delete PDFium source files at root:
  - core/ (exists in pdfium/)
  - fpdfsdk/ (exists in pdfium/)
  - fxbarcode/ (exists in pdfium/)
  - fxjs/ (exists in pdfium/)
  - public/ (exists in pdfium/)
  - constants/ (exists in pdfium/)
  - testing/ (exists in pdfium/)
  - All third_party/ (exists in pdfium/)
  - v8/ (exists in pdfium/)
  - build/, buildtools/, tools/

- [ ] **3.2** Delete junk files at root:
  - test_parallel_api, test_parallel_api.c
  - benchmark_baseline.sh, create_*_pdf.py, debug_*.py
  - detect_scanned_pages.py, dump_stats.py, find_scanned_pdfs.py
  - json_to_text.py, test_batch_stats.py, test_bisection_commit.sh
  - test_sips_parallel.py, run_worker.sh
  - All .py scripts at root

- [ ] **3.3** Keep at root:
  - integration_tests/ (our tests)
  - rust/ (our Rust tools)
  - README.md, CLAUDE.md, USAGE.md (our docs)
  - .gitignore, .gitattributes
  - .gclient (manages pdfium/ checkout)

- [ ] **3.4** Root should have ONLY:
  ```
  pdfium/           # Complete upstream (build here)
  integration_tests/  # Our tests
  rust/             # Our Rust FFI
  README.md         # Our docs
  CLAUDE.md
  USAGE.md
  .gclient          # Fetch config
  .gitignore
  ```

### [ ] Build Verification Phase

- [ ] **4.1** Verify build from pdfium/:
  ```bash
  cd pdfium
  gn gen out/Profile --args='is_debug=false'
  ninja -C out/Profile pdfium_cli
  ```

- [ ] **4.2** Verify binary exists: `pdfium/out/Profile/pdfium_cli`
- [ ] **4.3** Verify binary works: `./pdfium/out/Profile/pdfium_cli --help`

### [ ] Test Verification Phase

- [ ] **5.1** Check test paths in conftest.py
- [ ] **5.2** Update if needed to reference pdfium/out/Profile/
- [ ] **5.3** Run smoke tests: `cd integration_tests && pytest -m smoke`
- [ ] **5.4** Must pass: 67/67 (100%)

### [ ] Documentation Phase

- [ ] **6.1** Update README.md build instructions:
  ```bash
  cd pdfium
  gn gen out/Profile
  ninja -C out/Profile pdfium_cli
  ```

- [ ] **6.2** Update CLAUDE.md to reflect structure
- [ ] **6.3** Delete outdated docs (already done mostly)
- [ ] **6.4** Create simple EMBEDDING.md

### [ ] Final Commit

- [ ] **7.1** Review all changes
- [ ] **7.2** Commit: "[MANAGER] Repository Structure Cleaned - Simple v1.4 Interface"
- [ ] **7.3** Verify git status clean
- [ ] **7.4** Run final smoke test validation

---

## PROFESSIONAL ASSESSMENT

**As a Google C++ engineer:**

**Current state:** UNACCEPTABLE
- 13GB duplicate PDFium checkout
- Incomplete source at root that can't build
- Confusing hybrid structure
- 20+ Python scripts scattered at root
- Documentation contradicts reality

**Target state:** SIMPLE
- One PDFium checkout (in pdfium/ subdir, managed by .gclient)
- Our custom code: integration_tests/, rust/, docs
- Clean root with only essentials
- Build instructions that actually work
- Tests that pass

**Can I do this?** YES - straightforward cleanup, mostly deletions

**Time estimate:** 2-3 hours for careful execution

---

## RECOMMENDATION

**Delete pdfium/ subdir approach is WRONG** - it's the complete source.

**Delete root PDFium source approach is CORRECT:**
- Keep pdfium/ (13GB, complete, builds)
- Delete core/, fpdfsdk/, etc. at root (incomplete, can't build)
- Keep only our custom code at root
- Simple, clean, works

Execute checklist systematically.
