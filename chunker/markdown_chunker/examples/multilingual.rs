use markdown_chunker::Chunker;

fn main() {
    println!("=== Multilingual Chunking Examples ===\n");

    // Japanese example
    let japanese = r#"
# はじめに

Rustは、安全性、速度、並行性を重視したシステムプログラミング言語です。

## メモリ安全性

Rustの所有権システムは、ガベージコレクタを必要とせずにメモリ安全性を保証します。
これにより、パフォーマンスが重要なシステムプログラミングに最適です。

## 並行性

Rustの型システムは、コンパイル時にデータ競合を防ぎ、並行プログラミングをより安全で信頼性の高いものにします。

```rust
fn main() {
    println!("こんにちは、世界！");
}
```

## パフォーマンス

Rustはゼロコスト抽象化を提供し、使用しない機能に対して料金を支払う必要はありません。
コンパイルされたコードは、同等のCまたはC++コードと同じ速度で実行されます。
"#;

    // Chinese example
    let chinese = r#"
# 介绍

Rust是一种系统编程语言，专注于安全性、速度和并发性。

## 内存安全

Rust的所有权系统确保内存安全，无需垃圾收集器。
这使其成为性能至关重要的系统编程的理想选择。

## 并发性

Rust的类型系统有助于在编译时防止数据竞争，使并发编程更安全、更可靠。

```rust
fn main() {
    println!("你好，世界！");
}
```

## 性能

Rust提供零成本抽象，这意味着您不需要为不使用的功能付费。
编译后的代码运行速度与等效的C或C++代码一样快。
"#;

    // Korean example
    let korean = r#"
# 소개

Rust는 안전성、속도 및 동시성에 중점을 둔 시스템 프로그래밍 언어입니다。

## 메모리 안전성

Rust의 소유권 시스템은 가비지 수집기 없이도 메모리 안전성을 보장합니다。
이는 성능이 중요한 시스템 프로그래밍에 이상적입니다。

```rust
fn main() {
    println!("안녕하세요, 세계!");
}
```
"#;

    let chunker = Chunker::builder()
        .max_tokens(500)
        .min_tokens(50)
        .build();

    // Process Japanese
    println!("🇯🇵 Japanese Document");
    println!("{}", "=".repeat(60));
    let ja_chunks = chunker.chunk(japanese);
    print_language_stats("Japanese", &ja_chunks);

    // Process Chinese
    println!("\n🇨🇳 Chinese Document");
    println!("{}", "=".repeat(60));
    let zh_chunks = chunker.chunk(chinese);
    print_language_stats("Chinese", &zh_chunks);

    // Process Korean
    println!("\n🇰🇷 Korean Document");
    println!("{}", "=".repeat(60));
    let ko_chunks = chunker.chunk(korean);
    print_language_stats("Korean", &ko_chunks);

    // Mixed content
    let mixed = format!("{}\n\n{}\n\n{}", japanese, chinese, korean);
    println!("\n🌍 Mixed Multilingual Document");
    println!("{}", "=".repeat(60));
    let mixed_chunks = chunker.chunk(&mixed);
    print_language_stats("Mixed", &mixed_chunks);

    // Detailed view of Japanese chunks
    println!("\n=== Detailed Japanese Chunks ===\n");
    for (i, chunk) in ja_chunks.iter().enumerate() {
        println!("Chunk #{}", i + 1);
        println!("  Type: {:?}", chunk.metadata.chunk_type);
        println!("  Tokens: {} (CJK: ~{} chars)",
            chunk.metadata.token_count,
            chunk.metadata.token_count * 2);
        println!("  Actual chars: {}", chunk.metadata.char_count);

        if !chunk.metadata.header_hierarchy.is_empty() {
            println!("  Headers: {:?}", chunk.metadata.header_hierarchy);
        }

        // Show first line
        if let Some(first_line) = chunk.content.lines().next() {
            println!("  Preview: {}", first_line);
        }

        println!();
    }
}

fn print_language_stats(language: &str, chunks: &[markdown_chunker::Chunk]) {
    let total_tokens: usize = chunks.iter().map(|c| c.metadata.token_count).sum();
    let total_chars: usize = chunks.iter().map(|c| c.metadata.char_count).sum();
    let avg_tokens = if !chunks.is_empty() {
        total_tokens / chunks.len()
    } else {
        0
    };

    println!("Language: {}", language);
    println!("  Chunks: {}", chunks.len());
    println!("  Total tokens: {}", total_tokens);
    println!("  Total characters: {}", total_chars);
    println!("  Avg tokens/chunk: {}", avg_tokens);
    println!("  Chars/token ratio: {:.2}",
        if total_tokens > 0 { total_chars as f64 / total_tokens as f64 } else { 0.0 });
}
