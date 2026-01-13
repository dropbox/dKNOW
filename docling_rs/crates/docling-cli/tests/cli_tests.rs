//! Integration tests for all CLI commands
//!
//! Tests each command with real invocations.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to create a CLI command
fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_docling"))
}

/// Helper to get test corpus path
fn test_corpus_path(subdir: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("test-corpus")
        .join(subdir)
}

// ============ CONVERT COMMAND TESTS ============

#[test]
fn test_convert_help() {
    cli()
        .arg("convert")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Convert documents to markdown"));
}

#[test]
#[ignore = "test-corpus/md directory does not exist - test files never created"]
fn test_convert_markdown_file() {
    let output_dir = TempDir::new().unwrap();
    let output_path = output_dir.path().join("output.md");
    let input_path = test_corpus_path("md").join("duck.md");

    cli()
        .arg("convert")
        .arg(&input_path)
        .arg("-o")
        .arg(&output_path)
        .assert()
        .success();

    assert!(output_path.exists());
}

#[test]
#[ignore = "test-corpus/md directory does not exist - test files never created"]
fn test_convert_dry_run() {
    let input_path = test_corpus_path("md").join("duck.md");

    cli()
        .arg("convert")
        .arg(&input_path)
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains("Would convert"));
}

#[test]
#[ignore = "test-corpus/md directory does not exist - test files never created"]
fn test_convert_force_overwrite() {
    let output_dir = TempDir::new().unwrap();
    let output_path = output_dir.path().join("output.md");
    let input_path = test_corpus_path("md").join("duck.md");

    // Create existing file
    fs::write(&output_path, "existing content").unwrap();

    // Convert with --force should succeed
    cli()
        .arg("convert")
        .arg(&input_path)
        .arg("-o")
        .arg(&output_path)
        .arg("--force")
        .assert()
        .success();
}

#[test]
#[ignore = "test-corpus/md directory does not exist - test files never created"]
fn test_convert_no_clobber() {
    let output_dir = TempDir::new().unwrap();
    let output_path = output_dir.path().join("output.md");
    let input_path = test_corpus_path("md").join("duck.md");

    // Create existing file
    fs::write(&output_path, "existing content").unwrap();

    // Convert with --no-clobber should fail
    cli()
        .arg("convert")
        .arg(&input_path)
        .arg("-o")
        .arg(&output_path)
        .arg("--no-clobber")
        .assert()
        .failure();
}

#[test]
#[ignore = "test-corpus/md directory does not exist - test files never created"]
fn test_convert_quiet_mode() {
    let output_dir = TempDir::new().unwrap();
    let input_path = test_corpus_path("md").join("duck.md");

    cli()
        .arg("-q")
        .arg("convert")
        .arg(&input_path)
        .arg("-o")
        .arg(output_dir.path().join("out.md"))
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

#[test]
#[ignore = "test-corpus/md directory does not exist - test files never created"]
fn test_convert_json_format() {
    let output_dir = TempDir::new().unwrap();
    let output_path = output_dir.path().join("output.json");
    let input_path = test_corpus_path("md").join("duck.md");

    cli()
        .arg("convert")
        .arg(&input_path)
        .arg("-f")
        .arg("json")
        .arg("-o")
        .arg(&output_path)
        .assert()
        .success();

    let content = fs::read_to_string(&output_path).unwrap();
    assert!(content.starts_with('{') || content.starts_with('['));
}

// ============ FORMATS COMMAND TESTS ============

#[test]
fn test_formats_command() {
    cli()
        .arg("formats")
        .assert()
        .success()
        .stdout(predicate::str::contains("pdf"))
        .stdout(predicate::str::contains("docx"))
        .stdout(predicate::str::contains("html"));
}

#[test]
fn test_formats_json_output() {
    cli()
        .arg("formats")
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::starts_with("["));
}

// ============ INFO COMMAND TESTS ============

#[test]
#[ignore = "test-corpus/md directory does not exist - test files never created"]
fn test_info_command() {
    let input_path = test_corpus_path("md").join("duck.md");

    cli()
        .arg("info")
        .arg(&input_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Format:"));
}

#[test]
#[ignore = "test-corpus/md directory does not exist - test files never created"]
fn test_info_json_output() {
    let input_path = test_corpus_path("md").join("duck.md");

    cli()
        .arg("info")
        .arg(&input_path)
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::starts_with("{"));
}

#[test]
#[ignore = "test-corpus/md directory does not exist - test files never created"]
fn test_info_deep_analysis() {
    let input_path = test_corpus_path("md").join("duck.md");

    cli()
        .arg("info")
        .arg(&input_path)
        .arg("--deep")
        .assert()
        .success();
}

// ============ CONFIG COMMAND TESTS ============

#[test]
fn test_config_show() {
    cli().arg("config").arg("show").assert().success();
}

#[test]
fn test_config_init_creates_file() {
    let temp_dir = TempDir::new().unwrap();

    cli()
        .current_dir(temp_dir.path())
        .arg("config")
        .arg("init")
        .assert()
        .success();

    assert!(temp_dir.path().join(".docling.toml").exists());
}

#[test]
fn test_config_path() {
    cli().arg("config").arg("path").assert().success();
}

// ============ BENCHMARK COMMAND TESTS ============

#[test]
fn test_benchmark_help() {
    cli()
        .arg("benchmark")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("performance"));
}

#[test]
#[ignore = "test-corpus/md directory does not exist - test files never created"]
fn test_benchmark_basic() {
    let input_path = test_corpus_path("md").join("duck.md");

    cli()
        .arg("benchmark")
        .arg(&input_path)
        .arg("-n")
        .arg("1")
        .arg("-w")
        .arg("0")
        .assert()
        .success()
        .stdout(predicate::str::contains("Mean:"));
}

#[test]
#[ignore = "test-corpus/md directory does not exist - test files never created"]
fn test_benchmark_json_output() {
    let input_path = test_corpus_path("md").join("duck.md");

    cli()
        .arg("benchmark")
        .arg(&input_path)
        .arg("-n")
        .arg("1")
        .arg("-w")
        .arg("0")
        .arg("-f")
        .arg("json")
        .assert()
        .success()
        .stdout(predicate::str::starts_with("["));
}

// ============ COMPLETION COMMAND TESTS ============

#[test]
fn test_completion_bash() {
    cli()
        .arg("completion")
        .arg("bash")
        .assert()
        .success()
        .stdout(predicate::str::contains("_docling"));
}

#[test]
fn test_completion_zsh() {
    cli()
        .arg("completion")
        .arg("zsh")
        .assert()
        .success()
        .stdout(predicate::str::contains("#compdef"));
}

#[test]
fn test_completion_fish() {
    cli()
        .arg("completion")
        .arg("fish")
        .assert()
        .success()
        .stdout(predicate::str::contains("complete"));
}

// ============ GLOBAL FLAGS TESTS ============

#[test]
fn test_version_flag() {
    cli()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("docling"));
}

#[test]
fn test_help_flag() {
    cli()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Commands:"));
}

#[test]
#[ignore = "test-corpus/md directory does not exist - test files never created"]
fn test_verbose_convert() {
    let output_dir = TempDir::new().unwrap();
    let input_path = test_corpus_path("md").join("duck.md");

    cli()
        .arg("-v")
        .arg("convert")
        .arg(&input_path)
        .arg("-o")
        .arg(output_dir.path().join("out.md"))
        .assert()
        .success();
}
