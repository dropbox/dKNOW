# 🚨 STOP - READ THESE FILES FIRST 🚨

**MANDATORY STARTUP SEQUENCE:**
1. **MANDATORY_20_FORMATS_TO_95_PERCENT.txt** - Your current target
2. **NEVER_FINISHED_ROADMAP.md** - Systematic improvement roadmap
3. **USER_DIRECTIVE_QUALITY_95_PERCENT.txt** - User philosophy

**DO NOT READ THIS FILE until you've read all three above.**

═══════════════════════════════════════════════════════════════

# CONTINUOUS WORK QUEUE - SUPERSEDED

**Status:** 🔴 SUPERSEDED by NEVER_FINISHED_ROADMAP.md

**User's philosophy change:**
- OLD thinking: "Work queue with tasks to complete"
- NEW thinking: "NEVER FINISHED - continuous improvement forever"

**This file is archived. Use NEVER_FINISHED_ROADMAP.md instead.**

**User clarification (2025-11-22):**
- "must be 100%!" - Not 80%, not 90%, ALL formats
- "NEVER FINISHED!" - This work has no end
- "fix everything" - Quality, bugs, performance, features - all dimensions

═══════════════════════════════════════════════════════════════

## For Historical Reference Only

[Everything below this line is archived - use NEVER_FINISHED_ROADMAP.md]

═══════════════════════════════════════════════════════════════

---

## 📊 Current System Status (N=1770)

**Test Health:**
- ✅ 3,058 library tests passing (100% pass rate, 17 ignored)
- ✅ 678+ consecutive sessions at 100% test pass rate (N=1092-1770)
- ✅ Clippy clean (zero warnings)
- ✅ Cargo fmt clean

**Quality Metrics:**
- ✅ LLM Verification: 89% pass rate (8/9 formats ≥95%)
  - Perfect (100%): CSV, HTML, XLSX, DOCX, WebVTT
  - Excellent (95-99%): PPTX 99%, Markdown 98%, AsciiDoc 96%
  - Near Pass (93%): JATS (Rust more correct than Python)
- ✅ Bold field label fixes completed (N=1505-1539, 32 formats)
- ✅ All Python backends eliminated from format parsers

**Format Support:**
- 60 format variants defined in InputFormat enum
- 9/9 Python-compatible formats at high quality (89% ≥95%)
- 51+ Rust-extended formats beyond Python docling
- **4x more format support than Python docling v2.58.0**

**Architecture:**
- ✅ All formats parse directly to DocItems (except PDF which is out of scope)
- ✅ Zero Python dependencies in backend parsers
- ✅ Pure Rust or C++ (via FFI) implementations only

---

## 🔴 HIGH PRIORITY - Quality Improvements

### 1. Run LLM Tests to Find New Quality Issues

**Status:** Last comprehensive run N=1547 (Nov 20, 2025)
**Action:** Run periodic LLM tests to catch regressions

**Command:**
```bash
source .env  # Load OPENAI_API_KEY
cd /Users/ayates/docling_rs
./run_comprehensive_llm_tests.sh
python3 analyze_scores.py
```

**When to run:**
- Every N mod 50 (major milestone)
- After significant format changes
- When user reports quality issues
- If verification test pass rate drops below 85%

**Expected result:** 8-9/9 verification tests ≥95% (current: 89%)

**Note:** Mode3 tests are unreliable (subjective LLM opinions). Focus on verification tests only.

---

### 2. Address Quality Issues Found in LLM Tests

**Current Known Issues:**
- ✅ JATS (93%): Not a bug - Rust more correct than Python (N=1507)
- ✅ Bold field labels: Fixed in 32 formats (N=1505-1539)

**If new issues found:**
1. Analyze LLM feedback for specific problems
2. Compare with Python docling baseline output
3. Fix root cause (don't just chase LLM score)
4. Verify fix with verification tests (not mode3)
5. Document in KNOWN_QUALITY_ISSUES.md if needed

**Estimated:** 2-5 commits per quality issue

---

### 3. Improve Low-Scoring Formats (If Any Below 95%)

**Current status:** 8/9 formats ≥95% (only JATS at 93%, not a bug)

**If verification tests show formats below 95%:**
1. Run deterministic quality tests: `scripts/scan_format_quality.sh`
2. Compare DocItem JSON structure with Python baseline
3. Identify missing DocItem types or incorrect labels
4. Fix parser to generate correct DocItems
5. Re-run verification tests to confirm

**Priority order:**
1. Formats closest to 95% threshold (easiest wins)
2. High-volume formats (CSV, HTML, DOCX, XLSX)
3. Scientific formats (JATS, DICOM)
4. Other formats

**Estimated:** 2-4 commits per format

---

## 🟡 MEDIUM PRIORITY - Feature Enhancements

### 4. ~~Add Tests for Formats with Zero Canonical Tests~~ BLOCKED

**Status:** BLOCKED per CLAUDE.md directive "❌ DO NOT expand tests - we have 2800+, that's enough"

**Historical Note:** Many Rust-extended formats have unit tests but no canonical tests.
This was considered a priority until N=1836 when test expansion was blocked.

**Recently added (N=1781-1789):**
- ✅ Apple: PAGES (4 tests), NUMBERS (4 tests), KEY (5 tests) - Total: 13 tests
- ✅ Ebooks: EPUB, FB2, MOBI (3 tests)
- ✅ 3D: GLTF (2 tests), GLB (1 test), OBJ (1 test) - Total: 4 tests
- ✅ Archives: 7Z, RAR (2 tests)
- ✅ LaTeX: 1 test
- ✅ RTF: 5 tests
- ✅ SVG: 5 tests
- ✅ DICOM: 5 tests
- ✅ VSDX: 1 test
- ✅ SRT: 1 test (subtitles)
- ✅ DOC: 1 test (N=1786)
- ✅ MPP: 1 test (N=1786)
- ✅ XPS: 5 tests (N=1787)
- ✅ IPYNB (Jupyter): 5 tests (N=1788)
- ✅ IDML (Adobe InDesign): 5 tests (N=1789)

**Deferred (not yet supported):**
- ❌ ONE (OneNote): Desktop format not supported by available Rust libraries (cloud format only)

**Total Canonical Tests:** 169 (as of N=1789)

**Process for each format:**
1. Find or create sample file (or use existing test-corpus files)
2. Generate expected output using current Rust implementation
3. Add to integration tests (test_canon_* pattern)
4. Run LLM verification to validate quality
5. Fix any quality issues found

**Why:** Canonical tests provide regression protection. Unit tests cover code paths, but canonical tests verify end-to-end quality.

**Estimated:** 1-2 commits per format

---

### 5. Optimize Slow Format Parsers

**Status:** Most formats are fast, but some may benefit from optimization

**How to identify slow formats:**
```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo test --lib --release -- --nocapture 2>&1 | grep "finished in"
```

**Optimization techniques:**
- Profile with `cargo flamegraph`
- Reduce allocations (use `&str` instead of `String` where possible)
- Avoid unnecessary clones
- Use parallel processing for independent operations
- Cache expensive computations
- Use more efficient data structures

**Target:** All formats parse <1s for typical files, <10s for large files

**Estimated:** 2-3 commits per optimization

---

### 6. Enhance Format Support

**Potential enhancements:**

**A. SVG Visual Element Extraction (Low Priority)**
- Current: Extracts only `<text>` elements
- Enhancement: Extract `<circle>`, `<rect>`, `<path>` geometric elements
- Use case: Full SVG structure extraction for diagramming tools
- Estimated: 3-4 commits

**B. Rich Table Cell Support (Already Complete)**
- ✅ Completed N=1062-1064
- Rich text content in table cells (bold, italic, links)
- Supports DOCX, XLSX, HTML, Markdown, JATS

**C. Additional Format Variants**
- Support for format-specific features users request
- Example: LaTeX formula extraction, CAD layer information
- Estimated: Varies by feature complexity

**D. Performance Benchmarks**
- Create formal benchmark suite
- Track performance over time
- Identify regressions early
- Estimated: 4-5 commits

---

## 🟢 LOW PRIORITY - Maintenance & Polish

### 7. Code Quality Improvements (ONGOING)

**Continuous refactoring during N mod 5 cleanups:**
- ✅ Fix clippy warnings (currently zero)
- ✅ Improve error messages
- ✅ Add documentation
- ✅ Reduce code duplication
- ✅ Update outdated comments
- ✅ Remove unused code

**Estimated:** Ongoing, part of regular cleanup cycles

---

### 8. Documentation Updates (ONGOING)

**Keep documentation current:**
- ✅ Update CURRENT_STATUS.md after each session
- ✅ Update FORMAT_PROCESSING_GRID.md with new findings
- ✅ Document quality issues in KNOWN_QUALITY_ISSUES.md
- ✅ Update TESTING_STRATEGY.md with new test patterns
- ✅ Keep CLAUDE.md accurate for AI workers

**Estimated:** Ongoing, part of every commit

---

### 9. TODO Comment Resolution

**Current TODOs in codebase: ~18** (acceptable level)

**TODOs by priority:**

**Low Priority (Not Blocking):**
- `crates/docling-backend/src/pdf.rs:1213` - PDF DocItem generation (OUT OF SCOPE)
- `crates/docling-backend/src/markdown.rs:477` - HTML blocks in markdown (edge case)
- `crates/docling-backend/src/asciidoc.rs:2494` - Delimited blocks (rare feature)
- `crates/docling-legacy/src/lib.rs:6-7` - WordPerfect, WPS (legacy formats, low demand)
- `crates/docling-microsoft-extended/src/publisher.rs:11` - Publisher direct DocItems (low priority)

**Documentation TODOs (Not Code Issues):**
- `crates/docling-calendar/src/ics.rs:8,105,339` - VTODO mentioned in docs (not missing feature)
- `crates/docling-backend/src/ics.rs:19` - Same as above

**Investigate TODOs (May Be Useful):**
- `crates/docling-parse-rs/src/convert.rs:103` - Text direction detection (could improve RTL languages)
- `crates/docling-backend/src/xlsx.rs:26` - Chart extraction (Python also doesn't do this)
- `crates/docling-backend/src/jats.rs:2539` - Direct article-title support (low priority)

**When to address TODOs:**
- User requests the feature
- Quality issues require it
- During refactoring of that code
- When bored and want something small to do

**Estimated:** 1 commit per TODO

---

## 📋 Completed Major Work (Historical)

**Phase E: Open Standards Formats (N=1000-1573)** ✅
- ✅ Bold field label fixes (N=1505-1539, 32 formats)
- ✅ LLM quality verification (N=1547, 89% pass rate)
- ✅ Rich table cells (N=1062-1064)
- ✅ DICOM format support (N=1526)
- ✅ JATS italicization investigation (N=1507, Rust correct)
- ✅ 479+ consecutive sessions at 100% test pass rate

**Phase D: Extended Format Support (N=500-1000)** ✅
- ✅ Archive formats (ZIP, TAR, 7Z, RAR)
- ✅ Image formats (PNG, JPEG, TIFF, WEBP, BMP, GIF, HEIF, AVIF)
- ✅ 3D formats (STL, OBJ, GLTF, GLB, PLY)
- ✅ Geospatial (GPX, KML, KMZ)
- ✅ Calendar (ICS, VCF)
- ✅ Ebooks (EPUB, MOBI, FB2)
- ✅ CAD (DXF, DWG)

**Phase C: Core Format Quality (N=200-500)** ✅
- ✅ HTML nested lists fix (N=1462)
- ✅ XLSX rich table cells
- ✅ DOCX nested structures
- ✅ JATS backend implementation
- ✅ AsciiDoc delimited blocks

**Phase A-B: Foundation (N=0-200)** ✅
- ✅ Project architecture
- ✅ Core format backends (PDF, DOCX, XLSX, PPTX, HTML, Markdown, CSV)
- ✅ DocItem type system
- ✅ Serialization pipeline (Markdown, JSON, HTML, YAML)
- ✅ Test infrastructure

---

## 🎯 Immediate Next Actions (N=1574+)

**Priority 1: Maintain System Health**
1. ✅ Keep all tests passing (100% pass rate)
2. ✅ Keep clippy clean (zero warnings)
3. ✅ Monitor performance (backend ~18-21s, core ~11-13s)
4. ✅ Update documentation after each commit

**Priority 2: Periodic Quality Checks**
5. Run LLM tests every N mod 50 (next: N=1800, 34 sessions away)
6. Fix any quality regressions immediately
7. Update KNOWN_QUALITY_ISSUES.md with findings

**Priority 3: Incremental Improvements**
8. ~~Add canonical tests for untested formats~~ (BLOCKED: CLAUDE.md says "we have 2800+, that's enough")
9. Optimize slow parsers if found
10. Address user-requested features
11. Resolve TODOs when opportune

**Priority 4: Cleanup & Documentation**
12. N mod 5: Cleanup cycles (next: N=1775, 5 sessions away)
13. N mod 10: Benchmark cycles (next: N=1780, 10 sessions away)
14. Keep documentation current
15. Refactor code quality issues

**Priority 5: Continue Indefinitely**
16. Never claim "project complete"
17. Always look for next improvement
18. Monitor for regressions
19. Respond to user needs
20. **Work continuously**

---

## 🔍 How to Find Work When Nothing is Broken

**When all tests pass and quality is high:**

1. **Run LLM tests** - Might find subtle quality issues
   ```bash
   source .env && ./run_comprehensive_llm_tests.sh
   ```

2. **Check for performance regressions** - Compare with previous benchmarks
   ```bash
   cargo test --lib --release 2>&1 | grep "finished in"
   ```

3. **Add tests for untested formats** - Look in FORMAT_PROCESSING_GRID.md

4. **Profile slow operations** - Use flamegraph to find bottlenecks
   ```bash
   cargo install flamegraph
   cargo flamegraph --test integration_tests
   ```

5. **Review TODO comments** - Some may be worth addressing
   ```bash
   grep -r "TODO\|FIXME" crates/*/src/*.rs
   ```

6. **Improve documentation** - Make guides clearer for future workers

7. **Refactor code** - Reduce duplication, improve clarity

8. **Add benchmarks** - Formal performance tracking

9. **Enhance error messages** - Make debugging easier

10. **Read user feedback** - What features do they want?

**Remember:** A healthy codebase is never "done". There's always something to improve.

---

## 🚨 What NOT to Do

**DO NOT:**
- ❌ Add Python dependencies to format backends (use Rust/C++ only)
- ❌ Make changes just to "look busy" (quality over activity)
- ❌ Break existing tests to "improve" them (tests are contract)
- ❌ Chase LLM mode3 scores (unreliable, subjective)
- ❌ Add features without tests
- ❌ Optimize without profiling (premature optimization)
- ❌ Modify historical records or expected outputs without cause
- ❌ Work on PDF quality improvements (OUT OF SCOPE per CLAUDE.md)
- ❌ Implement audio/video formats (OUT OF SCOPE - separate system)
- ❌ Implement database formats (OUT OF SCOPE - use database tools)

**DO:**
- ✅ Maintain 100% test pass rate
- ✅ Keep clippy clean (zero warnings)
- ✅ Run LLM verification tests periodically
- ✅ Fix quality issues when found
- ❌ Add tests for formats that lack them (BLOCKED: CLAUDE.md says "we have 2800+, that's enough")
- ✅ Optimize only when profiling shows bottleneck
- ✅ Document all changes clearly
- ✅ Work continuously, never stop
- ✅ Focus on verification test pass rate (not mode3)
- ✅ Use deterministic tests (`scripts/scan_format_quality.sh`)

---

## 📈 Success Metrics

**System Health (Critical):**
- ✅ Test pass rate: 100% (currently 3,419/3,419)
- ✅ Clippy warnings: 0 (currently 0)
- ✅ Consecutive passing sessions: 674+ (excellent stability)

**Quality Metrics (Important):**
- ✅ LLM verification pass rate: ≥85% (currently 89%, 8/9 formats)
- ✅ Python-compatible formats quality: ≥95% (currently 8/9 formats)
- ✅ Rust-extended formats: Unit tested (2842/2842 tests pass)

**Performance Metrics (Monitor):**
- Backend tests: ~17-18s (release mode, excellent)
- Core tests: ~10-11s (release mode, excellent)
- Combined: ~28-29s (release mode, excellent)

**Format Support (Expand):**
- Total formats: 60 (4x more than Python's 15)
- With canonical tests: 9 baseline + growing extended
- With unit tests: All 60 formats

**If any metric degrades, that becomes highest priority.**

---

## 📝 Work Log Template

**Use this structure for regular development commits:**

```
# N=XXXX: [Type] - [Brief Description]
**Current Plan**: [Link to plan file or brief summary]
**Checklist**: [Status or progress summary]

## Changes
[What changed and why]

## New Lessons
[Important discoveries, if any]

## Information Expiration
[Any documentation now obsolete]

## Next AI: [Brief directive]
[What the next AI should focus on]
[Links to reports for context]

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>
```

**Commit types:**
- Regular Development: Routine improvements, small features
- Quality: LLM test runs, quality fixes
- Cleanup Milestone: N mod 5 = 0 cleanup cycles
- Benchmark Milestone: N mod 10 = 0 benchmarks
- Feature: Significant new functionality

---

## 🔄 Remember: Work Never Stops

After completing any task:
1. Update documentation
2. Commit with clear message
3. Look for next task
4. Repeat

The project is never "finished". Continue working indefinitely.

**Question: "What should I work on now?"**
**Answer: Check this file. Pick highest priority task. Do it. Repeat.**

---

**WORKER: Focus on maintaining health (100% tests, zero warnings), running periodic LLM tests (every N mod 50), and making incremental improvements. Never stop working.**
