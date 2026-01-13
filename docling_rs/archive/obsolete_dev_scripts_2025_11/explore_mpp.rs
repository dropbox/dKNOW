#!/usr/bin/env rust-script
//! ```cargo
//! [dependencies]
//! cfb = "0.8"
//! ```

use cfb::CompoundFile;
use std::env;
use std::fs::File;
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <file.mpp>", args[0]);
        std::process::exit(1);
    }

    let path = &args[1];
    println!("Exploring: {}", path);

    let file = File::open(path)?;
    let mut comp = CompoundFile::open(file)?;

    println!("\n=== OLE Streams ===");
    let entries = comp.walk();
    for entry in entries {
        let path = entry.path();
        let name = path.to_string_lossy();
        let size = entry.len();
        println!("  {} ({} bytes)", name, size);
    }

    println!("\n=== Root Entries ===");
    let root = comp.root_entry();
    for entry in root.child_entries() {
        let name = entry.name();
        let size = entry.len();
        println!("  {} ({} bytes) - {:?}", name, size, entry.obj_type());
    }

    // Try to read common streams
    println!("\n=== Trying Common Streams ===");

    let streams_to_try = vec![
        "\\005SummaryInformation",
        "\\005DocumentSummaryInformation",
        "Props",
        "VarData",
        "FixDat",
        "FixFix",
        "1Table",
        "0Table",
        "WordDocument",
        "Project",
    ];

    for stream_name in streams_to_try {
        match comp.open_stream(stream_name) {
            Ok(mut stream) => {
                let mut buf = vec![0u8; 256];
                let n = stream.read(&mut buf)?;
                println!("\n  {} (read {} bytes):", stream_name, n);
                println!("    {:02x?}", &buf[..n.min(64)]);

                // Try to find printable text
                let text: String = buf[..n]
                    .iter()
                    .filter(|&&b| b >= 32 && b <= 126)
                    .map(|&b| b as char)
                    .collect();
                if !text.is_empty() && text.len() > 3 {
                    println!("    ASCII: {}", text);
                }
            }
            Err(_) => {
                // Stream doesn't exist, skip
            }
        }
    }

    Ok(())
}
