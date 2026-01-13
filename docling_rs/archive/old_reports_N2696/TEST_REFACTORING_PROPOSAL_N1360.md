# Test Refactoring Proposal - N=1360

## Problem

**File**: `crates/docling-core/tests/llm_docitem_validation_tests.rs`
**Size**: 5,978 lines
**Tests**: 53 formats
**Average**: 113 lines per test

**Issue**: Massive code duplication. Each test follows identical pattern:
1. Parse file with backend
2. Serialize to JSON
3. Create LLM prompt (nearly identical structure)
4. Call verifier
5. Print results
6. Assert threshold

## Current Structure (Example)

```rust
#[tokio::test]
async fn test_llm_docitem_docx() {
    let verifier = create_verifier();

    let backend = DocxBackend;
    let result = backend.parse_file("path/to/docx", &Default::default())
        .expect("Failed to parse DOCX");

    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize");

    let prompt = format!(r#"
        Analyze if this DocItem JSON contains complete information...
        ORIGINAL DOCUMENT: {}
        PARSED DOCITEMS (JSON):
        ```json
        {}
        ```
        [... 80 lines of prompt template ...]
    "#, path.display(), truncate_json_for_llm(&json, 80000));

    let quality = verifier.custom_verification(&prompt).await.expect("LLM API failed");

    println!("=== DocItem Completeness: DOCX ===");
    println!("Overall Score: {:.1}%", quality.score * 100.0);
    // ... 20 lines of printing ...

    assert!(quality.score >= 0.95, "DocItem completeness: {:.1}% (need 95%)", ...);
}
```

**Repeated 53 times with minor variations!**

## Proposed Refactoring

### Option 1: Macro-Based (Type-Safe)

```rust
macro_rules! docitem_quality_test {
    (
        name: $test_name:ident,
        format: $format:expr,
        backend: $backend:expr,
        test_file: $test_file:expr,
        threshold: $threshold:expr,
        $(format_notes: $notes:expr)?
    ) => {
        #[tokio::test]
        async fn $test_name() {
            run_docitem_quality_test(
                $format,
                $backend,
                $test_file,
                $threshold,
                $($notes,)?
            ).await;
        }
    };
}

// Usage:
docitem_quality_test! {
    name: test_llm_docitem_docx,
    format: "DOCX",
    backend: DocxBackend,
    test_file: "test-corpus/docx/word_sample.docx",
    threshold: 0.95
}

docitem_quality_test! {
    name: test_llm_docitem_png,
    format: "PNG",
    backend: PngBackend,
    test_file: "test-corpus/images/png/sample.png",
    threshold: 0.90,  // Images use lower threshold
    format_notes: "Images validated for OCR text extraction"
}
```

**Benefits**:
- 5,978 lines → ~500 lines (92% reduction)
- Single implementation to maintain
- Type-safe at compile time
- Easy to add new formats

**Drawbacks**:
- Macros can be complex to debug
- Less flexibility for format-specific logic
- Harder for newcomers to understand

### Option 2: Function-Based (Simpler)

```rust
async fn run_docitem_quality_test<B: DocumentBackend>(
    format_name: &str,
    backend: B,
    test_file: &str,
    threshold: f64,
    custom_prompt_section: Option<&str>,
) {
    let verifier = create_verifier();
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../").join(test_file);

    let result = backend.parse_file(&path, &Default::default())
        .expect(&format!("Failed to parse {}", format_name));

    let json = serde_json::to_string_pretty(&result)
        .expect("Failed to serialize to JSON");

    let prompt = build_standard_prompt(format_name, &path, &json, custom_prompt_section);

    let quality = verifier.custom_verification(&prompt).await
        .expect("LLM API failed");

    print_quality_report(format_name, &quality);

    assert!(
        quality.score >= threshold,
        "DocItem completeness: {:.1}% (need {:.1}%)",
        quality.score * 100.0,
        threshold * 100.0
    );
}

// Usage:
#[tokio::test]
async fn test_llm_docitem_docx() {
    run_docitem_quality_test(
        "DOCX",
        DocxBackend,
        "test-corpus/docx/word_sample.docx",
        0.95,
        None
    ).await;
}
```

**Benefits**:
- Simpler than macros
- Easy to understand and modify
- Still ~90% code reduction
- Format-specific logic in test function

**Drawbacks**:
- Less concise than macros
- More boilerplate per test

### Option 3: Hybrid (Best of Both)

Use function for logic, macro for DRY test definitions:

```rust
// Shared implementation
async fn run_docitem_quality_test(...) { /* ... */ }

// Macro for concise test definitions
macro_rules! docitem_test {
    ($name:ident, $fmt:expr, $backend:ty, $file:expr) => {
        #[tokio::test]
        async fn $name() {
            run_docitem_quality_test(
                $fmt,
                <$backend>::default(),
                $file,
                0.95,
                None
            ).await;
        }
    };

    ($name:ident, $fmt:expr, $backend:ty, $file:expr, threshold: $threshold:expr) => {
        #[tokio::test]
        async fn $name() {
            run_docitem_quality_test(
                $fmt,
                <$backend>::default(),
                $file,
                $threshold,
                None
            ).await;
        }
    };
}

// Usage:
docitem_test!(test_llm_docitem_docx, "DOCX", DocxBackend, "test-corpus/docx/word_sample.docx");
docitem_test!(test_llm_docitem_png, "PNG", PngBackend, "test-corpus/png/sample.png", threshold: 0.90);
```

## Recommendation

**Choose Option 3 (Hybrid)**:
- Clean separation: logic in function, definitions in macro
- Easy to maintain shared logic
- Concise test definitions
- Format-specific logic possible via custom prompt section

## Implementation Plan

1. Create `run_docitem_quality_test()` helper function
2. Create `build_standard_prompt()` helper function
3. Create `print_quality_report()` helper function
4. Create `docitem_test!` macro with variants
5. Convert all 53 tests to use new macro
6. Verify all tests still pass
7. Delete old test implementations

**Estimated Time**: 2-3 hours
**Lines Saved**: ~5,400 lines (90% reduction)
**Maintainability**: Significantly improved

## Risks

**Breaking Changes**: None (test names unchanged)
**Test Behavior**: Should be identical
**Debugging**: May be slightly harder with macro indirection

## Future Extensions

After refactoring, easy to add:
- Batch test execution (run all formats in parallel)
- Retry logic for flaky LLM tests
- Score averaging (run each test 3x)
- Custom threshold per format
- Format-specific validation rules

## Conclusion

**Current state**: 6K lines of nearly-identical code
**Proposed state**: ~500 lines (function + macro + 53 test definitions)
**Benefit**: Easier maintenance, consistency, extensibility

**Recommendation**: Implement Option 3 in next AI session (N=1361)
