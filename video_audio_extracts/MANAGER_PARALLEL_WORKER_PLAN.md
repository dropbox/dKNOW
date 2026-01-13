# MANAGER PLAN: Parallel Worker Division

**Date:** 2025-11-10
**Current:** N=193 (single worker)
**Proposal:** Add Worker B to parallelize remaining work

---

## CURRENT WORKLOAD ANALYSIS

### **What Remains (Single Worker = ~40 commits):**

1. **AI Verification Expansion** (20 commits)
   - 51 tests created
   - ~20 verified so far
   - ~30 remaining to verify
   - Finding and fixing bugs as discovered

2. **Format Conversion Fixes** (5 commits)
   - 34/41 passing (82.9%)
   - 7 failures to fix

3. **Status Table Completion** (3 commits)
   - AI_VERIFICATION_STATUS.md (missing)
   - OFFICIAL_TEST_STATUS.md (missing)

4. **Docker Linux Testing** (15-25 commits)
   - Create Dockerfile
   - Test in Ubuntu container
   - Fix Linux-specific bugs

5. **Matrix Expansion** (5-10 commits)
   - Add TIFF/GIF support
   - Fill remaining gaps to 60%

**Total:** ~40 commits = 40 hours AI time (5 days calendar)

---

## PARALLEL DIVISION STRATEGY

### **Option 1: Vertical Split (By Task Type)**

**Worker A (Current, N=193):**
- Continue AI verification & bug fixes
- Fix format conversion issues
- Complete status tables
- **Focus:** Quality & correctness

**Worker B (New, Branch: linux-testing):**
- Docker Linux testing (entire Phase 3)
- Create Dockerfile.ubuntu
- Run tests in container
- Fix Linux-specific bugs
- **Focus:** Cross-platform

**Benefits:**
- ✅ No conflicts (different codebases/branches)
- ✅ Clear ownership
- ✅ Can merge independently

**Timeline:** ~2-3 days instead of 5 days

---

### **Option 2: Horizontal Split (By Operation)**

**Worker A (N=193):**
- Vision operations verification
- Format conversion
- Status tables

**Worker B (Branch: audio-verification):**
- Audio operations verification
- Audio format conversions
- Audio-specific tests

**Benefits:**
- ✅ Domain separation
- ✅ Parallel verification

**Drawbacks:**
- ⚠️ Merge conflicts in shared files

---

### **Option 3: Sequential Phases (Recommended)**

**Worker A (Continue on main):**
- **Phase 1:** Complete AI verification (N=194-210, ~17 commits, 2 days)
- **Phase 2:** Fix all found bugs
- **Phase 3:** Complete status tables

**Worker B (Parallel, Branch: docker-linux):**
- **Phase 1:** Docker Linux setup (5 commits, 1 day)
- **Phase 2:** Linux testing (10 commits, 1.5 days)
- **Phase 3:** Linux bug fixes (10 commits, 1.5 days)

**Both work simultaneously, merge at completion**

**Timeline:** 3 days (instead of 5 days sequential)

---

## RECOMMENDED DIVISION (Option 3)

### **Worker A (Current, main branch):**
```
Priority: Quality & Verification
Current: N=193

N=194-200: Run full AI verification suite (51 tests)
N=201-205: Fix all bugs found
N=206-210: Create final status tables
N=211-215: Matrix expansion (TIFF/GIF)

Estimated: 22 commits, ~22 hours, 3 days
```

### **Worker B (New, docker-linux branch):**
```
Priority: Cross-Platform
Starting: Branch from main at N=193

N=0-5: Docker setup (Dockerfile.ubuntu, dependencies)
N=6-15: Linux testing (run 647 tests in container)
N=16-25: Fix Linux bugs (path handling, FFmpeg versions, etc.)
N=26-30: Multi-platform CI/CD
N=31: Merge to main

Estimated: 30 commits, ~30 hours, 4 days
```

**Both run in parallel, merge when complete**

---

## HOW TO SPLIT

### **Start Worker B:**

```bash
# Create new branch for Linux work
git checkout -b docker-linux
git push -u origin docker-linux

# Worker B starts here
# Worker A continues on main
```

**Worker B's first task:**
```bash
# N=0 on docker-linux branch
# Create Dockerfile.ubuntu
# Document in DOCKER_LINUX_TESTING.md
```

### **Coordination:**

**Shared resources (no conflict):**
- Tests: Worker A modifies, Worker B runs
- Binaries: Both compile independently
- Docs: Separate files

**Potential conflicts:**
- CLAUDE.md (rare)
- Cargo.toml (rare)

**Merge strategy:**
- Worker A merges frequently to main
- Worker B rebases on main daily
- Final merge when both complete

---

## BENEFITS OF PARALLEL WORK

**Speed:**
- Sequential: 5 days
- Parallel: 3-4 days (40% faster)

**Coverage:**
- Worker A: Quality verification
- Worker B: Platform validation
- Both critical for production

**Risk:**
- Minimal merge conflicts
- Clear ownership
- Independent progress

---

## COMMUNICATION PROTOCOL

**Worker A (Quality focus):**
- Report AI verification results
- Report bugs found and fixed
- Merge to main frequently

**Worker B (Platform focus):**
- Report Linux test results
- Report platform-specific bugs
- Rebase on main daily

**Synchronization points:**
- Daily: Worker B rebases on Worker A's main
- End: Worker B merges to main
- Review: Both workers' work reviewed together

---

## DECISION POINT

**Do you want to split the work?**

**YES → I'll create:**
1. Docker Linux branch
2. Initial DOCKER_LINUX_TESTING.md directive
3. Clear split of responsibilities

**NO → Current worker continues sequentially**

---

**My recommendation: YES - Split the work. Linux testing is critical for Dropbox Dash and can run fully in parallel with AI verification.**

