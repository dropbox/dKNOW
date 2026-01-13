use anyhow::{Context, Result};
use regex::Regex;
use serde::Deserialize;
use sg_core::chunk_document;
use sg_core::code_preprocessor::{is_code_file, preprocess_code};
use sg_core::embedder::EmbedderBackend;
use sg_core::file_types::is_indexable_path;
use sg_core::{search_backend, LLMReranker, RerankOptions, Reranker, SearchOptions, DB};
use std::collections::HashSet;
use std::fs;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
pub struct EvalSpec {
    pub corpus: String,
    #[serde(default)]
    pub description: Option<String>,
    pub queries: Vec<EvalQuery>,
    #[serde(default)]
    pub include_extensions: Option<Vec<String>>,
    #[serde(default)]
    pub include_globs: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct EvalQuery {
    pub query: String,
    pub relevant: Vec<String>,
    /// Optional description of what the query tests (for documentation in JSON)
    #[serde(default)]
    #[allow(dead_code)]
    pub description: Option<String>,
}

pub struct EvalSpecFile {
    pub path: PathBuf,
    pub spec: EvalSpec,
}

pub struct EvalSummary {
    pub total_queries: usize,
    pub p_at_1: f32,
    pub mrr: f32,
    pub hits_at_1: usize,
}

struct EvalFilters {
    include_extensions: Option<HashSet<String>>,
    include_globs: Option<Vec<Regex>>,
}

pub fn discover_spec_paths() -> Result<Vec<PathBuf>> {
    let cwd = std::env::current_dir().context("Failed to resolve current directory")?;
    let eval_dir = cwd.join("eval");
    if !eval_dir.is_dir() {
        return Err(anyhow::anyhow!(
            "Eval directory not found at {}",
            eval_dir.display()
        ));
    }

    let mut paths = Vec::new();
    for entry in fs::read_dir(&eval_dir).context("Failed to read eval directory")? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.ends_with("_queries.json") {
                paths.push(path);
            }
        }
    }

    paths.sort();
    Ok(paths)
}

pub fn load_specs(paths: &[PathBuf]) -> Result<Vec<EvalSpecFile>> {
    let mut specs = Vec::new();
    for path in paths {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        let spec: EvalSpec = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse {}", path.display()))?;
        specs.push(EvalSpecFile {
            path: path.clone(),
            spec,
        });
    }
    Ok(specs)
}

pub fn run_eval_spec<E: EmbedderBackend>(
    spec_file: &EvalSpecFile,
    embedder: &mut E,
    verbose: bool,
    hybrid: bool,
    auto_hybrid: bool,
    rerank: bool,
) -> Result<EvalSummary> {
    let corpus_root = resolve_corpus_root(&spec_file.path, &spec_file.spec)?;
    let filters = build_filters(&spec_file.spec)?;
    let files = collect_corpus_files(&corpus_root, &filters)?;

    let db = DB::in_memory().context("Failed to create in-memory DB")?;
    index_files(&db, embedder, &files)?;

    // Initialize reranker if requested
    let reranker: Option<LLMReranker> = if rerank {
        match LLMReranker::from_env() {
            Ok(r) => Some(r),
            Err(e) => {
                eprintln!("Warning: LLM reranker unavailable: {e}");
                None
            }
        }
    } else {
        None
    };

    let mut hits_at_1 = 0;
    let mut mrr_total = 0.0f32;

    for (idx, query) in spec_file.spec.queries.iter().enumerate() {
        // Request more candidates if reranking
        let search_top_k = if reranker.is_some() { 50 } else { 10 };
        let options = SearchOptions {
            top_k: search_top_k,
            threshold: 0.0,
            hybrid,
            auto_hybrid,
            root: Some(corpus_root.clone()),
            context: 2,
            file_types: Vec::new(),
            exclude_file_types: Vec::new(),
            ..SearchOptions::default()
        };

        let mut results = search_backend(&db, embedder, &query.query, options)
            .with_context(|| format!("Search failed for query {}", idx + 1))?;

        // Apply reranking if available
        if let Some(ref r) = reranker {
            let rerank_options = RerankOptions {
                candidates: results.len(),
                top_k: 10,
                ..Default::default()
            };
            match r.rerank(&query.query, results.clone(), &rerank_options) {
                Ok(reranked) => results = reranked,
                Err(e) => {
                    if verbose {
                        eprintln!("  Reranking failed for query {}: {}", idx + 1, e);
                    }
                    results.truncate(10);
                }
            }
        }

        let rank = find_relevant_rank(&results, &query.relevant, &corpus_root);
        if let Some(rank) = rank {
            if rank == 1 {
                hits_at_1 += 1;
            }
            mrr_total += 1.0 / rank as f32;
        }

        if verbose {
            let status = if rank.is_some() { "hit" } else { "miss" };
            let rank_display = rank
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string());
            println!("  [{}] {} (rank {})", status, query.query, rank_display);
            // Show top results for debugging
            if results.is_empty() {
                println!("    (no results returned)");
            } else {
                for (i, r) in results.iter().take(3).enumerate() {
                    let filename = r.path.file_name().and_then(|n| n.to_str()).unwrap_or("?");
                    println!("    {}: {} (score: {:.3})", i + 1, filename, r.score);
                }
            }
        }
    }

    let total = spec_file.spec.queries.len();
    let p_at_1 = if total > 0 {
        hits_at_1 as f32 / total as f32
    } else {
        0.0
    };
    let mrr = if total > 0 {
        mrr_total / total as f32
    } else {
        0.0
    };

    Ok(EvalSummary {
        total_queries: total,
        p_at_1,
        mrr,
        hits_at_1,
    })
}

fn resolve_corpus_root(spec_path: &Path, spec: &EvalSpec) -> Result<PathBuf> {
    let corpus_path = PathBuf::from(&spec.corpus);
    let root = if corpus_path.is_absolute() {
        corpus_path
    } else {
        let cwd = std::env::current_dir().context("Failed to resolve current directory")?;
        let cwd_candidate = cwd.join(&corpus_path);
        if cwd_candidate.exists() {
            cwd_candidate
        } else {
            let base = spec_path
                .parent()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."));
            base.join(corpus_path)
        }
    };

    root.canonicalize()
        .with_context(|| format!("Failed to resolve corpus root {}", root.display()))
}

fn build_filters(spec: &EvalSpec) -> Result<EvalFilters> {
    let include_extensions = spec.include_extensions.as_ref().map(|values| {
        values
            .iter()
            .map(|ext| ext.trim_start_matches('.').to_ascii_lowercase())
            .collect::<HashSet<_>>()
    });

    let include_globs = match &spec.include_globs {
        Some(globs) if !globs.is_empty() => {
            let mut compiled = Vec::new();
            for glob in globs {
                compiled.push(glob_to_regex(glob)?);
            }
            Some(compiled)
        }
        _ => None,
    };

    Ok(EvalFilters {
        include_extensions,
        include_globs,
    })
}

fn glob_to_regex(pattern: &str) -> Result<Regex> {
    let mut regex = String::from("^");
    for ch in pattern.chars() {
        match ch {
            '*' => regex.push_str(".*"),
            '?' => regex.push('.'),
            '.' | '+' | '(' | ')' | '|' | '^' | '$' | '{' | '}' | '[' | ']' | '\\' => {
                regex.push('\\');
                regex.push(ch);
            }
            _ => regex.push(ch),
        }
    }
    regex.push('$');
    Regex::new(&regex).with_context(|| format!("Invalid glob pattern: {pattern}"))
}

fn collect_corpus_files(root: &Path, filters: &EvalFilters) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        for entry in
            fs::read_dir(&dir).with_context(|| format!("Failed to read {}", dir.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if should_index_path(&path, root, filters) {
                files.push(path);
            }
        }
    }

    files.sort();
    Ok(files)
}

fn should_index_path(path: &Path, corpus_root: &Path, filters: &EvalFilters) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|value| value.to_ascii_lowercase());

    if let Some(include_exts) = &filters.include_extensions {
        match ext {
            Some(ref ext) if include_exts.contains(ext) => {}
            _ => return false,
        }
    }

    if let Some(include_globs) = &filters.include_globs {
        // Match against relative path from corpus root for more flexible filtering
        let rel_path = path
            .strip_prefix(corpus_root)
            .ok()
            .and_then(|p| p.to_str())
            .unwrap_or_else(|| path.file_name().and_then(|n| n.to_str()).unwrap_or(""));
        if !include_globs.iter().any(|glob| glob.is_match(rel_path)) {
            return false;
        }
    }

    if ext.as_deref() == Some("pdf") {
        return true;
    }

    is_indexable_path(path)
}

fn index_files<E: EmbedderBackend>(db: &DB, embedder: &mut E, files: &[PathBuf]) -> Result<()> {
    use std::io::Write;
    let total = files.len();
    let mut indexed_count = 0;
    let mut empty_count = 0;
    let mut error_count = 0;
    let is_tty = std::io::stderr().is_terminal();
    for (idx, path) in files.iter().enumerate() {
        // Only show progress on TTY to avoid spam when piped
        if is_tty {
            eprint!(
                "\r  Indexing {}/{}: {:40}",
                idx + 1,
                total,
                path.file_name().and_then(|n| n.to_str()).unwrap_or("?")
            );
            let _ = std::io::stderr().flush();
        }

        let content = match read_document_text(path) {
            Ok(content) => content,
            Err(err) => {
                error_count += 1;
                tracing::debug!("Skipping {}: {}", path.display(), err);
                continue;
            }
        };
        if content.trim().is_empty() {
            empty_count += 1;
            continue;
        }
        indexed_count += 1;
        index_document_content(db, embedder, path, &content)?;
    }
    if is_tty {
        // Clear the progress line before printing summary
        eprint!("\r{:60}\r", "");
    }
    eprintln!(
        "  Indexed {total} files (content: {indexed_count}, empty: {empty_count}, errors: {error_count})"
    );
    Ok(())
}

fn index_document_content<E: EmbedderBackend>(
    db: &DB,
    embedder: &mut E,
    path: &Path,
    content: &str,
) -> Result<()> {
    let doc_id = db.add_document(path, content)?;
    db.delete_chunks_for_doc(doc_id)?;

    let chunks = chunk_document(content);
    if chunks.is_empty() {
        return Ok(());
    }

    // Preprocess code files for better tokenization
    // Split identifiers like "getUserName" -> "get user name" for the tokenizer
    let is_code = is_code_file(path);
    let processed_texts: Vec<String> = if is_code {
        chunks.iter().map(|c| preprocess_code(&c.content)).collect()
    } else {
        chunks.iter().map(|c| c.content.clone()).collect()
    };

    let texts: Vec<&str> = processed_texts.iter().map(|s| s.as_str()).collect();
    let embeddings = EmbedderBackend::embed_batch(embedder, &texts).with_context(|| {
        format!(
            "Failed to embed {} chunk(s) of {}",
            chunks.len(),
            path.display()
        )
    })?;
    if embeddings.len() != chunks.len() {
        return Err(anyhow::anyhow!(
            "Embedding count mismatch for {} (expected {}, got {})",
            path.display(),
            chunks.len(),
            embeddings.len()
        ));
    }

    let chunk_data: Vec<(usize, usize, usize, &str, &[f32], usize)> = chunks
        .iter()
        .zip(embeddings.iter())
        .map(|(chunk, embedding)| {
            (
                chunk.index,
                chunk.start_line,
                chunk.end_line,
                chunk.header_context.as_str(),
                embedding.data.as_slice(),
                embedding.num_tokens,
            )
        })
        .collect();
    db.batch_add_chunks_with_embeddings(doc_id, &chunk_data)?;

    Ok(())
}

fn read_document_text(path: &Path) -> Result<String> {
    if path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("pdf"))
    {
        return extract_pdf_text(path);
    }
    fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))
}

fn extract_pdf_text(path: &Path) -> Result<String> {
    // Suppress stderr for all PDF extraction to avoid noisy Unicode/glyph warnings
    // from both docling-backend (pdfium) and pdf_extract
    let _guard = suppress_stderr();

    // Try docling-backend first (better layout-aware extraction)
    match try_docling_pdf(path) {
        Ok(text) if !text.trim().is_empty() => {
            tracing::debug!("PDF extracted with docling-backend: {} chars", text.len());
            return Ok(text);
        }
        Ok(_) => {
            tracing::debug!("docling-backend returned empty text, trying fallback");
        }
        Err(e) => {
            tracing::debug!("docling-backend failed: {}, trying fallback", e);
        }
    }

    // Fallback to pdf_extract (pure Rust, no external libs)
    // Wrap in catch_unwind since pdf_extract can panic on malformed PDFs
    let path_clone = path.to_path_buf();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        pdf_extract::extract_text(&path_clone)
    }));

    match result {
        Ok(Ok(text)) => {
            tracing::debug!(
                "PDF extracted with pdf_extract fallback: {} chars",
                text.len()
            );
            Ok(text)
        }
        Ok(Err(e)) => Err(anyhow::anyhow!("PDF extraction failed: {e}")),
        Err(_) => Err(anyhow::anyhow!("PDF extraction panicked (malformed PDF)")),
    }
}

fn try_docling_pdf(path: &Path) -> Result<String> {
    let converter = docling_backend::RustDocumentConverter::new()
        .with_context(|| "Failed to create document converter")?;
    let result = converter
        .convert(path)
        .with_context(|| format!("Failed to convert PDF: {}", path.display()))?;
    Ok(result.document.markdown)
}

fn find_relevant_rank(
    results: &[sg_core::SearchResult],
    relevant: &[String],
    corpus_root: &Path,
) -> Option<usize> {
    if relevant.is_empty() {
        return None;
    }

    for (idx, result) in results.iter().enumerate() {
        if is_relevant_match(&result.path, relevant, corpus_root) {
            return Some(idx + 1);
        }
    }

    None
}

fn is_relevant_match(path: &Path, relevant: &[String], corpus_root: &Path) -> bool {
    let abs = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let abs_str = normalize_path(&abs);
    let rel_str = abs
        .strip_prefix(corpus_root)
        .ok()
        .map(normalize_path)
        .unwrap_or_default();
    let name_str = abs
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_string();

    relevant.iter().any(|entry| {
        let normalized = normalize_path_str(entry);
        if Path::new(entry).is_absolute() {
            abs_str == normalized
        } else {
            rel_str == normalized || name_str == *entry || abs_str.ends_with(&normalized)
        }
    })
}

fn normalize_path(path: &Path) -> String {
    normalize_path_str(&path.to_string_lossy())
}

fn normalize_path_str(value: &str) -> String {
    value.replace('\\', "/")
}

/// Guard that suppresses stdout and stderr while held
struct OutputSuppressor {
    original_stdout: Option<std::os::fd::OwnedFd>,
    original_stderr: Option<std::os::fd::OwnedFd>,
}

impl Drop for OutputSuppressor {
    fn drop(&mut self) {
        use std::os::fd::AsRawFd;
        // Restore original stdout
        if let Some(ref fd) = self.original_stdout {
            unsafe {
                libc::dup2(fd.as_raw_fd(), libc::STDOUT_FILENO);
            }
        }
        // Restore original stderr
        if let Some(ref fd) = self.original_stderr {
            unsafe {
                libc::dup2(fd.as_raw_fd(), libc::STDERR_FILENO);
            }
        }
    }
}

/// Suppress stdout/stderr output (returns guard that restores on drop)
fn suppress_stderr() -> OutputSuppressor {
    use std::os::fd::{FromRawFd, OwnedFd};

    // Save original stdout
    let stdout = unsafe { libc::dup(libc::STDOUT_FILENO) };
    let original_stdout = if stdout >= 0 {
        Some(unsafe { OwnedFd::from_raw_fd(stdout) })
    } else {
        None
    };

    // Save original stderr
    let stderr = unsafe { libc::dup(libc::STDERR_FILENO) };
    let original_stderr = if stderr >= 0 {
        Some(unsafe { OwnedFd::from_raw_fd(stderr) })
    } else {
        None
    };

    // Redirect both to /dev/null
    let devnull = unsafe { libc::open(c"/dev/null".as_ptr(), libc::O_WRONLY) };
    if devnull >= 0 {
        unsafe {
            libc::dup2(devnull, libc::STDOUT_FILENO);
            libc::dup2(devnull, libc::STDERR_FILENO);
            libc::close(devnull);
        }
    }

    OutputSuppressor {
        original_stdout,
        original_stderr,
    }
}
