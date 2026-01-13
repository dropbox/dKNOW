# SPEED ABOVE ALL ELSE

**User directive**: "If FFmpeg is so great, then use that. ACHIEVE THE GOAL. We care about efficiency not pretty abstractions. Fucking make it happen."

## THE BRUTAL TRUTH

We built a slow wrapper. **Stop defending it. Fix it.**

FFmpeg CLI: 4s
Us: 17.7s
**We're embarrassingly slow.**

## THE FIX: Use Whatever Is Fastest

**Rule 1**: If FFmpeg CLI is faster → **CALL FFmpeg CLI directly**
**Rule 2**: If Whisper CLI is faster → **CALL Whisper CLI directly**
**Rule 3**: If YOLO CLI is faster → **CALL YOLO CLI directly**
**Rule 4**: Strip EVERY line of code that doesn't make it faster
**Rule 5**: Speed is the ONLY metric that matters

## Immediate Actions

### 1. Keyframes: Call FFmpeg CLI Directly
```rust
// REMOVE: Our slow Rust decoder + perceptual hashing
// ADD: Direct FFmpeg CLI call

fn extract_keyframes_fast(video: &Path) -> Result<Vec<PathBuf>> {
    Command::new("ffmpeg")
        .args(&["-i", video, "-vf", "select='eq(pict_type\\,I)'",
                "-vsync", "vfr", "frame%04d.jpg"])
        .output()?;
    // Done. 4 seconds. Not 17.
}
```

### 2. Transcription: Call Whisper CLI Directly
```rust
fn transcribe_fast(audio: &Path) -> Result<String> {
    Command::new("whisper")
        .args(&[audio, "--model", "base", "--output_format", "txt"])
        .output()?
    // Read the output file, return it
}
```

### 3. Object Detection: Call YOLO CLI or Use Ultralytics
```rust
fn detect_objects_fast(image: &Path) -> Result<Vec<Detection>> {
    Command::new("yolo")
        .args(&["detect", "predict", "model=yolov8n.pt", "source=", image])
        .output()?
    // Parse YOLO output
}
```

### 4. Strip Everything Else
- **Remove**: Logging (500ms overhead)
- **Remove**: Perceptual hashing (10s overhead)
- **Remove**: JSON serialization (100ms overhead)
- **Remove**: Pretty abstractions (50ms overhead)
- **Remove**: Validation (if it slows things down)

**What's left**: Thin wrapper that calls fastest tools

## The Two Use Cases

### Use Case 1: Fastest Single File
**Strategy**: Call CLIs in parallel
```bash
# Parallel CLI calls (all start immediately)
ffmpeg -i video.mp4 ... &
whisper video.mp4 &
yolo detect video.mp4 &
wait
# All done in max(FFmpeg, Whisper, YOLO) time
```

**Expected**: Match fastest tool (not slower)

### Use Case 2: Bulk Throughput
**Strategy**: GNU parallel + CLIs
```bash
parallel -j 16 "ffmpeg -i {} ...; whisper {} ...; yolo {}" ::: *.mp4
```

**Expected**: N files × fastest tool time / cores

## If CLI Calls Are Ugly

**Then be ugly.** Speed > beauty.

```rust
// Don't care if this is "bad code"
std::process::Command::new("ffmpeg")
    .args(&["-hide_banner", "-loglevel", "error", ...])
    .spawn()?
    .wait()?;
```

**Measure**: If this is 4s and our Rust is 17s, use this.

## Worker Directive (N=192+)

**Task 1**: Replace keyframe extraction with FFmpeg CLI call
- Strip all Rust decoder code
- Call `ffmpeg` binary directly
- Benchmark: Must be ≤ 5s (close to FFmpeg 4s)

**Task 2**: Replace transcription with Whisper CLI call
- Call `whisper` binary directly
- Benchmark: Must match Whisper CLI speed

**Task 3**: Benchmark YOLO - keep if competitive, replace if not

**Task 4**: Remove ALL overhead
- Logging: Compile-time flag (default OFF)
- Perceptual hashing: Remove entirely
- Validation: Minimal only
- Pretty output: Strip it

**Task 5**: Re-benchmark and PROVE speed
- Every operation vs its CLI alternative
- Document: We're X% faster (or match speed)
- If slower: FIX IT or call CLI

## Success Criteria

- Keyframes: ≤ 5s (vs FFmpeg 4s)
- Transcription: ≤ Whisper CLI time
- Object detection: ≤ YOLO CLI time
- Bulk: Process 100 files in <10 minutes
- **No operation is slower than its alternative**

## If This Means Rewriting Everything

**Then rewrite it.** Speed > sunk cost.

We don't need:
- Pretty plugin architecture (if slow)
- Caching (if overhead > benefit)
- Abstractions (if they add latency)

We need:
- Fast CLI calls
- Parallel execution
- Minimal overhead

**Worker: Be ruthless. Delete slow code. Call fast tools. Prove speed.**
