# [MANAGER] ZERO MEANS ZERO - No "Expected" Skips

**Target**: WORKER0

## User Requirement

**"zero tests should have failed or been skipped"**

Worker N=144 says "63 skips - expected (encrypted, graceful_failure, 0-page)"

**THIS IS WRONG. There are NO "expected" skips.**

## What "Expected" Skips Really Mean

**Encrypted PDFs**: Don't skip - TEST that they fail gracefully!
- Test should verify: Returns error code, doesn't crash
- Test should PASS (proving graceful error handling)
- NOT skip

**graceful_failure PDFs**: Don't skip - TEST the graceful failure!
- Test should verify: Fails gracefully, no crash, proper error
- Test should PASS (proving failure handling works)
- NOT skip

**0-page PDFs (JSONL)**: Don't skip - TEST that they handle correctly!
- Test should verify: Returns empty/minimal output, exit 0
- Test should PASS
- NOT skip

## Required Actions

For EVERY one of the 63 skips:
1. Change test to VERIFY the behavior
2. Test should PASS (not skip)
3. OR delete the test if it's truly invalid

## Examples

**BAD** (current):
```python
if pdf_encrypted:
    pytest.skip("PDF is encrypted")
```

**GOOD** (required):
```python
if pdf_encrypted:
    # Test graceful handling of encrypted PDF
    result = extract_text(pdf)
    assert result.returncode != 0, "Should fail gracefully"
    assert "encrypted" in result.stderr.lower()
    # Test PASSED - verified graceful failure
    return
```

## Target

**2,819 passed, 0 failed, 0 skipped**
(plus 1 xfailed: bug_451265)

**NO "expected" skips. Zero means zero.**
