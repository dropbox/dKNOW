// Tokenizer for Idefics3 model (CodeFormula)
//
// Uses HuggingFace tokenizers library (same as Python implementation)
// Loads tokenizer.json and vocab.json from model directory
//
// Special tokens:
// - image_token_id: 100270 ("<image>")
// - bos_token_id: 100264 (beginning of sequence)
// - eos_token_id: 100338 (end of sequence, "<end_of_utterance>")
// - pad_token_id: 100256 ("<|pad|>")
//
// Chat template:
// "<|start_of_role|>{role}:{content}<end_of_utterance>\n"
// For code: "user:<image><code><end_of_utterance>\nassistant:"
// For formula: "user:<image><formula><end_of_utterance>\nassistant:"

use std::path::Path;
use tokenizers::Tokenizer;

/// Tokenizer wrapper for Idefics3 model
pub struct Idefics3Tokenizer {
    tokenizer: Tokenizer,
    image_token_id: u32,
    bos_token_id: u32,
    eos_token_id: u32,
    pad_token_id: u32,
}

impl std::fmt::Debug for Idefics3Tokenizer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Idefics3Tokenizer")
            .field("tokenizer", &"<Tokenizer>")
            .field("image_token_id", &self.image_token_id)
            .field("bos_token_id", &self.bos_token_id)
            .field("eos_token_id", &self.eos_token_id)
            .field("pad_token_id", &self.pad_token_id)
            .finish()
    }
}

impl Idefics3Tokenizer {
    /// Load tokenizer from model directory
    ///
    /// Loads tokenizer.json and reads special token IDs from config.json
    pub fn from_pretrained<P: AsRef<Path>>(
        model_dir: P,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let model_dir = model_dir.as_ref();

        // Load tokenizer from tokenizer.json
        let tokenizer_path = model_dir.join("tokenizer.json");
        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| format!("Failed to load tokenizer from {:?}: {}", tokenizer_path, e))?;

        // Load config to get special token IDs
        let config_path = model_dir.join("config.json");
        let config_str = std::fs::read_to_string(&config_path)
            .map_err(|e| format!("Failed to read config from {:?}: {}", config_path, e))?;
        let config: serde_json::Value = serde_json::from_str(&config_str)?;

        let image_token_id = config["image_token_id"].as_u64().unwrap_or(100270) as u32;
        let bos_token_id = config["bos_token_id"].as_u64().unwrap_or(100264) as u32;
        let eos_token_id = config["eos_token_id"].as_u64().unwrap_or(100338) as u32;
        let pad_token_id = config["pad_token_id"].as_u64().unwrap_or(100256) as u32;

        Ok(Self {
            tokenizer,
            image_token_id,
            bos_token_id,
            eos_token_id,
            pad_token_id,
        })
    }

    /// Encode text to token IDs
    pub fn encode(
        &self,
        text: &str,
        add_special_tokens: bool,
    ) -> Result<Vec<u32>, Box<dyn std::error::Error>> {
        let encoding = self
            .tokenizer
            .encode(text, add_special_tokens)
            .map_err(|e| format!("Failed to encode text: {}", e))?;
        Ok(encoding.get_ids().to_vec())
    }

    /// Decode token IDs to text
    pub fn decode(
        &self,
        token_ids: &[u32],
        skip_special_tokens: bool,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let text = self
            .tokenizer
            .decode(token_ids, skip_special_tokens)
            .map_err(|e| format!("Failed to decode tokens: {}", e))?;
        Ok(text)
    }

    /// Get special token IDs
    #[inline]
    #[must_use = "returns the image token ID"]
    pub const fn image_token_id(&self) -> u32 {
        self.image_token_id
    }

    #[inline]
    #[must_use = "returns the beginning-of-sequence token ID"]
    pub const fn bos_token_id(&self) -> u32 {
        self.bos_token_id
    }

    #[inline]
    #[must_use = "returns the end-of-sequence token ID"]
    pub const fn eos_token_id(&self) -> u32 {
        self.eos_token_id
    }

    #[inline]
    #[must_use = "returns the padding token ID"]
    pub const fn pad_token_id(&self) -> u32 {
        self.pad_token_id
    }

    /// Apply chat template to generate prompt
    ///
    /// Template: `"<|start_of_role|>{role}:{content}<end_of_utterance>\n"`
    ///
    /// For code:
    /// `"user:<image><code><end_of_utterance>\nassistant:"`
    ///
    /// For formula:
    /// `"user:<image><formula><end_of_utterance>\nassistant:"`
    pub fn apply_chat_template(
        &self,
        label: &str,
        add_generation_prompt: bool,
    ) -> Result<String, Box<dyn std::error::Error>> {
        // Validate label
        let query = match label {
            "code" => "<code>",
            "formula" => "<formula>",
            _ => return Err("Label must be 'code' or 'formula'".into()),
        };

        // Build prompt following Python's chat template
        // Python uses: processor.apply_chat_template(messages, add_generation_prompt=True)
        // messages = [{"role": "user", "content": [{"type": "image"}, {"type": "text", "text": query}]}]
        //
        // Template: "<|start_of_role|>user:<image>{query}<end_of_utterance>\nassistant:"
        let mut prompt = String::from("<|start_of_role|>user:<image>");
        prompt.push_str(query);
        prompt.push_str("<end_of_utterance>\n");

        if add_generation_prompt {
            prompt.push_str("assistant:");
        }

        Ok(prompt)
    }

    /// Post-process generated text
    ///
    /// Removes special tokens and unwanted substrings:
    /// - `"<end_of_utterance>"` (truncate at this point)
    /// - `"</code>"`, `"</formula>"`
    /// - `"<loc_0><loc_0><loc_500><loc_500>"` (location tokens)
    ///
    /// Returns (cleaned_text, language) where language is extracted from "<_Language_>" prefix
    pub fn post_process(&self, text: &str) -> (String, Option<String>) {
        let mut cleaned = text.to_string();

        // Truncate at <end_of_utterance>
        if let Some(idx) = cleaned.find("<end_of_utterance>") {
            cleaned.truncate(idx);
        }

        // Remove unwanted tokens
        let to_remove = ["</code>", "</formula>", "<loc_0><loc_0><loc_500><loc_500>"];
        for token in &to_remove {
            cleaned = cleaned.replace(token, "");
        }

        cleaned = cleaned.trim_start().to_string();

        // Extract language from "<_Language_>" prefix (code only)
        let language = extract_code_language(&cleaned);
        if let Some(lang) = &language {
            // Remove language prefix from text
            let prefix = format!("<_{}_>", lang);
            if let Some(rest) = cleaned.strip_prefix(&prefix) {
                cleaned = rest.trim_start().to_string();
            }
        }

        // N=4410: Normalize code formatting - remove spaces before punctuation
        // Python's Idefics3 produces `function add(a, b)` but Rust version produces `function add (a, b)`
        // This normalization ensures parity with Python groundtruth
        cleaned = normalize_code_spacing(&cleaned);

        (cleaned, language)
    }
}

/// Normalize code spacing by removing spaces before punctuation
///
/// Code tokens may have spurious spaces before punctuation like `(`, `)`, `,`, etc.
/// This matches Python's Idefics3 output which produces clean code formatting.
fn normalize_code_spacing(text: &str) -> String {
    // Remove spaces before: ( ) [ ] , ; :
    // Keep spaces after for readability
    let mut result = text.to_string();
    for punct in &['(', ')', '[', ']', ',', ';'] {
        result = result.replace(&format!(" {}", punct), &punct.to_string());
    }
    // Special case: preserve space after colon in Python type hints (a: int)
    // but remove space before colon
    result = result.replace(" :", ":");
    result
}

/// Extract programming language from "<_Language_>" prefix
///
/// Pattern: "^<_([^_>]+)_>\s*(.*)"
/// Example: "<_JavaScript_> code..." → Some("JavaScript")
fn extract_code_language(text: &str) -> Option<String> {
    use lazy_static::lazy_static;

    lazy_static! {
        // Pattern: <_language_>
        static ref CODE_LANG_RE: regex::Regex =
            regex::Regex::new(r"^<_([^_>]+)_>").expect("valid code language regex");
    }

    let caps = CODE_LANG_RE.captures(text)?;
    Some(caps.get(1)?.as_str().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use log;

    fn get_model_dir() -> std::path::PathBuf {
        std::path::Path::new(env!("HOME"))
            .join(".cache/huggingface/hub/models--ds4sd--CodeFormulaV2/snapshots/ecedbe111d15c2dc60bfd4a823cbe80127b58af4")
    }

    #[test]
    fn test_load_tokenizer() {
        let model_dir = get_model_dir();
        if !model_dir.exists() {
            log::warn!("Skipping test: model directory not found");
            return;
        }

        let tokenizer = Idefics3Tokenizer::from_pretrained(&model_dir);
        assert!(
            tokenizer.is_ok(),
            "Failed to load tokenizer: {:?}",
            tokenizer.err()
        );

        let tokenizer = tokenizer.unwrap();
        assert_eq!(tokenizer.image_token_id(), 100270);
        assert_eq!(tokenizer.bos_token_id(), 100264);
        assert_eq!(tokenizer.eos_token_id(), 100338);
        assert_eq!(tokenizer.pad_token_id(), 100256);
    }

    #[test]
    fn test_encode_decode() {
        let model_dir = get_model_dir();
        if !model_dir.exists() {
            log::warn!("Skipping test: model directory not found");
            return;
        }

        let tokenizer = Idefics3Tokenizer::from_pretrained(&model_dir).unwrap();

        // Test encode/decode round-trip
        let text = "Hello, world!";
        let token_ids = tokenizer.encode(text, false).unwrap();
        let decoded = tokenizer.decode(&token_ids, false).unwrap();

        // Decoded text should match original (with possible whitespace differences)
        assert_eq!(decoded.trim(), text.trim());
    }

    #[test]
    fn test_chat_template_code() {
        let model_dir = get_model_dir();
        if !model_dir.exists() {
            log::warn!("Skipping test: model directory not found");
            return;
        }

        let tokenizer = Idefics3Tokenizer::from_pretrained(&model_dir).unwrap();
        let prompt = tokenizer.apply_chat_template("code", true).unwrap();

        // Expected: "<|start_of_role|>user:<image><code><end_of_utterance>\nassistant:"
        assert!(prompt.contains("<|start_of_role|>"));
        assert!(prompt.contains("user:"));
        assert!(prompt.contains("<image>"));
        assert!(prompt.contains("<code>"));
        assert!(prompt.contains("<end_of_utterance>"));
        assert!(prompt.ends_with("assistant:"));
    }

    #[test]
    fn test_chat_template_formula() {
        let model_dir = get_model_dir();
        if !model_dir.exists() {
            log::warn!("Skipping test: model directory not found");
            return;
        }

        let tokenizer = Idefics3Tokenizer::from_pretrained(&model_dir).unwrap();
        let prompt = tokenizer.apply_chat_template("formula", true).unwrap();

        // Expected: "<|start_of_role|>user:<image><formula><end_of_utterance>\nassistant:"
        assert!(prompt.contains("<|start_of_role|>"));
        assert!(prompt.contains("user:"));
        assert!(prompt.contains("<image>"));
        assert!(prompt.contains("<formula>"));
        assert!(prompt.contains("<end_of_utterance>"));
        assert!(prompt.ends_with("assistant:"));
    }

    #[test]
    fn test_post_process_simple() {
        let model_dir = get_model_dir();
        if !model_dir.exists() {
            log::warn!("Skipping test: model directory not found");
            return;
        }

        let tokenizer = Idefics3Tokenizer::from_pretrained(&model_dir).unwrap();

        // Test truncation at <end_of_utterance>
        let text = "function foo() {}<end_of_utterance>extra text";
        let (cleaned, lang) = tokenizer.post_process(text);
        assert_eq!(cleaned, "function foo() {}");
        assert!(lang.is_none());

        // Test removal of special tokens
        let text = "</code>some code</code>";
        let (cleaned, lang) = tokenizer.post_process(text);
        assert_eq!(cleaned, "some code");
        assert!(lang.is_none());
    }

    #[test]
    fn test_post_process_with_language() {
        let model_dir = get_model_dir();
        if !model_dir.exists() {
            log::warn!("Skipping test: model directory not found");
            return;
        }

        let tokenizer = Idefics3Tokenizer::from_pretrained(&model_dir).unwrap();

        // Test language extraction
        let text = "<_JavaScript_> function add(a, b) { return a + b; }";
        let (cleaned, lang) = tokenizer.post_process(text);
        assert_eq!(cleaned, "function add(a, b) { return a + b; }");
        assert_eq!(lang, Some("JavaScript".to_string()));

        // Test with Python
        let text = "<_Python_> def add(a, b): return a + b";
        let (cleaned, lang) = tokenizer.post_process(text);
        assert_eq!(cleaned, "def add(a, b): return a + b");
        assert_eq!(lang, Some("Python".to_string()));
    }

    #[test]
    fn test_extract_code_language() {
        assert_eq!(
            extract_code_language("<_JavaScript_> code"),
            Some("JavaScript".to_string())
        );
        assert_eq!(
            extract_code_language("<_Python_> code"),
            Some("Python".to_string())
        );
        assert_eq!(
            extract_code_language("<_Rust_> code"),
            Some("Rust".to_string())
        );
        assert_eq!(extract_code_language("no language prefix"), None);
        assert_eq!(extract_code_language("<_Invalid"), None);
    }

    #[test]
    fn test_normalize_code_spacing() {
        // N=4410: Test code spacing normalization
        // Remove spaces before punctuation
        assert_eq!(
            normalize_code_spacing("function add (a , b)"),
            "function add(a, b)"
        );
        assert_eq!(
            normalize_code_spacing("console . log ( add (3 , 5) )"),
            "console . log(add(3, 5))"
        );
        // Colons should also be normalized
        assert_eq!(
            normalize_code_spacing("def foo (x : int) :"),
            "def foo(x: int):"
        );
        // Already clean code should stay unchanged
        assert_eq!(
            normalize_code_spacing("function add(a, b) { return a + b; }"),
            "function add(a, b) { return a + b; }"
        );
    }
}
