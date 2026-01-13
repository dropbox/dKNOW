# Bundled Binaries with Auto-Download

**MANAGER Direction (2025-12-29)**

**Priority:** HIGH - Critical for downstream projects (docling_rs, sg) to easily consume pdfium_fast.

## Problem

Current setup is fragile:
1. Users must clone pdfium_fast separately
2. Pre-built binaries in `releases/` are out of sync with code (missing `FPDFText_ExtractAllCells`)
3. Manual copying from `releases/` to `out/Release/` is error-prone
4. Building from source requires Google's depot_tools, gn, ninja - complex toolchain

## Solution

Implement auto-download mechanism in `rust/pdfium-sys/build.rs`:

```rust
// rust/pdfium-sys/build.rs
fn main() {
    let target = env::var("TARGET").unwrap();
    let out_dir = env::var("OUT_DIR").unwrap();

    // Allow user override
    if let Ok(lib_dir) = env::var("PDFIUM_LIB_DIR") {
        println!("cargo:rustc-link-search=native={}", lib_dir);
        return;
    }

    // Auto-download matching release
    let version = env!("CARGO_PKG_VERSION");
    let platform = match target.as_str() {
        t if t.contains("aarch64-apple") => "macos-arm64",
        t if t.contains("x86_64-apple") => "macos-x86_64",
        t if t.contains("x86_64-unknown-linux") => "linux-x86_64",
        t if t.contains("x86_64-pc-windows") => "windows-x86_64",
        _ => panic!("Unsupported platform: {}", target),
    };

    let url = format!(
        "https://github.com/ayates_dbx/pdfium_fast/releases/download/v{}/pdfium-{}.tar.gz",
        version, platform
    );

    download_and_extract(&url, &out_dir);
    println!("cargo:rustc-link-search=native={}", out_dir);
}
```

## Release Structure

```
releases/
└── v2.1.0/                    # Must match Cargo.toml version
    ├── macos-arm64/
    │   ├── libpdfium.dylib
    │   └── libpdfium_render_bridge.dylib
    ├── macos-x86_64/
    │   ├── libpdfium.dylib
    │   └── libpdfium_render_bridge.dylib
    ├── linux-x86_64/
    │   ├── libpdfium.so
    │   └── libpdfium_render_bridge.so
    └── windows-x86_64/
        ├── pdfium.dll
        └── pdfium_render_bridge.dll
```

## Tasks

**Task 1: Update releases/ with current binaries**

Build fresh binaries that include all current APIs (including `FPDFText_ExtractAllCells`):
```bash
# Build pdfium (requires depot_tools)
gn gen out/Release --args='is_debug=false'
ninja -C out/Release pdfium

# Build bridge library
cd rust && cargo build --release -p pdfium-render-bridge

# Copy to releases/
mkdir -p releases/v2.1.0/macos-arm64
cp out/Release/libpdfium.dylib releases/v2.1.0/macos-arm64/
cp rust/target/release/libpdfium_render_bridge.dylib releases/v2.1.0/macos-arm64/
```

**Task 2: Update rust/pdfium-sys/build.rs**

Implement auto-download with fallback:
1. Check `PDFIUM_LIB_DIR` env var (manual override)
2. Check `releases/` directory in repo (for development)
3. Download from GitHub releases (for cargo install)

**Task 3: Create GitHub release workflow**

```yaml
# .github/workflows/release.yml
name: Release
on:
  push:
    tags: ['v*']
jobs:
  build:
    strategy:
      matrix:
        os: [macos-latest, ubuntu-latest, windows-latest]
    # Build and upload artifacts...
```

**Task 4: Update Cargo.toml version**

Sync `rust/pdfium-sys/Cargo.toml` version with release tag.

**Task 5: Test downstream integration**

```bash
# In docling_rs
cargo build --release -p docling-cli --features pdfium-fast-ml
# Should auto-download pdfium binaries and build successfully
```

## Success Criteria

1. `cargo build` in docling_rs works without manual pdfium setup
2. Pre-built binaries include all current APIs
3. Version numbers are in sync (Cargo.toml matches release tag)
4. Override via `PDFIUM_LIB_DIR` still works for custom builds

## References

- Similar pattern: rusqlite, openssl-sys, libz-sys
- Current releases: `~/pdfium_fast/releases/` (v1.6.0, v1.9.0 - outdated)
