# Contributing Guide

Thank you for your interest in contributing to docling-rs! This guide will help you get started with development, testing, and submitting changes.

---

## Table of Contents

1. [Development Setup](#development-setup)
2. [Project Structure](#project-structure)
3. [Testing](#testing)
4. [Adding New Formats](#adding-new-formats)
5. [Coding Standards](#coding-standards)
6. [Git Workflow](#git-workflow)
7. [AI Collaboration](#ai-collaboration)

---

## Development Setup

### Prerequisites

- **Rust 1.70+** (https://rustup.rs/)
- **Git** for version control
- **Optional:** LibreOffice (for DOC/PUB legacy format support)
- **Optional:** LLVM/Clang (for building with OpenCV features)

### Clone and Build

```bash
# Clone repository
git clone https://github.com/ayates_dbx/docling_rs.git
cd docling_rs

# Build project
cargo build

# Run tests
cargo test

# Expected: All tests pass (3700+ tests)
```

### Optional: PDF ML Support

```bash
# Set up PyTorch C++ backend
source setup_env.sh  # Sets LIBTORCH_USE_PYTORCH=1

# Build with PDF ML features
cargo build --features pdf-ml
```

### IDE Setup

**VS Code:**
```bash
# Install extensions
code --install-extension rust-lang.rust-analyzer
code --install-extension serayuzgur.crates
```

**IntelliJ IDEA:**
- Install Rust plugin
- Enable Cargo integration

---

## Project Structure

```
docling_rs/
├── crates/
│   ├── docling-core/          # Main library (converter, types, serializers)
│   │   ├── src/
│   │   │   ├── converter.rs   # DocumentConverter
│   │   │   ├── document.rs    # Document types
│   │   │   ├── format.rs      # InputFormat enum
│   │   │   ├── error.rs       # Error types
│   │   │   ├── serializer/    # Markdown/HTML exporters
│   │   │   ├── archive.rs     # Archive backends (ZIP, TAR, etc.)
│   │   │   ├── ebook.rs       # E-book backends
│   │   │   ├── email.rs       # Email backends
│   │   │   └── ...            # Other format modules
│   │   └── tests/
│   │       └── integration_tests.rs  # Canonical tests
│   │
│   ├── docling-backend/       # Format-specific parsers
│   │   └── src/               # Individual parser implementations
│   │
│   └── ...                    # Other crates (genomics, etc.)
│
├── docs/                      # Documentation
│   ├── USER_GUIDE.md
│   ├── API.md
│   ├── FORMATS.md
│   ├── TROUBLESHOOTING.md
│   └── CONTRIBUTING.md (this file)
│
├── test-corpus/               # Test files (git-ignored)
│   ├── pdf/
│   ├── docx/
│   └── groundtruth/
│       └── docling_v2/        # Expected outputs
│
├── reports/                   # AI session reports
│   └── feature/
│       └── phase-e-open-standards/
│
├── CLAUDE.md                  # AI agent instructions
├── MASTER_PLAN.md            # Project roadmap
├── TESTING_STRATEGY.md       # Testing guidelines
└── README.md                 # Project overview
```

### Key Files

- **`converter.rs`** - Main DocumentConverter API
- **`serializer/markdown.rs`** - Markdown export logic
- **`format.rs`** - Format definitions and routing
- **`integration_tests.rs`** - Canonical test suite
- **`CLAUDE.md`** - Critical project conventions

---

## Testing

### Running Tests

```bash
# Unit tests (fast, 145 tests)
cargo test --lib

# Integration tests - canonical only (97 tests, ~3 min)
USE_HYBRID_SERIALIZER=1 cargo test test_canon

# Integration tests - all (913 tests, ~2 hours)
USE_HYBRID_SERIALIZER=1 cargo test

# Single test
USE_HYBRID_SERIALIZER=1 cargo test test_canon_pdf_multi_page_text -- --exact

# With output
cargo test -- --nocapture

# Sequential (required for pdfium thread safety)
cargo test -- --test-threads=1
```

### Test Organization

**Unit Tests:**
- Located in `#[cfg(test)]` modules within source files
- Test individual functions and components
- No external dependencies
- Fast (<1s total)

**Integration Tests:**
- Located in `crates/docling-core/tests/integration_tests.rs`
- Test end-to-end conversion against expected outputs
- Require test corpus files in `test-corpus/` directory
- Slow (2+ hours for full suite)

**Test Naming Convention:**
```rust
// Canonical tests (verify consistent output)
test_canon_pdf_2305_03393v1_text
test_canon_pdf_2305_03393v1_ocr
test_canon_docx_word_sample_text
test_canon_html_example_text

// Non-canonical tests (additional coverage)
test_more_pdf_large_document_text
test_more_docx_complex_formatting_text
```

### Writing Tests

**Unit Test Example:**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_markdown_heading() {
        let markdown = serialize_heading("Title", 1);
        assert_eq!(markdown, "# Title\n\n");
    }
}
```

**Integration Test Example:**

```rust
#[test]
fn test_canon_my_format_sample_text() {
    test_conversion(
        "my_format", // Format directory
        "sample",    // Filename (without extension)
        "myformat",  // Extension
        false,       // OCR disabled
    );
}
```

See [TESTING_STRATEGY.md](../TESTING_STRATEGY.md) for comprehensive testing guide.

---

## Adding New Formats

### Step 1: Implement Parser

Create new module in `crates/docling-core/src/`:

```rust
// crates/docling-core/src/my_format.rs

use crate::{DoclingError, Result};
use std::path::Path;

/// Process MY_FORMAT file and return markdown
pub fn process_my_format<P: AsRef<Path>>(path: P) -> Result<String> {
    let path = path.as_ref();

    // Read file
    let content = std::fs::read(path)
        .map_err(|e| DoclingError::IOError(e))?;

    // Parse format (implement your logic)
    let markdown = parse_my_format(&content)?;

    Ok(markdown)
}

fn parse_my_format(content: &[u8]) -> Result<String> {
    // Your parsing logic here
    // Return markdown string
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_my_format() {
        let markdown = process_my_format("test.myformat").unwrap();
        assert!(!markdown.is_empty());
    }
}
```

### Step 2: Add Format Enum

Edit `crates/docling-core/src/format.rs`:

```rust
pub enum InputFormat {
    // ... existing formats

    /// My new format (.myformat)
    #[serde(rename = "MYFORMAT")]
    MyFormat,
}

impl InputFormat {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            // ... existing cases
            "myformat" | "myfmt" => Some(Self::MyFormat),
            _ => None,
        }
    }

    pub fn extensions(&self) -> &[&str] {
        match self {
            // ... existing cases
            Self::MyFormat => &["myformat", "myfmt"],
        }
    }
}

impl std::fmt::Display for InputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            // ... existing cases
            Self::MyFormat => "MYFORMAT",
        };
        write!(f, "{}", s)
    }
}
```

### Step 3: Register in Converter

Edit `crates/docling-core/src/converter.rs`:

```rust
// Add module declaration at top
pub mod my_format;

// Add to convert_with_rust_backend method
fn convert_with_rust_backend<P: AsRef<Path>>(&self, path: P, start: Instant) -> Result<ConversionResult> {
    // ... existing code

    let markdown = match format {
        // ... existing cases
        InputFormat::MyFormat => crate::my_format::process_my_format(path)?,
        _ => return Err(DoclingError::ConversionError(format!("...")))
    };

    // ... rest of method
}
```

### Step 4: Update lib.rs

Edit `crates/docling-core/src/lib.rs`:

```rust
pub mod my_format;
```

### Step 5: Add Integration Tests

Create test files:

```bash
# Add test input
cp sample.myformat test-corpus/my_format/

# Generate expected output (manual verification)
# Place in: test-corpus/groundtruth/docling_v2/sample.md
```

Add test in `integration_tests.rs`:

```rust
#[test]
fn test_canon_my_format_sample_text() {
    test_conversion("my_format", "sample", "myformat", false);
}
```

### Step 6: Update Documentation

Update `docs/FORMATS.md`:

```markdown
| **MY_FORMAT** | `.myformat`, `.myfmt` | ✅ Integrated | Description of format |
```

### Step 7: Test and Submit

```bash
# Run tests
cargo test --lib
USE_RUST_BACKEND=1 cargo test test_canon_my_format

# Format code
cargo fmt

# Check lints
cargo clippy

# Build docs
cargo doc --no-deps

# Commit
git add .
git commit -m "Add MY_FORMAT support"
```

See [PARSER_PORTING_STRATEGY.md](../PARSER_PORTING_STRATEGY.md) for detailed guidance on porting Python parsers.

---

## Coding Standards

### Rust Style

**Follow Rust conventions:**
```bash
# Format code (required before commit)
cargo fmt

# Check lints (fix warnings)
cargo clippy -- -D warnings

# No unsafe code (unless absolutely necessary)
# Document all public APIs
# Write tests for new functionality
```

**Naming:**
- `snake_case` for functions, variables, modules
- `PascalCase` for types, traits, enums
- `SCREAMING_SNAKE_CASE` for constants

**Documentation:**
```rust
/// Process EPUB file and extract text
///
/// # Arguments
/// * `path` - Path to EPUB file
///
/// # Returns
/// Markdown representation of the e-book
///
/// # Errors
/// Returns `DoclingError::IOError` if file cannot be read
pub fn process_epub<P: AsRef<Path>>(path: P) -> Result<String> {
    // ...
}
```

### Error Handling

**Use `Result` type:**
```rust
// GOOD
fn parse_file(path: &Path) -> Result<String> {
    let content = std::fs::read_to_string(path)?;
    Ok(content)
}

// BAD
fn parse_file(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap() // Don't panic!
}
```

**Custom errors:**
```rust
return Err(DoclingError::ConversionError(
    format!("Failed to parse table: {}", err)
));
```

### Performance

**Profile before optimizing:**
```bash
cargo install flamegraph
cargo flamegraph --test integration_tests -- test_canon_pdf
```

**Prefer standard library:**
- Use `std::fs` over external file I/O crates (when sufficient)
- Use `std::collections` over third-party collections
- Minimize dependencies

**Reuse allocations:**
```rust
// GOOD: Reuse String
let mut markdown = String::with_capacity(1024);
for item in items {
    markdown.push_str(&format_item(item));
}

// BAD: Many allocations
let markdown: String = items.iter()
    .map(|item| format_item(item))
    .collect::<Vec<_>>()
    .join("");
```

---

## Git Workflow

### Branching Strategy

```bash
# Feature branches
git checkout -b feature/my-new-feature

# Bug fixes
git checkout -b fix/issue-123

# Documentation
git checkout -b docs/improve-user-guide
```

### Commit Messages

**Format:**
```
# <N++>: <Brief Title>
**Current Plan**: <Summary or link to plan>
**Checklist**: <Status of tasks>

## Changes
<Detailed description of changes>
<Why you made these changes, not what (git diff shows what)>

## New Lessons
<Important lessons learned (if any)>

## Information Expiration
<Any obsolete/wrong information>

## Next AI: <Instructions for next developer/AI>
<What to do next, references to reports>
```

**Example:**
```
# 102: Documentation Phase - User-Facing Docs
**Current Plan**: Create comprehensive documentation for production use
**Checklist**: FORMATS.md ✅, TROUBLESHOOTING.md ✅, USER_GUIDE.md ✅, API.md ✅, CONTRIBUTING.md ✅, README.md ⏳

## Changes
Created 5 new documentation files in docs/:
- FORMATS.md: Format support matrix with 51 formats, test coverage, known limitations
- TROUBLESHOOTING.md: Common issues, OCR problems, test failures, debugging techniques
- USER_GUIDE.md: Installation, basic/advanced usage, batch processing, best practices
- API.md: Complete API reference for DocumentConverter, Document, InputFormat, error handling
- CONTRIBUTING.md: Development setup, testing, adding formats, coding standards, git workflow

Updated documentation structure to improve production readiness. All docs cross-reference each other and include practical examples.

## New Lessons
None. Standard documentation work.

## Information Expiration
None. All references to N=101 analysis are still valid.

## Next AI: Update README.md
Update README.md with:
- Current status (N=306, 100% test pass rate, 65+ formats)
- Links to all new documentation
- Simplified quick start
- Code examples

Time: 30 minutes
```

See [CLAUDE.md](../CLAUDE.md) for full commit message requirements.

### Pull Requests

```bash
# Create PR from feature branch
gh pr create --title "Add MY_FORMAT support" \
  --body "Implements parser for MY_FORMAT files. Adds 5 integration tests. All tests passing."

# Or use GitHub web interface
```

**PR Checklist:**
- [ ] All tests pass (`cargo test`)
- [ ] Code formatted (`cargo fmt`)
- [ ] No lint warnings (`cargo clippy`)
- [ ] Documentation updated (if public API changes)
- [ ] Integration tests added (if new format)
- [ ] Commit message follows convention

---

## AI Collaboration

This project uses Claude Code (AI pair programmer) for development. If you're an AI agent:

### Read First
1. **[CLAUDE.md](../CLAUDE.md)** - Project conventions and requirements
2. **Last git commit** - Resume current work
3. **Reports in `reports/<branch>/`** - Context and decisions

### Key Principles
- **Factual reporting only** (no superlatives, no emojis)
- **Study Python source before porting** (don't invent heuristics)
- **Commit frequently** (git is permanent record)
- **No partial success** (either works or doesn't)
- **Re-read CLAUDE.md after each commit**

### Iteration Protocol
```bash
# Check current iteration
git log -1 --oneline
# Commit number is N, your iteration is N+1

# Work, commit as N+1
git commit -m "# <N+1>: <Title>..."

# Read CLAUDE.md again
cat CLAUDE.md
```

### For Humans
If working with an AI:
- Provide clear requirements
- Reference specific files/lines
- Ask for measurements, not estimates
- Review AI commits carefully

---

## Code Review Guidelines

**For Reviewers:**

1. **Correctness:** Does it match expected behavior?
2. **Tests:** Are there tests? Do they pass?
3. **Performance:** Any obvious performance issues?
4. **Documentation:** Is public API documented?
5. **Style:** Follows Rust conventions?

**Common Issues:**
- Missing error handling
- Unwrap/expect in library code
- Undocumented public APIs
- Missing tests
- Clippy warnings

---

## Getting Help

- **GitHub Issues:** https://github.com/ayates_dbx/docling_rs/issues
- **Documentation:** See `docs/` directory
- **Discord:** (link to project Discord if available)

---

## License

This project is licensed under [LICENSE] (see LICENSE file).

By contributing, you agree to license your contributions under the same license.

---

**Last Updated:** 2025-11-12 (N=308)
**Status:** Active development, contributions welcome
