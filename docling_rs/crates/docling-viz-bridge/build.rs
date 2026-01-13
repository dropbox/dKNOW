//! Build script for docling-viz-bridge
//!
//! Generates C header file using cbindgen for Swift/Objective-C interop.

fn main() {
    // Re-run if lib.rs changes
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=cbindgen.toml");

    let crate_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let output_dir = std::path::Path::new(&crate_dir).join("include");

    // Create include directory if it doesn't exist
    std::fs::create_dir_all(&output_dir).expect("Failed to create include directory");

    let header_path = output_dir.join("docling_viz_bridge.h");

    // Generate bindings
    let result = cbindgen::Builder::new()
        .with_crate(&crate_dir)
        .with_config(
            cbindgen::Config::from_file("cbindgen.toml").expect("Failed to load cbindgen.toml"),
        )
        .generate();

    match result {
        Ok(bindings) => {
            bindings.write_to_file(&header_path);
            println!(
                "cargo:warning=Generated C header at {}",
                header_path.display()
            );
        }
        Err(cbindgen::Error::ParseSyntaxError { .. }) => {
            // Syntax error during parsing - report but don't fail build
            eprintln!("cbindgen: Syntax error while parsing, header not generated");
        }
        Err(e) => {
            // Other errors - report but continue
            eprintln!("cbindgen error: {e:?}");
        }
    }
}
