# Performance Benchmark Plan - Beta Release Phase 3

**Created:** N=28 (2025-11-06)
**Status:** Planning
**Purpose:** Comprehensive performance documentation for beta release

---

## OBJECTIVE

Create comprehensive performance documentation for all 33 operations to satisfy Beta Release Phase 3 requirements.

**Beta Plan Requirements** (BETA_RELEASE_PLAN.md:89-102):
- Benchmark all 33 operations
- Document throughput (MB/s, files/s)
- Document latency (p50, p95, p99)
- Document memory usage (peak, average)
- Create performance comparison charts
- Identify performance regressions
- Document hardware requirements

---

## OPERATIONS TO BENCHMARK (33)

### Audio Operations (7)
1. audio-extraction
2. transcription
3. voice-activity-detection
4. diarization
5. profanity-detection
6. audio-classification
7. audio-enhancement-metadata

### Video/Image Operations (18)
8. keyframes
9. scene-detection
10. object-detection
11. face-detection
12. pose-estimation
13. action-recognition
14. shot-classification
15. content-moderation
16. logo-detection
17. depth-estimation
18. caption-generation
19. ocr
20. smart-thumbnail
21. image-quality-assessment
22. emotion-detection
23. motion-tracking
24. duplicate-detection

### Metadata/Utility Operations (5)
25. metadata-extraction
26. subtitle-extraction
27. format-conversion
28. embeddings (vision)
29. embeddings (text)
30. embeddings (audio)

### Advanced Operations (3)
31. music-source-separation
32. acoustic-scene-classification
33. background_removal (does NOT exist - confirmed N=27)

**Actual count:** 32 operations (background_removal doesn't exist)

---

## BENCHMARK METHODOLOGY

### Test Media Selection

Use diverse test media from COMPLETE_TEST_FILE_INVENTORY.md (3,526 files):

**Video files** (10 files, varied sizes):
- Small: 1-10 MB (2 files)
- Medium: 10-100 MB (4 files)
- Large: 100-500 MB (2 files)
- Very large: 500+ MB (2 files)

**Audio files** (6 files, varied durations):
- Short: <1 min (2 files)
- Medium: 1-5 min (2 files)
- Long: 5+ min (2 files)

**Image files** (6 files, varied resolutions):
- Low res: <1 MP (2 files)
- Medium res: 1-5 MP (2 files)
- High res: 5+ MP (2 files)

### Metrics to Capture

For each operation:

1. **Throughput**:
   - Files/second (for batch processing)
   - MB/second (for large files)
   - Frames/second (for video operations)

2. **Latency** (run 10 times, calculate percentiles):
   - p50 (median)
   - p95 (95th percentile)
   - p99 (99th percentile)
   - min/max

3. **Memory Usage**:
   - Peak memory (RSS)
   - Average memory during operation
   - Memory overhead vs input size

4. **Hardware Utilization**:
   - CPU usage (%)
   - GPU usage (% - macOS Activity Monitor)
   - Disk I/O (MB/s read/write)

### Benchmark Command Template

```bash
# Single-file latency (10 runs for percentiles)
hyperfine --warmup 2 --runs 10 \
  'VIDEO_EXTRACT_THREADS=4 target/release/video-extract debug --ops <operation> <file>'

# Throughput (batch processing)
time VIDEO_EXTRACT_THREADS=4 target/release/video-extract bulk \
  --ops <operation> \
  --max-concurrent 8 \
  <directory>

# Memory profiling (requires /usr/bin/time -l on macOS)
/usr/bin/time -l target/release/video-extract debug --ops <operation> <file>
```

---

## BENCHMARK EXECUTION PLAN

### Phase 1: Setup (N=28, 1 commit)
- Create benchmark test file lists (10 video, 6 audio, 6 image)
- Verify all files exist and are accessible
- Create benchmark script framework
- Document hardware specs (CPU, RAM, GPU, disk)

### Phase 2: Quick Operations (N=29-30, 2 commits)
Benchmark fast operations (<5s per file):
- metadata-extraction
- duplicate-detection
- keyframes
- scene-detection
- smart-thumbnail
- voice-activity-detection
- subtitle-extraction
- image-quality-assessment

### Phase 3: ML Inference Operations (N=31-33, 3 commits)
Benchmark ML-heavy operations (5-30s per file):
- object-detection
- face-detection
- pose-estimation
- emotion-detection
- ocr
- shot-classification
- content-moderation
- logo-detection
- caption-generation
- acoustic-scene-classification

### Phase 4: Slow Operations (N=34-36, 3 commits)
Benchmark long-running operations (30s+ per file):
- transcription
- diarization
- profanity-detection
- depth-estimation
- motion-tracking
- action-recognition
- audio-classification
- audio-enhancement-metadata
- music-source-separation

### Phase 5: Utility Operations (N=37-38, 2 commits)
Benchmark conversion/extraction operations:
- audio-extraction
- format-conversion
- embeddings (vision, text, audio)

### Phase 6: Documentation (N=39-40, 2 commits)
- Consolidate all benchmark results
- Create performance comparison charts
- Document hardware requirements
- Identify any performance regressions
- Create final PERFORMANCE_BENCHMARKS.md report

---

## OUTPUT FORMAT

### Per-Operation Report

```markdown
## <Operation Name>

**Category:** Audio|Video|Image|Utility
**ML Model:** <model name if applicable> (size: X MB)
**Hardware Acceleration:** CPU|CoreML GPU|None

### Test Files
- File 1: <name> (<size>, <duration/resolution>)
- File 2: ...

### Latency (10 runs)
- p50: X.XXs
- p95: X.XXs
- p99: X.XXs
- min: X.XXs
- max: X.XXs

### Throughput
- Files/s: X.XX
- MB/s: X.XX
- Frames/s: X.XX (if applicable)

### Memory Usage
- Peak RSS: XXX MB
- Average RSS: XXX MB
- Overhead: XXX MB (vs input size)

### Hardware Utilization
- CPU: XX%
- GPU: XX% (if applicable)
- Disk I/O: XX MB/s

### Notes
- Any observations about performance characteristics
- Bottlenecks identified
- Scaling behavior
```

### Consolidated Report (PERFORMANCE_BENCHMARKS.md)

- Summary table of all operations
- Performance comparison charts
- Hardware requirements matrix
- Regression analysis (vs alpha v0.2.0 if available)
- Optimization opportunities

---

## ESTIMATED WORK

**Total commits:** 13 AI commits (N=28-40)
**Estimated AI time:** ~2.6 hours (13 × 12 minutes per commit)
**Operations benchmarked:** 32 (33 minus non-existent background_removal)

---

## SUCCESS CRITERIA

✅ All 32 operations benchmarked with:
- Latency percentiles (p50, p95, p99)
- Throughput metrics (files/s, MB/s)
- Memory usage (peak, average)
- Hardware utilization

✅ Documentation created:
- Per-operation detailed reports
- Consolidated PERFORMANCE_BENCHMARKS.md
- Performance comparison charts
- Hardware requirements

✅ Beta release blocker satisfied:
- Phase 3 complete (BETA_RELEASE_PLAN.md)
- Ready for Phase 4 (RAW format testing) or Phase 2 (cross-platform testing)

---

## HARDWARE SPECS (Reference)

**System:** macOS (Darwin 24.6.0)
**CPU:** <to be determined>
**RAM:** <to be determined>
**GPU:** <to be determined> (CoreML acceleration)
**Disk:** <to be determined>

---

## NEXT STEPS FOR N=28

1. Determine hardware specs
2. Create benchmark test file lists
3. Set up benchmark script framework
4. Begin Phase 1 (Setup)
