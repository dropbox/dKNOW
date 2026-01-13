//! Search algorithms and result types
//!
//! Supports both semantic search (MaxSim) and keyword search,
//! with optional hybrid mode using Reciprocal Rank Fusion (RRF).

// Allow complex tuple types in internal batch operations - these are used for
// passing chunk data through indexing pipelines and refactoring to named types
// would create many small structs without improving clarity.
#![allow(clippy::type_complexity)]

use anyhow::{Context, Result};
use rayon::prelude::*;
use regex::RegexBuilder;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::thread;

use crate::chunker::chunk_document;
use crate::code_preprocessor::{
    detect_query_style, is_code_file, preprocess_code, preprocess_query, should_use_hybrid,
    QueryStyle,
};
use crate::embedder::{similarity_from_vecs, EmbedderBackend, EmbeddingResult};
use crate::query_cache::{CachedEmbedding, QueryCache};
use crate::storage::{StoredLink, DB};
use crate::Embedder;

/// A link found in a search result chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultLink {
    /// The display text of the link
    pub text: String,
    /// The link target (URL, path, or wiki-style reference)
    pub target: String,
    /// Whether this is an internal link (relative path) vs external (URL)
    pub is_internal: bool,
}

/// A single search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Document ID in the database
    pub doc_id: u32,
    /// Relevance score (higher is better)
    pub score: f32,
    /// File path
    pub path: PathBuf,
    /// Line number of best match (0 = whole file)
    pub line: usize,
    /// Snippet of matching content
    pub snippet: String,
    /// Header context for display (e.g., "# Title > ## Section")
    #[serde(default)]
    pub header_context: String,
    /// Programming language for code blocks (e.g., "rust", "python")
    #[serde(default)]
    pub language: Option<String>,
    /// Links found in this chunk (markdown links, wiki-style links, etc.)
    #[serde(default)]
    pub links: Vec<SearchResultLink>,
    /// One-line file summary (first line for text, xattr/MIME for binary)
    #[serde(default)]
    pub summary: Option<String>,
}

/// Search options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchOptions {
    /// Maximum results to return
    pub top_k: usize,
    /// Minimum score threshold
    pub threshold: f32,
    /// Combine with BM25 fulltext search
    pub hybrid: bool,
    /// Automatically select semantic vs hybrid based on query style
    /// When true, overrides `hybrid` setting based on query analysis:
    /// - Docstring-style queries → semantic-only
    /// - Natural language queries → hybrid
    /// - Code identifier queries → semantic-only
    #[serde(default)]
    pub auto_hybrid: bool,
    /// Limit search to this directory
    pub root: Option<PathBuf>,
    /// Number of context lines around matches (default: 2)
    #[serde(default = "default_context")]
    pub context: usize,
    /// Filter by file extensions (e.g., ["rs", "py"])
    #[serde(default)]
    pub file_types: Vec<String>,
    /// Exclude file extensions (e.g., ["test.rs", "spec.js"])
    #[serde(default)]
    pub exclude_file_types: Vec<String>,
    /// Preprocess query for code search (split identifiers like getUserName -> get user name)
    /// Default: true (auto-detect code-like queries)
    #[serde(default = "default_preprocess_query")]
    pub preprocess_query: bool,
}

fn default_preprocess_query() -> bool {
    true
}

fn default_context() -> usize {
    2
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            top_k: 10,
            threshold: 0.0,
            hybrid: false,
            auto_hybrid: false,
            root: None,
            context: default_context(),
            file_types: Vec::new(),
            exclude_file_types: Vec::new(),
            preprocess_query: default_preprocess_query(),
        }
    }
}

impl SearchOptions {
    /// Check if a file path matches the file type filter
    pub fn matches_file_type(&self, path: &str) -> bool {
        let path = std::path::Path::new(path);
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.trim_start_matches('.'));

        // First check exclusions - if excluded, reject
        if !self.exclude_file_types.is_empty()
            && path_matches_filters(file_name, ext, &self.exclude_file_types)
        {
            return false;
        }

        // Then check inclusions - if no includes, accept all; otherwise must match
        if self.file_types.is_empty() {
            return true;
        }

        path_matches_filters(file_name, ext, &self.file_types)
    }
}

/// Check if a file matches any of the given filters
fn path_matches_filters(file_name: &str, ext: Option<&str>, filters: &[String]) -> bool {
    filters.iter().any(|t| {
        let trimmed = t.trim();
        if trimmed.is_empty() {
            return false;
        }
        let trimmed = trimmed.trim_start_matches('.');

        if let Some(ext) = ext {
            if trimmed.eq_ignore_ascii_case(ext) {
                return true;
            }
        }

        file_name_matches_filter(file_name, trimmed)
    })
}

fn file_name_matches_filter(file_name: &str, filter: &str) -> bool {
    let file_name = file_name.to_ascii_lowercase();
    let filter = filter.to_ascii_lowercase();
    let file_name = file_name.strip_prefix('.').unwrap_or(&file_name);
    if file_name == filter {
        return true;
    }
    if filter.contains('.') {
        let suffix = format!(".{filter}");
        if file_name.ends_with(&suffix) {
            return true;
        }
    }
    if file_name.len() <= filter.len() {
        return false;
    }
    if !file_name.starts_with(&filter) {
        return false;
    }

    matches!(file_name.as_bytes()[filter.len()], b'.' | b'-' | b'_')
}

impl Default for SearchResult {
    fn default() -> Self {
        Self {
            doc_id: 0,
            score: 0.0,
            path: PathBuf::new(),
            line: 0,
            snippet: String::new(),
            header_context: String::new(),
            language: None,
            links: vec![],
            summary: None,
        }
    }
}

/// Execute a search query
///
/// Searches all indexed documents. If `options.hybrid` is true,
/// combines semantic (MaxSim) and keyword search using Reciprocal Rank Fusion.
pub fn search(
    db: &DB,
    embedder: &mut Embedder,
    query: &str,
    options: SearchOptions,
) -> Result<Vec<SearchResult>> {
    search_backend(db, embedder, query, options)
}

/// Execute a search query using any EmbedderBackend
pub fn search_backend<E: EmbedderBackend>(
    db: &DB,
    embedder: &mut E,
    query: &str,
    options: SearchOptions,
) -> Result<Vec<SearchResult>> {
    // Determine whether to use hybrid search
    let use_hybrid = if options.auto_hybrid {
        // Auto-select based on query style analysis
        should_use_hybrid(query)
    } else {
        // Use explicit setting
        options.hybrid
    };

    if use_hybrid {
        hybrid_search_backend(db, embedder, query, &options)
    } else {
        semantic_search_backend(db, embedder, query, &options)
    }
}

/// Execute semantic search using any EmbedderBackend
///
/// This is the generic version that works with any backend implementing EmbedderBackend.
pub fn semantic_search_backend<E: EmbedderBackend>(
    db: &DB,
    embedder: &mut E,
    query: &str,
    options: &SearchOptions,
) -> Result<Vec<SearchResult>> {
    // 0. Optionally preprocess query for code search
    // This transforms identifiers like "getUserName" -> "get user name"
    // to match how code content was indexed
    let processed_query = if options.preprocess_query {
        preprocess_query(query)
    } else {
        query.to_string()
    };

    // 1. Embed query using the trait method
    let query_result = EmbedderBackend::embed_query(embedder, &processed_query)
        .context("Failed to embed query")?;

    // 2. Get all chunk embeddings with metadata
    let chunk_embeddings = db.get_all_chunk_embeddings()?;

    if chunk_embeddings.is_empty() {
        return Ok(Vec::new());
    }

    // 3. Score all chunks in parallel using similarity function (backend-agnostic)
    // Supports both multi-vector (MaxSim) and single-vector (cosine) models
    // Each chunk's score is independent, so we use rayon for multicore scaling
    let embed_dim = embedder.embedding_dim();
    let threshold = options.threshold;
    let query_data = &query_result.data;
    let query_tokens = query_result.num_tokens;

    // Parallel scoring: (chunk_id, score, doc_id, start_line, end_line, header_context, language, links)
    let mut scored: Vec<(
        u32,
        f32,
        u32,
        usize,
        usize,
        String,
        Option<String>,
        Vec<StoredLink>,
    )> = chunk_embeddings
        .par_iter()
        .filter_map(|chunk| {
            let score = similarity_from_vecs(
                query_data,
                query_tokens,
                &chunk.embeddings,
                chunk.num_tokens,
                embed_dim,
            );

            if score >= threshold {
                Some((
                    chunk.chunk_id,
                    score,
                    chunk.doc_id,
                    chunk.start_line,
                    chunk.end_line,
                    chunk.header_context.clone(),
                    chunk.language.clone(),
                    chunk.links.clone(),
                ))
            } else {
                None
            }
        })
        .collect();

    // 4. Sort by score descending
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // 5. Deduplicate by doc_id, keeping best chunk per document
    let mut seen_docs = HashSet::new();
    let deduped: Vec<_> = scored
        .into_iter()
        .filter(|(_, _, doc_id, _, _, _, _, _)| seen_docs.insert(*doc_id))
        .take(options.top_k)
        .collect();

    // 6. Build results
    let mut results = Vec::new();
    for (_, score, doc_id, start_line, _, header_context, language, links) in deduped {
        if let Some(doc) = db.get_document(doc_id)? {
            // Apply filters
            if let Some(ref root) = options.root {
                if !doc.path.starts_with(root.to_string_lossy().as_ref()) {
                    continue;
                }
            }
            if !options.matches_file_type(&doc.path) {
                continue;
            }

            let snippet = extract_snippet_around(&doc.content, start_line, options.context);

            // Convert StoredLink to SearchResultLink
            let result_links: Vec<SearchResultLink> = links
                .into_iter()
                .map(|l| SearchResultLink {
                    text: l.text,
                    target: l.target,
                    is_internal: l.is_internal,
                })
                .collect();

            results.push(SearchResult {
                doc_id,
                score,
                path: std::path::PathBuf::from(&doc.path),
                line: start_line + 1, // 1-indexed for display
                snippet,
                header_context,
                language,
                links: result_links,
                summary: None,
            });
        }
    }

    Ok(results)
}

/// Execute semantic search with query embedding cache
///
/// Uses a cache to avoid re-embedding repeated queries. If the query
/// has been seen before, the cached embedding is used instead of
/// calling the embedder.
///
/// This is useful for:
/// - Interactive search sessions with query refinement
/// - Bulk search operations with duplicate queries
/// - Daemon mode with multiple clients searching
pub fn semantic_search_cached<E: EmbedderBackend>(
    db: &DB,
    embedder: &mut E,
    query: &str,
    options: &SearchOptions,
    cache: &mut QueryCache,
) -> Result<Vec<SearchResult>> {
    // 0. Optionally preprocess query for code search
    let processed_query = if options.preprocess_query {
        preprocess_query(query)
    } else {
        query.to_string()
    };

    // 1. Check cache for query embedding (using processed query as key)
    let query_result = if let Some(cached) = cache.get(&processed_query) {
        EmbeddingResult {
            data: cached.data.clone(),
            num_tokens: cached.num_tokens,
        }
    } else {
        // 2. Embed query using the trait method
        let result = EmbedderBackend::embed_query(embedder, &processed_query)
            .context("Failed to embed query")?;

        // 3. Cache the result
        cache.insert(
            processed_query.clone(),
            CachedEmbedding {
                data: result.data.clone(),
                num_tokens: result.num_tokens,
            },
        );

        result
    };

    // 4. Get all chunk embeddings with metadata
    let chunk_embeddings = db.get_all_chunk_embeddings()?;

    if chunk_embeddings.is_empty() {
        return Ok(Vec::new());
    }

    // 5. Score all chunks using similarity function (backend-agnostic)
    let mut scored: Vec<(
        u32,
        f32,
        u32,
        usize,
        usize,
        String,
        Option<String>,
        Vec<StoredLink>,
    )> = Vec::new();
    let embed_dim = embedder.embedding_dim();

    for chunk in chunk_embeddings {
        let score = similarity_from_vecs(
            &query_result.data,
            query_result.num_tokens,
            &chunk.embeddings,
            chunk.num_tokens,
            embed_dim,
        );

        if score >= options.threshold {
            scored.push((
                chunk.chunk_id,
                score,
                chunk.doc_id,
                chunk.start_line,
                chunk.end_line,
                chunk.header_context.clone(),
                chunk.language.clone(),
                chunk.links.clone(),
            ));
        }
    }

    // 6. Sort by score descending
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // 7. Deduplicate by doc_id, keeping best chunk per document
    let mut seen_docs = HashSet::new();
    let deduped: Vec<_> = scored
        .into_iter()
        .filter(|(_, _, doc_id, _, _, _, _, _)| seen_docs.insert(*doc_id))
        .take(options.top_k)
        .collect();

    // 8. Build results
    let mut results = Vec::new();
    for (_, score, doc_id, start_line, _, header_context, language, links) in deduped {
        if let Some(doc) = db.get_document(doc_id)? {
            // Apply filters
            if let Some(ref root) = options.root {
                if !doc.path.starts_with(root.to_string_lossy().as_ref()) {
                    continue;
                }
            }
            if !options.matches_file_type(&doc.path) {
                continue;
            }

            let snippet = extract_snippet_around(&doc.content, start_line, options.context);

            // Convert StoredLink to SearchResultLink
            let result_links: Vec<SearchResultLink> = links
                .into_iter()
                .map(|l| SearchResultLink {
                    text: l.text,
                    target: l.target,
                    is_internal: l.is_internal,
                })
                .collect();

            results.push(SearchResult {
                doc_id,
                score,
                path: std::path::PathBuf::from(&doc.path),
                line: start_line + 1,
                snippet,
                header_context,
                language,
                links: result_links,
                summary: None,
            });
        }
    }

    Ok(results)
}

/// Execute search with query embedding cache
///
/// Wrapper around semantic_search_cached that uses the cache.
pub fn search_cached<E: EmbedderBackend>(
    db: &DB,
    embedder: &mut E,
    query: &str,
    options: SearchOptions,
    cache: &mut QueryCache,
) -> Result<Vec<SearchResult>> {
    // Determine whether to use hybrid search
    let use_hybrid = if options.auto_hybrid {
        // Auto-select based on query style analysis
        should_use_hybrid(query)
    } else {
        // Use explicit setting
        options.hybrid
    };

    if use_hybrid {
        // For hybrid search, still use cache for semantic part
        hybrid_search_cached(db, embedder, query, &options, cache)
    } else {
        semantic_search_cached(db, embedder, query, &options, cache)
    }
}

/// Get adaptive RRF weights based on query style.
///
/// Returns (semantic_weight, keyword_weight) that sum to ~1.0
/// Different query types benefit from different balances:
/// - Docstring queries (from code comments): heavily favor semantic (0.9:0.1)
/// - Natural language queries: balanced (0.5:0.5)
/// - Code identifiers: favor semantic with some keyword help (0.75:0.25)
fn get_adaptive_rrf_weights(query: &str) -> (f32, f32) {
    match detect_query_style(query) {
        QueryStyle::Docstring => (0.9, 0.1),
        QueryStyle::NaturalLanguage => (0.5, 0.5),
        QueryStyle::CodeIdentifier => (0.75, 0.25),
    }
}

/// Compute confidence-aware weights based on actual similarity scores.
///
/// When semantic results have high confidence (high scores), trust them more.
/// When semantic confidence is low, blend in more keyword results.
fn get_confidence_aware_weights(
    query: &str,
    semantic_results: &[SearchResult],
    keyword_results: &[SearchResult],
) -> (f32, f32) {
    // Start with query-style-based weights
    let (base_semantic, base_keyword) = get_adaptive_rrf_weights(query);

    // Get top semantic score as confidence indicator
    let top_semantic_score = semantic_results.first().map(|r| r.score).unwrap_or(0.0);
    let top_keyword_score = keyword_results.first().map(|r| r.score).unwrap_or(0.0);

    // Adjust weights based on confidence
    // High semantic confidence (>0.7) = trust semantic more
    // Low semantic confidence (<0.4) = blend in more keywords
    let confidence_adjustment = if top_semantic_score > 0.7 {
        0.2 // Boost semantic weight
    } else if top_semantic_score < 0.4 && top_keyword_score > 0.5 {
        -0.2 // Boost keyword weight when semantic is weak but keyword is strong
    } else {
        0.0 // Keep base weights
    };

    let semantic_weight = (base_semantic + confidence_adjustment).clamp(0.3, 0.95);
    let keyword_weight = 1.0 - semantic_weight;

    (semantic_weight, keyword_weight)
}

/// Ensure minimum number of results from a source are included.
///
/// Adds a small boost to docs from the source if they're not already
/// well-represented in the top results.
fn ensure_minimum_results(
    scores: &mut HashMap<u32, f32>,
    source_docs: &[u32],
    min_count: usize,
    boost: f32,
) {
    // Count how many source docs are in top scores
    let mut sorted_scores: Vec<_> = scores.iter().collect();
    sorted_scores.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap_or(std::cmp::Ordering::Equal));

    let top_10: std::collections::HashSet<_> = sorted_scores.iter().take(10).map(|(id, _)| **id).collect();
    let source_in_top = source_docs.iter().filter(|id| top_10.contains(id)).count();

    // If not enough source docs in top 10, boost the top source docs
    if source_in_top < min_count {
        for (i, doc_id) in source_docs.iter().take(min_count).enumerate() {
            // Decreasing boost for lower-ranked source docs
            let doc_boost = boost * (1.0 - i as f32 * 0.1);
            *scores.entry(*doc_id).or_insert(0.0) += doc_boost;
        }
    }
}

/// Execute hybrid search with query embedding cache
///
/// Uses confidence-aware fusion that:
/// 1. Considers actual similarity scores, not just ranks
/// 2. Adjusts weights based on semantic confidence
/// 3. Guarantees minimum results from each source
fn hybrid_search_cached<E: EmbedderBackend>(
    db: &DB,
    embedder: &mut E,
    query: &str,
    options: &SearchOptions,
    cache: &mut QueryCache,
) -> Result<Vec<SearchResult>> {
    // Get expanded top-k for both methods
    let expanded_options = SearchOptions {
        top_k: options.top_k * 3,
        ..options.clone()
    };

    // Run semantic search (cached) and keyword search
    let semantic_results = semantic_search_cached(db, embedder, query, &expanded_options, cache)?;
    let keyword_results = keyword_search(db, query, &expanded_options)?;

    // Compute confidence-aware weights based on semantic scores
    let (semantic_weight, keyword_weight) =
        get_confidence_aware_weights(query, &semantic_results, &keyword_results);

    // Compute weighted fusion scores using both rank AND actual scores
    const RRF_K: f32 = 60.0;
    let mut fusion_scores: HashMap<u32, f32> = HashMap::new();
    let mut doc_data: HashMap<u32, (PathBuf, usize, String)> = HashMap::new();

    // Track which docs came from which source for minimum guarantee
    let mut semantic_docs: Vec<u32> = Vec::new();
    let mut keyword_docs: Vec<u32> = Vec::new();

    // Add semantic results - combine RRF with actual score
    for (rank, result) in semantic_results.iter().enumerate() {
        // Blend rank-based RRF with actual similarity score
        let rrf_component = 1.0 / (RRF_K + rank as f32 + 1.0);
        let score_component = result.score; // actual similarity
        let combined = semantic_weight * (0.5 * rrf_component + 0.5 * score_component);
        *fusion_scores.entry(result.doc_id).or_insert(0.0) += combined;
        doc_data.insert(
            result.doc_id,
            (result.path.clone(), result.line, result.snippet.clone()),
        );
        semantic_docs.push(result.doc_id);
    }

    // Add keyword results
    for (rank, result) in keyword_results.iter().enumerate() {
        let rrf_component = 1.0 / (RRF_K + rank as f32 + 1.0);
        let score_component = result.score;
        let combined = keyword_weight * (0.5 * rrf_component + 0.5 * score_component);
        *fusion_scores.entry(result.doc_id).or_insert(0.0) += combined;
        // Prefer keyword snippet/line if available
        if result.line > 0 {
            doc_data.insert(
                result.doc_id,
                (result.path.clone(), result.line, result.snippet.clone()),
            );
        }
        keyword_docs.push(result.doc_id);
    }

    // Guarantee minimum results from each source
    // This ensures semantic results aren't completely drowned out by keywords
    const MIN_SEMANTIC: usize = 5;
    const MIN_KEYWORD: usize = 2;
    ensure_minimum_results(
        &mut fusion_scores,
        &semantic_docs,
        MIN_SEMANTIC,
        0.001, // small boost to guarantee inclusion
    );
    ensure_minimum_results(&mut fusion_scores, &keyword_docs, MIN_KEYWORD, 0.0005);

    apply_filename_boost(&mut fusion_scores, &doc_data, query, |data| data.0.as_path());

    // Sort by fusion score, then by doc_id for deterministic ordering
    let mut scored: Vec<_> = fusion_scores.into_iter().collect();
    scored.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0)) // Break ties by doc_id (ascending)
    });
    scored.truncate(options.top_k);

    // Normalize scores to 0-1 range for better UX
    let max_score = scored.first().map(|(_, s)| *s).unwrap_or(1.0);
    let normalize = |s: f32| if max_score > 0.0 { s / max_score } else { s };

    // Build results
    let mut results = Vec::with_capacity(scored.len());
    for (doc_id, score) in scored {
        if let Some((path, line, snippet)) = doc_data.get(&doc_id) {
            results.push(SearchResult {
                doc_id,
                score: normalize(score),
                path: path.clone(),
                line: *line,
                snippet: snippet.clone(),
                header_context: String::new(), // Hybrid uses merged results
                language: None,                // Hybrid search doesn't preserve chunk metadata
                links: vec![],
                summary: None,
            });
        }
    }

    Ok(results)
}

fn apply_filename_boost<T, F>(
    rrf_scores: &mut HashMap<u32, f32>,
    doc_data: &HashMap<u32, T>,
    query: &str,
    mut path_for: F,
) where
    F: FnMut(&T) -> &Path,
{
    // Boost RRF scores when query terms match the file stem to prefer
    // implementation files over callers.
    const FILENAME_BOOST: f32 = 0.02;
    let query_terms: Vec<String> = query
        .split_whitespace()
        .map(str::to_lowercase)
        .filter(|w| w.len() >= 3)
        .collect();

    for (doc_id, score) in rrf_scores.iter_mut() {
        if let Some(data) = doc_data.get(doc_id) {
            let filename = path_for(data)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();
            for term in &query_terms {
                let is_filename_match = filename.contains(term.as_str());
                let is_simple_plural =
                    *term == format!("{filename}s") || *term == format!("{filename}es");
                let is_compound_match =
                    term.contains(&filename) && filename.len() >= 4 && !is_simple_plural;
                if is_filename_match || is_compound_match {
                    *score += FILENAME_BOOST;
                    break;
                }
            }
        }
    }
}

/// Execute keyword search using regex matching
fn keyword_search(db: &DB, query: &str, options: &SearchOptions) -> Result<Vec<SearchResult>> {
    let fts_query = build_fts_query(query);
    if let Some(fts_query) = fts_query {
        let results = keyword_search_fts(db, &fts_query, query, options)?;
        if !results.is_empty() {
            return Ok(results);
        }
    }

    keyword_search_regex(db, query, options)
}

fn keyword_search_fts(
    db: &DB,
    fts_query: &str,
    query: &str,
    options: &SearchOptions,
) -> Result<Vec<SearchResult>> {
    let words: Vec<&str> = query.split_whitespace().collect();
    if words.is_empty() {
        return Ok(Vec::new());
    }

    // Build case-insensitive regexes for each query word.
    // Keep substring matching to preserve prior behavior (e.g., "authenticate" matches "authentication").
    let word_regexes: Vec<regex::Regex> = words
        .iter()
        .filter_map(|w| {
            RegexBuilder::new(&regex::escape(w))
                .case_insensitive(true)
                .build()
                .ok()
        })
        .collect();

    if word_regexes.is_empty() {
        return Ok(Vec::new());
    }

    let fetch_limit = options.top_k.saturating_mul(5).max(options.top_k);
    let hits = db.search_documents_fts(fts_query, fetch_limit)?;

    let mut scored: Vec<(u32, f32, PathBuf, usize, String)> = Vec::new();
    for hit in hits {
        if let Some(ref root) = options.root {
            if !hit.path.starts_with(root.to_string_lossy().as_ref()) {
                continue;
            }
        }

        if !options.matches_file_type(&hit.path) {
            continue;
        }

        let (best_line, best_snippet) =
            find_best_word_match(&hit.content, &word_regexes, options.context);

        // Count how many query words match in the content (coverage boost)
        let matched_words = word_regexes
            .iter()
            .filter(|re| re.is_match(&hit.content))
            .count();
        let coverage_boost = if word_regexes.is_empty() {
            1.0
        } else {
            // Boost score by coverage ratio (0.5 to 1.5x)
            0.5 + (matched_words as f32 / word_regexes.len() as f32)
        };

        let base_score = 1.0 / (1.0 + hit.score.max(0.0));
        let score = base_score * coverage_boost;
        scored.push((
            hit.id,
            score,
            PathBuf::from(hit.path),
            best_line,
            best_snippet,
        ));
    }

    // Sort by score and take top-k
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(options.top_k);

    // Build results
    let results = scored
        .into_iter()
        .map(|(doc_id, score, path, line, snippet)| SearchResult {
            doc_id,
            score,
            path,
            line,
            snippet,
            header_context: String::new(), // Keyword search doesn't have chunk context
            language: None,                // Keyword search doesn't have chunk metadata
            links: vec![],
            summary: None,
        })
        .collect();

    Ok(results)
}

/// Execute keyword search using regex matching (fallback for FTS)
fn keyword_search_regex(
    db: &DB,
    query: &str,
    options: &SearchOptions,
) -> Result<Vec<SearchResult>> {
    let words: Vec<&str> = query.split_whitespace().collect();
    if words.is_empty() {
        return Ok(Vec::new());
    }

    // Build case-insensitive regexes for each query word.
    // Keep substring matching to preserve prior behavior (e.g., "authenticate" matches "authentication").
    let word_regexes: Vec<regex::Regex> = words
        .iter()
        .filter_map(|w| {
            RegexBuilder::new(&regex::escape(w))
                .case_insensitive(true)
                .build()
                .ok()
        })
        .collect();

    if word_regexes.is_empty() {
        return Ok(Vec::new());
    }

    // Get all documents
    let doc_ids = db.get_all_doc_ids()?;

    if doc_ids.is_empty() {
        return Ok(Vec::new());
    }

    // Score documents by keyword match quality
    let mut scored: Vec<(u32, f32, usize, String)> = Vec::new();

    for doc_id in doc_ids {
        if let Some(doc) = db.get_document(doc_id)? {
            // Filter by root directory if specified
            if let Some(ref root) = options.root {
                if !doc.path.starts_with(root.to_string_lossy().as_ref()) {
                    continue;
                }
            }

            // Filter by file type if specified
            if !options.matches_file_type(&doc.path) {
                continue;
            }

            // Count matches across the whole document for each term.
            let mut matched_terms = 0usize;
            let mut total_matches = 0usize;
            for re in &word_regexes {
                let count = re.find_iter(&doc.content).count();
                if count > 0 {
                    matched_terms += 1;
                    total_matches += count;
                }
            }

            if matched_terms > 0 {
                let line_count = doc.content.lines().count().max(1);
                let coverage = matched_terms as f32 / word_regexes.len() as f32;
                let density = total_matches as f32 / line_count as f32;
                let score = coverage + density;

                let (best_line, best_snippet) =
                    find_best_word_match(&doc.content, &word_regexes, options.context);
                scored.push((doc_id, score, best_line, best_snippet));
            }
        }
    }

    // Sort by score and take top-k
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(options.top_k);

    // Build results
    let mut results = Vec::with_capacity(scored.len());
    for (doc_id, score, line, snippet) in scored {
        if let Some(doc) = db.get_document(doc_id)? {
            results.push(SearchResult {
                doc_id,
                score,
                path: PathBuf::from(&doc.path),
                line,
                snippet,
                header_context: String::new(), // Keyword search doesn't have chunk context
                language: None,                // Keyword search doesn't have chunk metadata
                links: vec![],
                summary: None,
            });
        }
    }

    Ok(results)
}

fn build_fts_query(query: &str) -> Option<String> {
    let terms: Vec<String> = query
        .split_whitespace()
        .filter_map(|term| {
            let cleaned: String = term
                .chars()
                .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            if cleaned.is_empty() {
                return None;
            }
            let prefix_len = if cleaned.len() > 4 {
                cleaned.len().saturating_sub(2)
            } else {
                cleaned.len()
            };
            let prefix = &cleaned[..prefix_len];
            Some(format!("{prefix}*"))
        })
        .collect();

    if terms.is_empty() {
        None
    } else {
        Some(terms.join(" OR "))
    }
}
/// Hybrid search combining semantic and keyword results using RRF
fn hybrid_search_backend<E: EmbedderBackend>(
    db: &DB,
    embedder: &mut E,
    query: &str,
    options: &SearchOptions,
) -> Result<Vec<SearchResult>> {
    // Get expanded top-k for both methods
    let expanded_options = SearchOptions {
        top_k: options.top_k * 3,
        ..options.clone()
    };

    // Run both searches
    let semantic_results = semantic_search_backend(db, embedder, query, &expanded_options)?;
    let keyword_results = keyword_search(db, query, &expanded_options)?;

    // Get adaptive weights based on query style
    let (semantic_weight, keyword_weight) = get_adaptive_rrf_weights(query);

    // Compute weighted RRF scores
    // RRF(d) = sum(weight * 1 / (k + rank(d))) where k=60 is a constant
    const RRF_K: f32 = 60.0;
    let mut rrf_scores: HashMap<u32, f32> = HashMap::new();
    // Store full result data including chunk metadata
    let mut doc_data: HashMap<
        u32,
        (
            PathBuf,
            usize,
            String,
            String,
            Option<String>,
            Vec<SearchResultLink>,
        ),
    > = HashMap::new();

    // Add semantic results with semantic weight
    for (rank, result) in semantic_results.iter().enumerate() {
        let rrf_score = semantic_weight / (RRF_K + rank as f32 + 1.0);
        *rrf_scores.entry(result.doc_id).or_insert(0.0) += rrf_score;
        doc_data.insert(
            result.doc_id,
            (
                result.path.clone(),
                result.line,
                result.snippet.clone(),
                result.header_context.clone(),
                result.language.clone(),
                result.links.clone(),
            ),
        );
    }

    // Add keyword results with keyword weight
    for (rank, result) in keyword_results.iter().enumerate() {
        let rrf_score = keyword_weight / (RRF_K + rank as f32 + 1.0);
        *rrf_scores.entry(result.doc_id).or_insert(0.0) += rrf_score;
        // Prefer keyword snippet/line if available, but keep semantic metadata if we have it
        if result.line > 0 {
            if let Some(existing) = doc_data.get(&result.doc_id) {
                // Update snippet/line but keep header_context, language, and links from semantic
                doc_data.insert(
                    result.doc_id,
                    (
                        result.path.clone(),
                        result.line,
                        result.snippet.clone(),
                        existing.3.clone(),
                        existing.4.clone(),
                        existing.5.clone(),
                    ),
                );
            } else {
                // No semantic result for this doc, use keyword-only data
                doc_data.insert(
                    result.doc_id,
                    (
                        result.path.clone(),
                        result.line,
                        result.snippet.clone(),
                        String::new(),
                        None,
                        vec![],
                    ),
                );
            }
        }
    }

    apply_filename_boost(&mut rrf_scores, &doc_data, query, |data| data.0.as_path());

    // Sort by RRF score, then by doc_id for deterministic ordering
    let mut scored: Vec<_> = rrf_scores.into_iter().collect();
    scored.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0)) // Break ties by doc_id (ascending)
    });
    scored.truncate(options.top_k);

    // Normalize scores to 0-1 range for better UX
    // (raw RRF scores are very small, e.g. 0.02)
    let max_score = scored.first().map(|(_, s)| *s).unwrap_or(1.0);
    let normalize = |s: f32| if max_score > 0.0 { s / max_score } else { s };

    // Build results with preserved metadata
    let mut results = Vec::with_capacity(scored.len());
    for (doc_id, score) in scored {
        if let Some((path, line, snippet, header_context, language, links)) = doc_data.get(&doc_id)
        {
            results.push(SearchResult {
                doc_id,
                score: normalize(score),
                path: path.clone(),
                line: *line,
                snippet: snippet.clone(),
                header_context: header_context.clone(),
                language: language.clone(),
                links: links.clone(),
                summary: None,
            });
        }
    }

    Ok(results)
}

/// Find the best matching line and extract a snippet around it
fn find_best_match(content: &str, regex: &regex::Regex, context: usize) -> (usize, String) {
    for (line_num, line) in content.lines().enumerate() {
        if regex.is_match(line) {
            return (
                line_num + 1,
                extract_snippet_around(content, line_num, context),
            );
        }
    }
    (0, extract_snippet(content, context))
}

/// Build search results from scored doc IDs
///
/// Uses a multi-pass approach for snippet extraction:
/// 1. Try full query pattern (all words in order)
/// 2. Fall back to any single query word match
fn build_results(
    db: &DB,
    scored: &[(u32, f32)],
    options: &SearchOptions,
    query: &str,
) -> Result<Vec<SearchResult>> {
    let mut results = Vec::with_capacity(scored.len());

    // Build full pattern regex
    let words: Vec<&str> = query.split_whitespace().collect();
    let full_pattern = words
        .iter()
        .map(|w| regex::escape(w))
        .collect::<Vec<_>>()
        .join(".*");
    let full_regex = RegexBuilder::new(&full_pattern)
        .case_insensitive(true)
        .build()
        .ok();

    // Build individual word regexes for fallback
    let word_regexes: Vec<_> = words
        .iter()
        .filter_map(|w| {
            RegexBuilder::new(&regex::escape(w))
                .case_insensitive(true)
                .build()
                .ok()
        })
        .collect();

    for &(doc_id, score) in scored {
        if let Some(doc) = db.get_document(doc_id)? {
            // Filter by root directory if specified
            if let Some(ref root) = options.root {
                if !doc.path.starts_with(root.to_string_lossy().as_ref()) {
                    continue;
                }
            }

            // Filter by file type if specified
            if !options.matches_file_type(&doc.path) {
                continue;
            }

            // Try full pattern first
            let (line, snippet) = if let Some(ref re) = full_regex {
                let result = find_best_match(&doc.content, re, options.context);
                if result.0 > 0 {
                    result
                } else {
                    // Fall back to individual word matching
                    find_best_word_match(&doc.content, &word_regexes, options.context)
                }
            } else {
                find_best_word_match(&doc.content, &word_regexes, options.context)
            };

            results.push(SearchResult {
                doc_id,
                score,
                path: PathBuf::from(&doc.path),
                line,
                snippet,
                header_context: String::new(), // Query parsing doesn't have chunk context
                language: None,                // Query parsing doesn't have chunk metadata
                links: vec![],
                summary: None,
            });
        }
    }

    Ok(results)
}

/// Find best matching line using any of the word patterns
fn find_best_word_match(
    content: &str,
    word_regexes: &[regex::Regex],
    context: usize,
) -> (usize, String) {
    if word_regexes.is_empty() {
        return (0, extract_snippet(content, context));
    }

    // Score each line by number of matching words
    let mut best_line = 0;
    let mut best_score = 0;

    for (line_num, line) in content.lines().enumerate() {
        let match_count = word_regexes.iter().filter(|re| re.is_match(line)).count();
        if match_count > best_score {
            best_score = match_count;
            best_line = line_num;
        }
    }

    if best_score > 0 {
        (
            best_line + 1,
            extract_snippet_around(content, best_line, context),
        )
    } else {
        (0, extract_snippet(content, context))
    }
}

/// Extract first N lines as a snippet
fn extract_snippet(content: &str, max_lines: usize) -> String {
    content
        .lines()
        .take(max_lines)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Extract N lines around a target line
fn extract_snippet_around(content: &str, target_line: usize, context: usize) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let start = target_line.saturating_sub(context);
    let end = (target_line + context + 1).min(lines.len());
    lines[start..end].join("\n")
}

/// Index a single file
///
/// Reads the file, splits into chunks, generates embeddings for each chunk,
/// and stores in the database.
pub fn index_file(db: &DB, embedder: &mut Embedder, path: &std::path::Path) -> Result<Option<u32>> {
    // Use the generic backend function
    index_file_backend(db, embedder, path)
}

/// Index a single file using any EmbedderBackend
///
/// This is the generic version that works with any backend implementing EmbedderBackend.
/// Uses batch embedding and batch DB inserts for better performance.
pub fn index_file_backend<E: EmbedderBackend>(
    db: &DB,
    embedder: &mut E,
    path: &std::path::Path,
) -> Result<Option<u32>> {
    // Read file content (handles both plain text and documents like PDFs)
    let content = match crate::document::read_file_content(path)? {
        Some(c) => c,
        None => {
            return Ok(None);
        }
    };

    // Skip empty files
    if content.trim().is_empty() {
        return Ok(None);
    }

    // Check if we need to reindex
    if !db.needs_reindex(path, &content)? {
        // Get existing document ID
        if let Some(doc) = db.get_document_by_path(path)? {
            return Ok(Some(doc.id));
        }
    }

    // Add document to DB
    let doc_id = db.add_document(path, &content)?;

    // Delete old chunks if re-indexing
    db.delete_chunks_for_doc(doc_id)?;

    // Split into chunks
    let chunks = chunk_document(&content);
    if chunks.is_empty() {
        return Ok(Some(doc_id));
    }

    tracing::debug!("Indexing {} with {} chunk(s)", path.display(), chunks.len());

    // Preprocess code files for better tokenization
    // Split identifiers like "getUserName" -> "get user name" for the tokenizer
    let is_code = is_code_file(path);
    let processed_texts: Vec<String> = if is_code {
        chunks.iter().map(|c| preprocess_code(&c.content)).collect()
    } else {
        chunks.iter().map(|c| c.content.clone()).collect()
    };

    // Batch embed all chunks at once (3-5x faster than one-by-one)
    let texts: Vec<&str> = processed_texts.iter().map(|s| s.as_str()).collect();
    let embeddings = EmbedderBackend::embed_batch(embedder, &texts).with_context(|| {
        format!(
            "Failed to batch embed {} chunks of {}",
            chunks.len(),
            path.display()
        )
    })?;

    // Prepare batch insert data with all chunk metadata including links
    let chunk_data: Vec<(
        usize,
        usize,
        usize,
        &str,
        &str,
        Option<&str>,
        Vec<StoredLink>,
        &[f32],
        usize,
    )> = chunks
        .iter()
        .zip(embeddings.iter())
        .map(|(chunk, emb)| {
            // Convert ChunkLink to StoredLink
            let links: Vec<StoredLink> = chunk
                .links
                .iter()
                .map(|l| StoredLink {
                    text: l.text.clone(),
                    target: l.target.clone(),
                    is_internal: l.is_internal,
                })
                .collect();
            (
                chunk.index,
                chunk.start_line,
                chunk.end_line,
                chunk.header_context.as_str(),
                chunk.content_hash.as_str(),
                chunk.language.as_deref(),
                links,
                emb.data.as_slice(),
                emb.num_tokens,
            )
        })
        .collect();

    // Batch insert all chunks with embeddings and links (5-10x faster than one-by-one)
    db.batch_add_chunks_with_links(doc_id, &chunk_data)?;

    Ok(Some(doc_id))
}

/// Statistics from incremental file updates
#[derive(Debug, Clone, Default)]
pub struct IncrementalUpdateStats {
    /// Number of chunks that were unchanged (not re-embedded)
    pub unchanged_chunks: usize,
    /// Number of new chunks that were added
    pub new_chunks: usize,
    /// Number of old chunks that were deleted
    pub deleted_chunks: usize,
}

/// Index a single file with differential chunk updates
///
/// This is optimized for incremental updates where most chunks are unchanged:
/// - Only embeds chunks whose content has changed
/// - Keeps existing embeddings for unchanged chunks
/// - Deletes chunks that no longer exist
///
/// Expected: 10-100x faster than full re-index for small edits
pub fn index_file_incremental<E: EmbedderBackend>(
    db: &DB,
    embedder: &mut E,
    path: &std::path::Path,
) -> Result<(Option<u32>, IncrementalUpdateStats)> {
    let mut stats = IncrementalUpdateStats::default();

    // Read file content (handles both plain text and documents like PDFs)
    let content = match crate::document::read_file_content(path)? {
        Some(c) => c,
        None => {
            return Ok((None, stats));
        }
    };

    // Skip empty files
    if content.trim().is_empty() {
        return Ok((None, stats));
    }

    // Check if document exists
    let existing_doc = db.get_document_by_path(path)?;

    // If document doesn't exist, use regular indexing
    if existing_doc.is_none() {
        let doc_id = index_file_backend(db, embedder, path)?;
        if let Some(id) = doc_id {
            let chunk_count = db.get_chunks_for_doc(id)?.len();
            stats.new_chunks = chunk_count;
        }
        return Ok((doc_id, stats));
    }

    let doc = existing_doc.unwrap();

    // Check if content actually changed
    if !db.needs_reindex(path, &content)? {
        // No changes needed
        return Ok((Some(doc.id), stats));
    }

    // Update document content and hash
    let doc_id = db.add_document(path, &content)?;

    // Get old chunks with their embeddings for reuse
    // Map: content_hash -> (chunk_id, embeddings, num_tokens)
    let old_chunks = db.get_chunks_for_doc(doc_id)?;
    let mut old_embeddings_map: HashMap<String, (Vec<f32>, usize)> = HashMap::new();

    for old_chunk in &old_chunks {
        if !old_chunk.content_hash.is_empty() {
            if let Some((emb, num_tokens)) = db.get_chunk_embeddings(old_chunk.id)? {
                old_embeddings_map.insert(old_chunk.content_hash.clone(), (emb, num_tokens));
            }
        }
    }

    // Compute new chunks
    let new_chunks = chunk_document(&content);
    if new_chunks.is_empty() {
        stats.deleted_chunks = old_chunks.len();
        db.delete_chunks_for_doc(doc_id)?;
        return Ok((Some(doc_id), stats));
    }

    // Categorize chunks: unchanged (can reuse embeddings) vs new (need embedding)
    let mut chunks_to_embed: Vec<&crate::chunker::Chunk> = Vec::new();
    let mut reusable_chunks: Vec<(&crate::chunker::Chunk, &Vec<f32>, usize)> = Vec::new();

    for chunk in &new_chunks {
        if let Some((emb, num_tokens)) = old_embeddings_map.get(&chunk.content_hash) {
            reusable_chunks.push((chunk, emb, *num_tokens));
        } else {
            chunks_to_embed.push(chunk);
        }
    }

    // Count deleted chunks (old hashes not in new)
    let new_hashes: HashSet<&str> = new_chunks.iter().map(|c| c.content_hash.as_str()).collect();
    stats.deleted_chunks = old_chunks
        .iter()
        .filter(|c| !c.content_hash.is_empty() && !new_hashes.contains(c.content_hash.as_str()))
        .count();

    stats.new_chunks = chunks_to_embed.len();
    stats.unchanged_chunks = reusable_chunks.len();

    tracing::debug!(
        "Incremental update {}: {} unchanged, {} new, {} deleted",
        path.display(),
        stats.unchanged_chunks,
        stats.new_chunks,
        stats.deleted_chunks,
    );

    // Delete all old chunks (we'll re-insert with correct indices)
    db.delete_chunks_for_doc(doc_id)?;

    // If nothing to embed, just re-insert reusable chunks
    if chunks_to_embed.is_empty() {
        // Re-insert reusable chunks with their old embeddings and full metadata
        let chunk_data: Vec<(
            usize,
            usize,
            usize,
            &str,
            &str,
            Option<&str>,
            Vec<StoredLink>,
            &[f32],
            usize,
        )> = reusable_chunks
            .iter()
            .map(|(chunk, emb, num_tokens)| {
                // Convert ChunkLink to StoredLink
                let links: Vec<StoredLink> = chunk
                    .links
                    .iter()
                    .map(|l| StoredLink {
                        text: l.text.clone(),
                        target: l.target.clone(),
                        is_internal: l.is_internal,
                    })
                    .collect();
                (
                    chunk.index,
                    chunk.start_line,
                    chunk.end_line,
                    chunk.header_context.as_str(),
                    chunk.content_hash.as_str(),
                    chunk.language.as_deref(),
                    links,
                    emb.as_slice(),
                    *num_tokens,
                )
            })
            .collect();
        db.batch_add_chunks_with_links(doc_id, &chunk_data)?;
        return Ok((Some(doc_id), stats));
    }

    // Preprocess code files for better tokenization
    let is_code = is_code_file(path);
    let processed_texts: Vec<String> = if is_code {
        chunks_to_embed
            .iter()
            .map(|c| preprocess_code(&c.content))
            .collect()
    } else {
        chunks_to_embed.iter().map(|c| c.content.clone()).collect()
    };

    // Embed only the new chunks
    let texts: Vec<&str> = processed_texts.iter().map(|s| s.as_str()).collect();
    let new_embeddings = EmbedderBackend::embed_batch(embedder, &texts)
        .with_context(|| format!("Failed to embed {} new chunks", chunks_to_embed.len()))?;

    // Build a map of new embeddings by content_hash for lookup
    let new_emb_map: HashMap<&str, &crate::embedder::EmbeddingResult> = chunks_to_embed
        .iter()
        .zip(new_embeddings.iter())
        .map(|(chunk, emb)| (chunk.content_hash.as_str(), emb))
        .collect();

    // Prepare all chunks for insertion (in order by index)
    // Tuple: (index, start, end, header, hash, language, links, embeddings, num_tokens)
    let mut all_chunk_data: Vec<(
        usize,
        usize,
        usize,
        String,
        String,
        Option<String>,
        Vec<StoredLink>,
        Vec<f32>,
        usize,
    )> = Vec::new();

    for chunk in &new_chunks {
        let (emb_data, num_tokens) =
            if let Some((old_emb, old_num_tokens)) = old_embeddings_map.get(&chunk.content_hash) {
                // Reuse old embedding
                (old_emb.clone(), *old_num_tokens)
            } else if let Some(new_emb) = new_emb_map.get(chunk.content_hash.as_str()) {
                // Use newly computed embedding
                (new_emb.data.clone(), new_emb.num_tokens)
            } else {
                // Shouldn't happen, but embed if needed
                tracing::warn!("Chunk hash {} not found in either map", chunk.content_hash);
                let embed_text = if is_code {
                    preprocess_code(&chunk.content)
                } else {
                    chunk.content.clone()
                };
                let result = EmbedderBackend::embed_document(embedder, &embed_text)?;
                (result.data, result.num_tokens)
            };

        // Convert ChunkLink to StoredLink
        let links: Vec<StoredLink> = chunk
            .links
            .iter()
            .map(|l| StoredLink {
                text: l.text.clone(),
                target: l.target.clone(),
                is_internal: l.is_internal,
            })
            .collect();

        all_chunk_data.push((
            chunk.index,
            chunk.start_line,
            chunk.end_line,
            chunk.header_context.clone(),
            chunk.content_hash.clone(),
            chunk.language.clone(),
            links,
            emb_data,
            num_tokens,
        ));
    }

    // Insert all chunks with full metadata
    let chunk_refs: Vec<(
        usize,
        usize,
        usize,
        &str,
        &str,
        Option<&str>,
        Vec<StoredLink>,
        &[f32],
        usize,
    )> = all_chunk_data
        .iter()
        .map(|(idx, start, end, header, hash, lang, links, emb, n_tok)| {
            (
                *idx,
                *start,
                *end,
                header.as_str(),
                hash.as_str(),
                lang.as_deref(),
                links.clone(),
                emb.as_slice(),
                *n_tok,
            )
        })
        .collect();

    db.batch_add_chunks_with_links(doc_id, &chunk_refs)?;

    Ok((Some(doc_id), stats))
}

/// Index a single file with differential updates (convenience wrapper)
pub fn index_file_differential(
    db: &DB,
    embedder: &mut Embedder,
    path: &std::path::Path,
) -> Result<(Option<u32>, IncrementalUpdateStats)> {
    index_file_incremental(db, embedder, path)
}

/// Index all files in a directory
///
/// Recursively indexes all supported files.
pub fn index_directory(
    db: &DB,
    embedder: &mut Embedder,
    path: &std::path::Path,
    progress: Option<&indicatif::ProgressBar>,
) -> Result<IndexStats> {
    index_directory_with_options(
        db,
        embedder,
        path,
        progress,
        IndexDirectoryOptions::default(),
    )
}

/// Index all files in a directory with custom options
///
/// Recursively indexes all supported files. Use options to control behavior.
pub fn index_directory_with_options(
    db: &DB,
    embedder: &mut Embedder,
    path: &std::path::Path,
    progress: Option<&indicatif::ProgressBar>,
    options: IndexDirectoryOptions,
) -> Result<IndexStats> {
    index_directory_with_options_backend(db, embedder, path, progress, options)
}

/// Index all files in a directory using any EmbedderBackend
pub fn index_directory_backend<E: EmbedderBackend>(
    db: &DB,
    embedder: &mut E,
    path: &std::path::Path,
    progress: Option<&indicatif::ProgressBar>,
) -> Result<IndexStats> {
    index_directory_with_options_backend(
        db,
        embedder,
        path,
        progress,
        IndexDirectoryOptions::default(),
    )
}

/// Default cross-file batch size for embedding (P1 optimization)
///
/// When indexing directories, chunks are accumulated across files and embedded
/// in batches of this size. This maximizes GPU/NPU throughput by amortizing
/// kernel launch overhead across many chunks.
pub const DEFAULT_CROSSFILE_BATCH_SIZE: usize = 64;

/// Compute optimal batch size based on device type and backend (P3 optimization)
///
/// This function returns device-specific batch sizes that balance memory usage
/// with throughput. The values are based on:
/// - GPU memory constraints (larger batches may OOM on limited VRAM)
/// - Kernel launch overhead amortization
/// - Model size (~2GB for T5-base at batch size 64)
///
/// # Arguments
/// * `backend_kind` - The embedding backend being used
///
/// # Returns
/// Optimal batch size for the given device/backend combination
pub fn optimal_batch_size(backend_kind: &crate::EmbedderBackendKind) -> usize {
    use crate::EmbedderBackendKind;

    match backend_kind {
        // CUDA backend: tune based on typical VRAM availability
        // ~2GB VRAM per batch of 64 with T5-base model
        #[cfg(feature = "cuda")]
        EmbedderBackendKind::Cuda => 128, // Assume decent VRAM (>=4GB), can process larger batches

        // CoreML/Metal: Apple Silicon has unified memory, can handle larger batches
        // Neural Engine has its own memory management
        #[cfg(feature = "coreml")]
        EmbedderBackendKind::CoreMl => 64, // Conservative for ANE memory bandwidth

        // ONNX: Depends on execution provider, use conservative default
        #[cfg(feature = "onnx")]
        EmbedderBackendKind::Onnx => 32, // Conservative for cross-platform compatibility

        // Candle with Metal (macOS): unified memory allows moderate batches
        // Candle with CPU: memory-limited, smaller batches
        EmbedderBackendKind::Candle => {
            #[cfg(target_os = "macos")]
            {
                64 // Metal unified memory
            }
            #[cfg(not(target_os = "macos"))]
            {
                16 // CPU is memory-limited
            }
        }
    }
}

/// A chunk pending embedding during cross-file batch processing
struct PendingChunk {
    doc_id: u32,
    chunk_index: usize,
    start_line: usize,
    end_line: usize,
    header_context: String,
    content: String,
    content_hash: String,
    language: Option<String>,
    links: Vec<StoredLink>,
}

/// Process a batch of pending chunks: embed all and insert into DB
///
/// Uses cross-file deduplication with optional Bloom filter to skip embedding
/// for chunks whose content already exists elsewhere in the index.
///
/// When `force` is true, bypasses deduplication and re-embeds all chunks.
fn process_pending_batch<E: EmbedderBackend>(
    db: &DB,
    embedder: &mut E,
    batch: &mut Vec<PendingChunk>,
    bloom: Option<&mut crate::dedup::BloomDedup>,
    force: bool,
) -> Result<()> {
    if batch.is_empty() {
        return Ok(());
    }

    // Separate chunks into: those with existing embeddings vs those needing new embeddings
    let mut reused_chunks: Vec<(&PendingChunk, Vec<f32>, usize)> = Vec::new();
    let mut chunks_to_embed: Vec<&PendingChunk> = Vec::new();

    for chunk in batch.iter() {
        // Skip deduplication when force is true
        if force {
            chunks_to_embed.push(chunk);
            continue;
        }

        // Check if content_hash might exist (via Bloom filter if available)
        let might_exist = match bloom.as_ref() {
            Some(bf) => bf.might_contain(&chunk.content_hash),
            None => true, // Without Bloom filter, always check DB
        };

        if might_exist && !chunk.content_hash.is_empty() {
            // Query DB for existing embedding with this content hash
            if let Some((emb, num_tokens)) =
                db.get_embedding_by_content_hash(&chunk.content_hash)?
            {
                tracing::debug!(
                    "Cross-file dedup: reusing embedding for hash {} (doc_id={})",
                    &chunk.content_hash[..8.min(chunk.content_hash.len())],
                    chunk.doc_id
                );
                reused_chunks.push((chunk, emb, num_tokens));
                continue;
            }
        }

        chunks_to_embed.push(chunk);
    }

    let reused_count = reused_chunks.len();
    let embed_count = chunks_to_embed.len();

    if reused_count > 0 {
        tracing::debug!(
            "Cross-file dedup: reusing {} embeddings, computing {} new",
            reused_count,
            embed_count
        );
    }

    // Batch embed only chunks that need new embeddings
    let new_embeddings = if !chunks_to_embed.is_empty() {
        let texts: Vec<&str> = chunks_to_embed.iter().map(|c| c.content.as_str()).collect();
        EmbedderBackend::embed_batch(embedder, &texts)
            .with_context(|| format!("Failed to batch embed {} chunks", chunks_to_embed.len()))?
    } else {
        Vec::new()
    };

    // Group all chunks by doc_id for efficient DB writes
    // Store: (chunk, embedding_data, num_tokens)
    let mut by_doc: std::collections::HashMap<u32, Vec<(&PendingChunk, Vec<f32>, usize)>> =
        std::collections::HashMap::new();

    // Add reused chunks
    for (chunk, emb, num_tokens) in reused_chunks {
        by_doc
            .entry(chunk.doc_id)
            .or_default()
            .push((chunk, emb, num_tokens));
    }

    // Add newly embedded chunks
    for (chunk, emb) in chunks_to_embed.iter().zip(new_embeddings.iter()) {
        by_doc
            .entry(chunk.doc_id)
            .or_default()
            .push((chunk, emb.data.clone(), emb.num_tokens));
    }

    // Add new hashes to Bloom filter
    if let Some(bloom) = bloom {
        for chunk in &chunks_to_embed {
            if !chunk.content_hash.is_empty() {
                bloom.add(&chunk.content_hash);
            }
        }
    }

    // Batch insert per document (with content hashes, language, and links for full metadata)
    for (doc_id, chunks) in by_doc {
        let chunk_data: Vec<(
            usize,
            usize,
            usize,
            &str,
            &str,
            Option<&str>,
            Vec<StoredLink>,
            &[f32],
            usize,
        )> = chunks
            .iter()
            .map(|(chunk, emb, num_tokens)| {
                (
                    chunk.chunk_index,
                    chunk.start_line,
                    chunk.end_line,
                    chunk.header_context.as_str(),
                    chunk.content_hash.as_str(),
                    chunk.language.as_deref(),
                    chunk.links.clone(),
                    emb.as_slice(),
                    *num_tokens,
                )
            })
            .collect();
        db.batch_add_chunks_with_links(doc_id, &chunk_data)?;
    }

    batch.clear();
    Ok(())
}

/// Load or create a Bloom filter for content hash deduplication
///
/// If a persisted filter exists in the DB, loads it. Otherwise creates a new
/// filter sized appropriately for the existing index.
pub fn load_or_create_bloom_filter(db: &DB) -> Result<crate::dedup::BloomDedup> {
    // Try to load persisted filter
    if let Some(data) = db.load_bloom_filter()? {
        if let Some(filter) = crate::dedup::BloomDedup::from_bytes(&data) {
            tracing::debug!("Loaded Bloom filter with {} items", filter.len());
            return Ok(filter);
        }
        tracing::warn!("Invalid Bloom filter data; clearing persisted filter");
        if let Err(e) = db.clear_bloom_filter() {
            tracing::warn!("Failed to clear Bloom filter: {}", e);
        }
    }

    // Create new filter, sized for existing index + growth room
    let existing_count = db.content_hash_count().unwrap_or(0);
    let capacity = (existing_count + 10_000).max(100_000);

    let mut filter = crate::dedup::BloomDedup::with_capacity(capacity);

    // Initialize with existing hashes
    if existing_count > 0 {
        tracing::debug!(
            "Initializing Bloom filter with {} existing hashes",
            existing_count
        );
        for hash in db.get_all_content_hashes()? {
            filter.add(&hash);
        }
    }

    Ok(filter)
}

/// Index all files in a directory with custom options using any EmbedderBackend
///
/// Uses cross-file batch embedding (P1 optimization) for faster cold start.
/// Chunks are accumulated across files and embedded in batches of the configured
/// cross-file batch size, maximizing GPU throughput.
///
/// Cross-file deduplication uses a Bloom filter to avoid re-computing embeddings
/// for chunks that already exist elsewhere in the index.
///
/// When `options.pipelined` is true, uses the 3-thread pipeline architecture
/// which overlaps I/O, embedding, and DB writes for even faster throughput.
pub fn index_directory_with_options_backend<E: EmbedderBackend>(
    db: &DB,
    embedder: &mut E,
    path: &std::path::Path,
    progress: Option<&indicatif::ProgressBar>,
    options: IndexDirectoryOptions,
) -> Result<IndexStats> {
    // Use pipelined architecture if requested
    if options.pipelined {
        return index_directory_pipelined(db, embedder, path, progress, options);
    }

    let mut stats = IndexStats::default();

    // Collect all files first
    let files = collect_files(path, &options)?;
    stats.total_files = files.len();

    if let Some(pb) = progress {
        pb.set_length(files.len() as u64);
    }

    // Initialize Bloom filter for cross-file deduplication
    let mut bloom = if options.use_bloom_filter {
        Some(load_or_create_bloom_filter(db)?)
    } else {
        None
    };

    let batch_size = options.crossfile_batch_size.max(1);

    // Cross-file batch buffer for embedding
    let mut pending_chunks: Vec<PendingChunk> = Vec::with_capacity(batch_size);

    // Process each file: read, chunk, accumulate for batch embedding
    for file_path in &files {
        if let Some(pb) = progress {
            pb.set_message(
                file_path
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default(),
            );
        }

        // Read file content (handles both plain text and documents like PDFs)
        let content = match crate::document::read_file_content(file_path) {
            Ok(Some(c)) => c,
            Ok(None) | Err(_) => {
                stats.skipped_files += 1;
                if let Some(pb) = progress {
                    pb.inc(1);
                }
                continue;
            }
        };

        // Skip empty files
        if content.trim().is_empty() {
            stats.skipped_files += 1;
            if let Some(pb) = progress {
                pb.inc(1);
            }
            continue;
        }

        // Check if we need to reindex (skip check when force is true)
        if !options.force && !db.needs_reindex(file_path, &content)? {
            stats.skipped_files += 1;
            if let Some(pb) = progress {
                pb.inc(1);
            }
            continue;
        }

        // Add document to DB
        let doc_id = db.add_document(file_path, &content)?;
        db.delete_chunks_for_doc(doc_id)?;

        // Split into chunks
        let chunks = chunk_document(&content);
        if chunks.is_empty() {
            stats.indexed_files += 1;
            if let Some(pb) = progress {
                pb.inc(1);
            }
            continue;
        }

        tracing::debug!(
            "Queuing {} chunk(s) from {} for batch embedding",
            chunks.len(),
            file_path.display()
        );

        // Add chunks to pending batch
        let is_code = is_code_file(file_path);
        for chunk in chunks {
            let embed_content = if is_code {
                preprocess_code(&chunk.content)
            } else {
                chunk.content
            };
            // Convert ChunkLink to StoredLink
            let links: Vec<StoredLink> = chunk
                .links
                .iter()
                .map(|l| StoredLink {
                    text: l.text.clone(),
                    target: l.target.clone(),
                    is_internal: l.is_internal,
                })
                .collect();
            pending_chunks.push(PendingChunk {
                doc_id,
                chunk_index: chunk.index,
                start_line: chunk.start_line,
                end_line: chunk.end_line,
                header_context: chunk.header_context,
                content: embed_content,
                content_hash: chunk.content_hash,
                language: chunk.language,
                links,
            });

            // Process batch when full
            if pending_chunks.len() >= batch_size {
                process_pending_batch(
                    db,
                    embedder,
                    &mut pending_chunks,
                    bloom.as_mut(),
                    options.force,
                )?;
            }
        }

        stats.indexed_files += 1;
        if let Some(pb) = progress {
            pb.inc(1);
        }
    }

    // Process remaining chunks
    if !pending_chunks.is_empty() {
        tracing::debug!("Processing final batch of {} chunks", pending_chunks.len());
        process_pending_batch(
            db,
            embedder,
            &mut pending_chunks,
            bloom.as_mut(),
            options.force,
        )?;
    }

    // Persist Bloom filter for future sessions
    if let Some(ref filter) = bloom {
        let data = filter.to_bytes();
        if let Err(e) = db.save_bloom_filter(&data) {
            tracing::warn!("Failed to persist Bloom filter: {}", e);
        } else {
            tracing::debug!("Persisted Bloom filter with {} items", filter.len());
        }
    }

    stats.total_lines = db.total_lines()?;

    Ok(stats)
}

/// Statistics from indexing
#[derive(Debug, Clone, Default)]
pub struct IndexStats {
    pub total_files: usize,
    pub indexed_files: usize,
    pub skipped_files: usize,
    pub failed_files: usize,
    pub total_lines: usize,
}

/// Options for directory indexing
#[derive(Debug, Clone)]
pub struct IndexDirectoryOptions {
    /// Allow indexing files in system temp directories (for testing)
    pub allow_temp_paths: bool,
    /// Use pipelined architecture for faster indexing (overlaps I/O, embedding, DB writes)
    pub pipelined: bool,
    /// Cross-file embedding batch size (tune for GPU throughput vs. memory)
    pub crossfile_batch_size: usize,
    /// Use rayon for parallel file reading/chunking (P3 optimization)
    /// This parallelizes I/O and chunking CPU work across multiple cores.
    pub parallel_file_reading: bool,
    /// Use Bloom filter for cross-file deduplication
    /// When enabled, chunks with identical content across different files
    /// reuse embeddings instead of re-computing them.
    pub use_bloom_filter: bool,
    /// Force re-indexing of all files, ignoring content hashes.
    /// Useful when the embedding model has changed or embeddings are corrupted.
    pub force: bool,
}

impl Default for IndexDirectoryOptions {
    fn default() -> Self {
        Self {
            allow_temp_paths: false,
            pipelined: false,
            crossfile_batch_size: DEFAULT_CROSSFILE_BATCH_SIZE,
            parallel_file_reading: false,
            use_bloom_filter: true, // Enabled by default for faster indexing
            force: false,
        }
    }
}

// =============================================================================
// Pipeline Architecture (P1 Optimization)
// =============================================================================
//
// The pipeline overlaps two operations:
//   1. Reader Thread: reads files from disk and chunks them
//   2. Main Thread: handles DB operations, embedding, and writes
//
// This improves throughput by reading/chunking the next file while the current
// batch is being embedded. Most beneficial when:
// - Filesystem I/O is slow (network drives, spinning disks)
// - Many small files need to be read
// - Chunking is CPU-intensive (complex markdown parsing)

/// Channel capacity for the reader → main thread pipeline
const READER_CHANNEL_CAPACITY: usize = 128;

/// Message sent from reader thread to main thread
/// Contains file content and pre-computed chunks (no DB access needed)
struct ReaderMessage {
    path: PathBuf,
    content: String,
    chunks: Vec<RawChunk>,
}

/// Raw chunk data before DB operations (no doc_id yet)
struct RawChunk {
    index: usize,
    start_line: usize,
    end_line: usize,
    header_context: String,
    content: String,
    content_hash: String,
    language: Option<String>,
    links: Vec<StoredLink>,
}

/// Index a directory using pipelined architecture for maximum throughput.
///
/// This function overlaps file I/O (reader thread) with embedding and DB writes
/// (main thread). The reader thread reads files and chunks them in parallel with
/// the main thread's embedding computations.
///
/// This is faster than sequential processing when:
/// - Filesystem I/O is slow (network drives, spinning disks)
/// - Many small files need to be read
/// - Chunking is CPU-intensive
///
/// When `options.parallel_file_reading` is true, the reader thread uses rayon
/// to parallelize file I/O and chunking across multiple CPU cores (P3 optimization).
pub fn index_directory_pipelined<E: EmbedderBackend>(
    db: &DB,
    embedder: &mut E,
    path: &std::path::Path,
    progress: Option<&indicatif::ProgressBar>,
    options: IndexDirectoryOptions,
) -> Result<IndexStats> {
    use crossbeam_channel::{bounded, Receiver, Sender};

    let mut stats = IndexStats::default();

    // Collect all files first
    let files = collect_files(path, &options)?;
    stats.total_files = files.len();

    if files.is_empty() {
        return Ok(stats);
    }

    if let Some(pb) = progress {
        pb.set_length(files.len() as u64);
    }

    // Create channel for reader -> main thread communication
    let (reader_tx, reader_rx): (Sender<ReaderMessage>, Receiver<ReaderMessage>) =
        bounded(READER_CHANNEL_CAPACITY);

    let parallel_reading = options.parallel_file_reading;

    // ---------------------------------------------------------------------
    // Reader Thread: reads files and chunks them (no DB access)
    // When parallel_file_reading is enabled, uses rayon for parallel I/O
    // ---------------------------------------------------------------------
    let reader_handle = thread::spawn(move || -> Vec<PathBuf> {
        if parallel_reading {
            // P3 Optimization: Parallel file reading using rayon
            read_files_parallel(files, reader_tx)
        } else {
            // Sequential file reading (original behavior)
            read_files_sequential(files, reader_tx)
        }
    });

    let batch_size = options.crossfile_batch_size.max(1);

    // ---------------------------------------------------------------------
    // Main Thread: DB operations + embedding + writes
    // ---------------------------------------------------------------------
    let mut pending: Vec<(u32, PendingChunk)> = Vec::with_capacity(batch_size);

    for msg in reader_rx {
        if let Some(pb) = progress {
            pb.set_message(
                msg.path
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default(),
            );
        }

        // Check if we need to reindex (skip check when force is true)
        if !options.force && !db.needs_reindex(&msg.path, &msg.content)? {
            stats.skipped_files += 1;
            if let Some(pb) = progress {
                pb.inc(1);
            }
            continue;
        }

        // Skip files with no chunks
        if msg.chunks.is_empty() {
            // Add document even if no chunks
            let _ = db.add_document(&msg.path, &msg.content)?;
            stats.indexed_files += 1;
            if let Some(pb) = progress {
                pb.inc(1);
            }
            continue;
        }

        // Add document to DB
        let doc_id = db.add_document(&msg.path, &msg.content)?;
        db.delete_chunks_for_doc(doc_id)?;

        let is_code = is_code_file(&msg.path);
        // Add all chunks from this file to pending batch
        for chunk in msg.chunks {
            let embed_content = if is_code {
                preprocess_code(&chunk.content)
            } else {
                chunk.content
            };
            pending.push((
                doc_id,
                PendingChunk {
                    doc_id,
                    chunk_index: chunk.index,
                    start_line: chunk.start_line,
                    end_line: chunk.end_line,
                    header_context: chunk.header_context,
                    content: embed_content,
                    content_hash: chunk.content_hash,
                    language: chunk.language,
                    links: chunk.links,
                },
            ));

            // Process batch when full
            if pending.len() >= batch_size {
                process_pending_batch_pipelined(db, embedder, &mut pending)?;
            }
        }

        stats.indexed_files += 1;
        if let Some(pb) = progress {
            pb.inc(1);
        }
    }

    // Process remaining chunks
    if !pending.is_empty() {
        tracing::debug!("Main: processing final batch of {} chunks", pending.len());
        process_pending_batch_pipelined(db, embedder, &mut pending)?;
    }

    // Wait for reader thread to complete and get skipped files
    let skipped_by_reader = reader_handle
        .join()
        .map_err(|_| anyhow::anyhow!("Reader thread panicked"))?;
    stats.skipped_files += skipped_by_reader.len();

    stats.total_lines = db.total_lines()?;

    Ok(stats)
}

/// Helper to process a batch of pending chunks in pipelined mode
fn process_pending_batch_pipelined<E: EmbedderBackend>(
    db: &DB,
    embedder: &mut E,
    pending: &mut Vec<(u32, PendingChunk)>,
) -> Result<()> {
    if pending.is_empty() {
        return Ok(());
    }

    // Batch embed all chunks
    let texts: Vec<&str> = pending.iter().map(|(_, c)| c.content.as_str()).collect();
    let embeddings = EmbedderBackend::embed_batch(embedder, &texts)
        .context("Failed to batch embed chunks in pipeline")?;

    // Group by doc_id for efficient writes
    // Tuple: (index, start, end, header, hash, language, links, embeddings, num_tokens)
    let mut by_doc: HashMap<
        u32,
        Vec<(
            usize,
            usize,
            usize,
            String,
            String,
            Option<String>,
            Vec<StoredLink>,
            Vec<f32>,
            usize,
        )>,
    > = HashMap::new();

    for ((doc_id, chunk), emb) in pending.drain(..).zip(embeddings.into_iter()) {
        by_doc.entry(doc_id).or_default().push((
            chunk.chunk_index,
            chunk.start_line,
            chunk.end_line,
            chunk.header_context,
            chunk.content_hash,
            chunk.language,
            chunk.links,
            emb.data,
            emb.num_tokens,
        ));
    }

    // Batch insert per document with full metadata
    for (doc_id, chunks) in by_doc {
        let chunk_data: Vec<(
            usize,
            usize,
            usize,
            &str,
            &str,
            Option<&str>,
            Vec<StoredLink>,
            &[f32],
            usize,
        )> = chunks
            .iter()
            .map(|(idx, start, end, header, hash, lang, links, emb, n_tok)| {
                (
                    *idx,
                    *start,
                    *end,
                    header.as_str(),
                    hash.as_str(),
                    lang.as_deref(),
                    links.clone(),
                    emb.as_slice(),
                    *n_tok,
                )
            })
            .collect();
        db.batch_add_chunks_with_links(doc_id, &chunk_data)?;
    }

    Ok(())
}

// =============================================================================
// File Reading Helpers (P3 Optimization - Parallel File Reading)
// =============================================================================

/// Read files sequentially (original behavior)
///
/// Processes files one at a time, suitable for spinning disks or
/// when memory is limited.
fn read_files_sequential(
    files: Vec<PathBuf>,
    reader_tx: crossbeam_channel::Sender<ReaderMessage>,
) -> Vec<PathBuf> {
    let mut skipped = Vec::new();

    for file_path in files {
        // Read file content (handles both plain text and documents like PDFs)
        let content = match crate::document::read_file_content(&file_path) {
            Ok(Some(c)) => c,
            Ok(None) | Err(_) => {
                skipped.push(file_path);
                continue;
            }
        };

        // Skip empty files
        if content.trim().is_empty() {
            skipped.push(file_path);
            continue;
        }

        // Split into chunks (CPU work)
        let raw_chunks = chunk_document(&content);

        // Convert to RawChunk with links converted to StoredLink
        let chunks: Vec<RawChunk> = raw_chunks
            .into_iter()
            .map(|c| {
                let links: Vec<StoredLink> = c
                    .links
                    .iter()
                    .map(|l| StoredLink {
                        text: l.text.clone(),
                        target: l.target.clone(),
                        is_internal: l.is_internal,
                    })
                    .collect();
                RawChunk {
                    index: c.index,
                    start_line: c.start_line,
                    end_line: c.end_line,
                    header_context: c.header_context,
                    content: c.content,
                    content_hash: c.content_hash,
                    language: c.language,
                    links,
                }
            })
            .collect();

        tracing::debug!(
            "Reader: queuing {} chunks from {}",
            chunks.len(),
            file_path.display()
        );

        // Send to main thread (blocks if channel is full - backpressure)
        if reader_tx
            .send(ReaderMessage {
                path: file_path,
                content,
                chunks,
            })
            .is_err()
        {
            // Channel closed, main thread is done
            break;
        }
    }
    skipped
}

/// Result of parallel file reading for a single file
enum FileReadResult {
    /// Successfully read and chunked file
    Success(ReaderMessage),
    /// File was skipped (empty, unreadable, etc.)
    Skipped(PathBuf),
}

/// Read files in parallel using rayon (P3 optimization)
///
/// Uses rayon's parallel iterator to read and chunk multiple files
/// simultaneously. This maximizes I/O throughput on SSDs and parallelizes
/// CPU-intensive chunking work across all cores.
///
/// Results are collected in parallel, then sent sequentially to the main
/// thread to maintain ordering and avoid channel contention.
fn read_files_parallel(
    files: Vec<PathBuf>,
    reader_tx: crossbeam_channel::Sender<ReaderMessage>,
) -> Vec<PathBuf> {
    use rayon::prelude::*;

    // Process all files in parallel using rayon
    // This parallelizes both I/O (on SSDs) and chunking (CPU-bound)
    let results: Vec<FileReadResult> = files
        .into_par_iter()
        .map(|file_path| {
            // Read file content (handles both plain text and documents like PDFs)
            let content = match crate::document::read_file_content(&file_path) {
                Ok(Some(c)) => c,
                Ok(None) | Err(_) => {
                    return FileReadResult::Skipped(file_path);
                }
            };

            // Skip empty files
            if content.trim().is_empty() {
                return FileReadResult::Skipped(file_path);
            }

            // Split into chunks (CPU work - benefits from parallelization)
            let raw_chunks = chunk_document(&content);

            // Convert to RawChunk with links converted to StoredLink
            let chunks: Vec<RawChunk> = raw_chunks
                .into_iter()
                .map(|c| {
                    let links: Vec<StoredLink> = c
                        .links
                        .iter()
                        .map(|l| StoredLink {
                            text: l.text.clone(),
                            target: l.target.clone(),
                            is_internal: l.is_internal,
                        })
                        .collect();
                    RawChunk {
                        index: c.index,
                        start_line: c.start_line,
                        end_line: c.end_line,
                        header_context: c.header_context,
                        content: c.content,
                        content_hash: c.content_hash,
                        language: c.language,
                        links,
                    }
                })
                .collect();

            tracing::debug!(
                "Parallel reader: processed {} chunks from {}",
                chunks.len(),
                file_path.display()
            );

            FileReadResult::Success(ReaderMessage {
                path: file_path,
                content,
                chunks,
            })
        })
        .collect();

    // Send results sequentially to maintain channel ordering
    // and collect skipped files
    let mut skipped = Vec::new();
    for result in results {
        match result {
            FileReadResult::Success(msg) => {
                if reader_tx.send(msg).is_err() {
                    // Channel closed, main thread is done
                    break;
                }
            }
            FileReadResult::Skipped(path) => {
                skipped.push(path);
            }
        }
    }
    skipped
}

/// Collect all indexable files in a directory
fn collect_files(path: &std::path::Path, options: &IndexDirectoryOptions) -> Result<Vec<PathBuf>> {
    // Skip system temp directories entirely (unless explicitly allowed)
    if !options.allow_temp_paths && is_system_temp_path(path) {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    collect_files_recursive(path, &mut files)?;
    files.sort();
    Ok(files)
}

/// Check if a path is under a system temporary directory
pub fn is_system_temp_path(path: &std::path::Path) -> bool {
    let temp_prefixes = [
        std::path::Path::new("/tmp"),
        std::path::Path::new("/private/tmp"),
        std::path::Path::new("/var/tmp"),
        std::path::Path::new("/var/folders"),
    ];

    if temp_prefixes.iter().any(|prefix| path.starts_with(prefix)) {
        return true;
    }

    // Also canonicalize to resolve symlinks (e.g., /tmp -> /private/tmp on macOS)
    if let Ok(canonical) = path.canonicalize() {
        if temp_prefixes
            .iter()
            .any(|prefix| canonical.starts_with(prefix))
        {
            return true;
        }
    }

    false
}

fn collect_files_recursive(path: &std::path::Path, files: &mut Vec<PathBuf>) -> Result<()> {
    use std::fs;

    if path.is_file() {
        if should_index_file(path) {
            files.push(path.to_path_buf());
        }
        return Ok(());
    }

    if !path.is_dir() {
        return Ok(());
    }

    // Skip common non-source directories
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        if should_skip_dir(name) {
            return Ok(());
        }
    }

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let entry_path = entry.path();

        if entry_path.is_dir() {
            collect_files_recursive(&entry_path, files)?;
        } else if should_index_file(&entry_path) {
            files.push(entry_path);
        }
    }

    Ok(())
}

/// Check if a file should be indexed
fn should_index_file(path: &std::path::Path) -> bool {
    crate::file_types::is_indexable_path(path)
}

/// Check if a directory should be skipped
pub fn should_skip_dir(name: &str) -> bool {
    const SKIP_DIRS: &[&str] = &[
        // Version control
        ".git",
        ".svn",
        ".hg",
        // IDEs and editors
        ".idea",
        ".vscode",
        ".vs",
        // Package managers / dependencies
        "node_modules",
        "vendor",
        "bower_components",
        // Build outputs
        "target",
        "dist",
        "build",
        "out",
        ".output",
        // Build tool caches
        ".gradle",
        ".parcel-cache",
        ".turbo",
        // Python
        "__pycache__",
        ".cache",
        ".tox",
        ".venv",
        "venv",
        ".eggs",
        ".mypy_cache",
        ".pytest_cache",
        ".ruff_cache",
        "__pypackages__",
        // Test coverage
        "coverage",
        ".nyc_output",
        // Framework-specific
        ".next",
        ".nuxt",
        ".svelte-kit",
        ".angular",
        // .NET / C#
        "obj",
        "bin",
        "packages",
        // Elixir / Erlang
        "_build",
        "deps",
        ".elixir_ls",
        // Ruby
        ".bundle",
        // Jupyter
        ".ipynb_checkpoints",
        // Documentation generators
        "_site",
        ".docusaurus",
        ".vuepress",
        // Infrastructure as code
        ".terraform",
        ".vagrant",
        // CMake
        "CMakeFiles",
    ];

    SKIP_DIRS.iter().any(|dir| dir.eq_ignore_ascii_case(name))
}

/// Execute a clustered search using LazyIndex
///
/// Uses the pre-built LazyIndex for faster search on large indices.
/// Falls back to brute-force semantic search if the index is empty.
pub fn search_clustered(
    db: &DB,
    lazy_index: &crate::index::LazyIndex,
    embedder: &mut Embedder,
    query: &str,
    options: SearchOptions,
) -> Result<Vec<SearchResult>> {
    // If lazy index is empty, fall back to brute force
    if lazy_index.total_documents() == 0 {
        return semantic_search_backend(db, embedder, query, &options);
    }

    // Embed query
    let query_emb = embedder
        .embed_query(query)
        .context("Failed to embed query")?;

    let query_vec = crate::embedder::embeddings_to_vec(&query_emb)?;
    let num_tokens = query_emb.dims()[0];

    // Search using LazyIndex MaxSim
    let scored = lazy_index.search_maxsim(&query_vec, num_tokens, options.top_k * 2)?;

    // Filter by threshold and convert to (doc_id, score) format
    let scored: Vec<(u32, f32)> = scored
        .into_iter()
        .filter(|(score, _)| *score >= options.threshold)
        .take(options.top_k)
        .map(|(score, doc_id)| (doc_id, score))
        .collect();

    // Build results with document details
    build_results(db, &scored, &options, query)
}

/// Execute a clustered search using LazyIndex with any EmbedderBackend
///
/// Generic version of `search_clustered` that works with any embedding backend.
pub fn search_clustered_backend<E: EmbedderBackend>(
    db: &DB,
    lazy_index: &crate::index::LazyIndex,
    embedder: &mut E,
    query: &str,
    options: SearchOptions,
) -> Result<Vec<SearchResult>> {
    // If lazy index is empty, fall back to brute force
    if lazy_index.total_documents() == 0 {
        return semantic_search_backend(db, embedder, query, &options);
    }

    // Optionally preprocess query for code search
    let processed_query = if options.preprocess_query {
        preprocess_query(query)
    } else {
        query.to_string()
    };

    // Embed query using the trait method
    let query_result = EmbedderBackend::embed_query(embedder, &processed_query)
        .context("Failed to embed query")?;

    // Convert to vec for LazyIndex
    let num_tokens = query_result.num_tokens;

    // Search using LazyIndex MaxSim
    let scored = lazy_index.search_maxsim(&query_result.data, num_tokens, options.top_k * 2)?;

    // Filter by threshold and convert to (doc_id, score) format
    let scored: Vec<(u32, f32)> = scored
        .into_iter()
        .filter(|(score, _)| *score >= options.threshold)
        .take(options.top_k)
        .map(|(score, doc_id)| (doc_id, score))
        .collect();

    // Build results with document details
    build_results(db, &scored, &options, query)
}

/// Populate a LazyIndex from the database
///
/// Loads chunk embeddings when available, falling back to document embeddings.
/// This enables clustered search for faster queries.
pub fn populate_lazy_index(db: &DB, lazy_index: &mut crate::index::LazyIndex) -> Result<usize> {
    let chunk_embeddings = db.get_all_chunk_embeddings()?;
    if !chunk_embeddings.is_empty() {
        let mut seen_docs = HashSet::new();

        for chunk in chunk_embeddings {
            lazy_index.add_multi(chunk.doc_id, &chunk.embeddings, chunk.num_tokens)?;
            seen_docs.insert(chunk.doc_id);
        }

        return Ok(seen_docs.len());
    }

    let doc_ids = db.get_all_doc_ids()?;
    let mut count = 0;

    for doc_id in doc_ids {
        if let Some((emb_data, num_tokens)) = db.get_embeddings(doc_id)? {
            lazy_index.add_multi(doc_id, &emb_data, num_tokens)?;
            count += 1;
        }
    }

    Ok(count)
}

/// Get the index state file path for a given database path
///
/// Returns the path where the LazyIndex state should be stored.
/// For a database at `~/.local/share/sg/index.db`, returns `~/.local/share/sg/index_state.bin`.
pub fn get_index_state_path(db_path: &Path) -> PathBuf {
    db_path.with_extension("state.bin")
}

/// Load or create a LazyIndex for the given database
///
/// This function:
/// 1. Checks if a saved index state exists
/// 2. Loads the saved state if present (fast path)
/// 3. Creates a new index and populates it from the DB if not (slow path)
/// 4. Returns the index along with whether it was loaded from cache
///
/// # Arguments
/// * `db` - The database to load embeddings from
/// * `db_path` - Path to the database file (used to locate the index state file)
/// * `num_clusters` - Number of clusters for a new index (ignored if loading from cache)
///
/// # Returns
/// * `(LazyIndex, bool)` - The index and whether it was loaded from cache
pub fn load_or_create_index(
    db: &DB,
    db_path: &Path,
    num_clusters: usize,
) -> Result<(crate::index::LazyIndex, bool)> {
    let state_path = get_index_state_path(db_path);

    // Try to load from cache
    if state_path.exists() {
        match crate::index::LazyIndex::load(&state_path) {
            Ok(mut index) => {
                // Populate with embeddings from DB
                let doc_count = populate_lazy_index(db, &mut index)?;
                tracing::info!(
                    "Loaded index state from {:?}, populated with {} documents",
                    state_path,
                    doc_count
                );
                return Ok((index, true));
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to load index state from {:?}: {}, creating new index",
                    state_path,
                    e
                );
            }
        }
    }

    // Create new index and populate from DB
    let mut index = crate::index::LazyIndex::new(num_clusters);
    let doc_count = populate_lazy_index(db, &mut index)?;

    tracing::info!(
        "Created new index with {} clusters, populated with {} documents",
        num_clusters,
        doc_count
    );

    Ok((index, false))
}

/// Save the LazyIndex state for the given database
///
/// The state is saved to a file alongside the database.
pub fn save_index_state(index: &crate::index::LazyIndex, db_path: &Path) -> Result<()> {
    let state_path = get_index_state_path(db_path);
    index.save(&state_path)?;
    tracing::debug!("Saved index state to {:?}", state_path);
    Ok(())
}

// =========================================================================
// Image search (CLIP embeddings)
// =========================================================================

/// Result from image search
#[cfg(feature = "clip")]
#[derive(Debug, Clone)]
pub struct ImageSearchResult {
    /// Image ID in the database
    pub image_id: u32,
    /// Path to the image file
    pub path: String,
    /// Similarity score (cosine similarity, 0.0 to 1.0)
    pub score: f32,
}

/// Options for image search
#[cfg(feature = "clip")]
#[derive(Debug, Clone)]
pub struct ImageSearchOptions {
    /// Maximum number of results to return
    pub max_results: usize,
    /// Minimum similarity score threshold (0.0 to 1.0)
    pub min_score: f32,
}

#[cfg(feature = "clip")]
impl Default for ImageSearchOptions {
    fn default() -> Self {
        Self {
            max_results: 10,
            min_score: 0.0,
        }
    }
}

/// Search images by text query using CLIP embeddings
///
/// Uses CLIP's cross-modal capability to find images matching a text description.
/// The text query is embedded using CLIP's text encoder and compared against
/// pre-computed image embeddings using cosine similarity.
///
/// # Arguments
/// * `db` - Database containing indexed images
/// * `clip_embedder` - CLIP model for text embedding
/// * `query` - Text description to search for
/// * `options` - Search options (max_results, min_score)
///
/// # Returns
/// Vector of ImageSearchResult sorted by score (highest first)
#[cfg(feature = "clip")]
pub fn search_images(
    db: &crate::storage::DB,
    clip_embedder: &mut crate::embedder_clip::ClipEmbedder,
    query: &str,
    options: ImageSearchOptions,
) -> Result<Vec<ImageSearchResult>> {
    use crate::embedder::EmbedderBackend;

    // Embed the query text using CLIP
    let query_embedding = clip_embedder.embed_query(query)?;
    let query_vec: &[f32] = &query_embedding.data;

    // Get all image embeddings
    let image_embeddings = db.get_all_image_embeddings()?;

    if image_embeddings.is_empty() {
        return Ok(vec![]);
    }

    // Compute cosine similarity for each image
    let mut results: Vec<ImageSearchResult> = image_embeddings
        .iter()
        .map(|img| {
            let score = cosine_similarity(query_vec, &img.embedding);
            ImageSearchResult {
                image_id: img.image_id,
                path: img.path.clone(),
                score,
            }
        })
        .filter(|r| r.score >= options.min_score)
        .collect();

    // Sort by score descending
    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Truncate to max_results
    results.truncate(options.max_results);

    Ok(results)
}

/// Compute cosine similarity between two vectors
#[cfg(feature = "clip")]
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot_product / (norm_a * norm_b)
}

/// Index a single image file using CLIP embeddings
///
/// Computes the CLIP embedding for the image and stores it in the database.
/// Returns the image ID if successful.
#[cfg(feature = "clip")]
pub fn index_image(
    db: &crate::storage::DB,
    clip_embedder: &mut crate::embedder_clip::ClipEmbedder,
    path: &Path,
) -> Result<Option<u32>> {
    use sha2::{Digest, Sha256};

    // Check if file exists
    if !path.exists() {
        return Ok(None);
    }

    // Read file and compute hash
    let file_data = std::fs::read(path)?;
    let hash = format!("{:x}", Sha256::digest(&file_data));

    // Check if needs re-indexing
    if !db.needs_image_reindex(path, &hash)? {
        // Already indexed with same hash
        if let Some(img) = db.get_image_by_path(path)? {
            return Ok(Some(img.id));
        }
    }

    // Load image and get dimensions
    let image = image::open(path)?;
    let (width, height) = (image.width(), image.height());

    // Embed the image
    let embedding = clip_embedder.embed_image_file(path)?;

    // Store in database
    let image_id = db.add_image(path, &hash, Some(width), Some(height))?;
    db.add_image_embedding(image_id, &embedding.data)?;

    Ok(Some(image_id))
}

/// Index all images in a directory using CLIP embeddings
///
/// Recursively finds and indexes all image files in the directory.
/// Returns statistics about the indexing operation.
#[cfg(feature = "clip")]
pub fn index_images_in_directory(
    db: &crate::storage::DB,
    clip_embedder: &mut crate::embedder_clip::ClipEmbedder,
    dir: &Path,
    progress: Option<&indicatif::ProgressBar>,
) -> Result<ImageIndexStats> {
    let mut stats = ImageIndexStats::default();

    // Collect all image files first for progress tracking
    let mut image_files = Vec::new();
    collect_image_files_recursive(dir, &mut image_files);

    stats.total_found = image_files.len();

    if let Some(pb) = progress {
        pb.set_length(image_files.len() as u64);
        pb.set_message("Indexing images");
    }

    for path in &image_files {
        match index_image(db, clip_embedder, path) {
            Ok(Some(_)) => {
                stats.indexed += 1;
            }
            Ok(None) => {
                stats.skipped += 1;
            }
            Err(e) => {
                tracing::warn!("Failed to index image {:?}: {}", path, e);
                stats.failed += 1;
            }
        }

        if let Some(pb) = progress {
            pb.inc(1);
        }
    }

    if let Some(pb) = progress {
        pb.finish_with_message("Done indexing images");
    }

    Ok(stats)
}

/// Recursively collect image files from a directory
#[cfg(feature = "clip")]
fn collect_image_files_recursive(dir: &Path, files: &mut Vec<std::path::PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();

        if path.is_dir() {
            // Skip common directories that shouldn't be indexed
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if should_skip_dir(name) {
                    continue;
                }
            }
            collect_image_files_recursive(&path, files);
        } else if path.is_file() && crate::file_types::is_image_file(&path) {
            files.push(path);
        }
    }
}

/// Statistics from image indexing
#[cfg(feature = "clip")]
#[derive(Debug, Clone, Default)]
pub struct ImageIndexStats {
    /// Total images found
    pub total_found: usize,
    /// Images successfully indexed
    pub indexed: usize,
    /// Images skipped (already indexed)
    pub skipped: usize,
    /// Images that failed to index
    pub failed: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_name_matches_filter_exact() {
        assert!(file_name_matches_filter("test.rs", "test.rs"));
        assert!(file_name_matches_filter(".gitignore", "gitignore"));
        assert!(file_name_matches_filter("Makefile", "makefile"));
    }

    #[test]
    fn test_file_name_matches_filter_prefix_with_separator() {
        assert!(file_name_matches_filter("test.rs", "test"));
        assert!(file_name_matches_filter("test-file.rs", "test"));
        assert!(file_name_matches_filter("test_file.rs", "test"));
        assert!(!file_name_matches_filter("testing.rs", "test"));
    }

    #[test]
    fn test_path_matches_filters_extension_case_insensitive() {
        let filters = vec!["rs".to_string()];
        assert!(path_matches_filters("main.rs", Some("RS"), &filters));
        assert!(!path_matches_filters("main.py", Some("py"), &filters));
    }

    #[test]
    fn test_matches_file_type_exclusion_takes_precedence() {
        let options = SearchOptions {
            file_types: vec!["rs".to_string()],
            exclude_file_types: vec!["rs".to_string()],
            ..SearchOptions::default()
        };

        assert!(!options.matches_file_type("src/main.rs"));
    }

    #[test]
    fn test_matches_file_type_accepts_all_when_includes_empty() {
        let options = SearchOptions {
            exclude_file_types: vec!["test.rs".to_string()],
            ..SearchOptions::default()
        };

        assert!(options.matches_file_type("src/main.rs"));
        assert!(!options.matches_file_type("src/foo.test.rs"));
    }

    #[test]
    fn test_should_index_file() {
        assert!(should_index_file(std::path::Path::new("foo.rs")));
        assert!(should_index_file(std::path::Path::new("foo.py")));
        assert!(should_index_file(std::path::Path::new("foo.js")));
        assert!(should_index_file(std::path::Path::new("Dockerfile")));
        assert!(should_index_file(std::path::Path::new("Dockerfile.dev")));
        assert!(should_index_file(std::path::Path::new("Makefile")));
        assert!(should_index_file(std::path::Path::new("LICENSE-MIT")));
        assert!(!should_index_file(std::path::Path::new("foo.exe")));
        assert!(!should_index_file(std::path::Path::new("foo.bin")));
    }

    #[test]
    fn test_should_skip_dir() {
        // Version control
        assert!(should_skip_dir(".git"));
        assert!(should_skip_dir(".GIT"));
        assert!(should_skip_dir(".svn"));
        assert!(should_skip_dir(".hg"));

        // IDEs (case-insensitive)
        assert!(should_skip_dir(".idea"));
        assert!(should_skip_dir(".IDEA"));
        assert!(should_skip_dir(".vscode"));
        assert!(should_skip_dir(".VSCODE"));
        assert!(should_skip_dir(".vs"));
        assert!(should_skip_dir(".VS"));

        // Package managers
        assert!(should_skip_dir("node_modules"));
        assert!(should_skip_dir("Node_Modules"));
        assert!(should_skip_dir("vendor"));
        assert!(should_skip_dir("bower_components"));

        // Build outputs
        assert!(should_skip_dir("target"));
        assert!(should_skip_dir("Target"));
        assert!(should_skip_dir("dist"));
        assert!(should_skip_dir("build"));
        assert!(should_skip_dir("out"));
        assert!(should_skip_dir("OUT"));
        assert!(should_skip_dir(".output"));

        // Build tool caches
        assert!(should_skip_dir(".gradle"));
        assert!(should_skip_dir(".parcel-cache"));
        assert!(should_skip_dir(".turbo"));

        // Python
        assert!(should_skip_dir("__pycache__"));
        assert!(should_skip_dir(".mypy_cache"));
        assert!(should_skip_dir(".pytest_cache"));
        assert!(should_skip_dir(".ruff_cache"));
        assert!(should_skip_dir("__pypackages__"));
        assert!(should_skip_dir(".venv"));
        assert!(should_skip_dir("venv"));

        // Framework-specific
        assert!(should_skip_dir(".next"));
        assert!(should_skip_dir(".nuxt"));
        assert!(should_skip_dir(".svelte-kit"));
        assert!(should_skip_dir(".angular"));

        // .NET / C#
        assert!(should_skip_dir("obj"));
        assert!(should_skip_dir("OBJ"));
        assert!(should_skip_dir("bin"));
        assert!(should_skip_dir("packages"));

        // Elixir / Erlang
        assert!(should_skip_dir("_build"));
        assert!(should_skip_dir("deps"));
        assert!(should_skip_dir(".elixir_ls"));

        // Ruby
        assert!(should_skip_dir(".bundle"));

        // Jupyter
        assert!(should_skip_dir(".ipynb_checkpoints"));

        // Documentation generators
        assert!(should_skip_dir("_site"));
        assert!(should_skip_dir(".docusaurus"));
        assert!(should_skip_dir(".vuepress"));

        // Infrastructure as code
        assert!(should_skip_dir(".terraform"));
        assert!(should_skip_dir(".vagrant"));

        // CMake
        assert!(should_skip_dir("CMakeFiles"));

        // Should NOT skip
        assert!(!should_skip_dir("src"));
        assert!(!should_skip_dir("lib"));
        assert!(!should_skip_dir("output")); // Not .output
    }

    #[test]
    fn test_matches_file_type() {
        // Empty filter matches everything
        let opts = SearchOptions::default();
        assert!(opts.matches_file_type("src/main.rs"));
        assert!(opts.matches_file_type("lib/utils.py"));

        // Single type filter
        let opts = SearchOptions {
            file_types: vec!["rs".to_string()],
            ..Default::default()
        };
        assert!(opts.matches_file_type("src/main.rs"));
        assert!(!opts.matches_file_type("lib/utils.py"));
        assert!(!opts.matches_file_type("README.md"));

        // Multiple type filters
        let opts = SearchOptions {
            file_types: vec!["rs".to_string(), "py".to_string()],
            ..Default::default()
        };
        assert!(opts.matches_file_type("src/main.rs"));
        assert!(opts.matches_file_type("lib/utils.py"));
        assert!(!opts.matches_file_type("README.md"));

        // Case insensitive
        let opts = SearchOptions {
            file_types: vec!["RS".to_string()],
            ..Default::default()
        };
        assert!(opts.matches_file_type("src/main.rs"));
        assert!(opts.matches_file_type("src/main.RS"));

        // Leading dot and whitespace should be ignored
        let opts = SearchOptions {
            file_types: vec![" .rs".to_string(), ".py ".to_string()],
            ..Default::default()
        };
        assert!(opts.matches_file_type("src/main.rs"));
        assert!(opts.matches_file_type("lib/utils.py"));

        // Filename filters
        let opts = SearchOptions {
            file_types: vec!["makefile".to_string()],
            ..Default::default()
        };
        assert!(opts.matches_file_type("Makefile"));
        assert!(opts.matches_file_type("Makefile.am"));
        assert!(!opts.matches_file_type("src/main.rs"));

        let opts = SearchOptions {
            file_types: vec!["readme".to_string()],
            ..Default::default()
        };
        assert!(opts.matches_file_type("README"));
        assert!(opts.matches_file_type("README.md"));
        assert!(!opts.matches_file_type("notes.md"));

        // Dotfile filters
        let opts = SearchOptions {
            file_types: vec!["gitignore".to_string()],
            ..Default::default()
        };
        assert!(opts.matches_file_type(".gitignore"));
        assert!(!opts.matches_file_type(".gitattributes"));

        let opts = SearchOptions {
            file_types: vec!["env".to_string()],
            ..Default::default()
        };
        assert!(opts.matches_file_type(".env"));
        assert!(opts.matches_file_type(".env.local"));

        let opts = SearchOptions {
            file_types: vec!["tar.gz".to_string()],
            ..Default::default()
        };
        assert!(opts.matches_file_type("archive.tar.gz"));
        assert!(opts.matches_file_type("tar.gz"));
        assert!(!opts.matches_file_type("archive.gz"));

        // Exclude file type filter
        let opts = SearchOptions {
            exclude_file_types: vec!["rs".to_string()],
            ..Default::default()
        };
        assert!(!opts.matches_file_type("src/main.rs"));
        assert!(opts.matches_file_type("lib/utils.py"));
        assert!(opts.matches_file_type("README.md"));

        // Exclude multiple types
        let opts = SearchOptions {
            exclude_file_types: vec!["rs".to_string(), "py".to_string()],
            ..Default::default()
        };
        assert!(!opts.matches_file_type("src/main.rs"));
        assert!(!opts.matches_file_type("lib/utils.py"));
        assert!(opts.matches_file_type("README.md"));

        // Include and exclude combined - include takes precedence
        let opts = SearchOptions {
            file_types: vec!["rs".to_string(), "py".to_string()],
            exclude_file_types: vec!["rs".to_string()],
            ..Default::default()
        };
        // rs is included but also excluded - exclude wins
        assert!(!opts.matches_file_type("src/main.rs"));
        // py is included and not excluded
        assert!(opts.matches_file_type("lib/utils.py"));
        // md is not included, so rejected
        assert!(!opts.matches_file_type("README.md"));

        // Exclude by filename pattern
        let opts = SearchOptions {
            exclude_file_types: vec!["makefile".to_string()],
            ..Default::default()
        };
        assert!(!opts.matches_file_type("Makefile"));
        assert!(!opts.matches_file_type("Makefile.am"));
        assert!(opts.matches_file_type("src/main.rs"));
    }

    #[test]
    fn test_extract_snippet() {
        let content = "line 1\nline 2\nline 3\nline 4\nline 5";
        assert_eq!(extract_snippet(content, 2), "line 1\nline 2");
        assert_eq!(extract_snippet(content, 3), "line 1\nline 2\nline 3");
    }

    #[test]
    fn test_is_system_temp_path() {
        use std::path::Path;

        // Should skip /tmp paths
        assert!(is_system_temp_path(Path::new("/tmp")));
        assert!(is_system_temp_path(Path::new("/tmp/some_file.txt")));
        assert!(is_system_temp_path(Path::new(
            "/tmp/git.dropbox.traces/file.json"
        )));

        // Should skip /private/tmp paths (macOS)
        assert!(is_system_temp_path(Path::new("/private/tmp")));
        assert!(is_system_temp_path(Path::new("/private/tmp/some_file.txt")));

        // Should skip /var/tmp paths
        assert!(is_system_temp_path(Path::new("/var/tmp")));
        assert!(is_system_temp_path(Path::new("/var/tmp/some_file.txt")));

        // Should skip /var/folders paths (macOS temp folders)
        assert!(is_system_temp_path(Path::new(
            "/var/folders/abc/def/T/file.txt"
        )));

        // Should NOT skip regular paths
        assert!(!is_system_temp_path(Path::new("/home/user/code/project")));
        assert!(!is_system_temp_path(Path::new(
            "/Users/user/project/main.rs"
        )));
        assert!(!is_system_temp_path(Path::new("/code/my_tmp_project"))); // "tmp" in name is fine
        assert!(!is_system_temp_path(Path::new("/tmp_project"))); // prefix only should not match
    }

    #[test]
    fn test_populate_lazy_index() {
        use std::path::Path;

        let db = DB::in_memory().unwrap();
        let doc_with_embeddings = db
            .add_document(Path::new("/project/src/lib.rs"), "fn main() {}")
            .unwrap();
        let _doc_without_embeddings = db
            .add_document(Path::new("/project/src/empty.rs"), "fn empty() {}")
            .unwrap();

        let num_tokens = 2;
        let mut embeddings = vec![0.0f32; num_tokens * crate::EMBEDDING_DIM];
        embeddings[0] = 1.0;
        embeddings[crate::EMBEDDING_DIM + 1] = 1.0;
        db.add_embeddings(doc_with_embeddings, &embeddings, num_tokens)
            .unwrap();

        let mut lazy_index = crate::index::LazyIndex::new(8);
        let count = populate_lazy_index(&db, &mut lazy_index).unwrap();
        assert_eq!(count, 1);
        assert_eq!(lazy_index.total_documents(), num_tokens);

        let mut query = vec![0.0f32; crate::EMBEDDING_DIM];
        query[0] = 1.0;
        let results = lazy_index.search(&query, 5).unwrap();
        assert_eq!(results[0].1, doc_with_embeddings);
    }

    #[test]
    fn test_populate_lazy_index_with_chunks() {
        use std::path::Path;

        let db = DB::in_memory().unwrap();
        let doc_id = db
            .add_document(Path::new("/project/src/lib.rs"), "fn main() {}")
            .unwrap();
        let chunk_id = db.add_chunk(doc_id, 0, 0, 0, "").unwrap();

        let num_tokens = 1;
        let mut embeddings = vec![0.0f32; num_tokens * crate::EMBEDDING_DIM];
        embeddings[0] = 1.0;
        db.add_chunk_embeddings(chunk_id, &embeddings, num_tokens)
            .unwrap();

        let mut lazy_index = crate::index::LazyIndex::new(8);
        let count = populate_lazy_index(&db, &mut lazy_index).unwrap();
        assert_eq!(count, 1);
        assert_eq!(lazy_index.total_documents(), num_tokens);

        let mut query = vec![0.0f32; crate::EMBEDDING_DIM];
        query[0] = 1.0;
        let results = lazy_index.search(&query, 5).unwrap();
        assert_eq!(results[0].1, doc_id);
    }

    #[test]
    fn test_load_or_create_bloom_filter_clears_invalid_state() {
        let db = DB::in_memory().unwrap();
        db.save_bloom_filter(&[0u8; 10]).unwrap();

        let filter = load_or_create_bloom_filter(&db).unwrap();
        assert!(filter.is_empty());
        assert!(db.load_bloom_filter().unwrap().is_none());
    }

    #[test]
    fn test_extract_snippet_around() {
        let content = "line 0\nline 1\nline 2\nline 3\nline 4\nline 5\nline 6";

        // Target middle line with context
        let snippet = extract_snippet_around(content, 3, 1);
        assert_eq!(snippet, "line 2\nline 3\nline 4");

        // Target first line (context clips to start)
        let snippet = extract_snippet_around(content, 0, 2);
        assert_eq!(snippet, "line 0\nline 1\nline 2");

        // Target last line (context clips to end)
        let snippet = extract_snippet_around(content, 6, 2);
        assert_eq!(snippet, "line 4\nline 5\nline 6");

        // Zero context
        let snippet = extract_snippet_around(content, 3, 0);
        assert_eq!(snippet, "line 3");
    }

    #[test]
    fn test_find_best_word_match() {
        let content =
            "fn main() {\n    let x = 1;\n    handle_error();\n    println!(\"hello\");\n}";

        // Build regex for "error"
        let word_regexes: Vec<regex::Regex> = vec![regex::RegexBuilder::new(r"error")
            .case_insensitive(true)
            .build()
            .unwrap()];

        let (line, snippet) = find_best_word_match(content, &word_regexes, 1);
        assert_eq!(line, 3); // "handle_error()" is on line 3
        assert!(snippet.contains("handle_error"));

        // Multiple words - should find line with most matches
        let content2 = "foo bar\nbaz qux\nfoo baz\nfoo bar baz";
        let multi_regexes: Vec<regex::Regex> = vec![
            regex::RegexBuilder::new(r"foo")
                .case_insensitive(true)
                .build()
                .unwrap(),
            regex::RegexBuilder::new(r"bar")
                .case_insensitive(true)
                .build()
                .unwrap(),
            regex::RegexBuilder::new(r"baz")
                .case_insensitive(true)
                .build()
                .unwrap(),
        ];

        let (line, _) = find_best_word_match(content2, &multi_regexes, 0);
        assert_eq!(line, 4); // "foo bar baz" has all 3 words

        // Empty regexes returns line 0
        let (line, _) = find_best_word_match(content, &[], 1);
        assert_eq!(line, 0);
    }

    #[test]
    fn test_keyword_search_basic() {
        let db = DB::in_memory().unwrap();

        // Add test documents
        db.add_document(
            std::path::Path::new("/project/src/auth.rs"),
            "fn authenticate() {\n    validate_credentials();\n    check_token();\n}",
        )
        .unwrap();
        db.add_document(
            std::path::Path::new("/project/src/main.rs"),
            "fn main() {\n    println!(\"hello\");\n    run_server();\n}",
        )
        .unwrap();
        db.add_document(
            std::path::Path::new("/project/src/token.rs"),
            "fn generate_token() {\n    // authentication helper\n    create_jwt();\n}",
        )
        .unwrap();

        // Search for "authenticate"
        let options = SearchOptions {
            top_k: 10,
            ..Default::default()
        };
        let results = keyword_search(&db, "authenticate", &options).unwrap();

        // Should find auth.rs and token.rs (which mentions "authentication")
        assert!(!results.is_empty());
        assert!(results
            .iter()
            .any(|r| r.path.to_string_lossy().contains("auth.rs")));

        // Search with file type filter
        let filtered_options = SearchOptions {
            top_k: 10,
            file_types: vec!["rs".to_string()],
            ..Default::default()
        };
        let results = keyword_search(&db, "fn", &filtered_options).unwrap();
        assert_eq!(results.len(), 3); // All 3 files have "fn"

        // Search with root filter
        let root_options = SearchOptions {
            top_k: 10,
            root: Some(PathBuf::from("/project/src")),
            ..Default::default()
        };
        let results = keyword_search(&db, "fn", &root_options).unwrap();
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_keyword_search_multi_word_coverage() {
        let db = DB::in_memory().unwrap();

        db.add_document(
            std::path::Path::new("/books/sherlock.txt"),
            "Baker Street\nThe detective arrived in London.\n",
        )
        .unwrap();
        db.add_document(
            std::path::Path::new("/books/pride.txt"),
            "A visit to London was discussed at length.\n",
        )
        .unwrap();

        let options = SearchOptions {
            top_k: 10,
            ..Default::default()
        };

        let results = keyword_search(&db, "detective Baker Street London", &options).unwrap();
        assert!(!results.is_empty());
        assert!(results[0].path.to_string_lossy().contains("sherlock.txt"));
    }

    #[test]
    fn test_rrf_score_calculation() {
        // Test the RRF formula: 1 / (k + rank + 1)
        // k = 60 in our implementation
        const RRF_K: f32 = 60.0;

        // First rank (0) should give highest score
        let score_rank_0 = 1.0 / (RRF_K + 0.0 + 1.0);
        let score_rank_1 = 1.0 / (RRF_K + 1.0 + 1.0);
        let score_rank_9 = 1.0 / (RRF_K + 9.0 + 1.0);

        // Verify rank order
        assert!(score_rank_0 > score_rank_1);
        assert!(score_rank_1 > score_rank_9);

        // Verify approximate values
        assert!((score_rank_0 - 0.0164).abs() < 0.001); // ~1/61
        assert!((score_rank_1 - 0.0161).abs() < 0.001); // ~1/62

        // Combined score from both methods should be higher than single method
        let combined = score_rank_0 + score_rank_0;
        assert!(combined > score_rank_0);
        assert!(combined < 2.0 * score_rank_0 + 0.001);
    }

    #[test]
    fn test_rrf_deterministic_ordering_with_tied_scores() {
        // Test that when RRF scores are equal, results are ordered by doc_id
        use std::collections::HashMap;

        const RRF_K: f32 = 60.0;
        let mut rrf_scores: HashMap<u32, f32> = HashMap::new();

        // Insert scores in random order - all with the same score
        let same_score = 1.0 / (RRF_K + 1.0);
        rrf_scores.insert(5, same_score);
        rrf_scores.insert(2, same_score);
        rrf_scores.insert(8, same_score);
        rrf_scores.insert(1, same_score);
        rrf_scores.insert(3, same_score);

        // Apply deterministic sorting (same logic as in hybrid search)
        let mut scored: Vec<_> = rrf_scores.into_iter().collect();
        scored.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(&b.0)) // Break ties by doc_id (ascending)
        });

        // Verify deterministic order: sorted by doc_id when scores are equal
        let doc_ids: Vec<u32> = scored.iter().map(|(id, _)| *id).collect();
        assert_eq!(
            doc_ids,
            vec![1, 2, 3, 5, 8],
            "Tied scores should be ordered by doc_id"
        );
    }

    #[test]
    fn test_adaptive_rrf_weights() {
        // Test that different query styles get different weights

        // Docstring queries should heavily favor semantic
        let (sem, kwd) = get_adaptive_rrf_weights("Returns the count of items");
        assert_eq!((sem, kwd), (0.9, 0.1), "Docstring should favor semantic");

        // Natural language queries should be balanced
        let (sem, kwd) = get_adaptive_rrf_weights("where is the error handler?");
        assert_eq!(
            (sem, kwd),
            (0.5, 0.5),
            "Natural language should be balanced"
        );

        // Code identifiers should favor semantic but less than docstrings
        let (sem, kwd) = get_adaptive_rrf_weights("getUserName");
        assert_eq!(
            (sem, kwd),
            (0.75, 0.25),
            "Code identifiers should favor semantic"
        );

        // Verify weights for CodeSearchNet-style queries (docstrings)
        let (sem, _kwd) = get_adaptive_rrf_weights("@param name the user name");
        assert_eq!(sem, 0.9, "Javadoc-style should use high semantic weight");

        let (sem, _kwd) = get_adaptive_rrf_weights("Gets the current configuration");
        assert_eq!(sem, 0.9, "Verb-start docstring should use high semantic weight");
    }

    #[test]
    fn test_file_name_matches_filter_variants() {
        assert!(file_name_matches_filter("Makefile", "makefile"));
        assert!(file_name_matches_filter("Makefile.am", "makefile"));
        assert!(file_name_matches_filter(".env.local", "env"));
        assert!(file_name_matches_filter("foo_bar.rs", "foo"));
        assert!(file_name_matches_filter("archive.tar.gz", "tar.gz"));
        assert!(!file_name_matches_filter("foobar.rs", "foo"));
        assert!(!file_name_matches_filter("foo", "foobar"));
    }

    #[test]
    fn test_path_matches_filters() {
        let filters = vec![" .rs".to_string(), "makefile".to_string()];
        assert!(path_matches_filters("main.rs", Some("rs"), &filters));
        assert!(path_matches_filters("Makefile", None, &filters));
        assert!(!path_matches_filters("main.py", Some("py"), &filters));
    }

    #[test]
    fn test_build_results_falls_back_to_word_match() {
        use std::path::Path;

        let db = DB::in_memory().unwrap();
        let doc_id = db
            .add_document(Path::new("/project/src/lib.rs"), "foo bar\nbar baz\nfoo")
            .unwrap();
        let scored = vec![(doc_id, 0.9)];
        let options = SearchOptions {
            context: 0,
            ..Default::default()
        };

        let results = build_results(&db, &scored, &options, "foo bar baz").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].line, 1);
        assert_eq!(results[0].snippet, "foo bar");
    }

    #[test]
    fn test_index_directory_options_default() {
        let opts = IndexDirectoryOptions::default();
        assert!(!opts.allow_temp_paths);
        assert_eq!(opts.crossfile_batch_size, DEFAULT_CROSSFILE_BATCH_SIZE);
    }

    #[test]
    fn test_search_result_default() {
        let result = SearchResult::default();
        assert_eq!(result.doc_id, 0);
        assert_eq!(result.score, 0.0);
        assert_eq!(result.path, PathBuf::new());
        assert_eq!(result.line, 0);
        assert_eq!(result.snippet, String::new());
    }

    #[test]
    fn test_search_result_debug_trait() {
        let result = SearchResult {
            doc_id: 7,
            score: 0.42,
            path: PathBuf::from("/project/src/main.rs"),
            line: 12,
            snippet: "fn main() {}".to_string(),
            header_context: "# Main".to_string(),
            language: Some("rust".to_string()),
            links: vec![],
            summary: None,
        };
        let debug_str = format!("{result:?}");
        assert!(debug_str.contains("SearchResult"));
        assert!(debug_str.contains("7"));
        assert!(debug_str.contains("0.42"));
        assert!(debug_str.contains("main.rs"));
        assert!(debug_str.contains("12"));
        assert!(debug_str.contains("fn main()"));
    }

    #[test]
    fn test_search_result_clone_trait() {
        let result = SearchResult {
            doc_id: 99,
            score: 0.99,
            path: PathBuf::from("/project/src/lib.rs"),
            line: 3,
            snippet: "pub fn lib() {}".to_string(),
            header_context: "".to_string(),
            language: None,
            links: vec![],
            summary: None,
        };
        let cloned = result.clone();
        assert_eq!(cloned.doc_id, result.doc_id);
        assert_eq!(cloned.score, result.score);
        assert_eq!(cloned.path, result.path);
        assert_eq!(cloned.line, result.line);
        assert_eq!(cloned.snippet, result.snippet);
        assert_eq!(cloned.language, result.language);
    }

    #[test]
    fn test_search_result_serde_round_trip() {
        let result = SearchResult {
            doc_id: 42,
            score: 0.75,
            path: PathBuf::from("/project/src/utils.rs"),
            line: 27,
            snippet: "fn helper() {}".to_string(),
            header_context: "# Utils".to_string(),
            language: Some("rust".to_string()),
            links: vec![],
            summary: None,
        };
        let json = serde_json::to_string(&result).unwrap();
        let parsed: SearchResult = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.doc_id, result.doc_id);
        assert_eq!(parsed.score, result.score);
        assert_eq!(parsed.path, result.path);
        assert_eq!(parsed.line, result.line);
        assert_eq!(parsed.snippet, result.snippet);
        assert_eq!(parsed.language, result.language);
    }

    #[test]
    fn test_find_best_match_finds_match() {
        let content = "fn main() {\n    handle_error();\n    println!(\"done\");\n}";
        let regex = regex::RegexBuilder::new(r"error")
            .case_insensitive(true)
            .build()
            .unwrap();

        let (line, snippet) = find_best_match(content, &regex, 1);
        assert_eq!(line, 2); // "handle_error();" is on line 2
        assert!(snippet.contains("handle_error"));
    }

    #[test]
    fn test_find_best_match_no_match() {
        let content = "fn main() {\n    println!(\"hello\");\n}";
        let regex = regex::RegexBuilder::new(r"nonexistent")
            .case_insensitive(true)
            .build()
            .unwrap();

        let (line, snippet) = find_best_match(content, &regex, 1);
        // No match, returns line 0 and extract_snippet fallback
        assert_eq!(line, 0);
        assert!(snippet.contains("fn main"));
    }

    #[test]
    fn test_find_best_match_first_line() {
        let content = "// error handler\nfn main() {\n    run();\n}";
        let regex = regex::RegexBuilder::new(r"error")
            .case_insensitive(true)
            .build()
            .unwrap();

        let (line, snippet) = find_best_match(content, &regex, 1);
        assert_eq!(line, 1); // Match on first line
        assert!(snippet.contains("error handler"));
    }

    #[test]
    fn test_find_best_match_last_line() {
        let content = "fn main() {\n    run();\n}\n// error at end";
        let regex = regex::RegexBuilder::new(r"error")
            .case_insensitive(true)
            .build()
            .unwrap();

        let (line, snippet) = find_best_match(content, &regex, 1);
        assert_eq!(line, 4); // Match on last line
        assert!(snippet.contains("error at end"));
    }

    #[test]
    fn test_index_directory_options_allow_temp() {
        let opts = IndexDirectoryOptions {
            allow_temp_paths: true,
            ..Default::default()
        };
        assert!(opts.allow_temp_paths);
    }

    #[test]
    fn test_collect_files_sorted() {
        use std::path::Path;

        let temp = tempfile::TempDir::new().unwrap();
        let root = temp.path();
        std::fs::write(root.join("z.rs"), "fn z() {}").unwrap();
        std::fs::write(root.join("a.rs"), "fn a() {}").unwrap();
        std::fs::create_dir(root.join("sub")).unwrap();
        std::fs::write(root.join("sub").join("b.rs"), "fn b() {}").unwrap();

        let options = IndexDirectoryOptions {
            allow_temp_paths: true,
            ..Default::default()
        };
        let files = collect_files(root, &options).unwrap();
        let relative: Vec<String> = files
            .iter()
            .map(|path| {
                path.strip_prefix(root)
                    .unwrap_or(Path::new(path))
                    .to_string_lossy()
                    .to_string()
            })
            .collect();

        assert_eq!(relative, vec!["a.rs", "sub/b.rs", "z.rs"]);
    }

    #[test]
    fn test_search_options_default_values() {
        let opts = SearchOptions::default();
        assert_eq!(opts.top_k, 10, "default top_k should be 10");
        assert_eq!(opts.threshold, 0.0, "default threshold should be 0.0");
        assert!(!opts.hybrid, "default hybrid should be false");
        assert!(opts.root.is_none(), "default root should be None");
        assert_eq!(opts.context, 2, "default context should be 2");
        assert!(
            opts.file_types.is_empty(),
            "default file_types should be empty"
        );
        assert!(
            opts.exclude_file_types.is_empty(),
            "default exclude_file_types should be empty"
        );
    }

    #[test]
    fn test_search_options_serde_round_trip() {
        let opts = SearchOptions {
            top_k: 20,
            threshold: 0.5,
            hybrid: true,
            auto_hybrid: true,
            root: Some(PathBuf::from("/test/path")),
            context: 5,
            file_types: vec!["rs".to_string(), "py".to_string()],
            exclude_file_types: vec!["test.rs".to_string()],
            preprocess_query: true,
        };

        let json = serde_json::to_string(&opts).unwrap();
        let parsed: SearchOptions = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.top_k, 20);
        assert_eq!(parsed.threshold, 0.5);
        assert!(parsed.hybrid);
        assert_eq!(parsed.root, Some(PathBuf::from("/test/path")));
        assert_eq!(parsed.context, 5);
        assert_eq!(parsed.file_types, vec!["rs", "py"]);
        assert_eq!(parsed.exclude_file_types, vec!["test.rs"]);
    }

    #[test]
    fn test_search_options_serde_defaults_for_missing_fields() {
        // Test that serde defaults work for optional fields
        let json = r#"{"top_k": 5, "threshold": 0.1, "hybrid": false, "root": null}"#;
        let parsed: SearchOptions = serde_json::from_str(json).unwrap();

        assert_eq!(parsed.top_k, 5);
        assert_eq!(parsed.threshold, 0.1);
        assert!(!parsed.hybrid);
        assert!(parsed.root.is_none());
        // These should use serde defaults
        assert_eq!(parsed.context, 2, "context should default to 2");
        assert!(
            parsed.file_types.is_empty(),
            "file_types should default to empty"
        );
        assert!(
            parsed.exclude_file_types.is_empty(),
            "exclude_file_types should default to empty"
        );
    }

    #[test]
    fn test_search_options_serde_with_empty_arrays() {
        let json = r#"{
            "top_k": 10,
            "threshold": 0.0,
            "hybrid": true,
            "root": "/path",
            "context": 3,
            "file_types": [],
            "exclude_file_types": []
        }"#;
        let parsed: SearchOptions = serde_json::from_str(json).unwrap();

        assert!(parsed.file_types.is_empty());
        assert!(parsed.exclude_file_types.is_empty());
        assert_eq!(parsed.root, Some(PathBuf::from("/path")));
    }

    #[test]
    fn test_search_options_top_k_boundary_values() {
        // top_k = 0 is valid (returns no results)
        let opts_zero = SearchOptions {
            top_k: 0,
            ..Default::default()
        };
        assert_eq!(opts_zero.top_k, 0);

        // top_k = 1 is valid (returns single result)
        let opts_one = SearchOptions {
            top_k: 1,
            ..Default::default()
        };
        assert_eq!(opts_one.top_k, 1);

        // Large top_k is valid
        let opts_large = SearchOptions {
            top_k: 10000,
            ..Default::default()
        };
        assert_eq!(opts_large.top_k, 10000);
    }

    #[test]
    fn test_search_options_threshold_boundary_values() {
        // threshold = 0.0 means accept all results
        let opts_zero = SearchOptions {
            threshold: 0.0,
            ..Default::default()
        };
        assert!((opts_zero.threshold - 0.0).abs() < f32::EPSILON);

        // threshold = 1.0 is strict (only perfect matches)
        let opts_one = SearchOptions {
            threshold: 1.0,
            ..Default::default()
        };
        assert!((opts_one.threshold - 1.0).abs() < f32::EPSILON);

        // Negative threshold is technically valid (accepts all)
        let opts_neg = SearchOptions {
            threshold: -0.5,
            ..Default::default()
        };
        assert!((opts_neg.threshold - (-0.5)).abs() < f32::EPSILON);

        // Threshold > 1.0 would reject all results
        let opts_high = SearchOptions {
            threshold: 2.0,
            ..Default::default()
        };
        assert!((opts_high.threshold - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_search_options_context_boundary_values() {
        // context = 0 means no context lines
        let opts_zero = SearchOptions {
            context: 0,
            ..Default::default()
        };
        assert_eq!(opts_zero.context, 0);

        // Large context shows many surrounding lines
        let opts_large = SearchOptions {
            context: 100,
            ..Default::default()
        };
        assert_eq!(opts_large.context, 100);
    }

    #[test]
    fn test_search_options_matches_file_type_case_insensitive() {
        let opts = SearchOptions {
            file_types: vec!["rs".to_string(), "PY".to_string()],
            ..Default::default()
        };

        // Should match regardless of case
        assert!(opts.matches_file_type("main.rs"));
        assert!(opts.matches_file_type("main.RS"));
        assert!(opts.matches_file_type("script.py"));
        assert!(opts.matches_file_type("script.PY"));
        assert!(!opts.matches_file_type("main.js"));
    }

    #[test]
    fn test_search_options_file_type_with_dots() {
        // File types with leading dots are trimmed and still match
        let opts = SearchOptions {
            file_types: vec![".rs".to_string()],
            ..Default::default()
        };

        // Leading dot is trimmed, so ".rs" matches ".rs" extension
        assert!(opts.matches_file_type("main.rs"));
        assert!(!opts.matches_file_type("main.py"));
    }

    #[test]
    fn test_search_options_exclude_takes_precedence() {
        let opts = SearchOptions {
            file_types: vec!["rs".to_string()],
            exclude_file_types: vec!["test.rs".to_string()],
            ..Default::default()
        };

        // Regular .rs file should match
        assert!(opts.matches_file_type("main.rs"));

        // test.rs should be excluded (exact match)
        assert!(!opts.matches_file_type("test.rs"));

        // Files ending in .test.rs are also excluded
        assert!(!opts.matches_file_type("my.test.rs"));

        // my_test.rs doesn't match "test.rs" pattern (no dot before test)
        assert!(opts.matches_file_type("my_test.rs"));
    }

    #[test]
    fn test_index_directory_options_default_disallows_temp() {
        let opts = IndexDirectoryOptions::default();
        assert!(!opts.allow_temp_paths);
    }

    #[test]
    fn test_index_stats_default_is_empty() {
        let stats = IndexStats::default();
        assert_eq!(stats.indexed_files, 0);
        assert_eq!(stats.skipped_files, 0);
        assert_eq!(stats.failed_files, 0);
        assert_eq!(stats.total_lines, 0);
    }

    #[test]
    fn test_search_result_score_boundary_values() {
        // Score of 0.0 indicates no similarity
        let result_zero = SearchResult {
            score: 0.0,
            ..Default::default()
        };
        assert!((result_zero.score - 0.0).abs() < f32::EPSILON);

        // Score of 1.0 indicates perfect match
        let result_one = SearchResult {
            score: 1.0,
            ..Default::default()
        };
        assert!((result_one.score - 1.0).abs() < f32::EPSILON);

        // Negative scores can occur with cosine similarity
        let result_neg = SearchResult {
            score: -0.5,
            ..Default::default()
        };
        assert!((result_neg.score - (-0.5)).abs() < f32::EPSILON);
    }

    #[test]
    fn test_index_stats_debug_trait() {
        let stats = IndexStats {
            total_files: 100,
            indexed_files: 80,
            skipped_files: 15,
            failed_files: 5,
            total_lines: 5000,
        };
        let debug_str = format!("{stats:?}");
        assert!(debug_str.contains("IndexStats"));
        assert!(debug_str.contains("100"));
        assert!(debug_str.contains("80"));
        assert!(debug_str.contains("15"));
        assert!(debug_str.contains("5"));
        assert!(debug_str.contains("5000"));
    }

    #[test]
    fn test_index_stats_clone_trait() {
        let stats = IndexStats {
            total_files: 50,
            indexed_files: 40,
            skipped_files: 8,
            failed_files: 2,
            total_lines: 2500,
        };
        let cloned = stats.clone();

        assert_eq!(cloned.total_files, 50);
        assert_eq!(cloned.indexed_files, 40);
        assert_eq!(cloned.skipped_files, 8);
        assert_eq!(cloned.failed_files, 2);
        assert_eq!(cloned.total_lines, 2500);
    }

    #[test]
    fn test_index_stats_clone_is_independent() {
        let stats = IndexStats {
            total_files: 10,
            indexed_files: 8,
            skipped_files: 1,
            failed_files: 1,
            total_lines: 500,
        };
        let mut cloned = stats.clone();
        cloned.indexed_files = 100;

        // Original should be unchanged
        assert_eq!(stats.indexed_files, 8);
        assert_eq!(cloned.indexed_files, 100);
    }

    #[test]
    fn test_index_directory_options_debug_trait() {
        let opts = IndexDirectoryOptions {
            allow_temp_paths: true,
            ..Default::default()
        };
        let debug_str = format!("{opts:?}");
        assert!(debug_str.contains("IndexDirectoryOptions"));
        assert!(debug_str.contains("allow_temp_paths"));
        assert!(debug_str.contains("true"));
    }

    #[test]
    fn test_index_directory_options_clone_trait() {
        let opts = IndexDirectoryOptions {
            allow_temp_paths: true,
            ..Default::default()
        };
        let cloned = opts.clone();
        assert!(cloned.allow_temp_paths);

        let opts_false = IndexDirectoryOptions {
            allow_temp_paths: false,
            ..Default::default()
        };
        let cloned_false = opts_false.clone();
        assert!(!cloned_false.allow_temp_paths);
    }

    #[test]
    fn test_index_directory_options_clone_is_independent() {
        let opts = IndexDirectoryOptions {
            allow_temp_paths: false,
            ..Default::default()
        };
        let mut cloned = opts.clone();
        cloned.allow_temp_paths = true;

        // Original should be unchanged
        assert!(!opts.allow_temp_paths);
        assert!(cloned.allow_temp_paths);
    }

    #[test]
    fn test_index_directory_options_parallel_file_reading_default() {
        let opts = IndexDirectoryOptions::default();
        // Parallel file reading is off by default for compatibility
        assert!(!opts.parallel_file_reading);
    }

    #[test]
    fn test_index_directory_options_parallel_file_reading_enabled() {
        let opts = IndexDirectoryOptions {
            parallel_file_reading: true,
            pipelined: true,
            ..Default::default()
        };
        assert!(opts.parallel_file_reading);
        assert!(opts.pipelined);
    }

    #[test]
    fn test_incremental_update_stats_default() {
        let stats = IncrementalUpdateStats::default();
        assert_eq!(stats.unchanged_chunks, 0);
        assert_eq!(stats.new_chunks, 0);
        assert_eq!(stats.deleted_chunks, 0);
    }

    #[test]
    fn test_incremental_update_stats_debug_clone() {
        let stats = IncrementalUpdateStats {
            unchanged_chunks: 5,
            new_chunks: 2,
            deleted_chunks: 1,
        };

        // Test Debug
        let debug_str = format!("{stats:?}");
        assert!(debug_str.contains("IncrementalUpdateStats"));
        assert!(debug_str.contains("5"));
        assert!(debug_str.contains("2"));
        assert!(debug_str.contains("1"));

        // Test Clone
        let cloned = stats.clone();
        assert_eq!(cloned.unchanged_chunks, 5);
        assert_eq!(cloned.new_chunks, 2);
        assert_eq!(cloned.deleted_chunks, 1);
    }

    #[test]
    fn test_optimal_batch_size_candle() {
        let size = optimal_batch_size(&crate::EmbedderBackendKind::Candle);
        // Should return a reasonable batch size
        assert!(size >= 8);
        assert!(size <= 128);
        // On macOS it should be 64 (Metal), on other platforms 16 (CPU)
        #[cfg(target_os = "macos")]
        assert_eq!(size, 64);
        #[cfg(not(target_os = "macos"))]
        assert_eq!(size, 16);
    }

    #[cfg(feature = "onnx")]
    #[test]
    fn test_optimal_batch_size_onnx() {
        let size = optimal_batch_size(&crate::EmbedderBackendKind::Onnx);
        assert_eq!(size, 32); // Conservative for cross-platform
    }

    #[cfg(feature = "coreml")]
    #[test]
    fn test_optimal_batch_size_coreml() {
        let size = optimal_batch_size(&crate::EmbedderBackendKind::CoreMl);
        assert_eq!(size, 64); // Apple Neural Engine
    }

    #[cfg(feature = "cuda")]
    #[test]
    fn test_optimal_batch_size_cuda() {
        let size = optimal_batch_size(&crate::EmbedderBackendKind::Cuda);
        assert_eq!(size, 128); // High VRAM GPUs
    }

    #[test]
    fn test_optimal_batch_size_always_positive() {
        // Ensure all backend kinds return positive batch sizes
        let size = optimal_batch_size(&crate::EmbedderBackendKind::Candle);
        assert!(size >= 1, "batch size must be at least 1");
    }
}
