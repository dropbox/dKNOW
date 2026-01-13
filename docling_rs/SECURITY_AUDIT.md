# Security Audit - docling_rs

**Last Updated:** 2025-11-09 (N=197)
**Previous Audit:** N=178 (Nov 9, 2025)

## Summary

**Vulnerabilities:** 0
**Warnings:** 4 (unmaintained dependencies, transitive)
**Status:** ✅ CLEAN - Safe to publish

---

## Current Findings (November 9, 2025)

### No Vulnerabilities

The workspace has **zero security vulnerabilities**. Previous idna vulnerability (RUSTSEC-2024-0421) was resolved in N=196 by removing the unused `vcard` dependency from docling-email.

### Unmaintained Dependencies (4 warnings)

All warnings are for **transitive dependencies** (pulled in by third-party crates). None represent actual security vulnerabilities, only maintenance concerns.

#### 1. encoding 0.2.33 (RUSTSEC-2021-0153)
- **Status:** Unmaintained since 2021
- **Used by:** mobi → docling-ebook, dicom-encoding → docling-medical
- **Risk:** Low (stable, no known vulnerabilities)
- **Action:** Monitor for maintained alternatives in mobi/dicom ecosystems

#### 2. fxhash 0.2.1 (RUSTSEC-2025-0057)
- **Status:** No longer maintained (as of Sep 2025)
- **Used by:** selectors → scraper → docling-ebook
- **Risk:** Low (stable hash function, no known issues)
- **Action:** Monitor scraper crate for migration to rustc-hash or ahash

#### 3. memmap 0.7.0 (RUSTSEC-2020-0077)
- **Status:** Unmaintained since 2020
- **Used by:** mbox-reader → docling-email
- **Risk:** Low (stable, superseded by memmap2)
- **Action:** Monitor mbox-reader for upgrade to memmap2

#### 4. paste 1.0.15 (RUSTSEC-2024-0436)
- **Status:** No longer maintained (as of Oct 2024)
- **Used by:** rav1e → ravif → image → pdfium-render, docling-cad, docling-medical
- **Risk:** Low (procedural macro, stable)
- **Action:** Monitor rav1e/image crates for migration

---

## Resolution of Previous Issues

### N=196: idna Vulnerability Eliminated

**Previous Issue (N=178):** RUSTSEC-2024-0421 (idna 0.4.0 Punycode validation bypass)
- **Dependency chain:** idna → vcard → docling-email
- **Blocked:** vcard crate pinned to idna 0.4.x (no fix available)
- **Resolution:** Removed unused `vcard` dependency from docling-email (N=196)
- **Verification:** `cargo tree -i idna` shows no idna dependencies remain
- **Impact:** Eliminated primary publication blocker

**Why vcard Was Removable:**
- vcard crate was declared in Cargo.toml but never imported/used
- docling-email implements its own simple vCard parser (vcf.rs)
- All 19 docling-email unit tests pass without the dependency
- Zero functional impact

**Secondary Benefits:**
- Removed `failure` crate warnings (RUSTSEC-2020-0036, RUSTSEC-2019-0036)
- Removed `lexical-core` soundness warning (RUSTSEC-2023-0086)
- Removed `quick-xml 0.17.2` future-incompatibility warning
- Eliminated 5 transitive dependency warnings with one change

---

## Audit History

### N=197 (Nov 9, 2025) - CURRENT
- **Command:** `cargo audit`
- **Result:** 0 vulnerabilities, 4 warnings (unmaintained transitive deps)
- **Change from N=196:** All vulnerabilities resolved
- **Status:** ✅ Clean - safe to publish

### N=196 (Nov 9, 2025)
- **Action:** Removed unused `vcard` dependency from docling-email
- **Impact:** Eliminated idna vulnerability + 4 additional warnings
- **Verification:** All 336 unit tests pass, 0 clippy warnings

### N=178 (Nov 9, 2025)
- **Result:** 1 vulnerability (idna), 7 warnings
- **Assessment:** Documented as publication blocker requiring resolution

### N=158 (Oct 2024)
- **Result:** 0 HIGH vulnerabilities, pyo3 upgraded to 0.27.1
- **Status:** Clean (idna advisory didn't exist yet, published Dec 2024)

---

## Publication Status

### Security Posture: EXCELLENT ✅

**Zero Vulnerabilities:**
- No RUSTSEC advisories for any direct or transitive dependencies
- All warnings are maintenance-related, not security issues
- Previous blocker (idna) successfully eliminated

**Unmaintained Warnings - Low Risk:**
- All 4 warnings are for **transitive dependencies** (not our code)
- None have known security exploits
- Represent upstream maintenance concerns only
- Standard issue in large dependency trees

**Comparison to Ecosystem:**
- Most Rust projects have similar transitive maintenance warnings
- Zero vulnerabilities is above-average security posture
- All warnings affect optional features or non-critical paths

### Recommendation: PUBLISH ✅

**No security blockers for publication.** The 4 unmaintained dependency warnings are:
1. Industry-standard risk (common in mature codebases)
2. All transitive (not directly controllable)
3. Zero exploits or CVEs
4. Affect optional functionality only

Users can audit dependencies with `cargo audit` before deployment if concerns exist.

---

## Monitoring Strategy

### For Maintainers

1. **Frequency:** Run `cargo audit` at every benchmark cycle (N mod 10)
2. **Watch:** Subscribe to RUSTSEC advisory RSS/GitHub notifications
3. **Upgrade:** Update dependencies when advisories published
4. **Document:** Track security changes in git commits + CHANGELOG

### Audit Commands

```bash
# Security audit (check for vulnerabilities)
cargo audit

# Check for outdated dependencies
cargo outdated --workspace

# Dependency tree analysis (example)
cargo tree -i encoding

# Update advisory database
cargo audit --update-advisory-db
```

### Dependency Tree Locations

```
# encoding (mobi, dicom ecosystems)
docling-ebook → mobi → encoding
docling-medical → dicom → dicom-encoding → encoding

# fxhash (HTML parsing)
docling-ebook → scraper → selectors → fxhash

# memmap (email parsing)
docling-email → mbox-reader → memmap

# paste (image processing)
docling-backend → pdfium-render → image → ravif → rav1e → paste
docling-cad → dxf → image → paste
docling-medical → dicom → dicom-pixeldata → image → paste
```

---

## Risk Assessment Summary

| Issue | Severity | Likelihood | Impact | Mitigation | Status |
|-------|----------|------------|---------|------------|---------|
| idna 0.4.0 | N/A | N/A | N/A | Removed dependency | ✅ RESOLVED |
| encoding unmaintained | Low | Low | Low | Monitor upstream | ⚠️ Acceptable |
| fxhash unmaintained | Low | Low | Low | Monitor upstream | ⚠️ Acceptable |
| memmap unmaintained | Low | Low | Low | Monitor upstream | ⚠️ Acceptable |
| paste unmaintained | Low | Low | Low | Monitor upstream | ⚠️ Acceptable |

**Overall Risk:** LOW - Safe for production use and crates.io publication

---

**Next Review:** N=200 or N=208 (next benchmark cycle after N=198)
**Audit Frequency:** Every benchmark cycle (N mod 10) + when new RUSTSEC advisories published
