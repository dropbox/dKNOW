use markdown_chunker::Chunker;

fn main() {
    let markdown = r#"
# Introduction to Rust

Rust is a systems programming language that focuses on safety, speed, and concurrency.

## Memory Safety

Rust's ownership system ensures memory safety without needing a garbage collector.
This makes it ideal for systems programming where performance is critical.

## Concurrency

Rust's type system helps prevent data races at compile time, making concurrent
programming safer and more reliable.

```rust
use std::thread;

fn main() {
    let handle = thread::spawn(|| {
        println!("Hello from a thread!");
    });

    handle.join().unwrap();
}
```

## Performance

Rust provides zero-cost abstractions, meaning you don't pay for features you don't use.
The compiled code runs as fast as equivalent C or C++ code.

## Conclusion

Rust combines the performance of low-level languages with the safety of high-level
languages, making it an excellent choice for modern systems programming.
"#;

    println!("Chunking markdown document...\n");

    let chunker = Chunker::default();
    let chunks = chunker.chunk(markdown);

    println!("Generated {} chunks:\n", chunks.len());
    println!("{}", "=".repeat(80));

    for (i, chunk) in chunks.iter().enumerate() {
        println!("\n📄 Chunk #{}", i + 1);
        println!("   Type: {:?}", chunk.metadata.chunk_type);
        println!("   Tokens: {}", chunk.metadata.token_count);
        println!("   Characters: {}", chunk.metadata.char_count);

        if !chunk.metadata.header_hierarchy.is_empty() {
            println!("   Headers:");
            for (level, title) in &chunk.metadata.header_hierarchy {
                println!("      {} {}", "#".repeat(*level), title);
            }
        }

        // Print first 100 chars of content
        let preview = if chunk.content.len() > 100 {
            format!("{}...", &chunk.content[..100])
        } else {
            chunk.content.clone()
        };
        println!("   Preview: {}", preview.replace('\n', " "));

        println!("{}", "-".repeat(80));
    }

    println!("\n✅ Chunking complete!");
}
