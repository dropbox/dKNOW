// build.rs - rpath configuration for runtime library discovery
// This eliminates the need for `source setup_env.sh` before running binaries

fn main() {
    // macOS: Use @rpath with absolute paths to libraries
    #[cfg(target_os = "macos")]
    {
        // PDFium libraries - repo root has symlink to pdfium_fast
        // Note: These are absolute paths; for portable builds, use @executable_path
        let home = std::env::var("HOME").unwrap_or_else(|_| "/Users/ayates".to_string());
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}/docling_rs", home);
        println!(
            "cargo:rustc-link-arg=-Wl,-rpath,{}/pdfium_fast/out/Release",
            home
        );
        // PyTorch C++ libraries (libtorch) - used for ML inference
        println!(
            "cargo:rustc-link-arg=-Wl,-rpath,/opt/homebrew/lib/python3.14/site-packages/torch/lib"
        );
        // Python 3.12 venv torch (more stable)
        println!(
            "cargo:rustc-link-arg=-Wl,-rpath,{}/docling_rs/.venv_tableformer/lib/python3.12/site-packages/torch/lib",
            home
        );
        // LLVM libraries (for OpenMP if needed)
        println!("cargo:rustc-link-arg=-Wl,-rpath,/opt/homebrew/opt/llvm/lib");
    }

    // Linux: Use $ORIGIN-relative paths for portable binaries
    #[cfg(target_os = "linux")]
    {
        // Relative to binary location
        println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN/../lib");
        // System library paths
        println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/local/lib");
    }
}
