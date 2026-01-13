// build.rs - rpath configuration for runtime library discovery
// This eliminates the need for `source setup_env.sh` before running binaries

fn main() {
    // macOS: Use @rpath with absolute paths to PyTorch libraries
    #[cfg(target_os = "macos")]
    {
        // PyTorch libraries from Homebrew Python installation
        println!(
            "cargo:rustc-link-arg=-Wl,-rpath,/opt/homebrew/lib/python3.14/site-packages/torch/lib"
        );
        // LLVM libraries (for OpenMP if needed)
        println!("cargo:rustc-link-arg=-Wl,-rpath,/opt/homebrew/opt/llvm/lib");
        // Repo root for pdfium (relative to binary)
        println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path/../../../");
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
