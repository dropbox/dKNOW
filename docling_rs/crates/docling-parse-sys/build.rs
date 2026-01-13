use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-changed=vendor/docling-parse/src/c_api/docling_parse_c.h");

    // Get the path to the compiled library
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    // Check if we're in workspace (../../vendor exists) or standalone (vendor/ exists)
    let vendor_path = if manifest_dir.join("../../vendor/docling-parse").exists() {
        manifest_dir.join("../../vendor/docling-parse")
    } else {
        manifest_dir.join("vendor/docling-parse")
    };

    let lib_path = vendor_path.join("build");

    // Check if the C++ library exists before trying to link
    // If it doesn't exist, skip building this crate (it's optional/experimental)
    let lib_exists = if cfg!(target_os = "macos") {
        lib_path.join("libdocling_parse_c.dylib").exists()
    } else if cfg!(target_os = "windows") {
        lib_path.join("docling_parse_c.dll").exists()
    } else {
        lib_path.join("libdocling_parse_c.so").exists()
    };

    if !lib_exists {
        // Library not built - create a stub bindings file and skip linking
        // This allows the crate to compile without the C++ library
        eprintln!("Warning: docling-parse C++ library not found at {:?}", lib_path);
        eprintln!("Skipping docling-parse-sys build (this is optional)");
        eprintln!("To build the C++ library, see: crates/docling-parse-sys/README.md");

        // Create stub bindings file
        let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
        std::fs::write(
            out_path.join("bindings.rs"),
            "// Stub bindings - C++ library not available\n"
        ).expect("Failed to write stub bindings");
        return;
    }

    // Tell cargo to link the library
    println!("cargo:rustc-link-search=native={}", lib_path.display());
    println!("cargo:rustc-link-lib=dylib=docling_parse_c");

    // On macOS, set rpath so the dylib can be found at runtime
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib_path.display());
    }

    // Generate bindings
    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        // Tell bindgen where to find the header (in crate directory)
        .clang_arg(format!("-I{}", manifest_dir.display()))
        // Generate bindings
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        // Don't generate docs (they're in the C header)
        .generate_comments(false)
        .generate()
        .expect("Unable to generate bindings");

    // Write the bindings to $OUT_DIR/bindings.rs
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
