# PDFium Git Hooks

This directory contains git hooks for PDFium development to ensure code quality and prevent regressions.

## Installation

Run the installation script to set up the hooks:

```bash
./.githooks/install-hooks.sh
```

This copies the hooks from `.githooks/` (tracked by git) to `.git/` (not tracked).

## Hooks

### pre-commit

Runs before every commit to check:

1. **C++ Formatting** - Uses `clang-format` to ensure consistent C++ code style
   - Requires: `clang-format` (`brew install clang-format`)
   - Config: `.clang-format` in repository root
   - Fix with: `clang-format -i <files>`

2. **Python Linting** - Uses `ruff` (preferred) or `flake8` for Python code quality
   - Requires: `ruff` (`pip install ruff`) or `flake8`
   - Checks for syntax errors, import issues, and style violations
   - Fix with: `ruff check --fix`

3. **Rust Linting** - Uses `rustfmt` for formatting and `clippy` for linting
   - Requires: `rustfmt` and `clippy` (`rustup component add rustfmt clippy`)
   - Checks for formatting issues and code quality warnings
   - Fix with: `rustfmt <files>` and address clippy warnings

4. **PDFium Unit Tests** - Runs `pdfium_unittests` if built
   - Requires: Build PDFium with tests enabled
   - Location: `out/Test-*/pdfium_unittests`
   - Build instructions provided in hook output if not found

5. **Integration Smoke Tests** - Runs quick integration tests
   - Requires: `pytest` and test PDFs
   - Location: `integration_tests/`
   - Command: `pytest -m smoke`
   - Timeout: 60 seconds

## Bypassing Hooks

If you need to commit without running hooks (not recommended):

```bash
git commit --no-verify
```

**Warning:** Only bypass hooks if you're certain your changes won't break the build.

## Building Tests

To build PDFium tests for the pre-commit hook:

```bash
# Create a build ID
BUILD_ID="dev-$(git log -1 --format='%h')-$(date +%Y%m%d-%H%M)"
BUILD_DIR="out/Test-${BUILD_ID}"

# Generate build files
gn gen "$BUILD_DIR" --args="is_debug=false pdf_is_standalone=true"

# Build tests
ninja -C "$BUILD_DIR" pdfium_unittests pdfium_embeddertests
```

The pre-commit hook will automatically find and use the most recent build in `out/Test-*`.

## Updating Hooks

If you modify hooks in `.githooks/`, run the installation script again to update `.git/`:

```bash
./.githooks/install-hooks.sh
```

## Requirements

### Required Tools

- `git` - Version control
- `python3` - Python interpreter

### Optional Tools (for full functionality)

- `clang-format` - C++ formatting
- `ruff` or `flake8` - Python linting
- `rustfmt` and `clippy` - Rust formatting and linting
- `pytest` - Integration tests
- `gn` and `ninja` - PDFium build system

Install on macOS:

```bash
# Install Homebrew tools
brew install clang-format

# Install Python tools
pip install ruff pytest

# Install Rust tools (if Rust is installed)
rustup component add rustfmt clippy
```

## Troubleshooting

### "clang-format not found"

Install with: `brew install clang-format` (macOS) or your system's package manager.

### "pytest not found"

Install with: `pip install pytest`

### "rustfmt not found" or "clippy not found"

Install with: `rustup component add rustfmt clippy`

If you don't have Rust installed and don't need Rust linting, the hook will skip these checks.

### "PDFium unit tests not built"

Follow the build instructions in the hook output to build tests.

### Tests fail but you need to commit

Use `git commit --no-verify` to bypass, but fix the issues in a follow-up commit immediately.

## Philosophy

These hooks enforce:

1. **Code quality** - Consistent formatting and linting
2. **Correctness** - Tests must pass before committing
3. **Fast feedback** - Catch issues before pushing

The hooks are designed to be helpful, not obstructive:
- Warnings are non-blocking for missing tools
- Tests are skipped if not built (with instructions)
- Clear error messages with fix instructions
- Easy bypass option for emergencies
