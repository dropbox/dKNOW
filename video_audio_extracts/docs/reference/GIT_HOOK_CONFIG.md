# Git Pre-Commit Hook Configuration

**File**: `.git/hooks/pre-commit` (not tracked in git, local to repository)
**Status**: Active and validated (N=102)
**Last updated**: 2025-10-31

## What It Does

Automatically runs before every commit to ensure code quality and prevent regressions:

1. **Comprehensive Smoke Tests** (43 tests, ~40-60s) - ALWAYS RUN
   - Validates core functionality (keyframes, audio, transcription, plugins, edge cases)
   - Command: `VIDEO_EXTRACT_THREADS=4 cargo test --release --test smoke_test_comprehensive -- --ignored --test-threads=1 --quiet`
   - Uses thread limiting (VIDEO_EXTRACT_THREADS=4) to prevent system overload
   - Blocks commit if any test fails

2. **Rust Linting** (if .rs files changed)
   - `cargo fmt --check` - ensures code is formatted
   - `cargo clippy` - catches common mistakes and anti-patterns

3. **Python Linting** (if .py files changed)
   - `black --check` - Python formatting
   - `flake8` - Python linting

4. **C++ Linting** (if .cpp/.hpp files changed)
   - `clang-format` - C++ formatting

## Why Smoke Tests Are Always Run

Even if you're only changing documentation or non-code files, smoke tests run to ensure:
- The build is still clean
- Core functionality hasn't regressed
- Any dependencies or system state issues are caught early

This adds ~3 seconds to every commit but provides immediate feedback if something breaks.

## Bypassing the Hook

If you need to commit without running checks (not recommended):

```bash
git commit --no-verify -m "your message"
```

Only use `--no-verify` for:
- Work-in-progress commits on branches
- Emergency hotfixes (but fix tests immediately after)
- Documentation-only changes that you've verified don't break tests

## Performance

- Comprehensive smoke tests: ~40-60s (43 tests with thread limiting)
- Clippy: ~10s (if Rust files changed)
- Formatting: ~1s
- Total: ~40-70s depending on what's changed

**Note**: Thread limiting (`VIDEO_EXTRACT_THREADS=4`) prevents system overload. Without it, tests spawn 32-48 threads per process on 16-core systems, causing crashes. See TEST_THREAD_LIMITING.md for details.

## Maintenance

The hook file (`.git/hooks/pre-commit`) is executable and runs automatically. It's not tracked by git, so:

1. New clones of the repository will NOT have this hook active
2. If you pull changes that update CLAUDE.md requirements, you may need to manually update your local hook
3. To share hook updates, document them here and instruct team members to update manually

## Installation for New Clones

If someone clones this repository and wants the hook:

```bash
# The hook file should already exist in .git/hooks/pre-commit
# Just ensure it's executable:
chmod +x .git/hooks/pre-commit

# Test it works:
git commit --allow-empty -m "test: verify hook"
# Should run smoke tests and other checks
```

## History

- **N=102** (2025-10-31): Added VIDEO_EXTRACT_THREADS=4 to prevent thread oversubscription and system crashes
- **N=101** (2025-10-31): Upgraded to comprehensive smoke tests (43 tests)
- **N=62** (2025-10-31): Added smoke tests to existing hook (always run, regardless of file changes)
- **Pre-N=60**: Hook existed with Rust/Python/C++ linting only

## Thread Limiting (N=102)

The hook now uses `VIDEO_EXTRACT_THREADS=4` to limit thread pool sizes:

**Why**: Each test spawns a video-extract binary that creates:
- Rayon thread pool: 16 threads (all CPU cores)
- ONNX Runtime thread pool: 16 threads (all physical cores)
- FFmpeg threads: variable

Without limiting, this creates 32-48+ threads per test, overwhelming the system on high-core-count machines.

**Solution**: Set VIDEO_EXTRACT_THREADS=4 to limit thread pools to 4 threads each (8-12 total per test).

See TEST_THREAD_LIMITING.md for complete documentation.
