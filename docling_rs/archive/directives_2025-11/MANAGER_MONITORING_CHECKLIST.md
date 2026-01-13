# [MANAGER] Worker Monitoring Checklist - Phases 8-14

**Started:** 2025-11-23 14:30 PT
**Worker:** Restarted with directive to complete Phases 8-14
**Monitoring Mode:** Active
**Branch:** feature/pdf-ml-migration

---

## Immediate Next Steps (Worker Should Do)

### Step 1: Read Directive ✅ (Worker should be doing this now)
- WORKER_DIRECTIVE_RESUME_PHASE_8_NOW.md
- Understand Phase 8-14 requirements
- Note libtorch installation steps

### Step 2: Install libtorch (15 min) ⏳
**Command to check:**
```bash
python3 -c "import torch; print(torch.__version__)"
echo $LIBTORCH_USE_PYTORCH
```

**Expected:** torch installed, LIBTORCH_USE_PYTORCH=1

### Step 3: Verify Build (5 min) ⏳
```bash
cd ~/docling_rs
cargo build -p docling-pdf-ml --features "pytorch,opencv-preprocessing" --release
```

**Expected:** Clean build, zero warnings

### Step 4: Begin Phase 8 (2-3 days) ⏳
**First commit expected:** # 8: PDF ML Phase 8 - Assembly pipeline
**Files to copy:** pipeline_modular/*.rs from source

---

## Phase-by-Phase Monitoring

### Phase 8: Assembly Pipeline (2-3 days)
- [ ] Started: _____
- [ ] Files copied from source (7-10 files)
- [ ] Module structure created
- [ ] Imports adapted for docling_rs
- [ ] Tests added
- [ ] Commit # 8 created
- [ ] Build clean, zero warnings

**Watch for:**
- Worker copying code (good)
- Worker "refactoring" code (bad - stop this)
- Type conversion issues (help if stuck >2 hours)

### Phase 9: Reading Order (1-2 days)
- [ ] Started: _____
- [ ] stage10_reading_order.rs copied
- [ ] Tests added
- [ ] Commit # 9 created

**Watch for:**
- Spatial graph implementation questions
- Topological sort issues

### Phase 10: Orchestration (2-3 days)
- [ ] Started: _____
- [ ] executor.rs copied
- [ ] process_page() function working
- [ ] Stage sequencing correct
- [ ] Orchestrator tests passing (3 tests, 26 pages)
- [ ] Commit # 10 created

**Watch for:**
- Pipeline sequencing errors
- Memory management issues
- Performance concerns

### Phase 11: Export & Serialization (2-3 days)
- [ ] Started: _____
- [ ] DocItem conversion implemented
- [ ] Markdown export working
- [ ] JSON export working
- [ ] Comprehensive tests passing (21 tests)
- [ ] Commit # 11 created

**Watch for:**
- Type conversion complexity
- Serializer integration issues

### Phase 12: Integration (2-3 days) ⚠️ CRITICAL PHASE
- [ ] Started: _____
- [ ] Simple backend code DELETED from pdf.rs
- [ ] ML pipeline wired into pdf.rs
- [ ] Cargo.toml updated with docling-pdf-ml dependency
- [ ] End-to-end test working
- [ ] Canonical PDF tests passing
- [ ] Commit # 12 created with "INTEGRATION COMPLETE" in message

**CRITICAL CHECKS:**
- [ ] ~1,000 lines deleted from pdf.rs (heuristics removed)
- [ ] ~200 lines added to pdf.rs (ML integration)
- [ ] No fallback code remaining
- [ ] content_blocks always Some(doc_items), never None
- [ ] PDF parsing actually uses ML (not simple backend)

**Watch for:**
- Worker hesitating to delete code (encourage them)
- Worker creating fallback logic (stop this - no fallback!)
- Integration bugs (help debug immediately)

### Phase 13: Testing (3-4 days)
- [ ] Started: _____
- [ ] 165 unit tests ported from source
- [ ] 3 orchestrator tests ported
- [ ] 21 comprehensive tests ported
- [ ] 18 canonical PDF tests passing
- [ ] Total: 207/207 tests passing (100%)
- [ ] Commit # 13 created

**CRITICAL CHECKS:**
- [ ] Test pass rate: 207/207 (100%)
- [ ] Zero test failures
- [ ] Zero test skips/ignores

**Watch for:**
- Test failures (help debug immediately)
- Worker skipping failing tests (stop this)
- Worker saying "good enough" at <100% (not acceptable)

### Phase 14: Documentation (2-3 days)
- [ ] Started: _____
- [ ] Architecture diagram created
- [ ] Usage examples added
- [ ] Performance benchmarks documented
- [ ] README.md complete in docling-pdf-ml/
- [ ] CLAUDE.md updated (PDF marked complete)
- [ ] Commit # 14 created

**Final Checks:**
- [ ] All 14 phases complete
- [ ] 207/207 tests passing
- [ ] Zero warnings
- [ ] Zero compilation errors
- [ ] Documentation complete

---

## Intervention Triggers

### 🟢 GREEN - Continue Monitoring
- Worker making steady progress (1 phase per 2-3 days)
- Clean commits after each phase
- Tests passing
- Following plan

### 🟡 YELLOW - Increase Monitoring
- Worker stuck >4 hours on same issue
- Compilation errors accumulating
- Test failures increasing
- Timeline slipping >50%

**Action:** Offer guidance, check for blockers

### 🔴 RED - Immediate Intervention
- Worker deviating from plan (refactoring, "improving" code)
- Worker creating partial PR before Phase 14
- Test pass rate <100% and worker continuing
- Phase 12 skipped or incomplete (simple backend not deleted)
- Worker switching to other work

**Action:** Direct intervention, redirect to plan

---

## Communication Protocol

### Worker Commit Format (Expected)
```
# N: PDF ML Phase X - [One-line summary]

**Current Plan**: PDF ML Migration (Phases 8-14, N days remaining)
**Checklist**: Phase X/14 complete - [Deliverable]

## Changes
[What was copied/implemented]

## Tests
X/Y tests passing

## Next AI
Continue to Phase X+1: [Next phase name]
```

### Manager Reviews
- After each phase commit (quick review)
- If worker stuck >4 hours (guidance)
- After Phase 12 (critical review - integration complete?)
- Before Phase 14 PR (final check)

### User Notifications
- Daily progress summary (if requested)
- Phase 10 complete (50% of remaining work)
- Phase 12 complete (integration done! 80%)
- Phase 14 complete (100% - ready for PR)
- Any blockers (immediate)

---

## Expected Timeline

**Start:** 2025-11-23 14:30 PT
**Phase 8-9:** 3-5 days (by Nov 26-28)
**Phase 10-11:** 4-6 days (by Nov 30-Dec 4)
**Phase 12:** 2-3 days (by Dec 2-7) ⚠️ CRITICAL
**Phase 13-14:** 5-7 days (by Dec 7-14)

**Target completion:** Dec 7-14, 2025 (14-21 days from now)

---

## Success Criteria (ALL Required)

Before declaring "COMPLETE":
- [ ] All 14 phases committed (commits # 8 through # 14)
- [ ] 207/207 tests passing (100%)
- [ ] Simple backend code DELETED (~1,000 lines removed)
- [ ] ML pipeline integrated into pdf.rs (~200 lines added)
- [ ] PDF generates DocItems (content_blocks: Some)
- [ ] Zero compilation errors
- [ ] Zero warnings
- [ ] Complete documentation
- [ ] ONE comprehensive PR created with ALL work

**Not before. Not with partial work.**

---

## Current Status

**Worker:** Restarted 2025-11-23 14:30 PT
**Phase:** Should be reading directive, installing libtorch
**Next expected:** Commit # 8 (Phase 8 - assembly pipeline)
**Manager:** Monitoring actively

**Waiting for:** First sign of activity (libtorch install or Phase 8 commit)

---

## Manager Actions

**Now:**
- ✅ Monitoring checklist created
- ✅ Todo list tracking active
- ✅ Ready to help with blockers

**Next:**
- ⏳ Wait for worker activity
- ⏳ Review commit # 8 when it appears
- ⏳ Offer help if stuck >4 hours

**Ongoing:**
- Monitor commits daily
- Watch for deviations
- Provide guidance as needed
- Keep user informed

---

**Generated by:** Manager AI
**Purpose:** Active monitoring and intervention planning
**Status:** Monitoring active, worker should be starting Phase 8
