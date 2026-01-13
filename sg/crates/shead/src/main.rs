//! shead - Show one-line file summaries
//!
//! Like `head -n 1` but works for any file type:
//! - Text files: shows first non-empty line
//! - Binary files: shows xattr summary or MIME type

use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;

const XATTR_KEY: &str = "user.sg.summary";
const MAX_SUMMARY_LEN: usize = 200;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: shead <file> [file2 ...]");
        std::process::exit(1);
    }

    let multiple = args.len() > 2;

    for path_str in &args[1..] {
        let path = Path::new(path_str);

        if !path.exists() {
            eprintln!("shead: {path_str}: No such file");
            continue;
        }

        match get_summary(path) {
            Ok(summary) => {
                if multiple {
                    println!("{path_str}: {summary}");
                } else {
                    println!("{summary}");
                }
            }
            Err(e) => {
                eprintln!("shead: {path_str}: {e}");
            }
        }
    }
}

fn get_summary(path: &Path) -> Result<String, String> {
    // Check if text file
    if is_text_file(path) {
        return read_first_line(path);
    }

    // Binary file: try xattr first
    if let Some(summary) = read_xattr(path) {
        return Ok(summary);
    }

    // Fallback: MIME type detection
    Ok(detect_mime(path))
}

fn is_text_file(path: &Path) -> bool {
    // Check by extension first (fast path)
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        let text_exts = [
            "txt", "md", "rs", "py", "js", "ts", "jsx", "tsx", "c", "h", "cpp", "hpp",
            "java", "go", "rb", "sh", "bash", "zsh", "fish", "yaml", "yml", "json",
            "toml", "xml", "html", "css", "scss", "sass", "less", "sql", "swift",
            "kt", "scala", "pl", "pm", "lua", "vim", "el", "lisp", "clj", "hs",
            "ml", "fs", "ex", "exs", "erl", "r", "jl", "nim", "zig", "v", "d",
            "ada", "pas", "f90", "f95", "cob", "asm", "s", "makefile", "cmake",
            "dockerfile", "gitignore", "editorconfig", "env", "ini", "cfg", "conf",
            "log", "csv", "tsv", "rst", "tex", "bib", "org", "adoc", "diff", "patch",
        ];
        let ext_lower = ext.to_lowercase();
        if text_exts.contains(&ext_lower.as_str()) {
            return true;
        }
    }

    // Check by reading first bytes
    if let Ok(mut file) = File::open(path) {
        let mut buffer = [0u8; 512];
        if let Ok(n) = file.read(&mut buffer) {
            // Check for binary content (null bytes or high ratio of non-printable)
            let non_text = buffer[..n].iter().filter(|&&b| {
                b == 0 || (b < 32 && b != 9 && b != 10 && b != 13)
            }).count();
            return non_text == 0 || (non_text as f64 / n as f64) < 0.1;
        }
    }

    false
}

fn read_first_line(path: &Path) -> Result<String, String> {
    let file = File::open(path).map_err(|e| e.to_string())?;
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line = line.map_err(|e| e.to_string())?;
        let trimmed = line.trim();

        if trimmed.is_empty() {
            continue;
        }

        // Strip common comment prefixes
        let cleaned = strip_comment_prefix(trimmed);

        // Truncate if needed
        return Ok(if cleaned.len() > MAX_SUMMARY_LEN {
            format!("{}...", &cleaned[..MAX_SUMMARY_LEN - 3])
        } else {
            cleaned.to_string()
        });
    }

    Ok("[empty file]".to_string())
}

fn strip_comment_prefix(line: &str) -> &str {
    let line = line.trim();

    // Doc comments
    if let Some(rest) = line.strip_prefix("///") { return rest.trim(); }
    if let Some(rest) = line.strip_prefix("//!") { return rest.trim(); }
    if let Some(rest) = line.strip_prefix("//") { return rest.trim(); }

    // Hash comments (but not shebang)
    if line.starts_with('#') && !line.starts_with("#!") {
        if let Some(rest) = line.strip_prefix('#') { return rest.trim(); }
    }

    // Other comment styles
    if let Some(rest) = line.strip_prefix("--") { return rest.trim(); }
    if let Some(rest) = line.strip_prefix("/*") {
        let rest = rest.trim();
        if let Some(r) = rest.strip_prefix('*') { return r.trim(); }
        return rest;
    }

    // Python docstrings
    if let Some(rest) = line.strip_prefix("\"\"\"") { return rest.trim(); }
    if let Some(rest) = line.strip_prefix("'''") { return rest.trim(); }

    line
}

#[cfg(unix)]
fn read_xattr(path: &Path) -> Option<String> {
    match xattr::get(path, XATTR_KEY) {
        Ok(Some(data)) => String::from_utf8(data).ok(),
        _ => None,
    }
}

#[cfg(not(unix))]
fn read_xattr(_path: &Path) -> Option<String> {
    None
}

fn detect_mime(path: &Path) -> String {
    if let Ok(mut file) = File::open(path) {
        let mut buffer = [0u8; 8192];
        if let Ok(n) = file.read(&mut buffer) {
            if let Some(kind) = infer::get(&buffer[..n]) {
                let ext = path.extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("bin");
                return format!("{} ({})", kind.mime_type(), ext.to_uppercase());
            }
        }
    }

    let ext = path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("binary");
    format!("Binary file ({})", ext.to_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_strip_comment_prefix_rust_doc() {
        assert_eq!(strip_comment_prefix("/// Hello world"), "Hello world");
        assert_eq!(strip_comment_prefix("//! Module doc"), "Module doc");
        assert_eq!(strip_comment_prefix("// Comment"), "Comment");
    }

    #[test]
    fn test_strip_comment_prefix_hash() {
        assert_eq!(strip_comment_prefix("# Python comment"), "Python comment");
        // Shebang should NOT be stripped
        assert_eq!(strip_comment_prefix("#!/bin/bash"), "#!/bin/bash");
    }

    #[test]
    fn test_strip_comment_prefix_c_style() {
        assert_eq!(strip_comment_prefix("/* C comment"), "C comment");
        assert_eq!(strip_comment_prefix("/** Javadoc"), "Javadoc");
    }

    #[test]
    fn test_strip_comment_prefix_sql() {
        assert_eq!(strip_comment_prefix("-- SQL comment"), "SQL comment");
    }

    #[test]
    fn test_strip_comment_prefix_python_docstring() {
        assert_eq!(strip_comment_prefix("\"\"\"Docstring"), "Docstring");
        assert_eq!(strip_comment_prefix("'''Single quote docstring"), "Single quote docstring");
    }

    #[test]
    fn test_strip_comment_prefix_plain_text() {
        assert_eq!(strip_comment_prefix("No prefix here"), "No prefix here");
    }

    #[test]
    fn test_is_text_file_by_extension() {
        assert!(is_text_file(Path::new("test.rs")));
        assert!(is_text_file(Path::new("test.py")));
        assert!(is_text_file(Path::new("test.md")));
        assert!(is_text_file(Path::new("test.json")));
    }

    #[test]
    fn test_read_first_line() {
        let dir = std::env::temp_dir();
        let path = dir.join("shead_test.txt");

        {
            let mut file = File::create(&path).unwrap();
            writeln!(file).unwrap();
            writeln!(file, "  ").unwrap();
            writeln!(file, "/// First meaningful line").unwrap();
            writeln!(file, "Second line").unwrap();
        }

        let result = read_first_line(&path).unwrap();
        assert_eq!(result, "First meaningful line");

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_read_first_line_empty_file() {
        let dir = std::env::temp_dir();
        let path = dir.join("shead_test_empty.txt");

        File::create(&path).unwrap();

        let result = read_first_line(&path).unwrap();
        assert_eq!(result, "[empty file]");

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_read_first_line_truncation() {
        let dir = std::env::temp_dir();
        let path = dir.join("shead_test_long.txt");

        {
            let mut file = File::create(&path).unwrap();
            let long_line = "x".repeat(300);
            writeln!(file, "{long_line}").unwrap();
        }

        let result = read_first_line(&path).unwrap();
        assert!(result.len() <= MAX_SUMMARY_LEN);
        assert!(result.ends_with("..."));

        std::fs::remove_file(&path).ok();
    }
}
