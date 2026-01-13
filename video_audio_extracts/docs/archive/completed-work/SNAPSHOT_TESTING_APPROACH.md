# Snapshot Testing: Capture Outputs, Detect Changes

**Date**: 2025-10-30
**User Insight**: "We may not know the ground truth, but we can watch for changes"
**Approach**: Snapshot testing - capture all outputs, diff between runs

---

## The Smart Approach

**Don't need golden outputs if we:**
1. Capture ALL outputs from a known-good run
2. Store them with git hash
3. On future runs: Compare outputs to baseline
4. Flag any changes for review

**This catches:**
- Output format changes
- Detection accuracy changes
- Transcription changes
- Any regression in extracted values

---

## Updated Directory Structure

```
test_results/
├── 2025-10-30_22-45-13_6a8f2e1/
│   ├── metadata.json
│   ├── test_results.csv
│   ├── performance_summary.json
│   ├── system_snapshot.json
│   └── outputs/                      # NEW: Captured outputs
│       ├── format_mp4_quick_pipeline/
│       │   ├── stage_00_keyframes.json    # Keyframe metadata
│       │   ├── stage_01_object_detection.json  # Detections
│       │   ├── keyframes/                 # JPEG files (or checksums)
│       │   │   ├── frame_0001.jpg
│       │   │   └── frame_0002.jpg
│       │   └── checksums.txt              # SHA256 of each output file
│       ├── characteristic_audio_codec_aac/
│       │   ├── stage_00_audio.wav         # Extracted audio
│       │   ├── stage_01_transcription.json # Transcript
│       │   └── checksums.txt
│       └── ...
└── latest/ -> 2025-10-30_22-45-13_6a8f2e1/
```

---

## Capture Strategy

### Option A: Copy All Outputs (Simple)
```rust
fn capture_test_outputs(test_name: &str, output_dir: &Path) {
    let test_output_dir = output_dir.join("outputs").join(test_name);
    std::fs::create_dir_all(&test_output_dir)?;

    // Copy everything from debug_output/
    let debug_dir = PathBuf::from("debug_output");
    for entry in std::fs::read_dir(debug_dir)? {
        let entry = entry?;
        let dest = test_output_dir.join(entry.file_name());
        std::fs::copy(entry.path(), dest)?;
    }
}
```

**Storage**: ~100MB per test run (JPEGs, audio files)
**Pros**: Complete capture, easy to review
**Cons**: Disk usage

### Option B: Checksums Only (Efficient)
```rust
fn capture_checksums(test_name: &str, output_dir: &Path) {
    let checksums_file = output_dir.join("outputs").join(test_name).join("checksums.txt");
    let mut checksums = String::new();

    for entry in std::fs::read_dir("debug_output")? {
        let path = entry?.path();
        let hash = sha256_file(&path)?;
        checksums.push_str(&format!("{}  {}\n", hash, path.display()));
    }

    std::fs::write(checksums_file, checksums)?;
}
```

**Storage**: ~10KB per test run
**Pros**: Minimal disk usage, fast
**Cons**: Can't review actual outputs, only detect changes

### Option C: Hybrid (RECOMMENDED)
```rust
fn capture_test_outputs_hybrid(test_name: &str, output_dir: &Path) {
    let test_output_dir = output_dir.join("outputs").join(test_name);
    std::fs::create_dir_all(&test_output_dir)?;

    // 1. Copy JSON files (small, useful)
    for json_file in glob("debug_output/*.json")? {
        std::fs::copy(json_file, test_output_dir)?;
    }

    // 2. Checksum large files (JPEGs, audio, video)
    let mut checksums = String::new();
    for large_file in glob("debug_output/*.{jpg,wav,mp4}")? {
        let hash = sha256_file(&large_file)?;
        let size = large_file.metadata()?.len();
        checksums.push_str(&format!("{}  {}  {}\n", hash, size, large_file.display()));
    }
    std::fs::write(test_output_dir.join("checksums.txt"), checksums)?;
}
```

**Storage**: ~5-10MB per test run (JSON + checksums)
**Pros**: Balance of reviewability and efficiency
**Cons**: Can't visually inspect images without original files

---

## Change Detection

### On Test Run:

```rust
fn compare_to_baseline(test_name: &str, current_outputs: &Path) -> DiffResult {
    // Find previous run (most recent or specific baseline)
    let baseline = find_baseline_run("test_results/");

    let baseline_outputs = baseline.join("outputs").join(test_name);
    if !baseline_outputs.exists() {
        return DiffResult::NewTest;  // First run, no baseline
    }

    // Compare JSON files
    for json_file in glob("debug_output/*.json")? {
        let current = std::fs::read_to_string(&json_file)?;
        let baseline = std::fs::read_to_string(
            baseline_outputs.join(json_file.file_name().unwrap())
        )?;

        if current != baseline {
            return DiffResult::Changed {
                file: json_file.display().to_string(),
                diff: diff_json(&baseline, &current),
            };
        }
    }

    // Compare checksums
    let current_checksums = read_checksums(current_outputs)?;
    let baseline_checksums = read_checksums(&baseline_outputs)?;

    for (file, hash) in &current_checksums {
        if let Some(baseline_hash) = baseline_checksums.get(file) {
            if hash != baseline_hash {
                return DiffResult::Changed {
                    file: file.clone(),
                    diff: format!("Checksum mismatch: {} != {}", hash, baseline_hash),
                };
            }
        }
    }

    DiffResult::Identical
}
```

### Test Report:

```
Test: format_mp4_quick_pipeline
Status: PASSED
Duration: 3.45s
Output: ⚠️  CHANGED from baseline
  - stage_01_object_detection.json: 2 detections → 3 detections
  - keyframe_0001.jpg: SHA256 changed (different JPEG encoding?)

Test: characteristic_audio_codec_mp3
Status: PASSED
Duration: 0.52s
Output: ✅ IDENTICAL to baseline
```

---

## Baseline Management

### Setting Baseline

```bash
# Mark current run as baseline
ln -sf test_results/2025-10-30_22-45-13_6a8f2e1 test_results/baseline

# Or use specific commit as baseline
ln -sf test_results/2025-10-29_15-30-00_abc1234 test_results/baseline
```

### Comparing Runs

```bash
# Compare current to baseline
diff test_results/baseline/outputs/test_name/ \
     test_results/latest/outputs/test_name/

# Compare two specific runs
diff test_results/2025-10-30_20-00-00_abc1234/outputs/ \
     test_results/2025-10-30_22-45-13_6a8f2e1/outputs/
```

---

## Use Cases

**1. Detect Regressions:**
```
You: Did my optimization change outputs?
Compare: baseline vs latest
Result: 3 tests have different outputs (investigate)
```

**2. Track Changes Over Time:**
```
You: How have transcriptions changed since last week?
Compare: All runs from Oct 23-30
Result: WER improved by 5% (good!)
```

**3. Validate Optimizations:**
```
You: Does new JPEG encoder produce same quality?
Compare: Checksums of JPEG outputs
Result: Different hashes (investigate visual quality)
```

**4. Performance + Correctness:**
```
You: Did speedup hurt accuracy?
Check: Timing improved 2x, outputs identical
Result: Win! Faster with no accuracy loss
```

---

## Implementation Priority

### Phase 1 (N=55): Basic Capture
- Capture JSON files (small)
- Generate checksums for large files
- Basic diff reporting

### Phase 2 (N=56): Change Detection
- Compare to baseline
- Report diffs in test output
- Flag changed tests

### Phase 3 (Later): Advanced Analysis
- Visual diffs for JSON
- Image quality metrics (SSIM)
- Transcription WER calculation
- Detection accuracy metrics (IoU, mAP)

---

## Expected Test Output

**With snapshot testing:**
```
Running 98 tests...

✅ format_mp4_quick_pipeline (3.45s)
   Outputs: ✅ IDENTICAL to baseline

⚠️  format_mov_screen_recording (4.12s)
   Outputs: ⚠️  CHANGED from baseline
   - stage_01_object_detection.json: 5 detections → 7 detections
   - keyframe_0003.jpg: SHA256 mismatch

❌ characteristic_audio_codec_aac (12.65s)
   Status: FAILED (performance regression)
   Outputs: Not compared (test failed)

...

Summary:
- 97 tests passed
- 1 test failed
- 94 outputs identical
- 3 outputs changed (review needed)
```

---

## WORKER N=55 INSTRUCTIONS

**After fixing AAC regression:**

1. Implement TestResultTracker module
2. Add output capture (hybrid approach: JSON + checksums)
3. Add baseline comparison logic
4. Report changes in test output
5. Test with full suite run
6. Commit with example results

**Estimated**: 2-3 commits, 3-4 hours

**This gives you:**
- Historical record of all test outputs
- Change detection across commits
- Performance + correctness tracking
- Foundation for regression analysis
