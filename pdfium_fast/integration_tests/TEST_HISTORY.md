# Historical Test Results

**Archived**: 2025-11-14 (Git 2b29139f)
**Note**: This file contains historical test results from development iterations.
**Current Status**: See CLAUDE.md Production Status section for latest results.

**WARNING**: Do not rely on this data. Always run `pytest` to get current results.

---

**Test Results (N=39) - Latest Benchmark Run:**

Smoke Tests:
- Command: `pytest -m smoke --tb=line -q`
- Result: 67 passed (100% pass rate)
- Session: sess_20251112_024657_b35068f9
- Duration: 450.18s (7m 30s)
- Timestamp: 2025-11-12T02:46:57Z

Performance Tests:
- Command: `pytest -m performance --tb=line -v`
- Result: 8 passed (100% pass rate)
- Session: sess_20251112_025452_6b17dbcb
- Duration: 538.22s (8m 58s)
- Text speedup: 2.36x-3.10x at 4 workers
- Image speedup: 3.55x-3.72x at 4 workers

Scaling Tests:
- Command: `pytest -m scaling --tb=line -v`
- Result: 6 passed (100% pass rate)
- Session: sess_20251112_030404_42f2b37e
- Duration: 68.01s (1m 8s)
- Text scaling: 1.53x-3.08x at 4 workers
- Image scaling: 2.55x-3.52x at 4 workers
- 8 workers: 4.71x (text), 6.46x (image) on 821p PDF

**Test Results (N=40) - Current Health Check:**

Smoke Tests:
- Command: `pytest -m smoke --tb=line -q`
- Result: 67 passed (100% pass rate)
- Session: sess_20251112_030758_d75e3c4a
- Duration: 466.47s (7m 46s)
- Timestamp: 2025-11-12T03:07:58Z

**Test Results (N=44) - Test Bug Fixes:**

Test Fixes Applied:
- Fixed: Page counting now includes both PNG and JPEG files (smart mode mixed output)
- Fixed: PPM test now expects PPM output (not JPEG) when --ppm flag used
- Removed: 2 invalid comparative performance tests (always-on mode makes them meaningless)
- Result: All reported "failures" were test implementation bugs, not code regressions

Smoke Tests (Post-Fix):
- Command: `pytest -m smoke --tb=line -q`
- Result: 67 passed (100% pass rate)
- Session: sess_20251112_045452_6ccaffe5
- Duration: 462.33s (7m 42s)
- Timestamp: 2025-11-12T04:54:52Z

**Test Results (N=45) - N mod 5 Cleanup:**

Cleanup Actions:
- Archived resolved issue documents to integration_tests/archive/resolved_issues_N44_20251112/
- Verified system health (load 2.38 < 6.0, no hung processes)
- Confirmed no stale temporary files or caches

Smoke Tests (Post-Cleanup):
- Command: `pytest -m smoke --tb=line -q`
- Result: 67 passed (100% pass rate)
- Session: sess_20251112_053235_fd887dfb
- Duration: 460.95s (7m 40s)
- Timestamp: 2025-11-12T05:32:35Z

**Test Results (N=46-74) - Stability Verification:**

System Health Verification (N=46-74):
- 29 consecutive iterations with 100% test pass rate
- All smoke tests: 67/67 pass (100%)
- System load: 2.38-2.78 (all well below 6.0 threshold)
- No hung processes detected
- No regressions observed

Latest Test Run (N=74):
- Command: `pytest -m smoke --tb=line -q`
- Result: 67 passed (100% pass rate)
- Session: sess_20251112_111830_a11790db
- Duration: 463.36s (7m 43s)
- Timestamp: 2025-11-12T11:18:30Z

**Test Results (N=101) - System Health Verification:**

Smoke Tests:
- Command: `pytest -m smoke --tb=line -q`
- Result: 67 passed (100% pass rate)
- Session: sess_20251113_051511_05cdf20d
- Duration: 472.27s (7m 52s)
- Timestamp: 2025-11-13T05:15:11Z

System Health:
- Load average: 3.10 (healthy, < 6.0 threshold)
- Hung processes: 0
- No regressions observed

**Test Results (N=102) - Extended Test Verification:**

Extended Tests:
- Command: `pytest -m extended --tb=line -q`
- Result: 957 passed, 6 skipped, 1 xfailed (100% pass rate)
- Session: sess_20251113_053916_72cb7b78
- Duration: 1088.55s (18m 8s)
- Timestamp: 2025-11-13T05:39:16Z

Expected Skips/XFails:
- 6 skipped: Large PDFs > 500 pages (disk space management)
- 1 xfailed: bug_451265 - upstream PDFium infinite loop (timeout >300s)

**Test Results (N=107) - Priority 1 Complete - All Skips Fixed:**

Extended Tests:
- Command: `pytest -m extended --tb=line -q`
- Result: 963 passed, 1 xfailed (100% pass rate - 0 skips!)
- Session: sess_20251113_071014_5a47d0ca
- Duration: 1393.52s (23m 13s)
- Timestamp: 2025-11-13T07:33:46Z

Priority 1 Success:
- Removed MAX_PAGES_FOR_PPM=500 limit (N=105)
- All 6 large PDF skips now PASS (6 skips → 0 skips)
- Test count increased: 957 → 963 (6 new tests passing)
- Only expected XFail: bug_451265 (upstream infinite loop)

Smoke Tests:
- Command: `pytest -m smoke --tb=line -q`
- Result: 67 passed (100% pass rate)
- Session: sess_20251113_073406_ea16b462
- Duration: 472.47s (7m 52s)
- Timestamp: 2025-11-13T07:42:58Z

System Health:
- Load average: 5.05 (healthy, < 6.0 threshold)
- Hung processes: 0
- No regressions observed

**Test Results (N=115) - N mod 5 Cleanup Verification:**

Cleanup Actions:
- Archived resolved manager directives (MANAGER_*.md) to integration_tests/archive/resolved_manager_directives_N110-114_20251113/
- Archived obsolete status documents (N48, N50) to integration_tests/archive/obsolete_status_docs_N48-50_20251113/
- Verified system health (load 5.02 < 6.0, no hung processes)

Smoke Tests (Post-Cleanup):
- Command: `pytest -m smoke --tb=line -q`
- Result: 67 passed (100% pass rate)
- Session: sess_20251113_093515_09962feb
- Duration: 472.20s (7m 52s)
- Timestamp: 2025-11-13T09:35:15Z

System Health:
- Load average: 5.02 (healthy, < 6.0 threshold)
- Hung processes: 0
- No regressions observed

**Test Results (N=150-151) - ABSOLUTE ZERO: 0 Skips Achieved:**

Generator Fix (N=150):
- Deleted 2 lines in lib/generate_test_files.py:
  - Line 177: pytest.skip(extract_text_jsonl not found)
  - Line 343: pytest.skip(PPM baseline not found)
- Regenerated all 452 test files (1356 test functions)
- Result: Eliminated 904 bogus skips from tests/pdfs/*

Extended Tests (N=150):
- Command: `pytest -m extended --tb=line -q`
- Result: 963 passed, 0 skipped, 1 xfailed (100% pass rate - 0 skips!)
- Session: sess_20251113_222931_7b7b4be5
- Duration: 1409.45s (23m 23s)
- Timestamp: 2025-11-13T22:29:31Z

Verification (N=151):
- Smoke: 67 passed, 0 skipped (sess_20251113_225658_c6e71eae, 434.93s)
- Extended: 963 passed, 0 skipped, 1 xfailed (sess_20251113_230434_b9a7c17a, 1401.45s)
- Cleanup: Archived obsolete status documents from N=144-149

System Health (N=151):
- Load average: 6.19 (slightly elevated during test execution, acceptable)
- Hung processes: 0
- No regressions observed

**Test Results (N=154) - Test Regression Investigation Resolved:**

Investigation Summary:
- N=153: 1 FAILED (bug_451265 timeout) vs expected 1 XFAILED
- Root cause: Transient issue from hung processes (2 workers stuck 27+ minutes)
- Resolution: Killed hung processes, added venv/ to .gitignore

Extended Tests (N=154):
- Command: `pytest -m extended --tb=line -q`
- Result: 963 passed, 0 skipped, 1 xfailed (100% pass rate)
- Session: sess_20251114_013214_1d7f9e1e
- Duration: 1320.81s (22m 0s)
- Timestamp: 2025-11-14T01:32:14Z

Smoke Tests (N=154):
- Command: `pytest -m smoke --tb=line -q`
- Result: 67 passed (100% pass rate)
- Session: sess_20251114_015426_7b371997
- Duration: 423.52s (7m 3s)
- Timestamp: 2025-11-14T01:54:26Z

System Health (N=154):
- Load average: 3.54 (healthy, < 6.0 threshold)
- Hung processes: 0
- No regressions observed
- Conclusion: Failure was transient, does not reproduce

**Test Results (N=155) - N mod 5 Cleanup:**

Cleanup Actions:
- Archived resolved manager directives (N=148-154) to integration_tests/archive/resolved_manager_directives_N148-154_20251114/
  - DELETE_2_SKIP_LINES.md, FIND_AND_FIX_SKIPS.md, FIX_28_SKIPS_NOW.md
  - MANAGER_ABSOLUTE_ZERO.md, MANAGER_FINAL_REALITY_CHECK.md, MANAGER_ZERO_MEANS_ZERO.md
- Archived obsolete status documents (N=143-148) to integration_tests/archive/obsolete_status_docs_N143-148_20251114/
  - COMPLETE_TEST_SUITE_RESULTS_N143.md, TEST_SUITE_SKIP_ANALYSIS_N148.md
  - RUN_ALL_TESTS.md, TEST_SUITE_HIERARCHY.md
- Verified system health (load 3.52 < 6.0, no hung processes)
- Verified no stale temporary files or caches

Smoke Tests (N=155):
- Command: `pytest -m smoke --tb=line -q`
- Result: 67 passed (100% pass rate)
- Session: sess_20251114_020256_339304f8
- Duration: 423.22s (7m 3s)
- Timestamp: 2025-11-14T02:02:56Z

System Health (N=155):
- Load average: 3.52 (healthy, < 6.0 threshold)
- Hung processes: 0
- No regressions observed

**Test Results (N=156) - N mod 13 Benchmark:**

Smoke Tests:
- Command: `pytest -m smoke --tb=line -q`
- Result: 67 passed (100% pass rate)
- Session: sess_20251114_021341_53366a6b
- Duration: 423.06s (7m 3s)
- Timestamp: 2025-11-14T02:13:41Z

Performance Tests:
- Command: `pytest -m performance --tb=line -v`
- Result: 8 passed, 1 skipped (100% pass rate)
- Session: sess_20251114_022125_0d116921
- Duration: 541.68s (9m 1s)
- Text speedup: 2.39x-3.09x at 4 workers
- Image speedup: 3.61x-3.70x at 4 workers

Scaling Tests:
- Command: `pytest -m scaling --tb=line -v`
- Result: 6 passed (100% pass rate)
- Session: sess_20251114_023041_eb83cb32
- Duration: 68.00s (1m 8s)
- Text scaling: 1.45x-3.09x at 4 workers
- Image scaling: 2.49x-3.52x at 4 workers

System Health (N=156):
- Load average: 3.29 (healthy, < 6.0 threshold)
- Hung processes: 0
- No regressions observed

**Test Results (N=157) - System Health Verification:**

Smoke Tests:
- Command: `pytest -m smoke --tb=line -q`
- Result: 67 passed (100% pass rate)
- Session: sess_20251114_025314_b40ad5a7
- Duration: 425.60s (7m 5s)
- Timestamp: 2025-11-14T02:53:14Z

System Health:
- Load average: 3.02 (healthy, < 6.0 threshold)
- Hung processes: 0
- No regressions observed

**Test Results (N=167) - System Health Verification:**

Smoke Tests:
- Command: `pytest -m smoke --tb=line -q`
- Result: 67 passed (100% pass rate)
- Session: sess_20251114_080812_e0348e50
- Duration: 423.11s (7m 3s)
- Timestamp: 2025-11-14T08:08:12Z

System Health:
- Load average: 2.74 (healthy, < 6.0 threshold)
- Hung processes: 0
- No regressions observed

**Test Results (N=169) - System Health Verification:**

Smoke Tests:
- Command: `pytest -m smoke --tb=line -q`
- Result: 67 passed (100% pass rate)
- Session: sess_20251114_082541_fa9d4801
- Duration: 423.71s (7m 3s)
- Timestamp: 2025-11-14T08:25:41Z

System Health:
- Load average: 2.66 (healthy, < 6.0 threshold)
- Hung processes: 0
- No regressions observed

**Test Results (N=174) - System Health Verification:**

Smoke Tests:
- Command: `pytest -m smoke --tb=line -q`
- Result: 67 passed (100% pass rate)
- Session: sess_20251114_092431_7f1e1318
- Duration: 423.74s (7m 3s)
- Timestamp: 2025-11-14T09:24:31Z

System Health:
- Load average: 2.99 (healthy, < 6.0 threshold)
- Hung processes: 0
- No regressions observed

**Test Results (N=175) - N mod 5 Cleanup:**

Cleanup Actions:
- Checked for obsolete documents: None found (all previously archived)
- Verified system health: Load 2.18 (healthy, < 6.0)
- Verified hung processes: 0
- Confirmed no stale temporary files or caches

Smoke Tests:
- Command: `pytest -m smoke --tb=line -q`
- Result: 67 passed (100% pass rate)
- Session: sess_20251114_093324_61dedd2f
- Duration: 424.15s (7m 4s)
- Timestamp: 2025-11-14T09:33:24Z

System Health:
- Load average: 2.18 (healthy, < 6.0 threshold)
- Hung processes: 0
- No regressions observed

**Test Results (N=173) - System Health Verification:**

Smoke Tests:
- Command: `pytest -m smoke --tb=line -q`
- Result: 67 passed (100% pass rate)
- Session: sess_20251114_091555_24740e3f
- Duration: 423.04s (7m 3s)
- Timestamp: 2025-11-14T09:15:55Z

System Health:
- Load average: 3.28 (healthy, < 6.0 threshold)
- Hung processes: 0
- No regressions observed

**Test Results (N=172) - System Health Verification:**

Smoke Tests:
- Command: `pytest -m smoke --tb=line -q`
- Result: 67 passed (100% pass rate)
- Session: sess_20251114_085912_50f7441e
- Duration: 423.98s (7m 3s)
- Timestamp: 2025-11-14T08:59:12Z

System Health:
- Load average: 3.25 (healthy, < 6.0 threshold)
- Hung processes: 0
- No regressions observed

**Test Results (N=171) - System Health Verification:**

Smoke Tests:
- Command: `pytest -m smoke --tb=line -q`
- Result: 67 passed (100% pass rate)
- Session: sess_20251114_085026_3cca19a7
- Duration: 424.22s (7m 4s)
- Timestamp: 2025-11-14T08:50:26Z

System Health:
- Load average: 2.87 (healthy, < 6.0 threshold)
- Hung processes: 0
- No regressions observed

**Test Results (N=170) - System Health Verification:**

Smoke Tests:
- Command: `pytest -m smoke --tb=line -q`
- Result: 67 passed (100% pass rate)
- Session: sess_20251114_084214_fe27b797
- Duration: 423.45s (7m 3s)
- Timestamp: 2025-11-14T08:42:14Z

System Health:
- Load average: 2.87 (healthy, < 6.0 threshold)
- Hung processes: 0
- No regressions observed

**Test Results (N=168) - System Health Verification:**

Smoke Tests:
- Command: `pytest -m smoke --tb=line -q`
- Result: 67 passed (100% pass rate)
- Session: sess_20251114_081653_d474e265
- Duration: 423.15s (7m 3s)
- Timestamp: 2025-11-14T08:16:53Z

System Health:
- Load average: 2.81 (healthy, < 6.0 threshold)
- Hung processes: 0
- No regressions observed

