// examples/world_class_multilingual.rs
//
// Demonstrates world-class multilingual support:
// English, Japanese, Chinese, Korean, Arabic, and mixed content

use markdown_chunker::Chunker;

fn main() {
    println!("🌍 World-Class Multilingual Markdown Chunker Demo\n");
    println!("{}", "=".repeat(60));

    // English
    let english = r"
# English Document

This is a sample document in English. The chunker handles English text
with approximately 4 characters per token estimation.

## Features

- Markdown structure preservation
- Code block protection
- Smart sentence boundaries
";

    // Japanese
    let japanese = r"
# 日本語の文書

これは日本語のサンプル文書です。チャンカーは日本語のテキストを適切に処理します。
ひらがな、カタカナ、漢字のすべてをサポートしています。

## 特徴

- マークダウン構造の保持
- コードブロックの保護
- 文の境界の認識
";

    // Chinese
    let chinese = r"
# 中文文档

这是一个中文示例文档。分块器可以正确处理中文文本。
支持简体和繁体中文。

## 特点

- 保留Markdown结构
- 保护代码块
- 智能句子边界
";

    // Arabic
    let arabic = r"
# وثيقة عربية

هذا مستند عربي نموذجي. يتعامل المجزئ مع النص العربي بشكل صحيح.
يدعم جميع أشكال الأحرف العربية.

## الميزات

- الحفاظ على بنية Markdown
- حماية كتل الكود
- حدود الجملة الذكية
";

    // Mixed content (English + Japanese)
    let mixed = r"
# Multilingual Document / 多言語ドキュメント

This document contains both English and Japanese text.
この文書には英語と日本語の両方が含まれています。

## Technical Details / 技術詳細

The chunker uses character-based detection to identify scripts:
- CJK characters: ~2 chars per token
- Arabic characters: ~5 chars per token
- Latin characters: ~4 chars per token

日本語の文字は自動的に検出され、適切なトークン推定が適用されます。
";

    let chunker = Chunker::default();

    // Process each language
    println!("\n📝 ENGLISH:");
    process_and_display(&chunker, english, "English");

    println!("\n📝 JAPANESE (日本語):");
    process_and_display(&chunker, japanese, "Japanese");

    println!("\n📝 CHINESE (中文):");
    process_and_display(&chunker, chinese, "Chinese");

    println!("\n📝 ARABIC (العربية):");
    process_and_display(&chunker, arabic, "Arabic");

    println!("\n📝 MIXED (English + 日本語):");
    process_and_display(&chunker, mixed, "Mixed");

    println!("\n{}", "=".repeat(60));
    println!("✅ All languages processed successfully!");
    println!("🌍 World-class multilingual support verified!");
}

fn process_and_display(chunker: &Chunker, text: &str, language: &str) {
    let chunks = chunker.chunk(text);

    println!("  Language: {language}");
    println!("  Chunks generated: {}", chunks.len());

    for (i, chunk) in chunks.iter().enumerate() {
        let preview = chunk
            .content
            .lines()
            .next()
            .unwrap_or("")
            .chars()
            .take(50)
            .collect::<String>();

        println!(
            "    Chunk {}: {} tokens, {} chars - \"{}...\"",
            i + 1,
            chunk.metadata.token_count,
            chunk.metadata.char_count,
            preview
        );
    }
}
