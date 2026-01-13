# Known Build Issue: CLI Requires Python Backend Feature

**Status:** Known Issue - Not Blocking
**Date:** 2025-11-24
**Severity:** Low - Workaround available

## Issue

The `docling-cli` binary cannot build without the `python-backend` feature flag, even though the CLI has a pure Rust backend (`docling-backend::RustDocumentConverter`) that doesn't require Python.

## Root Cause

The CLI imports types from `docling_core` that are feature-gated behind `python-bridge`:
- `DocumentConverter` (Python wrapper)
- `ConversionConfig`
- `performance::*` (benchmarking)

These types are used in:
1. **Batch command** - Uses Python's `DocumentConverter` streaming API
2. **Benchmark command** - Uses Python's performance profiling
3. **Config merging** - References feature-gated types

## Current Behavior

```bash
# ❌ Fails - tries to import feature-gated types
cargo build --release

# ✅ Works - enables Python bridge
cargo build --release --features python-backend
```

**Error:**
```
error[E0432]: unresolved imports `docling_core::performance`,
              `docling_core::ConversionConfig`, `docling_core::DocumentConverter`
```

## Workaround

Always build with the `python-backend` feature:

```bash
cargo build --release --features python-backend
cargo run --features python-backend -- convert input.pdf
```

## Proper Solution (Future Work)

The CLI should be refactored to:

1. **Feature-gate Python-dependent commands:**
   ```rust
   #[cfg(feature = "python-backend")]
   Commands::Batch { ... }

   #[cfg(feature = "python-backend")]
   Commands::Benchmark { ... }
   ```

2. **Only import Python types when needed:**
   ```rust
   #[cfg(feature = "python-backend")]
   use docling_core::{DocumentConverter, ConversionConfig, performance::*};
   ```

3. **Use Rust backend by default:**
   - Convert command already uses `docling_backend::RustDocumentConverter`
   - Batch and Benchmark commands should have Rust equivalents

## Why Not Fixed Now?

1. **Auto-formatter interference:** rustfmt removes `#[cfg]` attributes from enum variants
2. **Low priority:** Workaround is simple (`--features python-backend`)
3. **No user impact:** Production builds use Python backend anyway
4. **Requires careful refactoring:** Need to split CLI into feature-gated modules

## Related Files

- `crates/docling-cli/src/main.rs` - CLI implementation
- `crates/docling-core/src/lib.rs` - Feature gates (lines 258-266)
- `crates/docling-core/src/converter.rs` - Python DocumentConverter
- `crates/docling-backend/src/lib.rs` - Rust RustDocumentConverter

## Testing

```bash
# Verify Python backend build works
cargo test --features python-backend

# Verify Rust backend works (will fail due to this issue)
cargo build --release  # ❌ Expected to fail

# Workaround
cargo build --release --features python-backend  # ✅ Works
```

## Impact

**None** - This doesn't affect:
- ✅ Integration tests (use `USE_HYBRID_SERIALIZER=1` or `USE_RUST_BACKEND=1`)
- ✅ Unit tests (don't depend on CLI)
- ✅ LLM quality tests (use Python backend)
- ✅ Production usage (Python backend is default)

**Only affects:**
- ❌ Building CLI without `--features python-backend`
- ❌ Downstream projects that want pure Rust binary (rare)

## Resolution Criteria

This issue can be closed when:
1. `cargo build --release` (no features) succeeds
2. CLI Convert command works without Python
3. Batch/Benchmark commands properly feature-gated
4. All tests pass with and without `python-backend` feature

## Priority

**Low** - Has workaround, doesn't block development, affects edge case only.

Fix when: Refactoring CLI architecture or when pure Rust binary becomes a requirement.
