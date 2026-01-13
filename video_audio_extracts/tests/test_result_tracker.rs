use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use sysinfo::System;

#[derive(Serialize, Deserialize, Debug)]
pub struct TestMetadata {
    pub timestamp: String,
    pub git_hash: String,
    pub git_branch: String,
    pub git_dirty: bool,
    pub cargo_version: String,
    pub rustc_version: String,
    pub build_profile: String,
    pub build_timestamp: String,
    pub binary_size: u64,
    pub binary_path: String,
    pub os: String,
    pub os_version: String,
    pub arch: String,
    pub hostname: String,
    pub test_runner: String,
    pub test_suite: String,
    pub test_count_total: usize,
    pub test_thread_count: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TestResultRow {
    pub test_name: String,
    pub suite: String,
    pub status: String, // "passed" | "failed" | "skipped"
    pub duration_secs: f64,
    pub error_message: Option<String>,
    pub file_path: Option<String>,
    pub operation: String,
    pub file_size_bytes: Option<u64>,
    pub output_md5_hash: Option<String>,
    pub output_metadata_json: Option<String>, // Comprehensive JSON metadata with type_specific fields
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SystemSnapshot {
    pub timestamp: String,
    pub cpu: CpuInfo,
    pub memory: MemoryInfo,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CpuInfo {
    pub model: String,
    pub cores_physical: usize,
    pub cores_logical: usize,
    pub load_avg_1min: f64,
    pub load_avg_5min: f64,
    pub load_avg_15min: f64,
    pub usage_percent: f32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MemoryInfo {
    pub total_gb: f64,
    pub available_gb: f64,
    pub used_gb: f64,
    pub usage_percent: f32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PerformanceSummary {
    pub total_tests: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub pass_rate: f64,
    pub total_duration_secs: f64,
    pub avg_test_duration: f64,
    pub fastest_test: Option<TestSummary>,
    pub slowest_test: Option<TestSummary>,
    pub failed_tests: Vec<FailedTest>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TestSummary {
    pub name: String,
    pub duration: f64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FailedTest {
    pub name: String,
    pub duration: f64,
    pub error: String,
}

pub struct TestResultTracker {
    output_dir: PathBuf,
    metadata: TestMetadata,
    system_snapshot: SystemSnapshot,
    test_results: Vec<TestResultRow>,
    #[allow(dead_code)]
    start_time: std::time::Instant,
}

impl TestResultTracker {
    pub fn new() -> anyhow::Result<Self> {
        let timestamp = chrono::Utc::now().format("%Y-%m-%d_%H-%M-%S").to_string();

        // Collect git info
        let git_hash = Self::get_git_hash()?;
        let git_branch = Self::get_git_branch()?;
        let git_dirty = Self::is_git_dirty();

        // Collect version info
        let cargo_version = Self::get_cargo_version()?;
        let rustc_version = Self::get_rustc_version()?;

        // Binary info
        let binary_path = "target/release/video-extract".to_string();
        let binary_metadata = std::fs::metadata(&binary_path).ok();
        let binary_size = binary_metadata.as_ref().map(|m| m.len()).unwrap_or(0);
        let build_timestamp = binary_metadata
            .as_ref()
            .and_then(|m| m.modified().ok())
            .map(|t| {
                let dt: chrono::DateTime<chrono::Utc> = t.into();
                dt.format("%Y-%m-%dT%H:%M:%SZ").to_string()
            })
            .unwrap_or_else(|| "unknown".to_string());

        // System info
        let hostname = hostname::get()
            .ok()
            .and_then(|h| h.into_string().ok())
            .unwrap_or_else(|| "unknown".to_string());

        // Collect system snapshot
        let mut sys = System::new_all();
        sys.refresh_all();

        let load_avg = System::load_average();
        let cpu_model = sys
            .cpus()
            .first()
            .map(|cpu| cpu.brand().to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        let system_snapshot = SystemSnapshot {
            timestamp: timestamp.clone(),
            cpu: CpuInfo {
                model: cpu_model,
                cores_physical: sys.physical_core_count().unwrap_or(0),
                cores_logical: sys.cpus().len(),
                load_avg_1min: load_avg.one,
                load_avg_5min: load_avg.five,
                load_avg_15min: load_avg.fifteen,
                usage_percent: sys.cpus().iter().map(|cpu| cpu.cpu_usage()).sum::<f32>()
                    / sys.cpus().len() as f32,
            },
            memory: MemoryInfo {
                total_gb: sys.total_memory() as f64 / 1_000_000_000.0,
                available_gb: sys.available_memory() as f64 / 1_000_000_000.0,
                used_gb: sys.used_memory() as f64 / 1_000_000_000.0,
                usage_percent: (sys.used_memory() as f32 / sys.total_memory() as f32) * 100.0,
            },
        };

        let metadata = TestMetadata {
            timestamp: timestamp.clone(),
            git_hash: git_hash.clone(),
            git_branch,
            git_dirty,
            cargo_version,
            rustc_version,
            build_profile: "release".to_string(),
            build_timestamp,
            binary_size,
            binary_path,
            os: std::env::consts::OS.to_string(),
            os_version: Self::get_os_version(),
            arch: std::env::consts::ARCH.to_string(),
            hostname,
            test_runner: "cargo test".to_string(),
            test_suite: "standard_test_suite".to_string(),
            test_count_total: 0, // Will be updated when saving
            test_thread_count: 1,
        };

        // Create output directory
        let dir_name = format!("{}_{}", timestamp, &git_hash[..7]);
        let output_dir = PathBuf::from("test_results").join(dir_name);
        std::fs::create_dir_all(&output_dir)?;

        Ok(Self {
            output_dir,
            metadata,
            system_snapshot,
            test_results: Vec::new(),
            start_time: std::time::Instant::now(),
        })
    }

    fn get_git_hash() -> anyhow::Result<String> {
        let output = Command::new("git").args(["rev-parse", "HEAD"]).output()?;
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn get_git_branch() -> anyhow::Result<String> {
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

    fn get_cargo_version() -> anyhow::Result<String> {
        let output = Command::new("cargo").args(["--version"]).output()?;
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn get_rustc_version() -> anyhow::Result<String> {
        let output = Command::new("rustc").args(["--version"]).output()?;
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn get_os_version() -> String {
        if cfg!(target_os = "macos") {
            Command::new("sw_vers")
                .arg("-productVersion")
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| "unknown".to_string())
        } else {
            "unknown".to_string()
        }
    }

    pub fn record_test(&mut self, result: TestResultRow) {
        self.test_results.push(result);
    }

    pub fn save(&mut self) -> anyhow::Result<PathBuf> {
        // Update total test count
        self.metadata.test_count_total = self.test_results.len();

        // Save metadata.json
        let metadata_json = serde_json::to_string_pretty(&self.metadata)?;
        std::fs::write(self.output_dir.join("metadata.json"), metadata_json)?;

        // Save system_snapshot.json
        let snapshot_json = serde_json::to_string_pretty(&self.system_snapshot)?;
        std::fs::write(self.output_dir.join("system_snapshot.json"), snapshot_json)?;

        // Save test_results.csv
        let mut wtr = csv::Writer::from_path(self.output_dir.join("test_results.csv"))?;
        for result in &self.test_results {
            wtr.serialize(result)?;
        }
        wtr.flush()?;

        // Generate and save performance_summary.json
        let summary = self.generate_summary();
        let summary_json = serde_json::to_string_pretty(&summary)?;
        std::fs::write(
            self.output_dir.join("performance_summary.json"),
            summary_json,
        )?;

        // Capture outputs if debug_output directory exists
        self.capture_outputs()?;

        // Update 'latest' symlink
        let latest_link = PathBuf::from("test_results/latest");
        let _ = std::fs::remove_file(&latest_link); // Ignore error if doesn't exist

        #[cfg(unix)]
        {
            // Use relative path for symlink
            let dir_name = self.output_dir.file_name().unwrap();
            std::os::unix::fs::symlink(dir_name, &latest_link)?;
        }

        Ok(self.output_dir.clone())
    }

    fn generate_summary(&self) -> PerformanceSummary {
        let passed = self
            .test_results
            .iter()
            .filter(|t| t.status == "passed")
            .count();
        let failed = self
            .test_results
            .iter()
            .filter(|t| t.status == "failed")
            .count();
        let skipped = self
            .test_results
            .iter()
            .filter(|t| t.status == "skipped")
            .count();
        let total = self.test_results.len();

        let total_duration: f64 = self.test_results.iter().map(|t| t.duration_secs).sum();
        let avg_duration = if total > 0 {
            total_duration / total as f64
        } else {
            0.0
        };

        let pass_rate = if total > 0 {
            passed as f64 / total as f64
        } else {
            0.0
        };

        let fastest_test = self
            .test_results
            .iter()
            .min_by(|a, b| a.duration_secs.partial_cmp(&b.duration_secs).unwrap())
            .map(|t| TestSummary {
                name: t.test_name.clone(),
                duration: t.duration_secs,
            });

        let slowest_test = self
            .test_results
            .iter()
            .max_by(|a, b| a.duration_secs.partial_cmp(&b.duration_secs).unwrap())
            .map(|t| TestSummary {
                name: t.test_name.clone(),
                duration: t.duration_secs,
            });

        let failed_tests = self
            .test_results
            .iter()
            .filter(|t| t.status == "failed")
            .map(|t| FailedTest {
                name: t.test_name.clone(),
                duration: t.duration_secs,
                error: t.error_message.clone().unwrap_or_default(),
            })
            .collect();

        PerformanceSummary {
            total_tests: total,
            passed,
            failed,
            skipped,
            pass_rate,
            total_duration_secs: total_duration,
            avg_test_duration: avg_duration,
            fastest_test,
            slowest_test,
            failed_tests,
        }
    }

    fn capture_outputs(&self) -> anyhow::Result<()> {
        let outputs_dir = self.output_dir.join("outputs");
        let debug_output_dir = PathBuf::from("debug_output");

        // Only capture if debug_output exists
        if !debug_output_dir.exists() {
            return Ok(());
        }

        std::fs::create_dir_all(&outputs_dir)?;

        // For each test, capture its outputs
        for test_result in &self.test_results {
            let test_output_dir = outputs_dir.join(&test_result.test_name);
            std::fs::create_dir_all(&test_output_dir)?;

            // Copy JSON files (small, useful for review)
            for entry in std::fs::read_dir(&debug_output_dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.extension().and_then(|e| e.to_str()) == Some("json") {
                    let dest = test_output_dir.join(path.file_name().unwrap());
                    std::fs::copy(&path, dest)?;
                }
            }

            // Generate checksums for large files (JPEGs, audio, video)
            let mut checksums = String::new();
            for entry in std::fs::read_dir(&debug_output_dir)? {
                let entry = entry?;
                let path = entry.path();

                let ext = path.extension().and_then(|e| e.to_str());
                if matches!(
                    ext,
                    Some("jpg") | Some("jpeg") | Some("wav") | Some("mp4") | Some("mov")
                ) {
                    let hash = Self::sha256_file(&path)?;
                    let size = path.metadata()?.len();
                    let filename = path.file_name().unwrap().to_string_lossy();
                    checksums.push_str(&format!("{}  {}  {}\n", hash, size, filename));
                }
            }

            if !checksums.is_empty() {
                std::fs::write(test_output_dir.join("checksums.txt"), checksums)?;
            }
        }

        Ok(())
    }

    fn sha256_file(path: &Path) -> anyhow::Result<String> {
        let mut file = File::open(path)?;
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 8192];

        loop {
            let n = file.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
        }

        Ok(format!("{:x}", hasher.finalize()))
    }
}
