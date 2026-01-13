# AI Verification - README

**Status:** Infrastructure Ready, Awaiting API Key
**Current Phase:** Phase 1 (50 tests)
**Next AI:** N=113

---

## Quick Start

**To execute Phase 1 verification:**

```bash
# 1. Set API key
export ANTHROPIC_API_KEY="sk-ant-..."

# 2. Run verification script
bash scripts/run_phase1_verification.sh
```

This will:
- Verify 50 tests from the sampling plan
- Generate outputs using video-extract
- Verify each output with Claude Sonnet 4
- Document results in NEW_TESTS_AI_VERIFICATION_REPORT.md

---

## Why AI Verification?

**The Problem:** 275 new tests added in N=93-109 have only been structurally validated.

**Structural validation checks:**
- JSON schema is correct
- Value ranges are valid (confidence 0-1, etc.)
- Required fields are present

**Structural validation DOES NOT check:**
- Are face detection bounding boxes around actual faces?
- Is transcription text accurate?
- Are object labels correct ("dog" vs "cat")?
- Are embeddings semantically meaningful?

**Solution:** Use Claude API with vision capabilities to verify semantic correctness.

---

## Files

### Documentation
- **README.md** (this file): Quick start and overview
- **AI_VERIFICATION_METHODOLOGY.md**: Complete methodology and process
- **PHASE_1_SAMPLING_PLAN.md**: Detailed list of 50 Phase 1 tests
- **NEW_TESTS_AI_VERIFICATION_REPORT.md**: Results (to be created)

### Scripts
- **scripts/ai_verify_outputs.py**: Core verification tool (Python + Claude API)
- **scripts/run_phase1_verification.sh**: Automated execution script

---

## Workflow

### Manual Verification (one test)

1. Generate output:
   ```bash
   ./target/release/video-extract debug --ops face-detection test.jpg
   ```

2. Verify output:
   ```bash
   python scripts/ai_verify_outputs.py \
       test.jpg \
       debug_output/stage_00_face_detection.json \
       face-detection
   ```

3. Review JSON response:
   ```json
   {
     "status": "CORRECT",
     "confidence": 0.95,
     "findings": "Face detection correctly identified 2 faces...",
     "errors": []
   }
   ```

### Automated Verification (all 50 tests)

```bash
bash scripts/run_phase1_verification.sh
```

This runs all 50 Phase 1 tests and documents results.

---

## Success Criteria

**Phase 1 goals:**
- Verify 50 tests (from 275 new tests)
- Achieve ≥90% confidence on ≥95% of tests
- Document all SUSPICIOUS and INCORRECT findings
- Fix any bugs discovered

**Status values:**
- **CORRECT:** Output matches expectations (confidence ≥0.90)
- **SUSPICIOUS:** Output partially correct or unclear (confidence 0.50-0.89)
- **INCORRECT:** Output wrong or nonsensical (confidence <0.50)

---

## Current Status

**Infrastructure:** ✅ Complete
- ai_verify_outputs.py: Created
- AI_VERIFICATION_METHODOLOGY.md: Created
- PHASE_1_SAMPLING_PLAN.md: Created
- run_phase1_verification.sh: Created

**Execution:** ⚠️ Blocked on ANTHROPIC_API_KEY
- API key not set in environment
- Cannot run verification until key is provided

**Next Steps:**
1. Set ANTHROPIC_API_KEY
2. Run Phase 1 verification script
3. Review results
4. Fix any bugs found

---

## Timeline

**Completed:**
- N=111: Created infrastructure and methodology

**Remaining:**
- N=112: Created sampling plan and execution script (current)
- N=113: Execute Phase 1 verification (25-50 tests)
- N=114: Investigate SUSPICIOUS/INCORRECT findings, fix bugs
- N=115: Execute Phase 2 verification (50 more tests)
- N=116: Final report and recommendations

**Estimated time:**
- Per test: ~2-3 minutes (generate + verify + document)
- 50 tests: ~2 hours of AI execution
- Bug fixes: Unknown (depends on findings)

---

## API Key Setup

**Required:** Claude API key with vision capabilities

**How to get:**
1. Go to https://console.anthropic.com/
2. Create API key
3. Export in shell:
   ```bash
   export ANTHROPIC_API_KEY="sk-ant-..."
   ```

**Cost estimate:**
- Per verification: ~$0.01-0.05 (depends on image size)
- 50 tests: ~$0.50-2.50
- 100 tests (both phases): ~$1-5

---

## Troubleshooting

### "ANTHROPIC_API_KEY environment variable not set"

**Solution:**
```bash
export ANTHROPIC_API_KEY="sk-ant-..."
```

### "File not found: test_files_..."

**Cause:** Test media file missing from local filesystem

**Solution:**
- Check if file exists: `ls -la <path>`
- If missing, skip test and document in report
- Large files (>10MB) were removed from git in N=432 but may still exist locally

### "Output file not found for operation"

**Cause:** video-extract failed to generate output

**Solution:**
- Run manually to see error:
  ```bash
  ./target/release/video-extract debug --ops <operation> <file>
  ```
- Check FFmpeg logs for decode errors
- Verify operation is supported for file format

### Claude returns non-JSON response

**Cause:** API response format unexpected

**Solution:**
- Check API logs
- Manually review verification_result variable
- May need to update JSON parsing in script

---

## Directory Structure

```
docs/ai-verification/
├── README.md                              (this file)
├── AI_VERIFICATION_METHODOLOGY.md         (complete methodology)
├── PHASE_1_SAMPLING_PLAN.md               (50 test list)
└── NEW_TESTS_AI_VERIFICATION_REPORT.md    (results, to be created)

scripts/
├── ai_verify_outputs.py                   (verification tool)
└── run_phase1_verification.sh             (execution script)
```

---

## References

- **MANAGER_CRITICAL_DIRECTIVE_AI_VERIFICATION.md**: Original directive
- **tests/smoke_test_comprehensive.rs**: 647 comprehensive tests (275 new, unverified)
- **N=111 commit**: Infrastructure creation
- **N=112 commit**: Sampling plan and execution script (current)

---

**Next AI (N=113): Run Phase 1 verification with `bash scripts/run_phase1_verification.sh`**
