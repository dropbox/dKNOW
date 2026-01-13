//! Integration tests for batch command
//!
//! Tests the `docling batch` CLI command with real files.

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

#[test]
#[ignore = "test-corpus/md directory does not exist - test files never created"]
fn test_batch_basic_markdown_files() {
    let output_dir = TempDir::new().unwrap();
    let md_dir = test_corpus_path("md");

    // Select a few markdown files
    let inputs = vec![
        md_dir.join("duck.md"),
        md_dir.join("nested.md"),
        md_dir.join("wiki.md"),
    ];

    // Run batch command
    let mut cmd = cli();
    cmd.arg("batch")
        .args(&inputs)
        .arg("-o")
        .arg(output_dir.path());

    cmd.assert()
        .success()
        .stderr(predicate::str::contains("Processing 3 files"))
        .stderr(predicate::str::contains("Succeeded:       3"));

    // Verify output files exist
    assert!(output_dir.path().join("duck.md").exists());
    assert!(output_dir.path().join("nested.md").exists());
    assert!(output_dir.path().join("wiki.md").exists());

    // Verify files are not empty
    let content = fs::read_to_string(output_dir.path().join("duck.md")).unwrap();
    assert!(!content.is_empty());
}

#[test]
#[ignore = "test-corpus/md directory does not exist - test files never created"]
fn test_batch_glob_pattern() {
    let output_dir = TempDir::new().unwrap();
    let md_dir = test_corpus_path("md");

    // Use glob pattern to select markdown files
    let pattern = format!("{}/*.md", md_dir.display());

    // Run batch command with glob pattern
    let mut cmd = cli();
    cmd.arg("batch")
        .arg(&pattern)
        .arg("-o")
        .arg(output_dir.path());

    cmd.assert()
        .success()
        .stderr(predicate::str::contains("Processing 9 files")) // 9 .md files in corpus
        .stderr(predicate::str::contains("Succeeded:       9"));

    // Verify some output files exist
    assert!(output_dir.path().join("duck.md").exists());
    assert!(output_dir.path().join("wiki.md").exists());
    assert!(output_dir.path().join("blocks.md").exists());
}

#[test]
#[ignore = "test-corpus/md directory does not exist - test files never created"]
fn test_batch_json_output_format() {
    let output_dir = TempDir::new().unwrap();
    let md_dir = test_corpus_path("md");

    let inputs = vec![md_dir.join("duck.md"), md_dir.join("nested.md")];

    // Run batch with JSON format
    let mut cmd = cli();
    cmd.arg("batch")
        .args(&inputs)
        .arg("-o")
        .arg(output_dir.path())
        .arg("--format")
        .arg("json");

    cmd.assert()
        .success()
        .stderr(predicate::str::contains("Succeeded:       2"));

    // Verify JSON files exist
    assert!(output_dir.path().join("duck.json").exists());
    assert!(output_dir.path().join("nested.json").exists());

    // Verify JSON is valid
    let content = fs::read_to_string(output_dir.path().join("duck.json")).unwrap();
    let _: serde_json::Value = serde_json::from_str(&content).unwrap();
}

#[test]
#[ignore = "test-corpus/md directory does not exist - test files never created"]
fn test_batch_yaml_output_format() {
    let output_dir = TempDir::new().unwrap();
    let md_dir = test_corpus_path("md");

    let inputs = vec![md_dir.join("duck.md")];

    // Run batch with YAML format
    let mut cmd = cli();
    cmd.arg("batch")
        .args(&inputs)
        .arg("-o")
        .arg(output_dir.path())
        .arg("--format")
        .arg("yaml");

    cmd.assert()
        .success()
        .stderr(predicate::str::contains("Succeeded:       1"));

    // Verify YAML file exists
    assert!(output_dir.path().join("duck.yaml").exists());

    // Verify YAML is valid
    let content = fs::read_to_string(output_dir.path().join("duck.yaml")).unwrap();
    let _: serde_yaml::Value = serde_yaml::from_str(&content).unwrap();
}

#[test]
#[ignore = "test-corpus/md directory does not exist - test files never created"]
fn test_batch_compact_json() {
    let output_dir = TempDir::new().unwrap();
    let md_dir = test_corpus_path("md");

    let inputs = vec![md_dir.join("duck.md")];

    // Run batch with compact JSON
    let mut cmd = cli();
    cmd.arg("batch")
        .args(&inputs)
        .arg("-o")
        .arg(output_dir.path())
        .arg("--format")
        .arg("json")
        .arg("--compact");

    cmd.assert().success();

    // Verify JSON is compact (no indentation)
    let content = fs::read_to_string(output_dir.path().join("duck.json")).unwrap();
    let _: serde_json::Value = serde_json::from_str(&content).unwrap();

    // Compact JSON should not have many newlines
    let newline_count = content.chars().filter(|c| *c == '\n').count();
    assert!(
        newline_count < 10,
        "Compact JSON should have minimal newlines"
    );
}

#[test]
#[ignore = "test-corpus/md directory does not exist - test files never created"]
fn test_batch_continue_on_error() {
    let output_dir = TempDir::new().unwrap();
    let md_dir = test_corpus_path("md");

    // Mix valid and invalid files
    let inputs = vec![
        md_dir.join("duck.md"),
        PathBuf::from("/nonexistent/file.md"), // This will fail
        md_dir.join("wiki.md"),
    ];

    // Run batch with continue-on-error
    let mut cmd = cli();
    cmd.arg("batch")
        .args(&inputs)
        .arg("-o")
        .arg(output_dir.path())
        .arg("--continue-on-error");

    cmd.assert()
        .success() // Should succeed despite errors
        .stderr(predicate::str::contains("Failed:          1"))
        .stderr(predicate::str::contains("Succeeded:       2"));

    // Verify successful conversions exist
    assert!(output_dir.path().join("duck.md").exists());
    assert!(output_dir.path().join("wiki.md").exists());
    assert!(!output_dir.path().join("file.md").exists());
}

#[test]
#[ignore = "test-corpus/md directory does not exist - test files never created"]
fn test_batch_fail_on_first_error() {
    let output_dir = TempDir::new().unwrap();
    let md_dir = test_corpus_path("md");

    // Mix valid and invalid files
    let inputs = vec![
        md_dir.join("duck.md"),
        PathBuf::from("/nonexistent/file.md"), // This will fail
        md_dir.join("wiki.md"),                // Should not be processed
    ];

    // Run batch WITHOUT continue-on-error (default behavior)
    // Default is sequential execution (parallel=1) for fail-fast behavior
    let mut cmd = cli();
    cmd.arg("batch")
        .args(&inputs)
        .arg("-o")
        .arg(output_dir.path());

    cmd.assert()
        .failure() // Should fail on first error
        .stderr(predicate::str::contains("✗")); // Error marker

    // First file should succeed
    assert!(output_dir.path().join("duck.md").exists());
    // Third file should not be processed
    assert!(!output_dir.path().join("wiki.md").exists());
}

#[test]
fn test_batch_empty_input() {
    let output_dir = TempDir::new().unwrap();

    // Run batch with no matching files
    let mut cmd = cli();
    cmd.arg("batch")
        .arg("/nonexistent/*.md")
        .arg("-o")
        .arg(output_dir.path());

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("No input files found"));
}

#[test]
#[ignore = "test-corpus/md directory does not exist - test files never created"]
fn test_batch_creates_output_directory() {
    let temp_root = TempDir::new().unwrap();
    let output_dir = temp_root.path().join("new_output_dir");
    let md_dir = test_corpus_path("md");

    let inputs = vec![md_dir.join("duck.md")];

    // Output directory does not exist yet
    assert!(!output_dir.exists());

    // Run batch command
    let mut cmd = cli();
    cmd.arg("batch").args(&inputs).arg("-o").arg(&output_dir);

    cmd.assert().success();

    // Output directory should be created
    assert!(output_dir.exists());
    assert!(output_dir.is_dir());
    assert!(output_dir.join("duck.md").exists());
}

#[test]
#[ignore = "test-corpus/md directory does not exist - test files never created"]
fn test_batch_output_is_file_not_directory() {
    let temp_root = TempDir::new().unwrap();
    let output_file = temp_root.path().join("output.txt");
    let md_dir = test_corpus_path("md");

    // Create a regular file at output path
    fs::write(&output_file, "test").unwrap();

    let inputs = vec![md_dir.join("duck.md")];

    // Run batch command with file as output (should fail)
    let mut cmd = cli();
    cmd.arg("batch").args(&inputs).arg("-o").arg(&output_file);

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("not a directory"));
}

#[test]
#[ignore = "Progress bars don't output to stderr in non-TTY environments"]
fn test_batch_progress_reporting() {
    let output_dir = TempDir::new().unwrap();
    let md_dir = test_corpus_path("md");

    let inputs = vec![
        md_dir.join("duck.md"),
        md_dir.join("nested.md"),
        md_dir.join("wiki.md"),
    ];

    // Run batch command
    let mut cmd = cli();
    cmd.arg("batch")
        .args(&inputs)
        .arg("-o")
        .arg(output_dir.path());

    let output = cmd.assert().success();

    // Check progress indicators are present
    let stderr = String::from_utf8_lossy(&output.get_output().stderr);

    // Should show [1/3], [2/3], [3/3]
    assert!(stderr.contains("[1/3]"));
    assert!(stderr.contains("[2/3]"));
    assert!(stderr.contains("[3/3]"));

    // Should show check marks for success
    assert!(stderr.contains("✓"));

    // Should show summary
    assert!(stderr.contains("Batch Conversion Summary"));
    assert!(stderr.contains("Total files:     3"));
    assert!(stderr.contains("Succeeded:       3"));
}

#[test]
fn test_batch_help_text() {
    let mut cmd = cli();
    cmd.arg("batch").arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Convert multiple documents"))
        .stdout(predicate::str::contains("--continue-on-error"))
        .stdout(predicate::str::contains("--format"));
}

#[test]
#[ignore = "Requires test corpus PDF files to be set up"]
fn test_batch_with_pdf_files() {
    let output_dir = TempDir::new().unwrap();
    let pdf_dir = test_corpus_path("pdf");

    // Use small PDF files for faster testing
    let inputs = vec![
        pdf_dir.join("2305.03393v1-pg9.pdf"),
        pdf_dir.join("multi_page.pdf"),
    ];

    // Run batch command
    let mut cmd = cli();
    cmd.arg("batch")
        .args(&inputs)
        .arg("-o")
        .arg(output_dir.path());

    cmd.assert()
        .success()
        .stderr(predicate::str::contains("Succeeded:       2"));

    // Verify output files exist
    assert!(output_dir.path().join("2305.03393v1-pg9.md").exists());
    assert!(output_dir.path().join("multi_page.md").exists());
}
