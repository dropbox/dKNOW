# How to Build PDFium Fast

This document explains how to build pdfium_fast from source using the Chromium build system.

---

## Quick Start

**Prerequisites:**
1. Install depot_tools (required for all builds)
2. Install Rust from https://rustup.rs
3. macOS: Install full Xcode (not just Command Line Tools)

**Then run:**
```bash
./setup.sh
```

This automated script handles everything: downloads dependencies, builds C++ components (pdfium_cli), and builds Rust tools (render_pages, extract_text). The sections below explain what it does and how to build manually.

---

## Understanding the Build System

PDFium Fast uses the **Chromium build system**, which consists of:

- **depot_tools**: Suite of tools including `gclient`, `gn`, `ninja`
- **gclient**: Downloads dependencies (~7.2GB)
- **gn**: Generates build files
- **ninja**: Compiles the code

This is the standard way Chromium and Chromium-based projects are built.

---

## Prerequisites

### 1. System Requirements

- **Operating System**: macOS or Linux
- **Disk Space**: 10GB free
  - 7.2GB for dependencies (third_party/, build/, buildtools/)
  - 2GB for build artifacts (out/Release/)
- **Time**: 60-90 minutes for first build
  - 30-60 min: Download dependencies
  - 20-40 min: Compilation

### 2. Install depot_tools

depot_tools contains the build tools (gclient, gn, ninja).

```bash
# Clone depot_tools
git clone https://chromium.googlesource.com/chromium/tools/depot_tools.git

# Add to PATH (add this to ~/.bashrc or ~/.zshrc for persistence)
export PATH="$HOME/depot_tools:$PATH"

# Verify installation
gn --version
ninja --version
gclient --version
```

### 2b. Install Rust

Rust is required for building the Rust-based tools.

```bash
# Install Rust (choose default installation)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Restart shell or source the environment
source "$HOME/.cargo/env"

# Verify installation
cargo --version
rustc --version
```

### 3. System Dependencies

**macOS:**
- Full Xcode (not just Command Line Tools) from the App Store
- Configure Xcode as the active developer directory:
  ```bash
  sudo xcode-select --switch /Applications/Xcode.app/Contents/Developer
  xcodebuild -version  # Verify Xcode is active
  ```

**Linux:**
- Build essentials: `apt-get install build-essential` (Ubuntu/Debian)
- Additional libraries may be needed depending on distribution

---

## Build Instructions

### Method 1: Automated (Recommended)

```bash
# Clone repository
git clone https://github.com/dropbox/dKNOW/pdfium_fast.git
cd pdfium_fast

# Run setup script
./setup.sh
```

The script will:
1. Check for depot_tools
2. Create `.gclient` configuration
3. Download 7.2GB dependencies
4. Configure build
5. Compile pdfium_cli

**Skip to "After Building" section below.**

---

### Method 2: Manual Build

For developers who want full control:

#### Step 1: Clone Repository

```bash
git clone https://github.com/dropbox/dKNOW/pdfium_fast.git
```

#### Step 2: Create Workspace Configuration

The Chromium build system requires a `.gclient` file in the **parent directory** of the repository.

**Directory structure:**
```
workspace/                    <- You are here
  .gclient                    <- Configuration file (create this)
  pdfium_fast/                <- The repository
    BUILD.gn
    core/
    examples/
    ...
```

Create the `.gclient` file:

```bash
cd ..  # Go to parent directory of pdfium_fast/

cat > .gclient << 'EOF'
solutions = [
  {
    "name": "pdfium_fast",
    "url": "https://github.com/dropbox/dKNOW/pdfium_fast.git",
    "managed": False,
  },
]
EOF
```

**Note:** `"managed": False` means the repo is already cloned. Set to `True` if you want gclient to clone it.

#### Step 3: Download Dependencies

```bash
# From the workspace/ directory (where .gclient is)
gclient sync
```

This downloads:
- `build/` - Chromium build system
- `buildtools/` - Build tools (gn binaries, clang)
- `third_party/` - Libraries (~5GB): freetype, icu, libjpeg, libpng, zlib, skia, etc.
- `v8/` - JavaScript engine (optional, can skip with custom_vars)

**This takes 30-60 minutes depending on network speed.**

#### Step 4: Configure Build

```bash
cd pdfium_fast/

gn gen out/Release --args='is_debug=false pdf_enable_v8=false pdf_enable_xfa=false'
```

**Build arguments:**
- `is_debug=false` - Release build (optimized)
- `pdf_enable_v8=false` - Disable JavaScript (smaller, faster)
- `pdf_enable_xfa=false` - Disable XFA forms (not needed)

**For debug builds:**
```bash
gn gen out/Debug --args='is_debug=true'
```

#### Step 5: Build

```bash
# Build C++ components
ninja -C out/Release pdfium_cli pdfium_render_bridge

# Build Rust components
cd rust/
cargo build --release --example render_pages --example extract_text --example extract_text_jsonl
cd ..
```

**This takes 20-40 minutes for first build.** Incremental rebuilds are much faster (1-5 minutes).

---

## After Building

### Verify Build

```bash
./out/Release/pdfium_cli --help
```

Should print usage information.

### Test the Binary

```bash
# Extract text from a PDF
./out/Release/pdfium_cli extract-text input.pdf output.txt

# Render pages to images
./out/Release/pdfium_cli render-pages input.pdf output_dir/
```

---

## Running Tests

### 1. Install Python Dependencies

```bash
cd integration_tests/
pip install -r requirements.txt
```

### 2. Get Test PDFs

Test PDFs (1.4GB compressed) are available in GitHub Releases.

**Quick download:**
```bash
cd integration_tests/
python3 download_test_pdfs.py
```

**Manual download:**
1. Visit: https://github.com/dropbox/dKNOW/pdfium_fast/releases
2. Download `pdfium_test_pdfs.tar.gz` from the `test-pdfs-v1` release
3. Extract in `integration_tests/`:
   ```bash
   tar xzf pdfium_test_pdfs.tar.gz
   ```

See `integration_tests/DOWNLOAD_TEST_PDFS.md` for full instructions and troubleshooting.

### 3. Run Tests

```bash
# Quick smoke tests (requires a few test PDFs)
pytest -m smoke

# Full test suite (requires complete test corpus)
pytest -m extended
```

---

## Build Targets

You can build different targets with `ninja -C out/Release <target>`:

| Target | Description |
|--------|-------------|
| `pdfium_cli` | Command-line interface (main tool) |
| `pdfium` | PDFium library only |
| `pdfium_test` | Upstream PDFium test tool |
| `pdfium_embeddertests` | Unit tests |
| `pdfium_unittests` | More unit tests |

---

## Build Configurations

### Release Build (Default)

```bash
gn gen out/Release --args='is_debug=false pdf_enable_v8=false pdf_enable_xfa=false'
```

- Optimized for performance
- No debug symbols
- Smaller binary size

### Debug Build

```bash
gn gen out/Debug --args='is_debug=true'
```

- Debug symbols included
- No optimizations
- Larger binary, easier to debug

### Profile Build

```bash
gn gen out/Profile --args='is_debug=false symbol_level=1 enable_dsyms=true'
```

- Optimized with debug symbols
- Good for profiling

---

## Troubleshooting

### "gn: command not found"

**Cause:** depot_tools not in PATH

**Fix:**
```bash
export PATH="$HOME/depot_tools:$PATH"
```

Add to `~/.bashrc` or `~/.zshrc` for persistence.

---

### "ninja: error: loading 'build.ninja': No such file or directory"

**Cause:** Build not configured

**Fix:**
```bash
gn gen out/Release
```

---

### "ninja: error: unknown target 'pdfium_cli'"

**Cause:** Dependencies not downloaded

**Fix:**
```bash
# Go to parent directory (where .gclient is)
cd ..
gclient sync

# Then try building again
cd pdfium_fast/
ninja -C out/Release pdfium_cli
```

---

### "No module named 'pytest'"

**Cause:** Python dependencies not installed

**Fix:**
```bash
pip install -r integration_tests/requirements.txt
```

---

### Build fails with Xcode errors (macOS)

**Error:** `xcode-select: error: tool 'xcodebuild' requires Xcode`

**Cause:** Command Line Tools installed but full Xcode required

**Fix:**
1. Install full Xcode from App Store
2. Configure xcode-select:
```bash
sudo xcode-select --switch /Applications/Xcode.app/Contents/Developer
xcodebuild -version  # Should show Xcode version, not error
```

---

### gclient sync fails with network errors

**Cause:** Timeout or network issues

**Fix:**
- Try again (sometimes servers are slow)
- Use a faster/more stable network
- Update depot_tools: `cd ~/depot_tools && git pull`

---

### Build fails with "missing required argument"

**Cause:** GN args syntax error

**Fix:**
Make sure args are in single string:
```bash
gn gen out/Release --args='is_debug=false pdf_enable_v8=false'
```

---

## Clean Builds

If your build is corrupted or you want to start fresh:

```bash
# Clean build artifacts
rm -rf out/

# Reconfigure and rebuild
gn gen out/Release --args='is_debug=false pdf_enable_v8=false pdf_enable_xfa=false'
ninja -C out/Release pdfium_cli
```

To completely reset dependencies:

```bash
# Go to workspace directory
cd ..

# Clean everything
rm -rf build/ buildtools/ third_party/ v8/

# Re-download
gclient sync
```

---

## Advanced: Minimal Checkout

To save disk space, you can use a minimal checkout (skip V8 and Skia):

Edit `.gclient`:
```python
solutions = [
  {
    "name": "pdfium_fast",
    "url": "https://github.com/dropbox/dKNOW/pdfium_fast.git",
    "managed": False,
    "custom_vars": {
      "checkout_configuration": "minimal",
    },
  },
]
```

Then run: `gclient sync`

**Note:** Some features may not work with minimal checkout.

---

## Build Performance Tips

1. **Use more CPU cores:**
   ```bash
   ninja -C out/Release -j16 pdfium_cli  # Use 16 cores
   ```

2. **Use ccache (Linux):**
   ```bash
   export CCACHE_DIR=$HOME/.ccache
   gn gen out/Release --args='cc_wrapper="ccache"'
   ```

3. **Incremental builds:**
   After first build, only modified files recompile (1-5 min)

---

## Platform-Specific Notes

### macOS

- Requires macOS 10.15 or later
- Apple Silicon (M1/M2) and Intel both supported
- Full Xcode required (not just Command Line Tools)
- Must configure xcode-select to point to Xcode.app

### Linux

- Tested on Ubuntu 20.04+
- May work on other distributions with adjustments
- Requires standard build tools

### Windows

- Not currently tested/supported
- Should work with Visual Studio 2019+
- Requires Windows SDK

---

## Next Steps

After building:

1. **Test your build:** `./out/Release/pdfium_cli --help`
2. **Try examples:** Extract text or render pages
3. **Run tests:** See integration_tests/README.md
4. **Read documentation:** See USAGE.md for API details

---

## Getting Help

- **Build issues:** Check "Troubleshooting" section above
- **Test issues:** See integration_tests/README.md
- **Repository issues:** https://github.com/dropbox/dKNOW/pdfium_fast/issues

---

## Build System References

- [depot_tools](https://commondatastorage.googleapis.com/chrome-infra-docs/flat/depot_tools/docs/html/depot_tools_tutorial.html)
- [GN Reference](https://gn.googlesource.com/gn/+/master/docs/reference.md)
- [Ninja Build](https://ninja-build.org/)
- [PDFium Upstream](https://pdfium.googlesource.com/pdfium/)
