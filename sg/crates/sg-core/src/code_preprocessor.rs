//! Code-specific preprocessing for improved tokenization
//!
//! Transforms code identifiers and syntax into forms more suitable for
//! the T5 tokenizer, which was trained primarily on natural language text.
//!
//! Key transformations:
//! - Split camelCase → "camel case"
//! - Split snake_case → "snake case"
//! - Split PascalCase → "pascal case"
//! - Normalize common operators and symbols
//! - Preserve natural language content (comments, strings) as-is
//!
//! Also provides query style detection for adaptive search routing:
//! - Docstring-style queries (contain @param, @return, etc.) → semantic-only
//! - Natural language queries → hybrid (semantic + keyword)

use std::path::Path;

/// Query style classification for adaptive search routing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryStyle {
    /// Docstring/comment style - contains annotations like @param, @return, {@link}
    /// Best handled by semantic-only search (exact embedding match)
    Docstring,
    /// Natural language description - conversational queries
    /// Best handled by hybrid search (semantic + keyword RRF)
    NaturalLanguage,
    /// Code identifier style - camelCase, snake_case, etc.
    /// Best handled by semantic search with preprocessing
    CodeIdentifier,
}

/// Detect the style of a query for adaptive search routing
///
/// Returns the detected QueryStyle which can be used to choose between
/// semantic-only and hybrid search modes.
///
/// # Examples
/// ```
/// use sg_core::code_preprocessor::{detect_query_style, QueryStyle};
///
/// // Docstring queries - semantic-only works best
/// assert_eq!(detect_query_style("@param name the user name"), QueryStyle::Docstring);
/// assert_eq!(detect_query_style("Returns the count of items"), QueryStyle::Docstring);
///
/// // Code identifiers - semantic with preprocessing
/// assert_eq!(detect_query_style("getUserName"), QueryStyle::CodeIdentifier);
///
/// // Natural language - hybrid works best
/// assert_eq!(detect_query_style("function that handles auth"), QueryStyle::NaturalLanguage);
/// ```
pub fn detect_query_style(query: &str) -> QueryStyle {
    // Check for docstring patterns first (most specific)
    if looks_like_docstring(query) {
        return QueryStyle::Docstring;
    }

    // Check for natural language patterns (question words, etc.)
    // This takes priority over code identifiers because queries like
    // "what function parses JSON?" should be natural language even though
    // they contain acronyms
    if looks_like_natural_language_query(query) {
        return QueryStyle::NaturalLanguage;
    }

    // Check for code identifiers (standalone like "getUserName")
    if looks_like_code_query(query) {
        return QueryStyle::CodeIdentifier;
    }

    // Default to natural language
    QueryStyle::NaturalLanguage
}

/// Check if a query looks like a natural language question/description
///
/// Detects patterns like:
/// - Question words: "how", "where", "what", "which", "why", "when"
/// - Question marks
/// - Common search phrases: "find", "search", "look for"
///
/// Note: This function is intentionally conservative to avoid misclassifying
/// terse docstrings as natural language. For CodeSearchNet-style evaluation
/// where queries are extracted from docstrings, semantic-only search performs
/// better than hybrid.
fn looks_like_natural_language_query(query: &str) -> bool {
    let query_lower = query.to_lowercase();
    let first_word = query_lower.split_whitespace().next().unwrap_or("");

    // Question words at start (strong signal)
    let question_starters = ["how", "where", "what", "which", "why", "when", "who"];
    if question_starters.contains(&first_word) {
        return true;
    }

    // Contains question mark (strong signal)
    if query.contains('?') {
        return true;
    }

    // Request/imperative phrases (strong signal for user queries vs docstrings)
    // Note: We don't include "find" alone because "Finds the..." is a docstring pattern
    let request_phrases = [
        "search for ", "look for ", "looking for ",
        "show me ", "give me ", "i need ", "i want ",
        "help me ", "need to ", "want to ",
        "function for ", "code for ", "implementation of ",
        "file that ", "code that ", "function that ",
        "can you ", "could you ", "would you ", "please ",
    ];
    if request_phrases.iter().any(|p| query_lower.contains(p)) {
        return true;
    }

    // Modal verbs at start indicate questions/requests (can, could, should, would)
    let modal_starters = ["can", "could", "would", "should", "is", "are", "does", "do"];
    if modal_starters.contains(&first_word) {
        return true;
    }

    false
}

/// Check if a query looks like a docstring or documentation comment
///
/// Detects patterns common in documentation:
/// - Javadoc: @param, @return, @throws, @see, {@link}, {@code}
/// - Python docstrings: :param, :return:, :raises:, Args:, Returns:
/// - JSDoc/TSDoc: similar to Javadoc
/// - Rust doc comments: # Examples, # Arguments, # Returns
/// - Go comments: // FunctionName does something
fn looks_like_docstring(query: &str) -> bool {
    let query_lower = query.to_lowercase();

    // Javadoc/JSDoc style annotations
    if query_lower.contains("@param")
        || query_lower.contains("@return")
        || query_lower.contains("@throws")
        || query_lower.contains("@see")
        || query_lower.contains("@author")
        || query_lower.contains("@since")
        || query_lower.contains("@deprecated")
        || query_lower.contains("@override")
        || query_lower.contains("{@link")
        || query_lower.contains("{@code")
    {
        return true;
    }

    // Python docstring patterns
    if query_lower.contains(":param ")
        || query_lower.contains(":return:")
        || query_lower.contains(":returns:")
        || query_lower.contains(":raises:")
        || query_lower.contains(":type ")
        || query_lower.contains(":rtype:")
        || query_lower.contains("args:")
        || query_lower.contains("returns:")
        || query_lower.contains("raises:")
        || query_lower.contains("yields:")
        || query_lower.contains("attributes:")
    {
        return true;
    }

    // Strip leading comment markers for verb detection
    // Go: // Comment, C: /* Comment, Python: # Comment
    let stripped = query
        .trim_start_matches("//")
        .trim_start_matches("/*")
        .trim_start_matches('#')
        .trim();
    // Handle bullet prefixes often used in doc comment lists.
    let stripped = stripped
        .strip_prefix("* ")
        .or_else(|| stripped.strip_prefix("- "))
        .or_else(|| stripped.strip_prefix("+ "))
        .unwrap_or(stripped);
    let stripped_lower = stripped.to_lowercase();
    let first_word = stripped_lower.split_whitespace().next().unwrap_or("");

    // Common docstring sentence starters (method descriptions)
    // Include both third-person singular ("returns") and imperative ("return") forms
    let doc_starters = [
        // Third-person singular (most common)
        "returns", "gets", "sets", "creates", "initializes",
        "computes", "calculates", "parses", "converts", "validates",
        "checks", "determines", "retrieves", "fetches", "loads",
        "saves", "stores", "deletes", "removes", "adds",
        "inserts", "updates", "processes", "handles", "executes",
        "closes", "opens", "provides", "ensures", "allows",
        "finds", "verifies", "registers", "formats", "runs",
        "estimates", "extracts", "builds", "sends", "receives",
        "reads", "writes", "copies", "moves", "calls",
        "invokes", "starts", "stops", "enables", "disables",
        "configures", "applies", "generates", "renders", "draws",
        "clears", "resets", "allocates", "frees", "releases",
        "acquires", "locks", "unlocks", "waits", "notifies",
        "signals", "throws", "catches", "retries", "aborts",
        "commits", "rolls", "maps", "filters", "reduces",
        "sorts", "orders", "groups", "flattens", "merges",
        "splits", "joins", "concatenates", "appends", "prepends",
        "encodes", "decodes", "encrypts", "decrypts", "hashes",
        "compresses", "decompresses", "serializes", "deserializes",
        "instantiates", "constructs", "destroys", "disposes",
        "implements", "overrides", "extends", "inherits",
        "publishes", "subscribes", "broadcasts", "listens",
        "authenticates", "authorizes",
        "transforms", "normalizes", "sanitizes", "escapes",
        "schedules", "dispatches", "routes", "forwards",
        "caches", "invalidates", "refreshes", "syncs",
        "imports", "exports", "uploads", "downloads",
        // Imperative/bare form (also common in docstrings)
        "return", "get", "set", "create", "initialize",
        "compute", "calculate", "parse", "convert", "validate",
        "check", "determine", "retrieve", "fetch", "load",
        "save", "store", "delete", "remove", "add",
        "insert", "update", "process", "handle", "execute",
        "close", "open", "provide", "ensure", "allow",
        "find", "verify", "register", "format", "run",
        "estimate", "extract", "build", "send", "receive",
        "read", "write", "copy", "move", "call",
        "invoke", "start", "stop", "enable", "disable",
        "configure", "apply", "generate", "render", "draw",
        "clear", "reset", "allocate", "free", "release",
        "test", "assert", "try", "attempt", "connect", "disconnect",
        "encode", "decode", "encrypt", "decrypt", "hash",
        "compress", "decompress", "serialize", "deserialize",
        "instantiate", "construct", "destroy", "dispose",
        "implement", "override", "extend", "inherit",
        "publish", "subscribe", "broadcast", "listen",
        "authenticate", "authorize",
        "transform", "normalize", "sanitize", "escape",
        "schedule", "dispatch", "route", "forward",
        "cache", "invalidate", "refresh", "sync",
        "import", "export", "upload", "download",
        // Additional patterns from CodeSearchNet
        "flush", "emit", "init", "setup", "teardown",
        "print", "prints", "log", "logs", "debug", "trace", "warn", "error",
        "match", "matches", "compare", "compares", "equals", "contains", "exists",
        "convert", "cast", "coerce", "wrap", "unwrap",
        // Conditional/descriptive starters (only as FIRST word to avoid false positives)
        // Note: "that" removed because it causes false positives
        // with phrases like "function that handles auth"
        "given", "this", "whether", "if", "when",
        "called", "used", "intended", "designed", "meant",
        "version", "variant", "implementation", "wrapper",
        "turn", "turns", "toggle", "toggles",
        "persist", "persists", "represents", "represent",
        // Past participle descriptions (e.g., "Marked with...", "Span marked...")
        "marked", "tagged", "labeled", "associated", "linked",
        "bound", "attached", "connected", "decorated",
        // Helper patterns
        "helper", "utility", "convenience",
        // Ruby-style: "An optional function that..."
        "an", "a", "the",
    ];

    // Check if first word is a doc starter
    if doc_starters.contains(&first_word) {
        // Docstrings usually describe what a function does
        // Only reject if it starts with a question word (not contains)
        // because "which" can appear as a relative pronoun mid-sentence
        let starts_with_question = ["how ", "where ", "what ", "which ", "why ", "who "]
            .iter()
            .any(|q| stripped_lower.starts_with(q));
        if !starts_with_question && !stripped.contains('?') {
            return true;
        }
    }

    // For Go-style comments: "// FunctionName verb description"
    // Also handles patterns like "Observer is a subscribable..."
    // Check if second word is a doc starter verb
    let words: Vec<&str> = stripped_lower.split_whitespace().collect();
    if words.len() >= 2 {
        let second_word = words[1];
        // Only match verb-like doc starters, not pronouns/articles that could be in natural queries
        let verb_doc_starters = [
            "returns", "gets", "sets", "creates", "initializes", "computes",
            "calculates", "parses", "converts", "validates", "checks",
            "determines", "retrieves", "fetches", "loads", "saves", "stores",
            "deletes", "removes", "adds", "inserts", "updates", "processes",
            "handles", "executes", "closes", "opens", "provides", "ensures",
            "allows", "finds", "verifies", "registers", "formats", "runs",
            "estimates", "extracts", "builds", "sends", "receives", "reads",
            "writes", "copies", "moves", "calls", "invokes", "starts", "stops",
            "enables", "disables", "configures", "applies", "generates",
            "renders", "draws", "clears", "resets", "allocates", "frees",
            "releases", "acquires", "locks", "unlocks", "waits", "notifies",
            "signals", "throws", "catches", "retries", "aborts", "commits",
            "is", "are", // "FunctionName is deprecated", "Items are sorted", "Observer is..."
            // Past participle patterns (for "Subject + past_participle" patterns)
            "marked", "tagged", "labeled", "associated", "linked", "bound",
            "attached", "connected", "decorated", "used", "called", "intended",
        ];
        if verb_doc_starters.contains(&second_word) {
            // Verify it's not a question - only check start, not mid-sentence relative pronouns
            let starts_with_question = ["how ", "where ", "what ", "which ", "why ", "who "]
                .iter()
                .any(|q| stripped_lower.starts_with(q));
            if !starts_with_question && !stripped.contains('?') {
                return true;
            }
        }
    }

    // Patterns for method documentation (e.g., "NewClient returns a client...")
    // Only applies when there's additional context (more than one word)
    let word_count = stripped.split_whitespace().count();
    if word_count > 1 {
        // "New*" pattern common in Go constructors (e.g., "NewClient returns...")
        if first_word.starts_with("new") && first_word.len() > 3 {
            return true;
        }

        // "Get*"/"Set*" pattern for accessor methods
        if (first_word.starts_with("get") || first_word.starts_with("set")) && first_word.len() > 3 {
            return true;
        }

        // "Has*" pattern for boolean getters (e.g., "HasFrom returns a boolean")
        if first_word.starts_with("has") && first_word.len() > 3 {
            return true;
        }
    }

    // HTML in docstrings (common in Javadoc)
    // <tt> is the teletype/monospace tag used in older Javadoc (deprecated in HTML5)
    if query.contains("<p>")
        || query.contains("</p>")
        || query.contains("<br>")
        || query.contains("<code>")
        || query.contains("</code>")
        || query.contains("<pre>")
        || query.contains("</pre>")
        || query.contains("<em>")
        || query.contains("</em>")
        || query.contains("<strong>")
        || query.contains("</strong>")
        || query.contains("<b>")
        || query.contains("</b>")
        || query.contains("<i>")
        || query.contains("</i>")
        || query.contains("<ul>")
        || query.contains("</ul>")
        || query.contains("<ol>")
        || query.contains("</ol>")
        || query.contains("<li>")
        || query.contains("</li>")
        || query.contains("<a ")
        || query.contains("<a>")
        || query.contains("</a>")
        || query.contains("<tt>")
        || query.contains("</tt>")
    {
        return true;
    }

    // Javadoc inline tags (e.g., {@code}, {@link})
    if query.contains("{@code")
        || query.contains("{@link")
        || query.contains("{@linkplain")
        || query.contains("{@literal")
        || query.contains("{@value")
        || query.contains("{@inheritDoc")
    {
        return true;
    }

    // Non-Javadoc comment pattern
    if query.contains("non-Javadoc") || query.contains("(non-Javadoc)") {
        return true;
    }

    // Function documentation pattern "Function : name"
    if query_lower.starts_with("function :") || query_lower.starts_with("function:") {
        return true;
    }

    // TODO/NOTE/FIXME/HACK patterns are documentation comments
    if stripped_lower.starts_with("todo")
        || stripped_lower.starts_with("note:")
        || stripped_lower.starts_with("fixme")
        || stripped_lower.starts_with("hack")
        || stripped_lower.starts_with("bug:")
        || stripped_lower.starts_with("xxx")
        || stripped_lower.starts_with("warning:")
    {
        return true;
    }

    false
}

/// Returns true if hybrid search should be preferred for this query
///
/// This is the inverse of semantic-only preference. Use this for
/// automatic search mode selection.
pub fn should_use_hybrid(query: &str) -> bool {
    matches!(detect_query_style(query), QueryStyle::NaturalLanguage)
}

/// File extension to language mapping for preprocessing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeLanguage {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
    Java,
    CSharp,
    Cpp,
    C,
    Ruby,
    Swift,
    Kotlin,
    Scala,
    Shell,
    Unknown,
}

impl CodeLanguage {
    /// Detect language from file extension
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "rs" => Self::Rust,
            "py" | "pyi" | "pyw" => Self::Python,
            "js" | "mjs" | "cjs" => Self::JavaScript,
            "ts" | "tsx" | "mts" | "cts" => Self::TypeScript,
            "jsx" => Self::JavaScript,
            "go" => Self::Go,
            "java" => Self::Java,
            "cs" => Self::CSharp,
            "cpp" | "cc" | "cxx" | "hpp" | "hxx" | "h" => Self::Cpp,
            "c" => Self::C,
            "rb" | "rake" | "gemspec" => Self::Ruby,
            "swift" => Self::Swift,
            "kt" | "kts" => Self::Kotlin,
            "scala" | "sc" => Self::Scala,
            "sh" | "bash" | "zsh" => Self::Shell,
            _ => Self::Unknown,
        }
    }

    /// Detect language from file path
    pub fn from_path(path: &Path) -> Self {
        path.extension()
            .and_then(|e| e.to_str())
            .map(Self::from_extension)
            .unwrap_or(Self::Unknown)
    }

    /// Check if this is a code language (vs Unknown)
    pub fn is_code(&self) -> bool {
        !matches!(self, Self::Unknown)
    }
}

/// Check if a path is a code file
pub fn is_code_file(path: &Path) -> bool {
    CodeLanguage::from_path(path).is_code()
}

/// Check if a query looks like it contains code identifiers
///
/// Detects patterns like:
/// - camelCase: "getUserName", "parseHTTP"
/// - snake_case: "get_user_name", "HTTP_STATUS"
/// - PascalCase: "GetUserName", "HTTPServer"
///
/// Returns true if the query likely contains code that should be preprocessed.
/// Check if query contains code identifiers that should be preprocessed
///
/// This is used for preprocessing decisions - split camelCase, snake_case, etc.
/// Returns true if ANY part of the query contains identifiers, regardless of length.
pub fn has_code_identifiers(query: &str) -> bool {
    // Check for snake_case (underscores between alphanumeric)
    if query.contains('_') {
        let has_snake = query
            .chars()
            .zip(query.chars().skip(1))
            .zip(query.chars().skip(2))
            .any(|((a, b), c)| a.is_alphanumeric() && b == '_' && c.is_alphanumeric());
        if has_snake {
            return true;
        }
    }

    // Check for camelCase/PascalCase (lowercase followed by uppercase)
    let has_camel = query
        .chars()
        .zip(query.chars().skip(1))
        .any(|(a, b)| a.is_lowercase() && b.is_uppercase());
    if has_camel {
        return true;
    }

    // Check for consecutive uppercase letters (acronyms like HTTP, URL)
    let consecutive_upper = query
        .chars()
        .zip(query.chars().skip(1))
        .zip(query.chars().skip(2))
        .filter(|((a, b), c)| a.is_uppercase() && b.is_uppercase() && c.is_uppercase())
        .count();
    if consecutive_upper >= 1 {
        return true;
    }

    false
}

/// Check if query looks like a code identifier for search mode selection
///
/// CodeIdentifier style is for SHORT identifier lookups like "getUserName" or "http_client".
/// Multi-word queries containing technical terms should use NaturalLanguage (hybrid search).
///
/// This is separate from has_code_identifiers() which is used for preprocessing.
pub fn looks_like_code_query(query: &str) -> bool {
    let word_count = query.split_whitespace().count();

    // Only short queries (1-2 words) can be code identifier style
    // Longer queries like "SQLite database documents store" are natural language
    if word_count > 2 {
        return false;
    }

    // Delegate to the identifier check
    has_code_identifiers(query)
}

/// Clean up markdown and HTML artifacts from a query
///
/// Handles:
/// - Markdown inline code spans: `[`Filesize`]` → `Filesize`
/// - HTML entities: `&gt;` → `>`, `&lt;` → `<`, `&amp;` → `&`
/// - HTML tags: `<br/>` → space, `<tt>text</tt>` → `text`
/// - Angle bracket links: `<https://...>` → URL content
fn clean_query_markup(query: &str) -> String {
    let mut result = query.to_string();

    // Remove markdown link/code syntax:
    // - [`name`] → name (Rustdoc links)
    // - `code` → code (inline code span)
    // - [text](url) → text (markdown links)
    let mut cleaned = String::with_capacity(result.len());
    let mut chars = result.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '[' {
            // Attempt to parse [content]
            let mut content = String::new();
            let mut found_close = false;
            #[allow(clippy::while_let_on_iterator)]
            while let Some(inner) = chars.next() {
                if inner == ']' {
                    found_close = true;
                    break;
                }
                content.push(inner);
            }
            if found_close {
                let mut consumed_link = false;
                if chars.peek() == Some(&'(') {
                    chars.next(); // consume '('
                    // Skip URL part until ')'
                    for link_char in chars.by_ref() {
                        if link_char == ')' {
                            consumed_link = true;
                            break;
                        }
                    }
                }
                let is_backticked = content.starts_with('`') && content.ends_with('`') && content.len() >= 2;
                if is_backticked {
                    cleaned.push_str(content.trim_matches('`'));
                } else if consumed_link {
                    cleaned.push_str(&content);
                } else {
                    cleaned.push('[');
                    cleaned.push_str(&content);
                    cleaned.push(']');
                }
            } else {
                cleaned.push('[');
                cleaned.push_str(&content);
            }
        } else if c == '`' {
            // Inline code span `code`
            let mut content = String::new();
            let mut found_close = false;
            #[allow(clippy::while_let_on_iterator)]
            while let Some(inner) = chars.next() {
                if inner == '`' {
                    found_close = true;
                    break;
                }
                content.push(inner);
            }
            if found_close {
                cleaned.push_str(&content);
            } else {
                cleaned.push('`');
                cleaned.push_str(&content);
            }
        } else {
            cleaned.push(c);
        }
    }
    result = cleaned;

    // Decode common HTML entities
    result = result
        .replace("&gt;", ">")
        .replace("&lt;", "<")
        .replace("&amp;", "&")
        .replace("&nbsp;", " ")
        .replace("&quot;", "\"")
        .replace("&#39;", "'");

    // Remove HTML tags but preserve their content
    // Also handle angle-bracket URLs like <https://...>
    let mut cleaned = String::with_capacity(result.len());
    let mut chars = result.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '<' {
            // Check what follows the <
            if let Some(&next) = chars.peek() {
                // Check for angle-bracket URLs like <https://...> or <http://...>
                if next == 'h' {
                    let rest: String = chars.clone().take(5).collect();
                    if rest.starts_with("http") {
                        // This is a URL in angle brackets - include content without brackets
                        for url_char in chars.by_ref() {
                            if url_char == '>' {
                                break;
                            }
                            cleaned.push(url_char);
                        }
                        continue;
                    }
                }
                // Check if this looks like an HTML tag (starts with letter, /, or !)
                if next.is_alphabetic() || next == '/' || next == '!' {
                    // Skip until closing >
                    for tag_char in chars.by_ref() {
                        if tag_char == '>' {
                            cleaned.push(' '); // Replace tag with space
                            break;
                        }
                    }
                    continue;
                }
            }
            cleaned.push(c);
        } else {
            cleaned.push(c);
        }
    }
    result = cleaned;

    // Collapse multiple spaces
    let mut prev_space = false;
    result
        .chars()
        .filter(|&c| {
            if c == ' ' {
                if prev_space {
                    return false;
                }
                prev_space = true;
            } else {
                prev_space = false;
            }
            true
        })
        .collect::<String>()
        .trim()
        .to_string()
}

/// Preprocess a query for code search
///
/// Applies markup cleanup (markdown code spans, HTML entities/tags) to all queries,
/// then if the query looks like it contains code identifiers, preprocess it
/// the same way indexed code content is preprocessed. This ensures
/// queries like "getUserName" match content indexed as "get user name".
pub fn preprocess_query(query: &str) -> String {
    // First, clean up any markup artifacts
    let cleaned = clean_query_markup(query);

    // Then apply code preprocessing if it contains identifiers
    // This uses has_code_identifiers() which triggers on ANY identifier,
    // not looks_like_code_query() which only triggers on short queries
    if has_code_identifiers(&cleaned) {
        preprocess_code(&cleaned)
    } else {
        cleaned
    }
}

/// Preprocess code content for better tokenization
///
/// Transforms code identifiers into space-separated natural language tokens
/// while preserving the semantic meaning of the code.
pub fn preprocess_code(content: &str) -> String {
    let mut result = String::with_capacity(content.len() * 2);
    let mut chars = content.chars().peekable();

    while let Some(c) = chars.next() {
        // Handle identifiers (sequences of alphanumeric + underscore)
        if c.is_alphabetic() || c == '_' {
            let mut identifier = String::new();
            identifier.push(c);

            while let Some(&next) = chars.peek() {
                if next.is_alphanumeric() || next == '_' {
                    identifier.push(chars.next().unwrap());
                } else {
                    break;
                }
            }

            // Split and normalize the identifier
            let normalized = split_identifier(&identifier);
            result.push_str(&normalized);
        }
        // Handle numbers (keep as-is)
        else if c.is_numeric() {
            result.push(c);
            while let Some(&next) = chars.peek() {
                if next.is_alphanumeric() || next == '.' || next == '_' {
                    result.push(chars.next().unwrap());
                } else {
                    break;
                }
            }
        }
        // Handle operators - convert to natural language
        else if let Some(word) = operator_to_word(c, chars.peek().copied()) {
            // Check for two-character operators
            if word.len() > 1 && chars.peek().is_some() {
                let peek = *chars.peek().unwrap();
                if let Some(two_char) = two_char_operator_to_word(c, peek) {
                    chars.next(); // consume second char
                    result.push(' ');
                    result.push_str(two_char);
                    result.push(' ');
                    continue;
                }
            }
            result.push(' ');
            result.push_str(word);
            result.push(' ');
        }
        // Keep whitespace and other characters
        else {
            result.push(c);
        }
    }

    // Clean up multiple spaces
    collapse_whitespace(&result)
}

/// Split an identifier into space-separated words
fn split_identifier(ident: &str) -> String {
    if ident.is_empty() {
        return String::new();
    }

    // Handle all-uppercase acronyms like "HTTP" or "URL"
    if ident.chars().all(|c| c.is_uppercase() || c == '_') {
        return ident.to_lowercase().replace('_', " ");
    }

    let mut words = Vec::new();
    let mut current_word = String::new();
    let mut prev_upper = false;

    for (i, c) in ident.chars().enumerate() {
        if c == '_' {
            // snake_case separator
            if !current_word.is_empty() {
                words.push(current_word.clone());
                current_word.clear();
            }
        } else if c.is_uppercase() {
            if !current_word.is_empty() {
                // Check if this is start of new word (camelCase)
                // or continuation of acronym (HTTPServer)
                let next_is_lower = ident.chars().nth(i + 1).is_some_and(char::is_lowercase);

                if !prev_upper || next_is_lower {
                    words.push(current_word.clone());
                    current_word.clear();
                }
            }
            current_word.push(c.to_lowercase().next().unwrap_or(c));
            prev_upper = true;
        } else {
            current_word.push(c);
            prev_upper = false;
        }
    }

    if !current_word.is_empty() {
        words.push(current_word);
    }

    words.join(" ")
}

/// Convert single-character operators to natural language
fn operator_to_word(c: char, _next: Option<char>) -> Option<&'static str> {
    match c {
        '+' => Some("plus"),
        '-' => Some("minus"),
        '*' => Some("times"),
        '/' => Some("divide"),
        '%' => Some("modulo"),
        '=' => Some("equals"),
        '<' => Some("less"),
        '>' => Some("greater"),
        '!' => Some("not"),
        '&' => Some("and"),
        '|' => Some("or"),
        '^' => Some("xor"),
        '~' => Some("complement"),
        '?' => Some("question"),
        ':' => Some("colon"),
        _ => None,
    }
}

/// Convert two-character operators to natural language
fn two_char_operator_to_word(c1: char, c2: char) -> Option<&'static str> {
    match (c1, c2) {
        ('=', '=') => Some("equal"),
        ('!', '=') => Some("not equal"),
        ('<', '=') => Some("less or equal"),
        ('>', '=') => Some("greater or equal"),
        ('&', '&') => Some("and"),
        ('|', '|') => Some("or"),
        ('+', '+') => Some("increment"),
        ('-', '-') => Some("decrement"),
        ('+', '=') => Some("plus equals"),
        ('-', '=') => Some("minus equals"),
        ('*', '=') => Some("times equals"),
        ('/', '=') => Some("divide equals"),
        ('-', '>') => Some("arrow"),
        ('=', '>') => Some("fat arrow"),
        ('<', '<') => Some("shift left"),
        ('>', '>') => Some("shift right"),
        _ => None,
    }
}

/// Collapse multiple whitespace characters into single spaces
fn collapse_whitespace(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut prev_space = false;

    for c in s.chars() {
        if c.is_whitespace() {
            if !prev_space {
                result.push(' ');
                prev_space = true;
            }
        } else {
            result.push(c);
            prev_space = false;
        }
    }

    result.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_camel_case() {
        assert_eq!(split_identifier("getUserName"), "get user name");
        assert_eq!(split_identifier("XMLParser"), "xml parser");
        assert_eq!(split_identifier("parseHTTPResponse"), "parse http response");
    }

    #[test]
    fn test_split_snake_case() {
        assert_eq!(split_identifier("get_user_name"), "get user name");
        assert_eq!(split_identifier("HTTP_STATUS_CODE"), "http status code");
    }

    #[test]
    fn test_split_pascal_case() {
        assert_eq!(split_identifier("GetUserName"), "get user name");
        assert_eq!(split_identifier("HTTPServer"), "http server");
    }

    #[test]
    fn test_split_mixed_case() {
        assert_eq!(split_identifier("getHTTPResponse"), "get http response");
        assert_eq!(split_identifier("XMLHttpRequest"), "xml http request");
    }

    #[test]
    fn test_language_detection() {
        assert_eq!(CodeLanguage::from_extension("rs"), CodeLanguage::Rust);
        assert_eq!(CodeLanguage::from_extension("py"), CodeLanguage::Python);
        assert_eq!(CodeLanguage::from_extension("js"), CodeLanguage::JavaScript);
        assert_eq!(CodeLanguage::from_extension("ts"), CodeLanguage::TypeScript);
        assert_eq!(CodeLanguage::from_extension("go"), CodeLanguage::Go);
        assert_eq!(CodeLanguage::from_extension("txt"), CodeLanguage::Unknown);
    }

    #[test]
    fn test_is_code_file() {
        assert!(is_code_file(Path::new("main.rs")));
        assert!(is_code_file(Path::new("script.py")));
        assert!(!is_code_file(Path::new("readme.md")));
        assert!(!is_code_file(Path::new("data.json")));
    }

    #[test]
    fn test_preprocess_code_simple() {
        let code = "fn getUserName()";
        let processed = preprocess_code(code);
        assert!(processed.contains("get user name"));
    }

    #[test]
    fn test_preprocess_code_operators() {
        let code = "x == y";
        let processed = preprocess_code(code);
        assert!(processed.contains("equal"));
    }

    #[test]
    fn test_preprocess_preserves_numbers() {
        let code = "let x = 42;";
        let processed = preprocess_code(code);
        assert!(processed.contains("42"));
    }

    #[test]
    fn test_collapse_whitespace() {
        assert_eq!(collapse_whitespace("  hello   world  "), "hello world");
        assert_eq!(collapse_whitespace("a  b\n\nc"), "a b c");
    }

    #[test]
    fn test_preprocess_rust_function() {
        let code = r"
fn process_http_request(request_id: u32) -> Result<Response> {
    let user_name = get_user_name();
    Ok(Response::new())
}
";
        let processed = preprocess_code(code);
        // Should contain split identifiers
        assert!(processed.contains("process http request"));
        assert!(processed.contains("request id"));
        assert!(processed.contains("user name"));
        assert!(processed.contains("get user name"));
    }

    #[test]
    fn test_looks_like_code_query_camel_case() {
        assert!(looks_like_code_query("getUserName"));
        assert!(looks_like_code_query("parseHTTPResponse"));
        assert!(looks_like_code_query("XMLHttpRequest"));
    }

    #[test]
    fn test_looks_like_code_query_snake_case() {
        assert!(looks_like_code_query("get_user_name"));
        assert!(looks_like_code_query("HTTP_STATUS_CODE"));
        assert!(looks_like_code_query("parse_http_response"));
    }

    #[test]
    fn test_looks_like_code_query_pascal_case() {
        assert!(looks_like_code_query("GetUserName"));
        assert!(looks_like_code_query("HTTPServer"));
        assert!(looks_like_code_query("XMLParser"));
    }

    #[test]
    fn test_looks_like_code_query_natural_language() {
        // Natural language queries should not be detected as code
        assert!(!looks_like_code_query("get user name"));
        assert!(!looks_like_code_query("parse the response"));
        assert!(!looks_like_code_query("vampire castle Transylvania"));
        assert!(!looks_like_code_query("search for files"));
    }

    #[test]
    fn test_looks_like_code_query_edge_cases() {
        // Short inputs
        assert!(!looks_like_code_query("a"));
        assert!(!looks_like_code_query("ab"));
        // Single underscore
        assert!(!looks_like_code_query("_"));
        // Trailing/leading underscore without word
        assert!(!looks_like_code_query("_foo"));
    }

    #[test]
    fn test_preprocess_query_code_like() {
        // Code-like queries should be preprocessed
        assert_eq!(preprocess_query("getUserName"), "get user name");
        assert_eq!(preprocess_query("get_user_name"), "get user name");
        assert_eq!(preprocess_query("GetUserName"), "get user name");
    }

    #[test]
    fn test_preprocess_query_natural_language() {
        // Natural language queries should pass through unchanged
        assert_eq!(preprocess_query("get user name"), "get user name");
        assert_eq!(
            preprocess_query("vampire Transylvania"),
            "vampire Transylvania"
        );
        assert_eq!(preprocess_query("search files"), "search files");
    }

    #[test]
    fn test_preprocess_query_mixed() {
        // Mixed queries with some code identifiers
        let result = preprocess_query("call getUserName function");
        assert!(result.contains("get user name"));
    }

    #[test]
    fn test_detect_query_style_docstring_javadoc() {
        // Javadoc-style annotations
        assert_eq!(
            detect_query_style("@param name the user name"),
            QueryStyle::Docstring
        );
        assert_eq!(
            detect_query_style("@Param name the user name"),
            QueryStyle::Docstring
        );
        assert_eq!(
            detect_query_style("@return the result value"),
            QueryStyle::Docstring
        );
        assert_eq!(
            detect_query_style("@throws IllegalArgumentException if null"),
            QueryStyle::Docstring
        );
        assert_eq!(
            detect_query_style("@see SomeClass#method"),
            QueryStyle::Docstring
        );
        assert_eq!(
            detect_query_style("Returns a {@link Class} constant"),
            QueryStyle::Docstring
        );
    }

    #[test]
    fn test_detect_query_style_docstring_python() {
        // Python docstring patterns
        assert_eq!(
            detect_query_style(":param name: the user name"),
            QueryStyle::Docstring
        );
        assert_eq!(
            detect_query_style(":Param name: the user name"),
            QueryStyle::Docstring
        );
        assert_eq!(
            detect_query_style(":returns: the computed value"),
            QueryStyle::Docstring
        );
        assert_eq!(
            detect_query_style("Args: name, value"),
            QueryStyle::Docstring
        );
        assert_eq!(
            detect_query_style("Returns: the result"),
            QueryStyle::Docstring
        );
    }

    #[test]
    fn test_detect_query_style_docstring_verb_starts() {
        // Common docstring sentence starters
        assert_eq!(
            detect_query_style("Returns the count of items"),
            QueryStyle::Docstring
        );
        assert_eq!(
            detect_query_style("Gets the current user name"),
            QueryStyle::Docstring
        );
        assert_eq!(
            detect_query_style("Parses the input string"),
            QueryStyle::Docstring
        );
        assert_eq!(
            detect_query_style("Validates the user input"),
            QueryStyle::Docstring
        );
    }

    #[test]
    fn test_detect_query_style_docstring_bullets() {
        // Bulleted doc comment lines should still look like docstrings.
        assert_eq!(
            detect_query_style("* Returns the count of items"),
            QueryStyle::Docstring
        );
        assert_eq!(
            detect_query_style("- Note: cached after first call"),
            QueryStyle::Docstring
        );
        assert_eq!(
            detect_query_style("+ TODO: add validation"),
            QueryStyle::Docstring
        );
    }

    #[test]
    fn test_detect_query_style_docstring_html() {
        // HTML tags in docstrings
        assert_eq!(
            detect_query_style("Computes the value.<p>Returns null if invalid."),
            QueryStyle::Docstring
        );
        assert_eq!(
            detect_query_style("Use <code>getValue()</code> to retrieve"),
            QueryStyle::Docstring
        );
        assert_eq!(
            detect_query_style("Examples:<pre>doThing();</pre>"),
            QueryStyle::Docstring
        );
        assert_eq!(
            detect_query_style("More details at <a href=\"https://example.com\">docs</a>"),
            QueryStyle::Docstring
        );
        // <tt> tag (teletype/monospace, deprecated in HTML5 but common in older Javadoc)
        assert_eq!(
            detect_query_style("Prove <tt>x = y implies g(x) = g(y)</tt>"),
            QueryStyle::Docstring
        );
    }

    #[test]
    fn test_detect_query_style_docstring_javadoc_inline_tags() {
        assert_eq!(
            detect_query_style("See {@link SomeClass} for the contract."),
            QueryStyle::Docstring
        );
        assert_eq!(
            detect_query_style("Returns {@code null} when missing."),
            QueryStyle::Docstring
        );
    }

    #[test]
    fn test_detect_query_style_natural_language() {
        // Natural language queries with question words
        assert_eq!(
            detect_query_style("how do I get the user name"),
            QueryStyle::NaturalLanguage
        );
        assert_eq!(
            detect_query_style("where is authentication handled"),
            QueryStyle::NaturalLanguage
        );
        assert_eq!(
            detect_query_style("what function parses JSON?"),
            QueryStyle::NaturalLanguage
        );
        // General natural language
        assert_eq!(
            detect_query_style("function that handles auth"),
            QueryStyle::NaturalLanguage
        );
        assert_eq!(
            detect_query_style("code for database connection"),
            QueryStyle::NaturalLanguage
        );
    }

    #[test]
    fn test_detect_query_style_code_identifier() {
        // Code identifiers
        assert_eq!(
            detect_query_style("getUserName"),
            QueryStyle::CodeIdentifier
        );
        assert_eq!(
            detect_query_style("get_user_name"),
            QueryStyle::CodeIdentifier
        );
        assert_eq!(
            detect_query_style("HTTPServer"),
            QueryStyle::CodeIdentifier
        );
    }

    #[test]
    fn test_should_use_hybrid() {
        // Natural language → hybrid
        assert!(should_use_hybrid("function that handles auth"));
        assert!(should_use_hybrid("where is the error handler"));

        // Docstrings → semantic-only (not hybrid)
        assert!(!should_use_hybrid("Returns the user name"));
        assert!(!should_use_hybrid("@param name the input"));

        // Code identifiers → semantic-only (not hybrid)
        assert!(!should_use_hybrid("getUserName"));
    }

    #[test]
    fn test_codesearchnet_queries_are_docstrings() {
        // These are real CodeSearchNet queries that should be detected as docstrings
        // because they come from actual code documentation

        // Java-style docstring with "Function :" prefix
        assert_eq!(
            detect_query_style("Function : diff_y -To compute differntiation along y axis."),
            QueryStyle::Docstring,
            "Function : prefix should be docstring"
        );

        // Ruby conditional docstring
        assert_eq!(
            detect_query_style("If the column type is nominal return true."),
            QueryStyle::Docstring,
            "'If the...' conditional should be docstring"
        );

        // FIXME comment
        assert_eq!(
            detect_query_style("FIXME: Eventually split into day_match?, hour_match? and monthdays_match?o"),
            QueryStyle::Docstring,
            "FIXME: should be docstring"
        );

        // "This is..." pattern
        assert_eq!(
            detect_query_style("This is just an orion/widgets/input/Select with a label."),
            QueryStyle::Docstring,
            "'This is...' should be docstring"
        );

        // Subject + "is" pattern (Observer is...)
        assert_eq!(
            detect_query_style("Observer is a subscribable which watches a function"),
            QueryStyle::Docstring,
            "'Subject is...' pattern should be docstring"
        );

        // Terse docstring
        assert_eq!(
            detect_query_style("Span marked with the correct expansion and transparency."),
            QueryStyle::Docstring,
            "Terse descriptive sentence should be docstring"
        );

        // Helper docstring
        assert_eq!(
            detect_query_style("Helper to set the speed from start of playback."),
            QueryStyle::Docstring,
            "'Helper to...' should be docstring"
        );

        // "Ladies and gents" - informal comment (not detected as docstring)
        // This is an edge case - informal humor in comments doesn't match typical patterns
        // Note: Both semantic and hybrid modes fail on this query anyway
        assert_eq!(
            detect_query_style("Ladies and gents, a prime example of async callback hell"),
            QueryStyle::NaturalLanguage,
            "Informal comments default to natural language"
        );
    }

    #[test]
    fn test_clean_query_markup_markdown_code_spans() {
        // Markdown inline code spans: [`name`] → name
        assert_eq!(
            clean_query_markup("Returns a [`Filesize`] representing the sign"),
            "Returns a Filesize representing the sign"
        );
        assert_eq!(
            clean_query_markup("Use [`Option`] or [`Result`] for error handling"),
            "Use Option or Result for error handling"
        );
        // Multiple markdown code spans (double space collapsed to single)
        assert_eq!(
            clean_query_markup("Rotates by the given [`Quat`].  [`Aabb`] of entities"),
            "Rotates by the given Quat. Aabb of entities"
        );
    }

    #[test]
    fn test_clean_query_markup_markdown_inline_code() {
        assert_eq!(
            clean_query_markup("Use `Option` or `Result` for error handling"),
            "Use Option or Result for error handling"
        );
        assert_eq!(
            clean_query_markup("Call `get_user_name()` to fetch data"),
            "Call get_user_name() to fetch data"
        );
    }

    #[test]
    fn test_clean_query_markup_markdown_links() {
        assert_eq!(
            clean_query_markup("See [the docs](https://example.com) for details"),
            "See the docs for details"
        );
        assert_eq!(
            clean_query_markup("Link to [Option](https://doc.rust-lang.org) types"),
            "Link to Option types"
        );
    }

    #[test]
    fn test_clean_query_markup_html_entities() {
        // Common HTML entities
        assert_eq!(
            clean_query_markup("(PHP 5 &gt;= 5.0.0)"),
            "(PHP 5 >= 5.0.0)"
        );
        assert_eq!(
            clean_query_markup("x &lt; y &amp;&amp; y &gt; z"),
            "x < y && y > z"
        );
        assert_eq!(
            clean_query_markup("&quot;hello&quot; &nbsp; world"),
            "\"hello\" world"
        );
    }

    #[test]
    fn test_clean_query_markup_html_tags() {
        // HTML tags should be removed, content preserved
        assert_eq!(
            clean_query_markup("Prove <tt>x = y implies g(x) = g(y)</tt>"),
            "Prove x = y implies g(x) = g(y)"
        );
        assert_eq!(
            clean_query_markup("See <code>getValue()</code> method"),
            "See getValue() method"
        );
        assert_eq!(
            clean_query_markup("This is<br/>a test"),
            "This is a test"
        );
        assert_eq!(
            clean_query_markup("Visit <a href=\"url\">docs</a> for more"),
            "Visit docs for more"
        );
    }

    #[test]
    fn test_clean_query_markup_angle_bracket_urls() {
        // URLs in angle brackets - extract URL content
        assert_eq!(
            clean_query_markup("<https://gpuweb.github.io/gpuweb/#dom-gpurenderencoderbase-draw>"),
            "https://gpuweb.github.io/gpuweb/#dom-gpurenderencoderbase-draw"
        );
        assert_eq!(
            clean_query_markup("See <http://example.com>"),
            "See http://example.com"
        );
    }

    #[test]
    fn test_clean_query_markup_collapse_spaces() {
        // Multiple spaces should be collapsed
        assert_eq!(
            clean_query_markup("hello    world"),
            "hello world"
        );
        assert_eq!(
            clean_query_markup("  leading and trailing  "),
            "leading and trailing"
        );
    }

    #[test]
    fn test_clean_query_markup_preserves_regular_text() {
        // Regular text should pass through unchanged
        assert_eq!(
            clean_query_markup("Returns the count of items"),
            "Returns the count of items"
        );
        assert_eq!(
            clean_query_markup("Helper to set the speed"),
            "Helper to set the speed"
        );
    }

    #[test]
    fn test_preprocess_query_with_markup() {
        // preprocess_query should clean markup first
        assert_eq!(
            preprocess_query("Returns a [`Filesize`] representing the sign"),
            "Returns a Filesize representing the sign"
        );
        // HTML entities are decoded, then code preprocessing normalizes case/operators
        // "(PHP 5 &gt;= 5.0.0)" → "(PHP 5 >= 5.0.0)" → "(php 5 greater or equal 5.0.0)"
        assert_eq!(
            preprocess_query("(PHP 5 &gt;= 5.0.0)"),
            "(php 5 greater or equal 5.0.0)"
        );
        // But clean_query_markup alone just decodes entities
        assert_eq!(
            clean_query_markup("(PHP 5 &gt;= 5.0.0)"),
            "(PHP 5 >= 5.0.0)"
        );
    }
}
