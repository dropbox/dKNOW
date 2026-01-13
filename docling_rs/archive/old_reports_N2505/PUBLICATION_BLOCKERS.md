# Publication Blockers - User Confirmation Required

**Status:** ✅ All technical requirements satisfied (28/30 crates publishable)
**Next Step:** User confirmation on 5 critical questions before publishing to crates.io

**Note:** Native backend crates (docling-parse-sys, docling-parse-rs) excluded from publication (marked `publish = false`). These require pre-compiled C library and workspace environment. Optional feature in docling-backend, not required for hybrid approach.

---

## Critical: Publishing to crates.io is IRREVERSIBLE

Once published, crates can only be **yanked** (hidden from new projects), not deleted. Old versions remain accessible forever. Please carefully review these questions before proceeding.

---

## 5 Questions Requiring User Confirmation

### 1. Repository URL Verification ⚠️

**Current setting:** `https://github.com/ayates_dbx/docling_rs`

**Questions:**
- Is `ayates_dbx` the correct GitHub username? (vs `ayates` or other)
- Is this repository public, or will it be made public before publication?
- Is this the correct permanent URL for the project?

**Impact:** This URL appears on crates.io and in all package metadata. Difficult to change after publication.

**Required action:** Confirm URL is correct, or provide corrected URL.

---

### 2. License Confirmation ⚠️

**Current setting:** MIT License
**Copyright holder:** "docling_rs contributors"

**Questions:**
- Confirm MIT license is appropriate for this project
- Verify copyright holder: "docling_rs contributors" vs specific person/organization
- Consider dual license? (MIT OR Apache-2.0 is common in Rust ecosystem)

**Impact:** License cannot be changed retroactively for published versions.

**Required action:** Confirm MIT license with current copyright holder, or specify changes.

---

### 3. Version Strategy ⚠️

**Current setting:** 2.58.0 (all 28 publishable crates synchronized)

**Options:**
1. **Keep 2.58.0** - Signals feature parity with Python docling v2.58.0
2. **Start at 0.1.0** - Standard for new implementations (signals "API may evolve")
3. **Start at 1.0.0** - Signals "production-ready, stable API commitment"

**Recommendation:** Keep 2.58.0 for version alignment with Python docling

**Questions:**
- Confirm version 2.58.0 is appropriate for initial Rust release?
- Future versioning: track Python docling versions, or independent Rust versions?

**Impact:** Version affects user expectations about stability and API guarantees.

**Required action:** Confirm 2.58.0 or specify alternative version.

---

### 4. Publication Timing & Strategy ⚠️

**Options:**

**A. Phased Publication (RECOMMENDED)**
- **Session 1:** Publish Tier 1 leaf crates only (19 crates, 1-2 hours)
  - Excludes: docling-parse-sys, docling-parse-rs (non-publishable)
- Verify installation works: `cargo add docling-models`
- Address any crates.io issues discovered
- **Session 2:** Publish Tier 2-5 after verification (9 crates, 1-2 hours)

**B. Full Publication**
- Publish all 28 publishable crates in single session (2.5-3 hours)
- Excludes: docling-parse-sys, docling-parse-rs (non-publishable)
- Higher risk if unexpected issues arise
- All-or-nothing approach

**C. Wait for Additional Polish**
- Run clippy workspace-wide, fix warnings
- Additional verification (dry-run tests, documentation builds)
- Publish at N=165 (cleanup cycle) or N=170 (benchmark cycle)

**Questions:**
- Publish immediately (N=163+), or wait?
- Phased approach (safer), or full publication (faster)?
- Need additional verification first (clippy, docs, etc.)?

**Required action:** Choose publication strategy and timing.

---

### 5. Maintenance Plan ⚠️

**Questions:**
- Who will maintain published crates on crates.io going forward?
- How to handle Python docling updates? (track versions, backport features?)
- Patch strategy for Rust-specific bugs?
- Semver strategy:
  - Patch (2.58.x) for bug fixes?
  - Minor (2.x.0) for new Rust features?
  - Major (x.0.0) for breaking changes?

**Impact:** Published crates require ongoing maintenance. Abandoned crates harm ecosystem trust.

**Required action:** Confirm maintenance commitment and versioning strategy.

---

## Technical Readiness Checklist

All technical requirements satisfied (with 1 note):

- ✅ **LICENSE file:** MIT license in repository root
- ✅ **Descriptions:** 100% coverage (28/28 publishable crates)
- ✅ **README files:** 100% coverage (28/28 publishable crates), comprehensive for Tier 1
- ✅ **Repository metadata:** Configured via workspace inheritance
- ✅ **Version sync:** All publishable crates at 2.58.0
- ✅ **Packaging:** Native backend crates marked non-publishable (docling-parse-sys, docling-parse-rs)
- ✅ **Path dependencies:** All have version requirements (41 declarations)
- ✅ **Keywords/Categories:** Tier 1 crates fully tagged
- ✅ **Security audit:** 0 vulnerabilities, 4 unmaintained dependency warnings (low risk)
  - encoding, fxhash, memmap, paste (transitive deps, no security issues)
  - All are unmaintained warnings, not vulnerabilities
  - Previous idna vulnerability resolved (vcard dependency removed N=196)
  - Recommended: Safe to publish
- ✅ **Unit tests:** 339/339 passing (28 packages, excludes native backend, verified N=214)
- ✅ **Integration tests:** 97/97 passing (100% pass rate, verified N=214)
- ✅ **Deprecation warnings:** 0 warnings (pyo3 API migration complete)
- ✅ **Documentation:** Comprehensive README for all crates
- ✅ **Code quality:** 0 clippy warnings (verified N=214)

---

## Pre-Publication Verification (Recommended)

Before publishing, recommend running these commands to catch any last-minute issues:

### 1. Dry-Run Tests (5 minutes)

```bash
# Test leaf crates (should succeed)
cargo publish --dry-run --allow-dirty -p docling-models
cargo publish --dry-run --allow-dirty -p docling-audio
cargo publish --dry-run --allow-dirty -p docling-ebook

# Cannot test Tier 3-5 until dependencies published (expected)
```

### 2. Clippy Lints (5-10 minutes)

```bash
# Check for warnings that will show on crates.io
cargo clippy --workspace --all-features -- -D warnings
```

### 3. Documentation Build (5-10 minutes)

```bash
# Verify rustdoc builds successfully
cargo doc --workspace --no-deps
```

### 4. Security Audit (1 minute)

```bash
# Verify no new vulnerabilities since N=158
cargo audit
```

---

## Publication Dependency Order

Crates must be published in dependency order:

1. **Tier 1 (19 leaf crates):** No internal dependencies, publish first
   - Excludes: docling-parse-sys, docling-parse-rs (marked `publish = false`)
2. **Tier 2 (5 mid-level crates):** Depend on Tier 1, wait for indexing (~2 min/crate)
3. **Tier 3 (docling-core):** Depends on 15 Tier 1 crates
4. **Tier 4 (docling-backend):** Depends on core + 16 format crates
5. **Tier 5 (docling-cli):** Depends on backend

**Total publishable:** 28 crates (19 + 5 + 1 + 1 + 1 + 1 = 28)

**Critical:** Each tier must be fully indexed on crates.io before publishing next tier.

---

## Detailed Reports

- **Publication Readiness:** `reports/feature-phase-e-open-standards/N162_publication_readiness_2025-11-09.md` (comprehensive 773-line assessment)
- **Changelog:** `CHANGELOG.md` (updated with 2.58.0 release notes)
- **Phase H Summary:** Last 3 git commits (N=159-161) document all preparation work

---

## Next Steps

1. **User provides answers to 5 questions above**
2. **AI runs pre-publication verification** (dry-run, clippy, docs, audit)
3. **AI begins publication** according to chosen strategy
4. **AI documents results** in next commit

**Note on PR/Merge:** Publication to crates.io can happen from this feature branch. Merging to main is optional and can happen before, after, or independently of crates.io publication. The codebase is stable and tests are passing on this branch.

---

**Generated:** N=162, 2025-11-09 (updated N=174 for native backend exclusion, N=201 for workflow clarification, N=214 for format expansion)
**Branch:** feature/phase-e-open-standards
**Last Verified:** N=214 (2025-11-09 11:20 PST)
