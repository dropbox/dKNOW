# LLM Test Execution Strategy - Efficient & Incremental

**User Requirements:**
1. LLM tests produce incremental reports
2. Execution order randomized
3. Sample top N tests for fast smoke testing
4. Avoid long-running full suites

---

## CURRENT SITUATION

**39 LLM tests exist**
- Each test: 2-5 seconds (OpenAI API call)
- Full suite: ~130 seconds (2+ minutes)
- Cost: ~$0.02 per full run
- **Too slow for frequent runs**

---

## SOLUTION: TIERED TESTING STRATEGY

### Tier 1: Critical Smoke Tests (Top 5) - 10 seconds

**Run these ALWAYS (fast smoke test):**
```bash
# Critical formats only
OPENAI_API_KEY="..." cargo test test_llm_verification_csv -- --ignored --nocapture
OPENAI_API_KEY="..." cargo test test_llm_verification_docx -- --ignored --nocapture
OPENAI_API_KEY="..." cargo test test_llm_verification_html -- --ignored --nocapture
OPENAI_API_KEY="..." cargo test test_llm_mode3_zip -- --ignored --nocapture
OPENAI_API_KEY="..." cargo test test_llm_mode3_epub -- --ignored --nocapture
```

**5 tests = 10-15 seconds, covers:**
- Office (DOCX)
- Web (HTML, CSV)
- Archives (ZIP)
- Ebooks (EPUB)

**Use for:** Quick quality checks, CI/CD, development

---

### Tier 2: Random Sample (10 tests) - 30 seconds

**Run random subset:**
```bash
# Get list of all LLM tests
ALL_TESTS=$(cargo test --test llm_verification_tests --list | grep "test_llm" | shuf | head -10)

# Run random 10
for test in $ALL_TESTS; do
    OPENAI_API_KEY="..." cargo test $test -- --ignored --nocapture
done
```

**10 tests = 30-40 seconds**
**Use for:** Daily checks, broader coverage

---

### Tier 3: Full Suite (39 tests) - 130 seconds

**Run everything:**
```bash
OPENAI_API_KEY="..." cargo test test_llm --test llm_verification_tests -- --ignored --nocapture | tee llm_full_results.txt
```

**39 tests = 2+ minutes**
**Use for:** Weekly validation, before releases, major changes

---

## INCREMENTAL REPORTING

**Modify llm_verification_tests.rs to write results incrementally:**

```rust
// In each test, write result to CSV immediately
#[tokio::test]
#[ignore]
async fn test_llm_mode3_zip() {
    let verifier = create_verifier();
    let result = // ... parse ...

    let quality = verifier.verify_standalone(...).await?;

    // INCREMENTAL: Write to CSV immediately
    write_llm_result_to_csv("ZIP", quality.score, &quality.findings)?;

    print_quality_report("ZIP", &quality);
    assert!(quality.score >= 0.75);
}

fn write_llm_result_to_csv(format: &str, score: f64, findings: &[QualityFinding]) -> Result<()> {
    use std::fs::OpenOptions;
    use std::io::Write;

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("test-results/llm_quality_scores.csv")?;

    writeln!(file, "{},{},{},{}",
        chrono::Utc::now(),
        format,
        score,
        findings.len()
    )?;

    Ok(())
}
```

**Benefits:**
- See results as tests run
- Partial results if interrupted
- Track progress over time
- Can monitor in real-time

---

## RANDOMIZED EXECUTION

**Use test randomization:**
```bash
# Cargo supports test ordering
cargo test test_llm --test llm_verification_tests -- --ignored --test-threads=1 --shuffle

# Or use external tool
cargo test --test llm_verification_tests --list | \
    grep "test_llm" | \
    shuf | \
    while read test; do
        OPENAI_API_KEY="..." cargo test $test -- --ignored --nocapture
    done
```

**Benefits:**
- Different order each run
- Avoid order-dependent bugs
- More thorough coverage over time

---

## PRIORITY TIERS

**Tier 1 (Critical - Always test):**
- CSV (baseline quality)
- DOCX (major format)
- HTML (major format, known issue)
- PPTX (major format, known issue)
- XLSX (major format)

**Tier 2 (Important - Sample 5 randomly):**
- Markdown, AsciiDoc, JATS
- Archives (ZIP, TAR)
- Email (EML, MBOX)
- Ebooks (EPUB, FB2)

**Tier 3 (Extended - Sample 5 randomly):**
- All other formats
- GPS, CAD, Images, etc.

---

## PRACTICAL COMMANDS

**Quick smoke (5 tests, 15 sec):**
```bash
cargo test test_llm_verification_csv test_llm_verification_docx test_llm_verification_html test_llm_mode3_zip test_llm_mode3_epub --test llm_verification_tests -- --ignored
```

**Daily check (15 tests, 60 sec):**
```bash
# Tier 1 (5) + Random Tier 2 (5) + Random Tier 3 (5)
cargo test --test llm_verification_tests --list | grep test_llm | shuf | head -15 | xargs -I {} cargo test {} -- --ignored
```

**Weekly full (39 tests, 2 min):**
```bash
cargo test test_llm --test llm_verification_tests -- --ignored | tee llm_weekly_$(date +%Y%m%d).txt
```

---

## CSV REPORTING FORMAT

**Create:** `test-results/llm_quality_scores.csv`

```csv
timestamp,format,quality_score,findings_count,test_duration_ms,status
2025-11-13T10:00:00Z,CSV,1.00,0,3840,PASS
2025-11-13T10:00:04Z,HTML,0.68,3,4120,FAIL
2025-11-13T10:00:08Z,DOCX,1.00,0,3950,PASS
...
```

**Benefits:**
- Track quality over time
- See improvements
- Identify regressions
- Generate reports

---

## SMOKE TEST OPTIMIZATION

**For CI/CD and quick checks:**

**Option A: Top 5 (fixed set)**
- Fastest (10-15 sec)
- Consistent
- Catches major issues

**Option B: Random 10 (varied)**
- Medium speed (30-40 sec)
- Better coverage over multiple runs
- Different formats each time

**Option C: Stratified sample**
- 2 critical (CSV, DOCX)
- 3 random from Tier 2
- 3 random from Tier 3
- Balanced coverage in 30 seconds

---

## RECOMMENDATIONS

**Daily development:**
- Run Tier 1 (5 tests) after major changes
- Cost: ~$0.003
- Time: 15 seconds

**Before commits:**
- Run random 10 tests
- Cost: ~$0.006
- Time: 40 seconds

**Weekly/releases:**
- Run full 39 tests
- Cost: ~$0.02
- Time: 2 minutes
- Document all scores

---

## IMPLEMENTATION

**Worker must:**
1. Add CSV reporting to each test
2. Create Tier 1/2/3 test groups
3. Add randomization script
4. Document smoke test commands

**Estimated:** 2-3 commits

**Then:** Can run efficient smoke tests frequently without long waits

---

**Efficient LLM testing: 5 critical tests (15 sec), random samples (40 sec), full suite weekly (2 min)**
