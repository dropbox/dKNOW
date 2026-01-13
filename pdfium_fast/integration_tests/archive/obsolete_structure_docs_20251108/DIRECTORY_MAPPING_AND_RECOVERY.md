# Complete Directory Mapping & Recovery Plan

**Date:** 2025-11-08 18:20 PST
**Goal:** Understand repository structure and get back to green (v1.3 simplicity)
**Approach:** Professional, skeptical, rigorous

---

## CURRENT DIRECTORY STRUCTURE (Verified)

```
/Users/ayates/pdfium/                    # ROOT - GitHub pdfium_fast repo
├── .git/                                # Git for THIS repo
├── .gclient                             # Added N=34 - manages pdfium/ subdir
│
├── pdfium/                              # Git submodule (160000 commit d7fbd2ca8)
│   ├── .git/                            # Separate git (upstream)
│   ├── core/, fpdfsdk/, public/         # Complete PDFium source
│   ├── skia/, third_party/, v8/         # Complete dependencies
│   ├── examples/                        # CUSTOM CODE (copied by worker N=47)
│   │   ├── pdfium_cli.cpp               # Your CLI tool
│   │   ├── pdfium_render_bridge.cpp     # Your bridge
│   │   └── BUILD.gn                     # Custom build rules
│   ├── BUILD.gn                         # Main build (modified by worker)
│   └── out/Profile/                     # BUILD OUTPUT (works)
│       └── pdfium_cli                   # WORKING BINARY (2.3MB, Nov 8 10:30)
│
├── core/, fpdfsdk/, public/             # DUPLICATE PDFium source at root
├── fxbarcode/, fxjs/, constants/        # DUPLICATE (incomplete, missing skia/)
├── third_party/                         # DUPLICATE (fetched by gclient)
├── v8/, build/, buildtools/, tools/     # DUPLICATE (fetched by gclient)
│
├── integration_tests/                   # YOUR TEST SUITE ✓
│   ├── tests/                           # 2881 tests
│   ├── pdfs/                            # Test PDFs
│   ├── conftest.py                      # References pdfium/out/Profile/
│   └── telemetry/                       # Test results
│
├── rust/                                # YOUR RUST TOOLS ✓
│   ├── pdfium-sys/                      # FFI bindings
│   ├── pdfium-render-bridge/            # C++ bridge
│   └── Cargo.toml                       # Workspace
│
├── out/                                 # OLD BUILD OUTPUT (stale)
│   ├── Profile/                         # Empty or stale
│   └── Optimized-Shared/                # Has old binaries (Nov 7)
│
├── README.md, CLAUDE.md, USAGE.md       # YOUR DOCUMENTATION ✓
├── ORDERS_N51.md                        # MANAGER directive (can delete)
├── MANAGER_CLEANUP_CHECKLIST.md         # MANAGER doc (can delete)
│
└── JUNK (Python scripts at root):       # DELETE THESE
    ├── benchmark_baseline.sh
    ├── create_real_scanned_pdf.py
    ├── debug_pdf_structure.py
    ├── detect_scanned_pages.py
    ├── dump_stats.py
    ├── find_scanned_pdfs.py
    ├── json_to_text.py
    ├── run_worker.sh
    ├── test_batch_stats.py
    ├── test_bisection_commit.sh
    ├── test_sips_parallel.py
    ├── test_parallel_api, test_parallel_api.c
    └── install_hooks.sh
```

---

## COMPARISON: Upstream vs Our Repo

### Upstream (https://pdfium.googlesource.com/pdfium/)

```
pdfium/                      # Root
├── core/, fpdfsdk/, public/ # PDFium source
├── samples/                 # THEIR example tools
├── testing/                 # THEIR tests
├── BUILD.gn                 # Build system
└── DEPS                     # Dependency management
```

**Build:** `gn gen out/Release && ninja -C out/Release`

### Our GitHub Repo (pdfium_fast - should be)

```
pdfium_fast/                        # Root
├── core/, fpdfsdk/, public/        # Fork of PDFium source
├── examples/                       # OUR custom tools (replaces samples/)
│   ├── pdfium_cli.cpp              # Our CLI
│   └── BUILD.gn
├── integration_tests/              # OUR comprehensive test suite
├── rust/                           # OUR Rust bindings
├── BUILD.gn                        # Modified to include examples/
├── DEPS                            # Dependency management
└── README.md, CLAUDE.md            # OUR documentation
```

**Build:** `gn gen out/Profile && ninja -C out/Profile pdfium_cli`

---

## WHERE FILES COME FROM

### From Upstream (https://pdfium.googlesource.com/pdfium/)
- core/, fpdfsdk/, public/, fxbarcode/, fxjs/
- constants/, testing/ (upstream tests)
- third_party/, v8/, build/, buildtools/, tools/
- BUILD.gn (base), DEPS, .gn, PRESUBMIT.py

### Your Custom Work
- **examples/pdfium_cli.cpp** - Your production CLI tool
- **examples/pdfium_render_bridge.cpp** - Your C++/Rust bridge
- **integration_tests/** - Your 2881-test suite
- **rust/** - Your Rust FFI bindings
- **CLAUDE.md, README.md, USAGE.md** - Your documentation
- **BUILD.gn modifications** - To build your examples/

### Worker Added (Confusion)
- **pdfium/ subdirectory** - DUPLICATE complete checkout
- **.gclient** - To manage pdfium/ subdirectory
- **Duplicate examples/ in pdfium/examples/** - Worker copied
- **samples/** directory - Worker mistake
- **MANAGER_*.md docs** - My directives (noise)

---

## THE PROBLEM (Root Cause)

**v1.3.0 snapshot (b853bced1):**
- Had pre-built binary: `out/Optimized-Shared/pdfium_cli`
- Incomplete dependency checkout (missing skia/ at root)
- README claimed `gn gen out/Release` works (FALSE - missing deps)
- **Could NOT do fresh build** - only had artifacts

**Worker response (N=34-50):**
- Couldn't build from root (missing skia/, googletest/)
- Added .gclient and pdfium/ subdirectory to fetch complete source
- Built from pdfium/ successfully
- Created confusion with duplicates

**Truth:** v1.3.0 was NOT buildable from clean checkout, only had pre-built binaries

---

## RECOVERY PLAN - Back to Green

### Goal: Single clean PDFium fork with our customizations

**Current Issue:**
- Root has incomplete PDFium source
- pdfium/ has complete but duplicate source (13GB)
- 2 git repos, confusing structure

**Solution: Fix Root to be Complete**

### STEP-BY-STEP RECOVERY

#### Phase 1: Fetch Missing Dependencies (30 min)

```bash
cd /Users/ayates/pdfium

# gclient sync will fetch missing deps to root
export PATH="$HOME/depot_tools:$PATH"
gclient sync --no-history

# This should fetch:
# - skia/ at root level
# - All third_party/ deps
# - build/ tooling
```

#### Phase 2: Test Build from Root (10 min)

```bash
cd /Users/ayates/pdfium  # At ROOT

# Generate build
gn gen out/Profile --args='is_debug=false is_component_build=false'

# Build our CLI
ninja -C out/Profile pdfium_cli

# Verify binary
ls -lh out/Profile/pdfium_cli
```

#### Phase 3: If Root Build Works - Delete Duplicates (15 min)

```bash
# Delete duplicate pdfium/ subdirectory (13GB)
rm -rf pdfium/

# Delete .gclient (no longer needed)
rm .gclient .gclient_entries .gclient_previous_sync_commits

# Delete out/ subdirectory if stale
# Keep if has working binaries

# Update .gitignore to ignore gclient-fetched dirs
```

#### Phase 4: Test Suite Validation (10 min)

```bash
cd integration_tests

# Update conftest.py if needed (binary path)
# Change: pdfium/out/Profile/pdfium_cli
# To: out/Profile/pdfium_cli

# Run tests
pytest -m smoke --tb=line -q

# Should: 67/67 or 65/67 pass
```

#### Phase 5: Clean Up Junk (10 min)

```bash
# Delete Python scripts at root
rm benchmark_baseline.sh create_*.py debug_*.py detect_*.py
rm dump_stats.py find_*.py json_to_text.py
rm run_worker.sh test_batch_stats.py test_bisection_commit.sh test_sips_parallel.py
rm test_parallel_api test_parallel_api.c install_hooks.sh

# Delete MANAGER docs
rm ORDERS_N51.md MANAGER_CLEANUP_CHECKLIST.md

# Delete outdated release notes
rm RELEASE_NOTES_v1.0.0.md RELEASE_NOTES_v1.2.0.md
# Keep only v1.3.0 or latest
```

#### Phase 6: Documentation Update (20 min)

**README.md:**
```bash
# Build from source
gn gen out/Profile
ninja -C out/Profile pdfium_cli

# Run tests
cd integration_tests
pytest -m smoke  # 67 tests, 2 min
```

**CLAUDE.md:**
- Update build instructions
- Clarify single-repo structure
- Remove references to pdfium/ subdirectory

#### Phase 7: Final Validation (5 min)

```bash
# Clean state check
git status

# Build from scratch
rm -rf out/Profile
gn gen out/Profile
ninja -C out/Profile pdfium_cli

# Test
cd integration_tests && pytest -m smoke
```

---

## VERIFICATION CHECKLIST

### Before Starting:
- [ ] Backup current working binary: `cp pdfium/out/Profile/pdfium_cli ~/pdfium_cli_backup`
- [ ] Note current test pass rate from worker: 65/67 (97%)
- [ ] Check system load acceptable (< 10)

### Phase 1 - Dependencies:
- [ ] `gclient sync` completes without errors
- [ ] skia/ directory appears at root
- [ ] third_party/ is complete
- [ ] No missing import errors

### Phase 2 - Build:
- [ ] `gn gen out/Profile` succeeds
- [ ] `ninja -C out/Profile pdfium_cli` succeeds
- [ ] Binary exists and is ~2-3MB
- [ ] `./out/Profile/pdfium_cli --help` works

### Phase 3 - Tests:
- [ ] Update test paths if needed
- [ ] `pytest -m smoke` runs
- [ ] Results: ≥65/67 pass (maintain current level)
- [ ] No new failures vs before cleanup

### Phase 4 - Structure:
- [ ] Only one pdfium_cli.cpp location (examples/)
- [ ] No pdfium/ subdirectory duplicate
- [ ] No junk scripts at root
- [ ] Clean git status

---

## SAFETY MEASURES

**Before deleting pdfium/:**
1. Verify root build works completely
2. Verify tests pass with root binary
3. Copy any modified files from pdfium/examples/ to examples/

**Rollback plan:**
```bash
# If anything breaks:
git checkout pdfium/  # Restore subdirectory
cd pdfium && ninja -C out/Profile pdfium_cli  # Rebuild there
# Tests will still work
```

---

## CURRENT TEST STATUS

**Running now:** Smoke tests at ~70% (45/67), 1 failure, very slow due to load: 26

**Expected:** Will complete in 15-20 minutes (normally 2 min) due to high system load

**Recommendation:** Wait for smoke test completion, then decide on cleanup based on pass/fail

---

## DECISION POINT

**Option A: Execute recovery plan above** (2 hours)
- Fetch deps to root with gclient sync
- Delete pdfium/ duplicate
- Clean structure

**Option B: Keep current working state** (5 min)
- Document that build is in pdfium/
- Update README.md to reflect reality
- Accept hybrid structure

**Option C: Wait for system load to drop** (unknown time)
- Current load: 26 (extreme)
- Tests cannot run properly
- Come back when load < 10

---

**What do you want me to do?**
