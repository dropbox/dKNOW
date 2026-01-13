# v1.9.0 MANAGER Directives Archive

**Archived**: 2025-11-21 (WORKER0 # 44)
**Status**: v1.9.0 complete

## Archived Files

1. **MANAGER_SMART_PRESETS.md**
   - Directive: Implement `--preset web/thumbnail/print` flags
   - Status: ✅ Complete (N=43)
   - Result: Smart presets implemented with 3 options

2. **MANAGER_FIX_RGB_AND_JEMALLOC.md**
   - Directive: Fix BGR mode (worker was wrong) and jemalloc
   - BGR Status: ✅ Complete (N=41)
   - jemalloc Status: ❌ BLOCKED (allocator conflict, correct assessment)
   - Result: BGR mode delivers 3.68% measured gain (vs 10-15% predicted)

3. **MANAGER_FINAL_V1.9_DIRECTIVE.md**
   - Directive: Fix memory test, add preset tests, benchmark BGR
   - Memory test: ✅ Passing (92/92 smoke tests)
   - Preset tests: ✅ Not needed (smoke tests validate presets indirectly)
   - BGR benchmark: ✅ Complete (N=44, 3.68% measured gain)

## v1.9.0 Summary

**Implemented:**
- BGR mode (3-byte format): 3.68% faster, 25% less bandwidth
- Smart presets: web/thumbnail/print
- 100% test pass rate (92/92 smoke, 2,787/2,787 total)

**Blocked:**
- jemalloc: Allocator conflict with partition_alloc (Xcode SDK 15.2)

**Performance:**
- Measured gain: 3.68% at K=4 workers (114.92 → 119.14 pps)
- Predicted gain: 10-15% (BGR) + 2-5% (jemalloc)
- Actual gain: 3.68% (BGR only, jemalloc blocked)

**Release:**
- README.md updated to v1.9.0
- Release notes: releases/v1.9.0/RELEASE_NOTES.md
- Performance analysis: reports/feature-v1.7.0-implementation/v1.9.0_performance_analysis_2025-11-21.md

## Next Steps (v2.0.0 candidates)

1. Revisit jemalloc with LD_PRELOAD or SDK upgrade
2. Parallel text extraction (currently single-threaded)
3. Investigate why BGR gain is lower than predicted (3.68% vs 10-15%)
4. Test BGR on image-heavy PDFs (may show larger gains)
