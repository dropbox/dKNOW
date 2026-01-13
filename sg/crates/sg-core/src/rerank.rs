//! Reranking module for improving search result precision
//!
//! Provides two-stage retrieval: fast semantic search (Stage 1) followed by
//! optional reranking (Stage 2) using LLM or cross-encoder scoring.
//!
//! Architecture:
//! ```text
//! Query → [Semantic Search] → Top 50 → [Reranker] → Top 10
//! ```

use crate::search::SearchResult;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tracing::{debug, info};

/// Configuration for reranking behavior
#[derive(Debug, Clone)]
pub struct RerankOptions {
    /// Number of candidates to consider from Stage 1
    pub candidates: usize,
    /// Number of final results after reranking
    pub top_k: usize,
    /// Timeout for reranking operation
    pub timeout: Duration,
}

impl Default for RerankOptions {
    fn default() -> Self {
        Self {
            candidates: 50,
            top_k: 10,
            timeout: Duration::from_secs(10),
        }
    }
}

/// Trait for reranking search results
pub trait Reranker: Send + Sync {
    /// Rerank search results based on query relevance
    fn rerank(
        &self,
        query: &str,
        candidates: Vec<SearchResult>,
        options: &RerankOptions,
    ) -> Result<Vec<SearchResult>>;

    /// Get the name of this reranker for logging
    fn name(&self) -> &str;
}

/// LLM-based reranker using Claude Haiku for fast, accurate reranking
pub struct LLMReranker {
    api_key: String,
    model: String,
    base_url: String,
}

impl LLMReranker {
    /// Create a new LLM reranker with the given API key
    ///
    /// Uses Claude 3 Haiku by default for optimal speed/quality tradeoff.
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            model: "claude-3-haiku-20240307".to_string(),
            base_url: "https://api.anthropic.com/v1".to_string(),
        }
    }

    /// Use a different model (e.g., claude-3-5-sonnet for higher quality)
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Use a custom base URL (for proxies or alternative endpoints)
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Create from environment variable ANTHROPIC_API_KEY
    pub fn from_env() -> Result<Self> {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .context("ANTHROPIC_API_KEY environment variable not set")?;
        Ok(Self::new(api_key))
    }

    #[allow(clippy::unused_self)]
    fn build_prompt(&self, query: &str, candidates: &[SearchResult]) -> String {
        let mut prompt = format!(
            r"You are a code search relevance expert. Rank the following code snippets by their relevance to the query.

Query: {query}

Code snippets to rank (numbered 1-{count}):

",
            count = candidates.len()
        );

        for (i, result) in candidates.iter().enumerate() {
            let path = result.path.display();
            let snippet = truncate_snippet(&result.snippet, 500);
            prompt.push_str(&format!(
                "--- Snippet {} ({}:{}) ---\n{}\n\n",
                i + 1,
                path,
                result.line,
                snippet
            ));
        }

        prompt.push_str(
            r"Return ONLY a JSON array of snippet numbers in order of relevance (most relevant first).
Example: [3, 1, 5, 2, 4]

Rankings:",
        );

        prompt
    }

    #[allow(clippy::unused_self)]
    fn parse_rankings(&self, response: &str, count: usize) -> Result<Vec<usize>> {
        // Find JSON array in response
        let start = response
            .find('[')
            .context("No JSON array found in response")?;
        let end = response
            .rfind(']')
            .context("No closing bracket found in response")?;
        let json_str = &response[start..=end];

        let rankings: Vec<usize> =
            serde_json::from_str(json_str).context("Failed to parse rankings JSON")?;

        let zero_based = rankings.contains(&0);
        let valid_rankings: Vec<usize> = if zero_based {
            rankings
                .into_iter()
                .filter(|&r| r < count)
                .collect()
        } else {
            rankings
                .into_iter()
                .filter(|&r| r >= 1 && r <= count)
                .map(|r| r - 1) // Convert to 0-indexed
                .collect()
        };

        if valid_rankings.is_empty() {
            anyhow::bail!("No valid rankings found");
        }

        Ok(valid_rankings)
    }
}
fn complete_rankings(rankings: Vec<usize>, total: usize) -> Vec<usize> {
    let mut ordered = Vec::with_capacity(total);
    let mut seen = std::collections::HashSet::new();

    for idx in rankings {
        if idx < total && seen.insert(idx) {
            ordered.push(idx);
        }
    }

    for idx in 0..total {
        if seen.insert(idx) {
            ordered.push(idx);
        }
    }

    ordered
}


#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<Message>,
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<ContentBlock>,
}

#[derive(Deserialize)]
struct ContentBlock {
    text: String,
}

impl Reranker for LLMReranker {
    fn rerank(
        &self,
        query: &str,
        candidates: Vec<SearchResult>,
        options: &RerankOptions,
    ) -> Result<Vec<SearchResult>> {
        if candidates.is_empty() {
            return Ok(vec![]);
        }

        let start = Instant::now();

        // Limit candidates to avoid huge prompts
        let candidates: Vec<SearchResult> = candidates
            .into_iter()
            .take(options.candidates.min(30)) // Max 30 for LLM context
            .collect();

        let prompt = self.build_prompt(query, &candidates);
        debug!("Rerank prompt length: {} chars", prompt.len());

        let request = AnthropicRequest {
            model: self.model.clone(),
            max_tokens: 256,
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt,
            }],
        };

        let response = ureq::post(&format!("{}/messages", self.base_url))
            .set("x-api-key", &self.api_key)
            .set("anthropic-version", "2023-06-01")
            .set("content-type", "application/json")
            .timeout(options.timeout)
            .send_json(&request)
            .context("Failed to call Anthropic API")?;

        let response: AnthropicResponse = response
            .into_json()
            .context("Failed to parse Anthropic response")?;

        let text = response
            .content
            .first()
            .map(|c| c.text.as_str())
            .unwrap_or("");

        debug!("LLM response: {}", text);

        let rankings = self.parse_rankings(text, candidates.len())?;
        let rankings = complete_rankings(rankings, candidates.len());
        let elapsed = start.elapsed();

        info!(
            "LLM reranking took {:?}, ranked {} candidates",
            elapsed,
            rankings.len()
        );

        // Reorder candidates according to LLM rankings
        let mut reranked: Vec<SearchResult> = rankings
            .into_iter()
            .filter_map(|idx| candidates.get(idx).cloned())
            .take(options.top_k)
            .collect();

        // Update scores to reflect new ranking (1.0 = best, decreasing)
        for (i, result) in reranked.iter_mut().enumerate() {
            result.score = 1.0 - (i as f32 * 0.05);
        }

        Ok(reranked)
    }

    fn name(&self) -> &str {
        "llm"
    }
}

/// Simple pass-through "reranker" that just truncates results (no actual reranking)
pub struct NoOpReranker;

impl Reranker for NoOpReranker {
    fn rerank(
        &self,
        _query: &str,
        candidates: Vec<SearchResult>,
        options: &RerankOptions,
    ) -> Result<Vec<SearchResult>> {
        Ok(candidates.into_iter().take(options.top_k).collect())
    }

    fn name(&self) -> &str {
        "noop"
    }
}

/// Rerank using search result scores with optional boosting
pub struct ScoreBoostReranker {
    /// Boost factor for filename matches (default: 1.3)
    pub filename_boost: f32,
    /// Boost factor for shorter code (default: 1.1)
    pub brevity_boost: f32,
}

impl Default for ScoreBoostReranker {
    fn default() -> Self {
        Self {
            filename_boost: 1.3,
            brevity_boost: 1.1,
        }
    }
}

impl Reranker for ScoreBoostReranker {
    fn rerank(
        &self,
        query: &str,
        mut candidates: Vec<SearchResult>,
        options: &RerankOptions,
    ) -> Result<Vec<SearchResult>> {
        let query_lower = query.to_lowercase();
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();

        for result in &mut candidates {
            let filename = result
                .path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();

            // Boost if filename contains query words
            let filename_matches = query_words
                .iter()
                .any(|word| filename.contains(word) || word.contains(&filename));

            if filename_matches {
                result.score *= self.filename_boost;
            }

            // Slight boost for shorter, more focused code
            let lines = result.snippet.lines().count();
            if lines < 20 {
                result.score *= self.brevity_boost;
            }
        }

        // Sort by boosted score
        candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        Ok(candidates.into_iter().take(options.top_k).collect())
    }

    fn name(&self) -> &str {
        "score-boost"
    }
}

/// Truncate snippet to max characters, preserving whole lines
fn truncate_snippet(snippet: &str, max_chars: usize) -> String {
    if snippet.len() <= max_chars {
        return snippet.to_string();
    }

    let mut result = String::with_capacity(max_chars);
    for line in snippet.lines() {
        if result.len() + line.len() + 1 > max_chars {
            break;
        }
        if !result.is_empty() {
            result.push('\n');
        }
        result.push_str(line);
    }

    if result.is_empty() {
        // Single very long line - just truncate
        snippet.chars().take(max_chars).collect()
    } else {
        result.push_str("\n...");
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_result(path: &str, score: f32, snippet: &str) -> SearchResult {
        SearchResult {
            doc_id: 0,
            score,
            path: PathBuf::from(path),
            line: 1,
            snippet: snippet.to_string(),
            header_context: String::new(),
            language: None,
            links: vec![],
            summary: None,
        }
    }

    #[test]
    fn test_noop_reranker() {
        let reranker = NoOpReranker;
        let candidates = vec![
            make_result("a.rs", 0.9, "fn a()"),
            make_result("b.rs", 0.8, "fn b()"),
            make_result("c.rs", 0.7, "fn c()"),
        ];

        let options = RerankOptions {
            top_k: 2,
            ..Default::default()
        };

        let results = reranker.rerank("test", candidates, &options).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].path.to_str().unwrap(), "a.rs");
    }

    #[test]
    fn test_score_boost_filename_match() {
        let reranker = ScoreBoostReranker::default();
        let candidates = vec![
            make_result("other.rs", 0.9, "fn other()"),
            make_result("auth.rs", 0.8, "fn login()"),
        ];

        let options = RerankOptions::default();

        let results = reranker.rerank("auth", candidates, &options).unwrap();
        // auth.rs should be boosted to top despite lower original score
        assert_eq!(results[0].path.to_str().unwrap(), "auth.rs");
    }

    #[test]
    fn test_truncate_snippet() {
        let snippet = "line1\nline2\nline3\nline4";
        let truncated = truncate_snippet(snippet, 15);
        assert!(truncated.len() <= 20); // Allows for "..."
        assert!(truncated.starts_with("line1"));
    }

    #[test]
    fn test_llm_parse_rankings() {
        let reranker = LLMReranker::new("test".to_string());

        let response = "Based on relevance, I rank them: [3, 1, 2]";
        let rankings = reranker.parse_rankings(response, 3).unwrap();
        assert_eq!(rankings, vec![2, 0, 1]); // 0-indexed

        let response = "[5, 2, 1, 3, 4]";
        let rankings = reranker.parse_rankings(response, 5).unwrap();
        assert_eq!(rankings, vec![4, 1, 0, 2, 3]);
    }

    #[test]
    fn test_llm_parse_rankings_zero_based() {
        let reranker = LLMReranker::new("test".to_string());

        let response = "[0, 2, 1]";
        let rankings = reranker.parse_rankings(response, 3).unwrap();
        assert_eq!(rankings, vec![0, 2, 1]);
    }

    #[test]
    fn test_complete_rankings_adds_missing() {
        let rankings = complete_rankings(vec![2, 2, 0], 4);
        assert_eq!(rankings, vec![2, 0, 1, 3]);
    }
}
