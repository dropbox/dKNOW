# Python 3.9 Linking Issue - N=2139

**Date:** 2025-11-24
**Session:** N=2139
**Status:** BLOCKING - Tests fail on packages with pyo3 dependency

---

## Issue Summary

Unit tests fail for `docling-backend`, `docling-core`, and `docling-apple` with Python library linking error:

```
dyld[91958]: Library not loaded: @rpath/Python3.framework/Versions/3.9/Python3
  Referenced from: <...> /Users/ayates/docling_rs/target/debug/deps/docling_backend-...
  Reason: tried: '/opt/homebrew/lib/python3.14/site-packages/torch/lib/Python3.framework/Versions/3.9/Python3' (no such file)
```

---

## Root Cause

1. **pyo3 dependency:** docling-core has `pyo3.workspace = true` (Cargo.toml line 18)
2. **PyTorch compiled against Python 3.9:** PyTorch at `/opt/homebrew/lib/python3.14/site-packages/torch/` was built with Python 3.9
3. **System has Python 3.14:** Current environment uses Python 3.14
4. **Dynamic linker mismatch:** When test binaries load, dyld tries to find Python 3.9 framework but only finds Python 3.14

---

## Affected Packages

- ❌ `docling-backend` - imports docling-core (dev-dependency)
- ❌ `docling-core` - has pyo3 dependency for python_bridge
- ❌ `docling-apple` - imports docling-core

---

## Why This Wasn't An Issue Before

Looking at commit history:
- N=2125 (most recent cleanup): "Unit tests: 3447 (100% pass, 31 ignored)"
- N=2014-2017: Multiple commits mention "3447 unit tests passing (100%)"
- Previous AIs likely had:
  - Python 3.9 installed alongside 3.14, OR
  - Different PyTorch installation built with correct Python version, OR
  - Environment variable configured to find Python 3.9

---

## Workaround Solutions

### Option A: Exclude Python-Dependent Packages (Current)

```bash
$HOME/.cargo/bin/cargo test --workspace --lib \
  --exclude docling-backend \
  --exclude docling-core \
  --exclude docling-apple \
  -- --test-threads=8
```

**Pros:**
- Simple, works immediately
- Other packages (27 total) can be tested

**Cons:**
- Doesn't test critical docling-backend and docling-core
- Loses 3447 → ~500 tests (estimate)

### Option B: Install/Link Python 3.9

```bash
# Install Python 3.9 via pyenv or homebrew
pyenv install 3.9.18
pyenv global 3.9.18

# Reinstall PyTorch with Python 3.9
pip install torch torchvision torchaudio

# OR symlink Python 3.14 → 3.9 (risky, may break PyTorch)
```

**Pros:**
- Full test coverage restored
- Matches PyTorch requirements

**Cons:**
- Requires system reconfiguration
- May break other Python 3.14 dependencies
- PyTorch may need full reinstallation

### Option C: Rebuild PyTorch with Python 3.14

```bash
# Reinstall PyTorch for Python 3.14
pip3.14 install --upgrade --force-reinstall torch torchvision torchaudio
```

**Pros:**
- Aligns with current system Python
- Long-term solution

**Cons:**
- PyTorch compilation takes hours if building from source
- May not be available as pre-built wheel for Python 3.14

### Option D: Make pyo3 Optional Feature

Modify docling-core/Cargo.toml:
```toml
[dependencies]
pyo3 = { workspace = true, optional = true }

[features]
default = []
python-bridge = ["pyo3"]
```

**Pros:**
- Tests run without Python dependency
- python_bridge still available when needed (integration tests)
- Clean architectural separation

**Cons:**
- Requires code refactoring
- python_bridge module needs feature gating
- Integration tests (USE_HYBRID_SERIALIZER=1) still need Python

---

## Recommended Solution

**Use Option D (Make pyo3 optional) + Option A (exclude packages temporarily)**

**Rationale:**
1. Per CLAUDE.md: "Python is ONLY for Testing Infrastructure"
2. python_bridge should be optional, not required dependency
3. Unit tests shouldn't need Python at all
4. Integration tests can enable python-bridge feature explicitly

**Implementation Plan:**
1. Make pyo3 optional feature in docling-core
2. Gate python_bridge module behind feature flag
3. Update integration tests to use `--features python-bridge`
4. Document Python 3.9 requirement for integration tests

---

## Current Status (N=2139)

- ✅ Build succeeds: `cargo build --workspace` (3.36s)
- ✅ Clippy passes: `cargo clippy --workspace --lib -- -D warnings` (0 warnings)
- ❌ Tests fail: `cargo test --workspace --lib` (Python 3.9 linking error)
- ⚠️  Workaround: Testing 27/30 packages (excluding backend, core, apple)

---

## Test Coverage Impact

**Previous (N=2125):** 3447 unit tests (100% pass, 31 ignored)

**Estimated without Python packages:**
- docling-backend: ~200 tests (estimate)
- docling-core: ~600 tests (estimate)
- docling-apple: ~15 tests (estimate)
- **Remaining:** ~2632 tests (76% coverage)

**Note:** Need actual test run to confirm numbers

---

## Next Steps

1. **Document this issue** ✅ (this file)
2. **Run partial tests** to establish baseline (27 packages)
3. **Decide:** Quick fix (Option A) vs Long-term fix (Option D)
4. **Update FORMAT_PROCESSING_GRID.md** with current test status
5. **Commit with clear status** for next AI

---

## References

- CLAUDE.md: "Python is ONLY for Testing Infrastructure"
- pyo3 dependency: Cargo.toml line 82, docling-core/Cargo.toml line 18
- python_bridge: crates/docling-core/src/python_bridge.rs
- Previous test success: N=2125 commit (9afcb543)

---

**Status:** ✅ RESOLVED (Option D Implemented at N=2140)
**Solution:** Made pyo3 optional feature - unit tests now run without Python

**Work Completed (N=2140):**
- ✅ Made pyo3 optional in docling-core/Cargo.toml
- ✅ Added python-bridge feature flag
- ✅ Gated python_bridge module behind feature
- ✅ Gated converter module behind feature (DocumentConverter)
- ✅ Gated performance module behind feature
- ✅ Gated archive module behind feature
- ✅ Gated pyo3::PyErr error conversion
- ✅ Moved ConversionResult to document.rs (always available)
- ✅ Updated docling-cli to enable python-bridge feature
- ✅ Build succeeds without python-bridge
- ✅ Unit tests run without python-bridge (3493 tests passing)

**Test Results:**
- **Without python-bridge:** 3493 unit tests pass (100% success rate)
- **With python-bridge:** Integration tests require Python 3.9 (still blocked by environment issue)

**Integration Tests:**
- Integration tests (test_canon_*) still require Python 3.9 for pyo3
- Run with: `USE_HYBRID_SERIALIZER=1 cargo test --test integration_tests --features python-bridge`
- Environment issue: System has Python 3.14, PyTorch compiled against 3.9
- Solution for integration tests: Install Python 3.9 or rebuild PyTorch for 3.14

**Impact:**
- ✅ **Unit tests unblocked** - can run on any system without Python
- ⚠️ Integration tests still need Python 3.9 environment (expected behavior)
- ✅ Production builds don't require Python (python-bridge is optional)
- ✅ CLI still works (enables python-bridge feature explicitly)
