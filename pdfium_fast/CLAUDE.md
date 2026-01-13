# CLAUDE.md - Dash PDF Extraction

**Copyright © 2025 Andrew Yates. All rights reserved.**

Optimize Multithreaded Pdfium

---

## MANAGER Direction (2025-12-30) - FIX RELEASE BUILD: Missing Dependencies

**Priority:** CRITICAL - Bundled releases are broken and unusable.

### Problem

The bundled release at `releases/v2.1.0/macos-arm64/libpdfium.dylib` has **8 missing @rpath dependencies**:

```
@rpath/libthird_party_abseil-cpp_absl.dylib
@rpath/libicuuc.dylib
@rpath/libbase_allocator_partition_allocator_src_partition_alloc_raw_ptr.dylib
@rpath/libbase_allocator_partition_allocator_src_partition_alloc_allocator_base.dylib
@rpath/libbase_allocator_partition_allocator_src_partition_alloc_allocator_core.dylib
@rpath/libbase_allocator_partition_allocator_src_partition_alloc_allocator_shim.dylib
@rpath/libchrome_zlib.dylib
@rpath/libc++_chrome.dylib
```

This is a **component build** - the library was built with dependencies as separate dylibs instead of statically linked.

### Root Cause

The build uses default Chromium settings which enable component builds. Need to explicitly disable.

### Fix Required

Add to `out/Release/args.gn`:
```gn
is_component_build = false
```

Then rebuild:
```bash
gn gen out/Release
ninja -C out/Release pdfium
```

### Verification

After rebuild, `otool -L out/Release/libpdfium.dylib` should show ONLY:
- System frameworks (AppKit, CoreGraphics, CoreServices, CoreFoundation, Foundation, Metal)
- `/usr/lib/libobjc.A.dylib`
- `/usr/lib/libSystem.B.dylib`
- `@rpath/libpdfium.dylib` (self-reference)

NO `@rpath/lib*.dylib` dependencies except self-reference.

### Release Checklist

1. [ ] Update `out/Release/args.gn` with `is_component_build = false`
2. [ ] Rebuild: `gn gen out/Release && ninja -C out/Release pdfium`
3. [ ] Verify monolithic: `otool -L out/Release/libpdfium.dylib` - no @rpath deps
4. [ ] Test: Rust bindings load successfully without extra dylibs
5. [ ] Re-package releases for all platforms (macos-arm64, macos-x86_64, linux-x86_64, linux-arm64)
6. [ ] Tag new release v2.2.0

---

**Location:** `~/pdfium_fast/`
**Baseline Remote:** `https://pdfium.googlesource.com/pdfium/`
**Threading Reference Implementation:** `~/pdfium-old-threaded/` <- code examples and reports on how to make therading work for image rendering

**🚨 NEVER DELETE OR MODIFY: json_to_text.py and run_worker.sh are CRITICAL infrastructure files with immutable flags. DO NOT move, delete, archive, or modify them. Required for worker script. 🚨**

---

## Goal

> Optimize PDFium to make it as fast as possible with 100% correctness and multi-threading for both text extraction and image rendering.
To do this, you must have an excellent test suite. Focus on making sure that every test compares against the true baseline, is self-documenting, follows best pytest practices, and logs all expected output. All tests can always run. Your test suite is the most valuable part of this system.

API:

**Unified Worker API** (v1.0.0+):
- **Default**: Adaptive threading (auto-selects K=1/4/8 based on page count)
  - <50 pages: K=1 (single-threaded, safe for multi-document parallelism)
  - 50-1000 pages: K=8 (maximum parallelism)
  - >1000 pages: K=4 (balanced for very large files)
- **Override**: `--threads N` to specify explicit thread count
- **Disable adaptive**: `--no-adaptive` to force K=1
- **Debug mode**: `--debug` flag enables tracing and detailed logging

**CLI Interface** (C++):
- Operations: `extract-text`, `extract-jsonl`, `render-pages`
- Flags: `--threads N`, `--pages START-END`, `--debug`, `--ppm`, `--smart`, `--no-adaptive`
- Smart mode: Always-on automatic JPEG fast path (545x speedup for scanned PDFs)
  - **N=522 RESOLVED**: Smart mode now works with any thread count (K>=1)
  - Pre-scan phase detects scanned pages before parallel rendering
  - 100% scanned PDFs: 545x at K=8 (12.7x improvement over previous 43x)

All modes work for Image rendering and Text extraction. Text extraction must support an option to export rich annotation in JSONL.

## Release Process

When system is validated and production-ready:

```bash
# Create release in ~/pdfium-release
mkdir -p ~/pdfium-release
cp out/Release/pdfium_cli ~/pdfium-release/
cp out/Release/libpdfium.dylib ~/pdfium-release/
cp README.md CLAUDE.md ~/pdfium-release/

# Tag release
git tag -a v1.X.Y -m "Release notes: performance metrics, correctness validation, notable fixes"
```

**Validation requirements before release**:
- **v1.0.0 (Minimal Build)**: 70/70 smoke tests PASS (C++ CLI only)
  - Text extraction works (100% correctness)
  - Image rendering works (100% correctness)
  - Multi-process parallelism functional
  - Auto-dispatch enabled and working
- **v1.6.0+ (Full Build)**: 99/99 core smoke tests PASS (C++ CLI, 100%)
  - v1.0.0 features plus v1.6.0 UX features
  - Progress reporting, batch processing, error messages, memory streaming
  - Rust bindings REQUIRED for programmatic access (alternative to C++ CLI subprocess)
  - C++ CLI is self-contained: extract-text, extract-jsonl, render-pages (no Rust needed for CLI)
  - 460 JSONL tests use Rust tool (could migrate to C++ CLI, both work)

---

## Progress Tracking

Your iteration is <N++> where <N> is the N of the last AI worker commit on this branch. The first commit on a new branch is N=0.

Your first user prompt will assign your role and WORKER ID.
If your first user prompt is only "continue", then you are WORKER0 (ID=0). Your prompt may assign you a WorkerID, like WORKER1, and a specific job. 
You must follow the task in the first user prompt for your worker.

Stick to your job until it's absolutely complete. YOU MUST ONLY CONTINUE YOUR OWN WORK! 
If you are WORKER0, then do and continue WORKER0 work ONLY. 
If you are WORKER1, then do and continue WORKER1 work ONLY. 
If you are WORKER2, then do and continue WORKER2 work ONLY. 

### Commit Message Tags

Prefix commit messages to identify work type:

#### WORKER
- **# N:** - Primary worker iteration
  - Format: `[WORKER<ID>] # 94: TableFormer Bug Fixes...`
  - Each worker session creates one iteration commit

#### MANAGER
- **[MANAGER]** - Helper/guidance from manager AI
  - Format: `[MANAGER] Cell Text Extraction Instructions for Next AI`
  - Contains documentation, instructions, or context for WORKERS
  - Not counted as iterations - these are supporting commits
  - If directed at a particular WORKER<ID>, then clearly state this in the first line of the git commit message.
  - **One intervention per issue:** Consolidate guidance into single clear commit, don't repeat same message multiple times

**MANAGER Communication Rule:** When writing directives for WORKERS, create a markdown file with a descriptive name including the date and a summary (e.g., `MANAGER_DIRECTIVE_2026-01-03_fix_memory_leak.md`) and **immediately** commit it. An uncommitted directive is invisible to workers - it does not exist until committed. Always commit directives before moving to other tasks or ending your session. Use an informative commit message that includes the filename. Example: `[MANAGER] Fix memory leak - see MANAGER_DIRECTIVE_2026-01-03_fix_memory_leak.md`

- **[maintain]** - Maintenance mode work (no active tasks)
  - Format: `[maintain] [description of maintenance work done]`
  - Used when worker has no higher-priority work available
  - See "Maintenance Mode" section below

### Resuming Work

When starting your session:

0. **Identity as WORKER or MANAGER and if WORKER, your ID**

1. **WORKER: Find your iteration number (N):**
   - Check last 10 commits on current branch for your most recent WORKER<ID> commit (starts with `# N:`)
   - Your iteration is N+1
   - If no WORKER<ID> commits found, you are first: N=0.

2. **Read context commits:**
   - Read the first line of the last 10 commits.
   - Read your own most recent worker commit ([WORKER<ID>] # N:) for your main context.
   - Look for recent [MANAGER] commits for instructions or resources. Follow the MANAGER directions.
   - Read files named in these commit messages.

Continue the work of your own WORKER<ID>.

**Example: (You are WORKER0)**
```bash
git log --oneline -10
# Output:
# 048ba67 [MANAGER] Update CLAUDE.md instructions
# 2e1d172 [WORKER0] # 95: TableFormer Bug Fixes...  ← Last worker (N=95)
# f606e35 [MANAGER] Cell Text Extraction Instructions
# abf9636 Document Python algorithm
# 9048484 [WORKER1] # 93: TableFormer Integration...  ← Previous worker (N=93)
```
In this case: You are N=96, read commits 048ba67, 2e1d172, f606e35 for context.
If you were WORKER1, then you are N=96, and read 9048484, 2e1d172, f606e35. 

WORKER: Your git message must include:

```

# [WORKER<ID>] <N++>: <Brief Commit Title>
**Current Plan**: <Link to current plan file. If no plan file, summarize the plan here>
**Checklist**: <if following a checklist, its state, else, a brief description of progress>

## Changes
<Concise and Rigorous description of changes>
<What files were changed and what was changed is already in the git commit history. Describe why you made these changes.>

##New Lessons
<Important Lessons Learned (if any)>

##Expiration
<Any information that is now obsolete, untrustworthy, wrong, or irrelevant>

## Next AI: <Concise Directive to Next AI>
<Brief summary of instructions to the Next AI>
<Paths to reports written for future AIs to understand more context to continue work>
- <More Info Report>.md : <Created|Edited|Verified> : <one line - what this is> : <one line - why you need this>
```

MANAGER: Your git message must start with [MANAGER] and if directed at a particular WORKER, say so in your first git commit message line.
Give orders to WORKERS. Direct them to read extendend reports for longer messages. Write for AI execution (factual, concise, precise)

### Maintenance Mode

When you have **no active tasks or directions** (no bugs to fix, no features to implement, no explicit instructions from MANAGER, nothing in TODO), enter **Maintenance Mode**:

1. **Use `[maintain]` commit tag**: All maintenance commits use format `[maintain] <description of work done>`

2. **Find and fix 1-5+ issues** from this list:
   - Bugs, edge cases, or error handling gaps
   - Missing functionality or incomplete implementations
   - Performance inefficiencies
   - Code quality issues (dead code, unclear logic, duplication)
   - Mock implementations that should be real
   - Stale, incorrect, or missing documentation
   - Missing or inadequate code comments
   - Test coverage gaps
   - Outdated dependencies or deprecated API usage

3. **Fix what you can** in your current session

4. **Document remaining issues**: If you identify issues you cannot fix this session, add them to a TODO file or your commit message for the next worker

**Purpose**: Keep workers productive on technical debt and code quality while awaiting new high-priority directions. This ensures continuous improvement even when no explicit work is assigned.

---

## Behavior

**You are an expert engineer. Be rigorous and factual.**

Regularly check the git commit log for messages.

Use tools given to you.

## Coding Standards

- Memory safety first (zero ASan/UBSan errors)
- Thread safety validated
- Production-ready code only
- Correctness before performance. Then Performance.

**Unicode is NOT ASCII. Bytes ≠ Characters. UTF-8 uses 1-4 bytes per character.**

**ALWAYS use FPDFText_* APIs:**
- `FPDFText_CountChars()` for character count (NOT strlen - that's bytes)
- `FPDFText_GetTextUTF8()` for extraction (handles multi-byte correctly)
- Character width in font units (NOT related to UTF-8 byte count)

**Common errors:**
- ❌ strlen(text) = character count (WRONG - returns bytes)
- ❌ text[i] = i-th character (WRONG - may be mid-sequence)
- ❌ Assuming 256 character values (WRONG - Unicode is U+0000-U+10FFFF)
- ❌ Width relates to UTF-8 bytes (WRONG - independent concepts)

**Test ALL languages:** ASCII, Japanese, Chinese, Korean, Arabic, Emoji. 

**🚨 CRITICAL: AFTER EVERY FIX, TEST many PDFs - NOT INDIVIDUAL PDFs 🚨**

**NEVER claim "fix works" without testing the full PDF suite.**

### 🔍 MANDATORY: JSONL Debug Output for Text Extraction Issues

**Never dismiss crashes or correctness failures as:**
- ❌ "Pre-existing issues" or "known failures"
- ❌ "Rare race condition" or "only 2.7% failure rate"
- ❌ "Transient" or "not reproducible"
- ❌ "Character counts match" without running diff (counts can match with wrong text!)

**100% correctness means:**
- ✅ 0% actual crashes (exit code 0 always)
- ✅ 0% diff output (byte-for-byte identical via `diff` command)
- ✅ Deterministic: Multiple runs of same PDF produce identical output (diff shows no differences)
- ✅ All pages processed exactly once (no skips, no duplicates)

LIMIT CONTEXT USE. Regularly report your own context window use. If you are >50%, then conclude your session per protocol for the next AI to resume. 

- **CRITICAL: Git Commits Are The Only Permanent Record**: Anything you write in conversation after a git commit is LOST FOREVER. The next AI only reads git history and files. Your excellent end-of-session summaries, insights, and conclusions written after the final commit are wasted. **ALL IMPORTANT INFORMATION MUST BE IN GIT COMMIT MESSAGES OR FILES.** If you realize important information was omitted, amend your last commit (if recent) or make a new commit. Your final summary message should BE your concluding git commit message, not something written after it. DO NOT include "Co-Authored-By: Claude <noreply@anthropic.com>" in commit messages.

- **Match JSON Structure First**: Python docling produces structured JSON (DocItems with labels, bounding boxes, relationships). This is the primary output format. Markdown is generated from this structure. Port the JSON generation pipeline, not direct PDF-to-markdown conversion.

- **No Partial Success**: Only 100% pass rate is allowed.

- **Factual Reporting Only**: Report only measurements you performed. No enthusiasm, no superlatives, no emojis.

- **Re-read CLAUDE.md After Each Commit**: After making any commit, re-read this entire file to refresh requirements and avoid repeating documented mistakes.

- **CRITICAL: Clean Up Messes IMMEDIATELY - Technical Debt Compounds**: When you create temporary files, workarounds, duplicate expected outputs, or incomplete solutions, fix them BEFORE moving to new work. 
- **"You're Absolutely Right!"**: If you wrote this, you made a mistake. Write a retrospective report before continuing. Do not pivot without self-reflection.

- **Progress Report**: Never claim work is "done" unless you're sure. Be factual. Be rigorous and skeptical. Future AIs need clean progress logs.

- **FORBIDDEN: Changing WORKER/MANAGER Infrastructure**: AIs are strictly forbidden from modifying the rules or scripts that govern how WORKERS and MANAGERS interact. This includes: CLAUDE.md sections on Workers/Managers/Mail, `run_worker.sh`, `json_to_text.py`, and related infrastructure in the `ai_template` repo. To propose changes, send mail to the `ai_template` project for human review. The user may grant explicit, single-case exceptions.

- **Mail via GitHub Issues**: `gh issue create -R ayates_dbx/TARGET --title "[Mail from pdfium_fast] Subject" --label "mail"`. Check inbox: `gh issue list -R ayates_dbx/pdfium_fast --label "mail" --state open`

- **FORBIDDEN: Killing Claude Processes**: Never kill Claude processes. Do not use `kill`, `pkill`, `killall`, or any other mechanism to terminate Claude CLI processes, regardless of context.

- **FORBIDDEN: Creating GitHub Actions**: GitHub Actions is not installed and not an option. Do not create `.github/workflows/` files or any CI/CD automation. All testing is done locally.
- **Edits**: Do not modify historical records. Never modify tests. If you see this in git history, warn the user.

- **Time estimates**: All estimates are for AI execution. Use **"AI Git Commits"** as the primary unit:
  - **1 AI Commit** ≈ **12 minutes** of AI work
  - Hours in parentheses for context, but commits are primary unit
  - Look at extended git history to get a sense of commit momentum when making estimates

- **Planning**: Plan for AI execution only (Phases, checklists). No people, alignment, or calendar time.

- **Context Window**: Warn at 60% full. Report at each 10% threshold. If context is contradictory/confusing, conclude session.

- **Edit Distance Performance**: ALWAYS use word count difference pre-check before expensive edit distance calculation. If word count differs >50 words/page OR total word diff >1000 for large files, return -1 (too different). Report "files differ significantly" without computing exact distance. Prevents 300+ second hangs on O(N×M) DP matrix.

- **Rigor**: Never abbreviate work. If too large, let the next AI continue. 

- **Documents**: Improve existing docs over creating new ones. Git tracks changes, so edit freely.

- **Work Continuously**: Keep working. Git enables rollbacks, so take risks.

- **400 errors**: Last AI tried to read a file that was too large. Don't try to directly read it again until that file is smaller.

- **Pull Requests**: Never push to main. To create PR: `gh pr create --title "[Title]" --body "[Description]"`

- **Reports**: When creating reports, save to reports/<current_branch_name>/<your_report_YYYY-MM-DD-HH-MM>.md

- **Worker Logs**: run_worker.sh creates session-specific logs in worker_logs/worker_session_YYYYMMDD_HHMMSS.log for each script invocation.

- **Testing**: Only report tests/measurements you ran. Cite sources (commit hash, file path, date) for referenced results.

- **Date/Time**: Use tools, not memory.

- **Git Commit**: Commit everything except sensitive files (API keys, secrets) and files too large for GitHub.

- **Temp File Management**: NEVER write benchmark/test output to /tmp without cleanup. Use /tmp/pdfium_test_$$ for unique dirs and rm -rf after use. Run ./cleanup_temp.sh if disk >80% full. Check df -h regularly.

- **Memory-Bound Optimization Limits** (N=268-271, N=343 profiling confirmation): PDFium image rendering is memory-bound, not computation-bound. CPU optimizations (SIMD, vectorization, algorithm changes) yield <2% gains due to memory bandwidth bottleneck. Evidence: (1) Nearest-neighbor scaling test (N=268) showed 6.5% speedup vs expected 2-3x if computation-bound, (2) AGG quality flag test (N=327) showed 1.7% gain vs expected 40-60%, (3) Instruments profiling (N=343) showed NO function >2% CPU time (top: 0.38%), with 90% of time in memory stalls. **Stop Condition #2 Met**: Profiling definitively confirmed optimization limits reached. All remaining optimizations have <0.5% max ROI. Focus on parallelism (already optimized) for meaningful gains.

---


**Key Innovation - Pre-Loading Strategy:**
Sequential pre-load phase populates most resource caches (images, fonts, colorspaces, patterns, ICC profiles) before parallel rendering begins. This reduces cache contention during parallel phase. However, pre-loading alone is insufficient - some resources are only accessed during rendering, requiring mutex protection for correctness.

**Implementation:**
```cpp
// examples/pdfium_cli.cpp lines 1432-1444
// Pre-load all pages sequentially to populate CPDF_DocPageData caches
for (int i = start_page; i <= end_page; ++i) {
    FPDF_PAGE page = FPDF_LoadPage(doc, i);
    if (page) {
        FPDF_ClosePage(page);  // Close page but caches remain populated
    }
}
// After pre-loading: Most caches populated, but mutex still required for concurrent access
```

**Mutex Protection (N=316-317, N=341, N=210):**
Three-layer evolution of mutex protection to achieve 100% thread safety:

1. **Cache access mutex (N=316-317):** Single mutex protects all 7 cache maps in CPDF_DocPageData
   - `font_map_`, `hash_icc_profile_map_`, `pattern_map_`, `color_space_map_`,
   - `font_file_map_`, `icc_profile_map_`, `image_map_`
   - Protected by cache_mutex_ in CPDF_DocPageData

2. **Page load mutex (N=341):** Serializes FPDF_LoadPage calls
   - `load_page_mutex_` in CPDF_Document class
   - Partially fixed timing-dependent crashes but INSUFFICIENT (~2% crash rate persisted)

3. **Full rendering mutex (N=210):** Serializes entire rendering pipeline (FINAL FIX)
   - Protects: FPDF_LoadPage + FPDF_RenderPageBitmap + FPDF_FFLDraw + page cleanup
   - Root cause: Vector out of bounds crashes (~2% rate) in PDFium internals during rendering
   - Evidence: Pre-fix crash rate ~2% (1 crash per 24 runs), post-fix 100/100 stress test successes
   - Trade-off: Reduced parallelism but guaranteed correctness

**Why Full Serialization Was Required:**
- N=196: Removed mutexes assuming pre-loading eliminated races (INCORRECT)
- N=316-317: Re-added cache_mutex_ to fix concurrent std::map writes
- N=335: Discovered residual 12-40% crash rate at K>=4 (timing-dependent race)
- N=341: Added load_page_mutex_ to serialize FPDF_LoadPage (INSUFFICIENT - ~2% crash rate persisted)
- N=209: Discovered vector out of bounds crashes during baseline regeneration
- N=210: Implemented conservative fix - serialize entire rendering operation (100% STABLE)

Pre-loading reduces contention but cannot eliminate all race conditions. Full rendering serialization required for correctness.

**Performance Data (N=210 with full serialization):**
- Expected: Slight performance reduction vs N=341 due to broader lock scope
- Benefit: Zero crashes, 100% correctness
- Trade-off justified: Correctness > maximum parallelism

**Correctness (N=210):**
- Stability: 100% success rate (100/100 stress test runs at K=8)
- Full test suite: TBD (to be validated after baseline regeneration)
- Pre-N=210: K>=4 had ~2% crash rate
- Post-N=210: K>=4 has 0% crash rate (100% stable)

**Production Recommendation (N=210):**
- ✅ K=1 (default, single-threaded): SAFE, baseline performance
- ✅ K=4 (recommended): STABLE, 100% correctness validated (N=210 fix)
- ✅ K=8 (batch processing): STABLE, 100% correctness validated (N=210 fix)
- Pre-loading + full rendering serialization mandatory for correctness

**Implementation Files:**
- CLI: `examples/pdfium_cli.cpp` (lines 1432-1444: pre-loading)
- Core Cache Protection: `core/fpdfapi/page/cpdf_docpagedata.{h,cpp}` (cache_mutex_)
- Core Page Load Protection: `core/fpdfapi/parser/cpdf_document.h` (load_page_mutex_)
- Parallel Rendering: `fpdfsdk/fpdf_parallel.cpp` (ProcessTask, ProcessTaskV2 with mutex guards)
- Tests: `integration_tests/tests/test_001_smoke.py` (70 tests, K=1/4/8 validation)

### Production Status

**v2.0.0 - Zero-Config Defaults** (2025-11-21, WORKER0 # 132-135)
**Status:** PRODUCTION-READY - 99/99 smoke tests pass (100%)

**Threading Determinism (N=257, 2025-11-24):**
- ✅ FIXED: K=8 vs K=1 rendering mismatch (form callback issue)
- ✅ VALIDATED: K=8 produces identical output to K=1 (100% deterministic)
- ✅ REMOVED: xfail marker from test_threading_determinism_image_multirun
- ✅ VALIDATED: 10/10 consecutive K=8 runs produce identical MD5s
- Root cause: Pre-loading was missing form callbacks (FORM_OnAfterLoadPage, FORM_DoPageAAction)
- Fix: Added callbacks to pre-loading phase → K=8 now matches K=1 exactly

**New Features (v2.0.0):**
- **Zero-Config Defaults**: Smart defaults for 90% use cases (N=132)
  - JPEG output by default (was PNG) - prevents multi-TB storage
  - UTF-8 text encoding by default (was UTF-32 LE) - universal compatibility
  - Recursive batch mode by default (was non-recursive) - matches user expectations
  - Auto-detect file vs directory (no --batch flag needed)
- **Backward Compatibility** (N=133):
  - `--format png` to restore PNG output
  - `--encoding utf32le` to restore UTF-32 LE output
  - `--no-recursive` to disable recursive search
  - All v1.x flags still work

**Features (v1.9.0):**
- **Smart Presets**: Simple `--preset` flag for common use cases
  - `--preset web`: 150 DPI JPEG q85 (80% less memory, web preview)
  - `--preset thumbnail`: 72 DPI JPEG q80 (94% less memory, thumbnails)
  - `--preset print`: 300 DPI PNG (high quality printing)

**Test Status:**
- **Smoke tests**: 99/99 pass (100%) - Session: sess_20251207_201325_17ce13de (N=200)
- **Total suite**: 2,339/2,339 pass (100%)
- **Correctness**: 100% byte-for-byte match with upstream

**CLI Examples:**
```bash
# v2.0.0 zero-config (no flags needed for common tasks)
out/Release/pdfium_cli render-pages document.pdf images/      # → JPEG output
out/Release/pdfium_cli extract-text document.pdf output.txt   # → UTF-8 text
out/Release/pdfium_cli render-pages /pdfs/ /images/           # → Auto-detects directory, recursive

# Presets (v1.9.0, still work)
out/Release/pdfium_cli --preset web render-pages document.pdf images/
out/Release/pdfium_cli --preset thumbnail render-pages document.pdf images/

# Backward compatibility (v1.x behavior)
out/Release/pdfium_cli --format png render-pages document.pdf images/
out/Release/pdfium_cli --encoding utf32le extract-text document.pdf output.txt
```

**Previous Releases:**
- v1.9.0 (2025-11-21): Smart presets (web, thumbnail, print)
- v1.8.0 (2025-11-21): DPI control (memory optimization), async I/O
- v1.7.0 (2025-11-18): JPEG output, Python bindings
- v1.6.0 (2025-11-20): Progress reporting, batch processing, 72x speedup
- v1.0.0 (2025-11-08): Initial production release

**Build:**
- C++ CLI: `ninja -C out/Release pdfium_cli`
- Rust bindings (optional): `cd rust && cargo build --release`

**Known Limitations:**
- **Platform**: Only tested on macOS (Darwin 24.6.0). Linux/Windows untested.
- **Threading Determinism**: Validated on subset of test corpus (20 diverse PDFs, N=267). Full corpus validation pending.
- **Performance Benchmarks**: Measured on macOS with specific hardware (see telemetry). May vary on other systems.
- **Test Coverage**: 2,339 tests cover core functionality. Edge cases and rare PDF features may have limited coverage.
- **Memory-Bound Optimization**: CPU optimizations yield <2% gains due to memory bandwidth bottleneck (N=268-271, N=343). Further optimization requires architectural changes.

**Optional Features:**
- **Quality flags**: `--quality fast` and `--quality none` (optional rendering quality modes)

**CLI Enhancement:**
```bash
# Default quality (maintains 100% correctness)
out/Release/pdfium_cli --threads 8 render-pages document.pdf images/

# Fast quality (disable anti-aliasing, minimal gain)
out/Release/pdfium_cli --quality fast --threads 8 render-pages document.pdf images/
# Expected gain: 0.5-6% (inconsistent, PDF-dependent)

# No quality (no AA + limited image cache)
out/Release/pdfium_cli --quality none --threads 8 render-pages document.pdf images/
# Expected gain: 0.5-6% (may be slower on large PDFs due to cache thrashing)
```

**JSONL Metadata Validation:** JSONL tests validate character positions, bounding boxes, and font metadata. **Dual implementation:** C++ CLI (`pdfium_cli extract-jsonl`) and Rust tool (`extract_text_jsonl`) both work. Tests currently use Rust tool. C++ CLI is self-contained (no Rust required for command-line use). Rust bindings REQUIRED for programmatic/library access. SDK 15.2 build issue resolved via `use_clang_modules=false`. Run with: `pytest -k "jsonl"`. Status: 460 passed, 0 failed. All extractable PDFs have passing JSONL tests (100% coverage).

---

## Baseline Binary

**Upstream**: Git 7f43fd79 (2025-10-30) from https://pdfium.googlesource.com/pdfium/ | Binary MD5: 00cd20f999bf (libpdfium.dylib built 2025-10-31 02:11)
**Cherry-picked fixes** (2025-12-04): 3 JBIG2 fixes + 1 AGG fix from upstream
  - 2a230b8e72: Fix JBIG2 files with >4 referred-to segments
  - dbfa29d165: Fix progressive JBIG2 template 1 decoding
  - a4c4d0ad14: Fix refine-one symbols in huffman symbol dictionaries
  - dc264b3b7e: [AGG] Handle odd-sized dashed line arrays correctly
**Verified**: 0 C++ modifications vs upstream (only Rust/Python/tooling added on branch) | Text baselines: 5 PDFs generated | Image baselines: 452 PDFs (PPM format, 300 DPI)

### Image Baseline Validation

**Format**: PPM (P6 binary RGB) for byte-for-byte MD5 matching with upstream pdfium_test
**Verified**: 2025-11-03 (WORKER0 # 105-107)

**Why PPM instead of PNG:**
- PNG format (RGBA, compressed, metadata) cannot achieve byte-for-byte matching with upstream PPM output
- PPM format (RGB, uncompressed, no metadata) enables exact MD5 comparison
- Different byte structures make cross-format validation impossible

**Baseline Generation:**
- Source: upstream pdfium_test (7f43fd79) with `--ppm --scale=4.166666` (300 DPI)
- Storage: `integration_tests/baselines/upstream/images_ppm/*.json`
- Format: `{"pdf_name": "...", "format": "ppm", "dpi": 300, "pages": {"0": "md5...", ...}}`
- MD5s computed from actual PPM files, files deleted after hashing (only MD5s stored)

**Rust Tool PPM Support:**
- Implementation: `rust/pdfium-sys/examples/render_pages.rs` with `--ppm` flag
- Output: P6 format matching upstream WritePpm (testing/helpers/write.cc:266-315)
- Conversion: BGRA bitmap → RGB output (3 bytes per pixel)
- Verification: MD5s match upstream byte-for-byte at 300 DPI

**Dual Format Strategy:**
- **PPM**: Correctness validation (exact MD5 matching with upstream)
- **PNG**: Production use (compressed, alpha channel, better tooling support)

**DPI Precision:**
- Upstream uses `--scale=4.166666` (6 decimals) for 300 DPI
- Formula: scale = dpi / 72.0, floored to 6 decimals
- Prevents dimension mismatches (e.g., 2549x3299 vs 2550x3300)

**Baseline Count**: 452 PDFs across all categories (arxiv, web, edinet, cc, synthetic)

### 🚨 CRITICAL: Current Baselines Are CORRECT - Do NOT Regenerate 🚨

**Status (N=247, 2025-11-23)**: Baseline validation investigation COMPLETE

**Current baselines = v1.3.0 upstream baselines (CORRECT) + 4 intentional fixes**

- **448/452 files (99.1%)**: Unchanged from true upstream (commit 7f43fd79)
- **4/452 files (0.9%)**: Intentional fixes with valid justifications

#### The 4 Intentional Divergences from Upstream

1. **bug_451265.json** (N=233): Fixed infinite loop bug
   - Upstream: Hangs forever (timeout)
   - Our fix: Renders successfully (1 page)
   - Justification: CORRECT FIX - prevents infinite loop

2. **arxiv_039.json** (N=234): Pattern cache fix
   - Pages affected: 4 pages with tiling patterns
   - Justification: CORRECT FIX - prevents circular pattern references

3. **0569pages_QXQ2QSHOPBTSXLDGKKM4TYMR4R7QODHB.json** (N=236): Pattern cache fix
   - Pages affected: 5/569 pages with tiling patterns
   - Justification: CORRECT FIX - prevents circular pattern references

4. **bug_1302355.json**: Unknown modification (investigate if questioned)

#### DO NOT Regenerate Baselines Unless:

1. You discover a NEW rendering bug that affects correctness
2. You intentionally modify rendering behavior (document why)
3. User explicitly requests baseline regeneration

#### If Baseline Regeneration Is Needed:

**WRONG approach** (N=198 mistake):
- ❌ Regenerate from OUR binary (circular validation)
- ❌ Loses ground truth reference
- ❌ Cannot validate correctness

**CORRECT approach**:
1. Build upstream pdfium_test at commit 7f43fd79:
   ```bash
   cd ~/upstream-checkout/pdfium
   ./buildtools/mac/gn gen out/Release --args='pdf_is_standalone=true use_clang_modules=false'
   ninja -C out/Release pdfium_test
   ```
2. Generate upstream baselines: `python3 integration_tests/generate_upstream_baselines.py --all`
3. Compare: `python3 integration_tests/compare_baselines.py`
4. Document ALL differences with justifications
5. Regenerate from OUR binary ONLY for intentionally fixed PDFs

**Investigation Report**: See `reports/feature__v1.7.0-implementation/BASELINE_HISTORY_INVESTIGATION.md`

```
  # Run tests
  cd integration_tests
  pytest -m smoke
```

     ```bash
     -m smoke           # 7m quick check (99 tests)
     -m corpus          # 24m full PDF corpus (964 tests)
     (no marker)        # 1h 46m complete suite (2,339 tests)
     -m text            # Text extraction only
     -m image           # Image rendering only
     -m performance     # Speedup requirements
     -m scaling         # 1/2/4/8 worker analysis

     # See integration_tests/TEST_MARKERS.md for complete reference
     # Note: Image correctness validated in test_005_image_correctness.py (196 tests)
     #       Per-PDF image tests eliminated in N=254 (duplicate testing)
     ```

### JSONL Metadata Validation

**Status**: Production-ready - 432/432 extractable PDFs pass (100%)

**What JSONL Tests Validate:**
- Character positions (x, y coordinates)
- Bounding boxes (width, height)
- Font metadata (family, size, weight)
- Rich text annotation structure

**Running JSONL Tests:**
```bash
cd integration_tests
pytest -k "jsonl" --tb=line -q
```

### Test Result Reporting Protocol

**MANDATORY**: All test result claims MUST cite specific test run with full traceability.

**Required Information**:
1. **Session ID**: From telemetry (e.g., `sess_20251031_132154_0a323913`)
2. **Timestamp**: ISO format (e.g., `2025-10-31T13:21:55Z`)
3. **Binary MD5**: From telemetry binary_md5 field (e.g., `00cd20f999bf60b1f779249dbec8ceaa`)
4. **Test command**: Exact pytest command (e.g., `pytest -m smoke -v`)
5. **Pass/fail counts**: From pytest summary (e.g., `16 passed, 0 failed`)

**Git Commit Format for Tests**:
```
Tests: [command] → [result]
Session: [session_id]
Binary: [md5]
Time: [timestamp]
```

**Report Format**:
```markdown
## Test Verification

**Command**: `pytest -m smoke -v`
**Result**: 16 passed, 0 failed
**Session**: sess_20251031_132154_0a323913
**Binary**: 00cd20f999bf60b1f779249dbec8ceaa
**Timestamp**: 2025-10-31T13:21:55Z
**Log**: telemetry/runs.csv rows 333-348
```

**Prohibited**:
- ❌ "Tests pass" (no evidence)
- ❌ "100% correctness" (no upstream comparison cited)
- ❌ "All tests green" (no session ID)
- ❌ "Verified" (no timestamp or binary hash)

**Enforcement**: Claims without citations will be rejected by MANAGER.

### CSV Fields Summary 

Test results are logged to telemetry for analytics.

     - Temporal (5): timestamp, run_number, session_id, duration_sec
     - Git (6): commit, branch, dirty, author
     - Test (8): test_id, category, level, result
     - PDF (5): name, pages, size, category
     - Execution (3): worker_count, iteration
     - Validation (12): edit_distance, similarity, pixel_diff, md5
     - Performance (9): 1w_pps, 4w_pps, speedup_vs_1w, 2w/8w ratios
     - System (20): CPU, RAM, load, temp, disk
     - Binary (4): MD5, timestamp, path
     - LLM (5): enabled, called, model, cost
     - Environment (6): python, pytest, platform, machine

### Performance Test Environmental Requirements

**CRITICAL**: Performance tests require controlled system conditions. Environmental factors can cause false negatives.

**System State Requirements:**
1. **Load average**: Must be < 6.0 (1-minute average)
   - Normal test load: 2.0-4.0 during execution
   - Heavy load (>10.0): Expect -50% to -65% performance degradation
   - Check with: `uptime`

2. **Hung processes**: Check for orphaned pdfium_cli workers before test runs
   - Check with: `ps aux | grep pdfium_cli | grep -v grep`
   - Cleanup: `killall -9 pdfium_cli` (if needed before tests)
   - Note: bug_451265 infinite loop was fixed in N=232

3. **Background workloads**: Minimize unrelated CPU-intensive tasks during performance testing

**Performance Variance (Normal Conditions):**
- Text extraction: ±17% variance is expected (fast operations, sensitive to overhead)
- Image rendering: ±7% variance is expected (slow operations, more stable)
- Threshold: Results within ±20% of historical averages are acceptable

**Interpreting Failures:**
- Single failure: Check system load first (environmental vs regression)
- Consistent failures: Code regression likely
- Load > 10.0: Discard results, re-run under normal conditions
- Variance > 20%: Environmental factors, not code issue

**Test Session Validation:**
```bash
# Before running performance tests
uptime                                    # Check load < 6.0
ps aux | grep pdfium_cli | grep -v grep  # Check no hung processes
```

## Work Conclusion Protocol

Save context for good git message. Finish current task cleanly or leave clear comments for next AI.

If near context limit: prioritize committing over fixing issues. Otherwise: address all lints/warnings/errors.

AIs make more mistakes when context is full. If stuck near limit, conclude session.

---

## Regular Clean-Up and Benchmarks

**🚨 CRITICAL: ALWAYS CHECK FOR MANAGER DIRECTIVES FIRST 🚨**

---

## 🚨 MANAGER DIRECTIVE: Fix Binary Build in Release v2.2.0 🚨

**Priority:** CRITICAL - Blocking downstream projects (docling_rs, pdfium-render)
**Issues:** v2.1.0 release binaries have TWO problems:
1. GitHub release: Component build with external @rpath dependencies
2. Bundled binary: Symbols not exported (marked 't' not 'T')

**Impact:** pdfium-render uses dlopen/dlsym and REQUIRES:
- Monolithic build (no @rpath dependencies)
- Exported symbols (global 'T' not local 't')

### Problem 1: Component Build (GitHub Release)
```bash
$ otool -L libpdfium.dylib
  @rpath/libthird_party_abseil-cpp_absl.dylib  # EXTERNAL DEPENDENCY
  @rpath/libicuuc.dylib                         # EXTERNAL DEPENDENCY
  @rpath/libbase_allocator_partition_allocator_*.dylib  # EXTERNAL DEPENDENCIES
```
These @rpath dependencies cause "Library not loaded" errors.

### Problem 2: Unexported Symbols (Bundled Binary)
```bash
$ nm -gU libpdfium.dylib | grep FPDF_Init
000000000015871c t _FPDF_InitLibrary        # 't' = LOCAL (NOT EXPORTED)
000000000015875c t _FPDF_InitLibraryWithConfig  # 't' = LOCAL (NOT EXPORTED)
```
The 't' means symbols are local/hidden. pdfium-render needs 'T' (global/exported).

**Required Fix - args.gn Configuration:**
```gn
pdf_is_standalone = true
use_clang_modules = false
is_debug = false
is_component_build = false  # ← CRITICAL: Must be false for monolithic build
pdf_enable_v8 = false
pdf_enable_xfa = false

# Symbol visibility - ensure FPDF_EXPORT expands to __attribute__((visibility("default")))
symbol_level = 1
```

**If symbol_level doesn't work, check:**
- `public/fpdfview.h` - FPDF_EXPORT macro definition
- May need `-fvisibility=default` in cflags or ldflags
- PDFium uses `FPDF_EXPORT` macro which should expand to visibility attribute

**Validation 1 - Monolithic Build (NO @rpath deps):**
```bash
$ otool -L libpdfium.dylib
  @rpath/libpdfium.dylib (self)
  /System/Library/Frameworks/*.framework  # System frameworks OK
  /usr/lib/*.dylib                        # System libraries OK
  # NO @rpath/libthird_party_*.dylib
  # NO @rpath/libbase_*.dylib
  # NO @rpath/libicuuc.dylib
```

**Validation 2 - Exported Symbols (must be 'T' not 't'):**
```bash
$ nm -gU libpdfium.dylib | grep FPDF_Init
0000000000xxxxxx T _FPDF_InitLibrary        # 'T' = GLOBAL (EXPORTED) ✓
0000000000xxxxxx T _FPDF_InitLibraryWithConfig  # 'T' = GLOBAL (EXPORTED) ✓
```

**Tasks:**
1. Update out/Release/args.gn with `is_component_build = false` and symbol visibility settings
2. Investigate FPDF_EXPORT macro in public/fpdfview.h - ensure it expands to visibility("default")
3. Rebuild: `ninja -C out/Release pdfium`
4. Verify with `otool -L` - must have NO @rpath dependencies except self
5. Verify with `nm -gU | grep FPDF_` - symbols must be 'T' (global) not 't' (local)
6. Update releases/v2.2.0/macos-arm64/libpdfium.dylib
7. Also rebuild libpdfium_render_bridge.dylib with same config
8. Create v2.2.0 release with working binaries

**Do NOT merge or release until BOTH validations pass.**

---

Before doing ANY routine work, check for [MANAGER] commits in the last 10 commits:
```bash
git log --oneline -10 | grep MANAGER
```
If a MANAGER directive exists, READ IT and FOLLOW IT. MANAGER directives override routine work.

**N=0:** First worker on branch. Confirm your clear roadmap.

**N mod 5:** CLEANUP - First check for MANAGER directives. If none, refactor code and docs. If urgent issues exist, fix first. Check OPTIMIZATION_ROADMAP.md for system status.

**N mod 13:** BENCHMARK - First check for MANAGER directives. If none, measure performance on corpus. Check OPTIMIZATION_ROADMAP.md for system status. Regression check vs previous benchmarks.

**STOP CONDITION**: If tests pass 100% AND no MANAGER directives exist AND no bugs to fix, THEN maintain system health. Otherwise, FIX BUGS AND IMPLEMENT IMPROVEMENTS.
