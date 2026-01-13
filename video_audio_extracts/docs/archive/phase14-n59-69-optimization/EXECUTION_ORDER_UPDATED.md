# Updated Execution Order - Git Hook Priority

**Date**: 2025-10-30
**User Directive**: "do phase 4 as phase 2"
**Reason**: Git hook provides immediate value, should be done early

---

## REVISED EXECUTION ORDER

### Phase 1: Medium Priority (N=59-60, 2-3h)
✅ SAME - Test validation and cleanup

### Phase 2: Git Commit Hook (N=61, 30min) ← MOVED UP
**Quick win - enables automatic validation immediately**

### Phase 3: Low Priority (N=62-64, 4-6h) ← WAS PHASE 2
Audio C FFI, more test media, snapshot testing

### Phase 4: Profiling + Optimization (N=65-69, 6-10h) ← WAS PHASE 3
Profile, identify bottlenecks, optimize

---

## DETAILED PLAN

### Phase 1: Validation (N=59-60)

**N=59**: Full test suite
```bash
cargo test --release --test standard_test_suite -- --ignored --test-threads=1
# Target: 98/98 passing
```

**N=60**: Cleanup (N mod 5)
- Archive obsolete docs
- Update README
- Clean working tree

---

### Phase 2: Git Commit Hook (N=61) ← PRIORITIZED

**File**: `.git/hooks/pre-commit`

```bash
#!/bin/bash
# Pre-commit validation: smoke tests + clippy + fmt

set -e

echo "🔍 Pre-commit validation..."

# Smoke tests (2.8s)
echo "🧪 Smoke tests..."
cargo test --release --test smoke_test -- --ignored --quiet || {
    echo "❌ Smoke tests failed"
    exit 1
}

# Clippy (10s)
echo "🔍 Clippy..."
cargo clippy --release --quiet 2>&1 | grep -E "warning|error" && {
    echo "❌ Clippy warnings"
    exit 1
}

# Formatting (1s)
echo "📝 Formatting..."
cargo fmt -- --check || {
    echo "❌ Run: cargo fmt"
    exit 1
}

echo "✅ All checks passed"
```

**Commands:**
```bash
chmod +x .git/hooks/pre-commit

# Test it
echo "test" >> README.md
git add README.md
git commit -m "test: verify hook"
# Should run smoke tests automatically
```

**Estimated**: 30 minutes

**Value**: Immediate - prevents bad commits from this point forward

---

### Phase 3: Low Priority (N=62-64)

**N=62**: Audio extraction C FFI (3-4h)
- Implement audio decode + resample in C FFI
- Replace 4 remaining spawns
- Complete "no spawning" mandate

**N=63**: Snapshot testing (1-2h)
- Implement output capture
- Add baseline comparison
- Enable change detection

**N=64**: More test media (optional, 2-3h)
- Find/generate 20-30 additional files
- Long duration videos
- More codec variety

---

### Phase 4: Profiling + Optimization (N=65-69)

**N=65**: Profile (2-3h)
```bash
# Flamegraph
cargo flamegraph --test smoke_test -- --ignored

# Memory
cargo instruments -t Allocations

# Detailed benchmarks
hyperfine --runs 10 './target/release/video-extract fast --op keyframes video.mp4'
```

**N=66**: Identify bottlenecks (1h)
- Analyze profiles
- Rank optimization opportunities
- Choose top 2-3

**N=67-69**: Implement optimizations (3-6h)
- Optimize highest impact items
- Benchmark improvements
- Validate no regressions

---

## WHY THIS ORDER IS BETTER

**Old order:**
1. Test validation
2. Audio C FFI (4-6h)
3. Profiling (6-10h)
4. Git hook (30min)

**New order:**
1. Test validation
2. Git hook (30min) ← QUICK WIN
3. Audio C FFI (4-6h)
4. Profiling (6-10h)

**Benefits:**
- ✅ Git hook active sooner (protects all future work)
- ✅ Quick win builds momentum
- ✅ Validation infrastructure in place before heavy optimization work
- ✅ All commits after N=61 are protected by hook

---

## WORKER N=61 INSTRUCTIONS

**After Phase 1 complete (N=59-60):**

1. Create/update .git/hooks/pre-commit script
2. Add smoke test execution
3. Add clippy check
4. Add fmt check
5. Make executable (chmod +x)
6. Test hook works
7. Commit with verification

**Estimated**: 30 minutes
**Priority**: HIGH - quick value

**Then continue to Phase 3 (audio C FFI) and Phase 4 (profiling).**

---

## SUMMARY

**Phase 2 moved up** (git hook before audio C FFI) because:
- 30 minutes vs 4-6 hours
- Immediate protective value
- Enables safe iteration on remaining work
- User priority: "do phase 4 as phase 2"

**Worker has clear execution order in EXECUTION_ORDER_UPDATED.md**
