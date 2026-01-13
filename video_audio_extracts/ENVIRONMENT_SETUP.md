# Environment Setup for Development

**Last Updated:** N=53, 2025-11-07

This document describes the required environment configuration for developing on this codebase.

---

## Required System Dependencies

### 1. Rust Toolchain

The Rust compiler (cargo, rustc) must be installed and available in PATH.

**Installation:**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

**Verification:**
```bash
cargo --version
rustc --version
```

### 2. GNU Coreutils (macOS only)

macOS requires GNU coreutils for the `gtimeout` command used in file validation.

**Installation:**
```bash
brew install coreutils
```

**Verification:**
```bash
gtimeout --version
```

### 3. FFmpeg Libraries

FFmpeg development libraries must be installed with pkg-config files.

**Installation:**
```bash
brew install ffmpeg
```

**Verification:**
```bash
pkg-config --modversion libavcodec libavformat libavutil dav1d
```

---

## Environment Variables

### Required for All Operations

Set these environment variables in your shell or before running cargo commands:

```bash
# Add Rust toolchain and Homebrew to PATH (macOS)
export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:$PATH"

# Add FFmpeg and dav1d pkg-config files
export PKG_CONFIG_PATH="/opt/homebrew/lib/pkgconfig:/opt/homebrew/opt/ffmpeg/lib/pkgconfig"
```

**Note**: /opt/homebrew/bin is required on macOS for the `gtimeout` command (used for file validation). On most systems this is already in PATH by default, but it must be explicitly set when running tests in non-interactive shells.

### Required for Testing

Limit thread count to prevent system overload during tests:

```bash
# Limit video-extract binary thread pools (Rayon, ONNX Runtime, FFmpeg decoders)
export VIDEO_EXTRACT_THREADS=4
```

**Why this is needed:** Each test spawns a video-extract binary that creates multiple thread pools. Without limiting, high-core-count systems can be overwhelmed (32-48+ threads per test). Production workloads should NOT set this variable to maximize performance. See TEST_THREAD_LIMITING.md for details.

---

## Shell Configuration

### Persistent Setup (Recommended)

Add to your shell profile (`~/.zshrc`, `~/.bashrc`, etc.):

```bash
# Rust development (macOS)
export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:$PATH"
export PKG_CONFIG_PATH="/opt/homebrew/lib/pkgconfig:/opt/homebrew/opt/ffmpeg/lib/pkgconfig"
```

Then reload: `source ~/.zshrc` (or restart terminal)

### Per-Session Setup

If you prefer not to modify shell config, export variables each session:

```bash
export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:$PATH"
export PKG_CONFIG_PATH="/opt/homebrew/lib/pkgconfig:/opt/homebrew/opt/ffmpeg/lib/pkgconfig"
```

---

## Pre-Commit Hook Configuration

The pre-commit hook at `.git/hooks/pre-commit` has been updated to export these environment variables automatically (N=53, N=141, N=144). This ensures tests run correctly during `git commit`.

**Hook contents:**
```bash
#!/bin/bash
set -e

# Set up environment for Rust and dependencies
export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:$PATH"
export PKG_CONFIG_PATH="/opt/homebrew/lib/pkgconfig:/opt/homebrew/opt/ffmpeg/lib/pkgconfig"

# Run comprehensive smoke tests
VIDEO_EXTRACT_THREADS=4 cargo test --release --test smoke_test_comprehensive -- --ignored --test-threads=1 --quiet
```

---

## Common Issues

### "cargo not found"

**Symptom:** `command not found: cargo`

**Cause:** Rust toolchain not installed OR not in PATH

**Solution:**
1. Check if installed: `ls ~/.cargo/bin/cargo`
2. If exists: Export PATH (see above)
3. If missing: Install Rust toolchain (see above)

### "Failed to run validation command: No such file or directory (os error 2)"

**Symptom:** Tests fail with validation command error

**Cause:** Missing timeout command (Linux) or gtimeout (macOS)

**Solution:**
- macOS: `brew install coreutils`
- Linux: `sudo apt-get install coreutils` (usually pre-installed)

### "pkg-config did not find dav1d"

**Symptom:** Cargo build fails looking for dav1d.pc

**Cause:** PKG_CONFIG_PATH not set or FFmpeg not installed

**Solution:**
1. Install FFmpeg: `brew install ffmpeg`
2. Export PKG_CONFIG_PATH (see above)
3. Verify: `pkg-config --exists dav1d && echo "Found"`

---

## Verification Checklist

After setting up environment, verify everything works:

```bash
# 1. Check Rust toolchain
cargo --version && rustc --version

# 2. Check GNU coreutils (macOS)
gtimeout --version

# 3. Check FFmpeg libraries
pkg-config --modversion libavcodec libavformat libavutil dav1d

# 4. Build project
cargo build --release

# 5. Run tests
VIDEO_EXTRACT_THREADS=4 cargo test --release --test smoke_test_comprehensive -- --ignored --test-threads=1
```

**Expected results:**
- Build completes successfully
- 647/647 tests pass (100% pass rate)
- Runtime: ~415 seconds

---

## History

**N=52:** Identified cargo PATH issue after 23 iterations of blocked development
**N=53:** Fixed timeout command cross-platform compatibility, documented environment setup
