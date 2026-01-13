# Subprocess PATH Inheritance Fix Plan

**Created:** N=31 (2025-11-07)
**Status:** Ready to execute when cargo becomes available
**Blocker:** Requires cargo to rebuild binary

---

## EXECUTIVE SUMMARY

The binary execution blocker (N=29, N=30) is caused by `Command::new()` calls that spawn subprocess commands (ffmpeg, ffprobe, timeout) without inheriting the parent process PATH environment. This document provides a complete fix plan with all affected locations and exact code changes needed.

**Impact:** Fixes binary execution for 50-70 AI commits of blocked work

---

## ROOT CAUSE

Rust's `std::process::Command::new()` uses a minimal environment by default and does not automatically inherit PATH from the parent process. When the binary spawns subprocesses like `ffprobe` or `ffmpeg`, these commands cannot be found even if they exist in the user's shell PATH.

**Current behavior:**
```rust
Command::new("ffprobe")  // Fails: subprocess doesn't know where to find ffprobe
```

**Required behavior:**
```rust
Command::new("ffprobe")
    .env("PATH", std::env::var("PATH").unwrap_or_default())  // Now subprocess can find ffprobe
```

---

## ALL AFFECTED LOCATIONS (8 total)

### 1. metadata-extraction/src/lib.rs:164
**Command:** `ffprobe`
**Current code:**
```rust
let output = Command::new("ffprobe")
    .args([
        "-v", "quiet",
        "-print_format", "json",
        "-show_format",
        "-show_streams",
        path.to_str().unwrap(),
    ])
    .output()
    .map_err(|e| {
        MediaError::ProcessingFailed(format!("Failed to run validation command: {}", e))
    })?;
```

**Fixed code:**
```rust
let output = Command::new("ffprobe")
    .env("PATH", std::env::var("PATH").unwrap_or_default())  // ADD THIS LINE
    .args([
        "-v", "quiet",
        "-print_format", "json",
        "-show_format",
        "-show_streams",
        path.to_str().unwrap(),
    ])
    .output()
    .map_err(|e| {
        MediaError::ProcessingFailed(format!("Failed to run validation command: {}", e))
    })?;
```

---

### 2. keyframe-extractor/src/lib.rs:131
**Command:** `ffmpeg`
**Current code:**
```rust
let output = Command::new("ffmpeg")
    .args(&args)
    .output()
    .map_err(|e| MediaError::ProcessingFailed(format!("FFmpeg execution failed: {}", e)))?;
```

**Fixed code:**
```rust
let output = Command::new("ffmpeg")
    .env("PATH", std::env::var("PATH").unwrap_or_default())  // ADD THIS LINE
    .args(&args)
    .output()
    .map_err(|e| MediaError::ProcessingFailed(format!("FFmpeg execution failed: {}", e)))?;
```

---

### 3. audio-extractor/src/lib.rs:174
**Command:** `ffmpeg`
**Current code:**
```rust
let mut cmd = Command::new("ffmpeg");
cmd.args(&args);

let output = cmd.output().map_err(|e| {
    MediaError::ProcessingFailed(format!("FFmpeg execution failed: {}", e))
})?;
```

**Fixed code:**
```rust
let mut cmd = Command::new("ffmpeg");
cmd.env("PATH", std::env::var("PATH").unwrap_or_default());  // ADD THIS LINE
cmd.args(&args);

let output = cmd.output().map_err(|e| {
    MediaError::ProcessingFailed(format!("FFmpeg execution failed: {}", e))
})?;
```

---

### 4. format-conversion/src/lib.rs:372
**Command:** `ffmpeg`
**Current code:**
```rust
let mut cmd = Command::new("ffmpeg");
cmd.args(&args);

let output = cmd.output().map_err(|e| {
    MediaError::ProcessingFailed(format!("FFmpeg execution failed: {}", e))
})?;
```

**Fixed code:**
```rust
let mut cmd = Command::new("ffmpeg");
cmd.env("PATH", std::env::var("PATH").unwrap_or_default());  // ADD THIS LINE
cmd.args(&args);

let output = cmd.output().map_err(|e| {
    MediaError::ProcessingFailed(format!("FFmpeg execution failed: {}", e))
})?;
```

---

### 5. scene-detector/src/lib.rs:195
**Command:** `ffmpeg`
**Current code:**
```rust
let mut cmd = Command::new("ffmpeg");
cmd.args(&args);

let output = cmd.output().map_err(|e| {
    MediaError::ProcessingFailed(format!("FFmpeg execution failed: {}", e))
})?;
```

**Fixed code:**
```rust
let mut cmd = Command::new("ffmpeg");
cmd.env("PATH", std::env::var("PATH").unwrap_or_default());  // ADD THIS LINE
cmd.args(&args);

let output = cmd.output().map_err(|e| {
    MediaError::ProcessingFailed(format!("FFmpeg execution failed: {}", e))
})?;
```

---

### 6. subtitle-extraction/src/lib.rs:222
**Command:** `ffmpeg`
**Current code:**
```rust
let output = Command::new("ffmpeg")
    .args(&args)
    .output()
    .map_err(|e| MediaError::ProcessingFailed(format!("FFmpeg execution failed: {}", e)))?;
```

**Fixed code:**
```rust
let output = Command::new("ffmpeg")
    .env("PATH", std::env::var("PATH").unwrap_or_default())  // ADD THIS LINE
    .args(&args)
    .output()
    .map_err(|e| MediaError::ProcessingFailed(format!("FFmpeg execution failed: {}", e)))?;
```

---

### 7. video-extract-cli/src/commands/debug.rs:485
**Command:** `timeout` (macOS/Linux only)
**Current code:**
```rust
std::process::Command::new("timeout")
    .args([
        timeout_str,
        "cargo",
        "run",
        "--release",
        "--bin",
        "video-extract",
        "--",
        "debug",
        "--ops",
        ops_str,
        file_path,
    ])
    .env("VIDEO_EXTRACT_THREADS", "4")
    .output()
```

**Fixed code:**
```rust
std::process::Command::new("timeout")
    .env("PATH", std::env::var("PATH").unwrap_or_default())  // ADD THIS LINE
    .args([
        timeout_str,
        "cargo",
        "run",
        "--release",
        "--bin",
        "video-extract",
        "--",
        "debug",
        "--ops",
        ops_str,
        file_path,
    ])
    .env("VIDEO_EXTRACT_THREADS", "4")
    .output()
```

**Note:** This command also uses `cargo` which must be in PATH. This is CLI test infrastructure code, not production code.

---

### 8. video-extract-cli/src/commands/fast.rs:380
**Command:** `timeout` (macOS/Linux only)
**Current code:**
```rust
let output = std::process::Command::new("timeout")
    .args([
        timeout_str,
        "cargo",
        "run",
        "--release",
        "--bin",
        "video-extract",
        "--",
        "performance",
        "--ops",
        ops_str,
        &file.to_string_lossy(),
    ])
    .env("VIDEO_EXTRACT_THREADS", "4")
    .output()
```

**Fixed code:**
```rust
let output = std::process::Command::new("timeout")
    .env("PATH", std::env::var("PATH").unwrap_or_default())  // ADD THIS LINE
    .args([
        timeout_str,
        "cargo",
        "run",
        "--release",
        "--bin",
        "video-extract",
        "--",
        "performance",
        "--ops",
        ops_str,
        &file.to_string_lossy(),
    ])
    .env("VIDEO_EXTRACT_THREADS", "4")
    .output()
```

**Note:** This command also uses `cargo` which must be in PATH. This is CLI test infrastructure code, not production code.

---

## PRIORITY OF FIXES

### Critical (Production Code) - Fix First
1. ✅ **metadata-extraction/src/lib.rs:164** - ffprobe (HIGHEST PRIORITY - causes all tests to fail)
2. ✅ **keyframe-extractor/src/lib.rs:131** - ffmpeg
3. ✅ **audio-extractor/src/lib.rs:174** - ffmpeg
4. ✅ **format-conversion/src/lib.rs:372** - ffmpeg
5. ✅ **scene-detector/src/lib.rs:195** - ffmpeg
6. ✅ **subtitle-extraction/src/lib.rs:222** - ffmpeg

### Lower Priority (CLI Test Infrastructure)
7. ⚠️ **video-extract-cli/src/commands/debug.rs:485** - timeout/cargo (CLI test helper)
8. ⚠️ **video-extract-cli/src/commands/fast.rs:380** - timeout/cargo (CLI test helper)

**Recommendation:** Fix all 8 locations in a single commit to ensure complete resolution.

---

## EXECUTION STEPS (When Cargo Available)

### Step 1: Verify Cargo Available
```bash
which cargo
# Expected: /path/to/cargo (not empty)
```

### Step 2: Apply All 8 Fixes
Use the Edit tool to apply each fix above. All fixes are identical in pattern:
```rust
.env("PATH", std::env::var("PATH").unwrap_or_default())
```

### Step 3: Rebuild Binary
```bash
cargo build --release
```

Expected output:
- Compilation should succeed (0 warnings)
- Binary created at `target/release/video-extract`
- Build time: ~5-10 minutes

### Step 4: Verify Fix with Smoke Test
```bash
VIDEO_EXTRACT_THREADS=4 cargo test --release --test smoke_test_comprehensive -- --ignored --test-threads=1
```

**Expected result:**
- 363/363 tests pass (or 450+ if new tests added)
- No "No such file or directory" errors
- Test duration: ~4-5 minutes

### Step 5: Verify Individual Operation
```bash
VIDEO_EXTRACT_THREADS=4 ./target/release/video-extract debug \
  --ops metadata-extraction \
  test_edge_cases/sample.mp4
```

**Expected result:**
- No errors
- metadata.json created with format/stream info
- No "Failed to run validation command" errors

### Step 6: Run Full Test Suite (Optional)
```bash
VIDEO_EXTRACT_THREADS=4 cargo test --release --test standard_test_suite -- --ignored --test-threads=1
```

Expected: All 485 tests pass

---

## ALTERNATIVE FIX (Better Long-Term)

Instead of `.env("PATH", ...)`, use explicit binary paths:

```rust
// Option A: Use which::which() to find binary at runtime
use which::which;

let ffprobe_path = which("ffprobe")
    .unwrap_or_else(|_| PathBuf::from("ffprobe"));
let output = Command::new(&ffprobe_path)
    .args([...])
    .output()?;

// Option B: Use environment variable for binary path
let ffprobe_path = std::env::var("FFPROBE_PATH")
    .unwrap_or_else(|_| "ffprobe".to_string());
let output = Command::new(&ffprobe_path)
    .env("PATH", std::env::var("PATH").unwrap_or_default())
    .args([...])
    .output()?;
```

**Recommendation:** Start with PATH inheritance fix (simple, low-risk). Refactor to explicit paths in future iteration if needed.

---

## VERIFICATION CHECKLIST

After applying fixes and rebuilding:

- [ ] `which cargo` shows cargo is available
- [ ] All 8 Command::new() locations have `.env("PATH", ...)` added
- [ ] `cargo build --release` succeeds with 0 warnings
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Binary executes without "No such file or directory" errors
- [ ] metadata-extraction works: `video-extract debug --ops metadata-extraction test.mp4`
- [ ] keyframes works: `video-extract debug --ops keyframes test.mp4`
- [ ] Smoke tests pass: 363/363 (or 450+ if new tests added)
- [ ] Full test suite passes: 485/485 tests

---

## DEPENDENCIES

**Required for fix:**
- ✅ ffmpeg installed at `/opt/homebrew/bin/ffmpeg` (already installed, N=28)
- ✅ ffprobe installed at `/opt/homebrew/bin/ffprobe` (already installed, N=28)
- ❌ cargo available in PATH (BLOCKER - not yet installed)

**User action required:**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

---

## SUCCESS METRICS

**Before fix:**
- Binary execution: ❌ FAIL (No such file or directory)
- Test suite: ❌ Cannot run (binary blocked)
- Development work: ⏳ BLOCKED (50-70 commits)

**After fix:**
- Binary execution: ✅ SUCCESS
- Test suite: ✅ 363/363 passing (or 450+ with new tests)
- Development work: ✅ UNBLOCKED (can proceed with Beta Phase 3, Production Phase 1)

---

## RELATED DOCUMENTS

- N29_BLOCKER_ANALYSIS.md - Complete root cause analysis
- BETA_RELEASE_PLAN.md - Beta work blocked by this issue (Phases 3-4)
- PRODUCTION_READINESS_PLAN.md - Production work blocked (Phases 1-6)
- PERFORMANCE_BENCHMARK_PLAN_N28.md - Ready to execute after fix
- RUN_STANDARD_TESTS.md - Test framework documentation

---

## NEXT AI INSTRUCTIONS

**If cargo is available when you start your session:**

1. Read this document completely
2. Verify cargo: `which cargo`
3. Apply all 8 fixes using the Edit tool (exact code provided above)
4. Rebuild: `cargo build --release`
5. Test: `VIDEO_EXTRACT_THREADS=4 cargo test --release --test smoke_test_comprehensive -- --ignored --test-threads=1`
6. If tests pass (363/363 or 450+/450+):
   - Commit with message: "# N: Fix subprocess PATH inheritance (8 Command::new() locations)"
   - Proceed to Beta Phase 3 (PERFORMANCE_BENCHMARK_PLAN_N28.md)
7. If tests fail:
   - Read failure output carefully
   - Check if other issues exist (not just PATH)
   - Document findings and continue debugging

**If cargo is still not available:**

1. Verify: `which cargo`
2. Confirm blocker still exists
3. Check if user provided instructions or workarounds
4. If no new information: commit brief status update and conclude session
5. Do not create workarounds or attempt other fixes (will fail)

---

**End of SUBPROCESS_PATH_FIX_PLAN.md**
