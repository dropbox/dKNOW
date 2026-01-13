# Test Results Tracking System - Historical Analysis Infrastructure

**Date**: 2025-10-30
**Authority**: USER directive via MANAGER
**Goal**: Comprehensive test history with metadata for analysis

---

## USER REQUIREMENT

> "we want a directory of test results that save CSVs of test results plus ample metadata about the build, date, version, githash, and system state like CPU load for analysis later"

---

## IMPLEMENTATION PLAN

### Directory Structure

```
test_results/
├── 2025-10-30_22-45-13_6a8f2e1/
│   ├── metadata.json              # Build and system info
│   ├── test_results.csv           # Test outcomes and timing
│   ├── performance_summary.json   # Aggregate statistics
│   └── system_snapshot.json       # CPU, memory, disk at test time
├── 2025-10-30_23-12-05_7b9c3f2/
│   ├── metadata.json
│   ├── test_results.csv
│   ├── performance_summary.json
│   └── system_snapshot.json
└── latest -> 2025-10-30_23-12-05_7b9c3f2/  # Symlink to latest
```

---

## File Formats

### 1. metadata.json

```json
{
  "timestamp": "2025-10-30T22:45:13Z",
  "git_hash": "6a8f2e1abc...",
  "git_branch": "build-video-audio-extracts",
  "git_dirty": false,
  "cargo_version": "1.75.0",
  "rustc_version": "1.75.0",
  "build_profile": "release",
  "build_timestamp": "2025-10-30T22:30:00Z",
  "binary_size": 27262976,
  "binary_path": "target/release/video-extract",
  "os": "Darwin",
  "os_version": "24.6.0",
  "arch": "arm64",
  "hostname": "ayates-macbook",
  "test_runner": "cargo test",
  "test_suite": "standard_test_suite",
  "test_count_total": 98,
  "test_thread_count": 1
}
```

### 2. test_results.csv

```csv
test_name,suite,status,duration_secs,error_message,file_path,operation,file_size_bytes
format_mp4_quick_pipeline,format_validation,passed,3.45,,~/Desktop/stuff/video.mp4,keyframes+object-detection,35651584
format_mov_screen_recording,format_validation,passed,4.12,,~/Desktop/stuff/screen.mov,keyframes+object-detection,39845632
characteristic_audio_codec_aac,audio_characteristics,failed,12.65,Performance regression,test_files_local/sample.aac,transcription,149504
edge_case_4k_resolution,edge_cases,passed,2.34,,test_edge_cases/video_4k.mp4,keyframes,158720
...
```

### 3. performance_summary.json

```json
{
  "total_tests": 98,
  "passed": 97,
  "failed": 1,
  "pass_rate": 0.9898,
  "total_duration_secs": 336.75,
  "avg_test_duration": 3.44,
  "fastest_test": {
    "name": "edge_case_single_frame",
    "duration": 0.15
  },
  "slowest_test": {
    "name": "stress_test_1_3gb_video",
    "duration": 89.23
  },
  "failed_tests": [
    {
      "name": "characteristic_audio_codec_aac",
      "duration": 12.65,
      "error": "Performance regression: took 12.65s (expected <2.0s)"
    }
  ],
  "performance_by_operation": {
    "transcription": {"count": 15, "avg_duration": 3.21, "total": 48.15},
    "keyframes": {"count": 42, "avg_duration": 1.89, "total": 79.38},
    "object-detection": {"count": 18, "avg_duration": 4.56, "total": 82.08}
  }
}
```

### 4. system_snapshot.json

```json
{
  "timestamp": "2025-10-30T22:45:13Z",
  "cpu": {
    "model": "Apple M1 Max",
    "cores_physical": 10,
    "cores_logical": 10,
    "load_avg_1min": 2.34,
    "load_avg_5min": 1.89,
    "load_avg_15min": 1.45,
    "usage_percent": 23.5
  },
  "memory": {
    "total_gb": 32.0,
    "available_gb": 18.5,
    "used_gb": 13.5,
    "usage_percent": 42.2
  },
  "disk": {
    "total_gb": 500.0,
    "available_gb": 234.5,
    "used_gb": 265.5,
    "usage_percent": 53.1
  },
  "thermal": {
    "temperature_celsius": 45.2,
    "throttled": false
  },
  "network": {
    "connected": true,
    "dropbox_running": false
  }
}
```

---

## Implementation

### File: tests/test_result_tracker.rs (NEW)

```rust
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use sysinfo::{System, SystemExt, CpuExt};

#[derive(Serialize, Deserialize)]
pub struct TestMetadata {
    pub timestamp: String,
    pub git_hash: String,
    pub git_branch: String,
    pub git_dirty: bool,
    pub cargo_version: String,
    pub rustc_version: String,
    pub build_profile: String,
    pub binary_size: u64,
    pub os: String,
    pub os_version: String,
    pub arch: String,
    pub hostname: String,
}

#[derive(Serialize, Deserialize)]
pub struct TestResultRow {
    pub test_name: String,
    pub suite: String,
    pub status: String,  // "passed" | "failed" | "skipped"
    pub duration_secs: f64,
    pub error_message: Option<String>,
    pub file_path: Option<String>,
    pub operation: String,
    pub file_size_bytes: Option<u64>,
}

#[derive(Serialize, Deserialize)]
pub struct SystemSnapshot {
    pub timestamp: String,
    pub cpu_model: String,
    pub cpu_cores_physical: usize,
    pub cpu_load_1min: f64,
    pub cpu_load_5min: f64,
    pub cpu_load_15min: f64,
    pub cpu_usage_percent: f32,
    pub memory_total_gb: f64,
    pub memory_available_gb: f64,
    pub memory_usage_percent: f32,
    pub disk_available_gb: f64,
    pub thermal_throttled: bool,
}

pub struct TestResultTracker {
    output_dir: PathBuf,
    metadata: TestMetadata,
    system_snapshot: SystemSnapshot,
    test_results: Vec<TestResultRow>,
    start_time: std::time::Instant,
}

impl TestResultTracker {
    pub fn new() -> Result<Self> {
        // Collect metadata
        let git_hash = Self::get_git_hash()?;
        let git_branch = Self::get_git_branch()?;
        let git_dirty = Self::is_git_dirty();

        // Create timestamped directory
        let timestamp = chrono::Utc::now().format("%Y-%m-%d_%H-%M-%S").to_string();
        let dir_name = format!("{}_{}", timestamp, &git_hash[..7]);
        let output_dir = PathBuf::from("test_results").join(dir_name);
        std::fs::create_dir_all(&output_dir)?;

        // Collect system info
        let mut sys = System::new_all();
        sys.refresh_all();

        let system_snapshot = SystemSnapshot {
            timestamp: timestamp.clone(),
            cpu_model: sys.global_cpu_info().brand().to_string(),
            cpu_cores_physical: sys.physical_core_count().unwrap_or(0),
            cpu_load_1min: System::load_average().one,
            cpu_load_5min: System::load_average().five,
            cpu_load_15min: System::load_average().fifteen,
            cpu_usage_percent: sys.global_cpu_info().cpu_usage(),
            memory_total_gb: sys.total_memory() as f64 / 1_000_000_000.0,
            memory_available_gb: sys.available_memory() as f64 / 1_000_000_000.0,
            memory_usage_percent: (sys.used_memory() as f32 / sys.total_memory() as f32) * 100.0,
            disk_available_gb: 0.0,  // Implement if needed
            thermal_throttled: false,  // Implement if available
        };

        Ok(Self {
            output_dir,
            metadata: /* ... */,
            system_snapshot,
            test_results: Vec::new(),
            start_time: Instant::now(),
        })
    }

    fn get_git_hash() -> Result<String> {
        let output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .output()?;
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn get_git_branch() -> Result<String> {
        let output = Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .output()?;
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn is_git_dirty() -> bool {
        Command::new("git")
            .args(["diff", "--quiet"])
            .status()
            .map(|s| !s.success())
            .unwrap_or(false)
    }

    pub fn record_test(&mut self, result: TestResultRow) {
        self.test_results.push(result);
    }

    pub fn save(&self) -> Result<()> {
        // Save metadata.json
        let metadata_json = serde_json::to_string_pretty(&self.metadata)?;
        std::fs::write(self.output_dir.join("metadata.json"), metadata_json)?;

        // Save system_snapshot.json
        let snapshot_json = serde_json::to_string_pretty(&self.system_snapshot)?;
        std::fs::write(self.output_dir.join("system_snapshot.json"), snapshot_json)?;

        // Save test_results.csv
        let mut csv = String::from("test_name,suite,status,duration_secs,error_message,file_path,operation,file_size_bytes\n");
        for result in &self.test_results {
            csv.push_str(&format!(
                "{},{},{},{},{},{},{},{}\n",
                result.test_name,
                result.suite,
                result.status,
                result.duration_secs,
                result.error_message.as_deref().unwrap_or(""),
                result.file_path.as_deref().unwrap_or(""),
                result.operation,
                result.file_size_bytes.unwrap_or(0),
            ));
        }
        std::fs::write(self.output_dir.join("test_results.csv"), csv)?;

        // Save performance_summary.json
        let summary = self.generate_summary();
        let summary_json = serde_json::to_string_pretty(&summary)?;
        std::fs::write(self.output_dir.join("performance_summary.json"), summary_json)?;

        // Update 'latest' symlink
        let latest_link = PathBuf::from("test_results/latest");
        let _ = std::fs::remove_file(&latest_link); // Remove old symlink
        std::os::unix::fs::symlink(&self.output_dir, &latest_link)?;

        Ok(())
    }

    fn generate_summary(&self) -> PerformanceSummary {
        let passed = self.test_results.iter().filter(|t| t.status == "passed").count();
        let failed = self.test_results.iter().filter(|t| t.status == "failed").count();

        // ... aggregate stats ...
    }
}
```

### File: tests/standard_test_suite.rs (MODIFY)

**Add at top:**
```rust
mod test_result_tracker;
use test_result_tracker::TestResultTracker;

static TRACKER: Mutex<Option<TestResultTracker>> = Mutex::new(None);
```

**In each test:**
```rust
#[test]
#[ignore]
fn format_mp4_quick_pipeline() {
    let file = PathBuf::from("...");
    let result = run_video_extract("keyframes,object-detection", &file);

    // Record result
    if let Ok(mut tracker) = TRACKER.lock() {
        if tracker.is_none() {
            *tracker = Some(TestResultTracker::new().unwrap());
        }
        tracker.as_mut().unwrap().record_test(TestResultRow {
            test_name: "format_mp4_quick_pipeline".to_string(),
            suite: "format_validation".to_string(),
            status: if result.passed { "passed" } else { "failed" }.to_string(),
            duration_secs: result.duration_secs,
            error_message: result.error.clone(),
            file_path: Some(file.display().to_string()),
            operation: "keyframes,object-detection".to_string(),
            file_size_bytes: file.metadata().ok().map(|m| m.len()),
        });
    }

    assert!(result.passed, "Test failed");
}
```

**At test suite end:**
```rust
#[test]
fn zzz_save_results() {
    // Runs last (alphabetically)
    if let Ok(tracker) = TRACKER.lock() {
        if let Some(t) = tracker.as_ref() {
            t.save().expect("Failed to save test results");
            println!("📊 Test results saved to: test_results/latest/");
        }
    }
}
```

---

## CSV Format

```csv
test_name,suite,status,duration_secs,error_message,file_path,operation,file_size_bytes,git_hash,timestamp
format_mp4_quick_pipeline,format_validation,passed,3.45,,~/Desktop/video.mp4,keyframes+object-detection,35651584,6a8f2e1,2025-10-30T22:45:13Z
format_mov_screen_recording,format_validation,passed,4.12,,~/Desktop/screen.mov,keyframes+object-detection,39845632,6a8f2e1,2025-10-30T22:45:17Z
characteristic_audio_codec_aac,audio_characteristics,failed,12.65,Performance regression,test_files_local/sample.aac,transcription,149504,6a8f2e1,2025-10-30T22:45:30Z
```

---

## System State Capture

### CPU Load (sysinfo crate)
```rust
use sysinfo::{System, SystemExt};

let mut sys = System::new_all();
sys.refresh_all();

let cpu_load = System::load_average();
// load_avg_1min, load_avg_5min, load_avg_15min
```

### Memory (sysinfo crate)
```rust
let memory_total = sys.total_memory();
let memory_used = sys.used_memory();
let memory_available = sys.available_memory();
```

### Git Info
```bash
git rev-parse HEAD                    # Full hash
git rev-parse --short HEAD            # Short hash
git rev-parse --abbrev-ref HEAD       # Branch
git diff --quiet || echo "dirty"      # Dirty flag
git log -1 --format="%ci"             # Commit timestamp
```

### Build Info
```bash
rustc --version                       # Rust version
cargo --version                       # Cargo version
stat target/release/video-extract     # Binary size and timestamp
```

---

## Analysis Use Cases

**With this data, you can:**

1. **Track performance over time:**
```bash
# Compare test durations across commits
grep "format_mp4_quick_pipeline" test_results/*/test_results.csv
```

2. **Find regressions:**
```bash
# Plot duration trends
cat test_results/*/test_results.csv | grep "characteristic_audio_codec_aac"
```

3. **Correlate with system state:**
```bash
# Check if CPU load affects performance
jq '.cpu.load_avg_1min' test_results/*/system_snapshot.json
```

4. **Historical comparisons:**
```bash
# Compare specific commit vs current
diff test_results/2025-10-30_20-00-00_abc1234/test_results.csv \
     test_results/2025-10-30_22-45-13_6a8f2e1/test_results.csv
```

5. **Generate reports:**
```bash
# Create performance dashboard
python3 analyze_test_results.py test_results/
```

---

## Dependencies Needed

```toml
[dev-dependencies]
sysinfo = "0.30"    # System information
chrono = "0.4"      # Timestamps
csv = "1.3"         # CSV writing
```

---

## WORKER N=55 INSTRUCTIONS

**After fixing AAC regression (N=54):**

1. Create `tests/test_result_tracker.rs` module
2. Add sysinfo, chrono, csv dependencies
3. Modify standard_test_suite.rs to record results
4. Create test_results/ directory structure
5. Run test suite to generate first tracked results
6. Verify CSV and JSON files created
7. Commit with example output

**Estimated**: 2-3 commits (~3-4 hours)

**Success criteria:**
- test_results/ directory created
- CSV contains all test results
- metadata.json has git hash, version, etc.
- system_snapshot.json has CPU/memory at test time
- Latest symlink points to newest results

---

## Expected Outcome

**After implementation:**
```bash
$ ls test_results/
2025-10-30_22-45-13_6a8f2e1/
2025-10-30_23-12-05_7b9c3f2/
2025-10-31_08-30-22_8c1d4a3/
latest -> 2025-10-31_08-30-22_8c1d4a3/

$ cat test_results/latest/performance_summary.json
{
  "total_tests": 98,
  "passed": 98,
  "pass_rate": 1.0,
  "total_duration_secs": 328.45
}

$ head test_results/latest/test_results.csv
test_name,suite,status,duration_secs,error_message,...
format_mp4_quick_pipeline,format_validation,passed,3.45,,...
```

**User can now analyze test performance trends over time!**
