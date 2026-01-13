use std::env;
use std::path::PathBuf;

fn main() {
    // Get PDFium root directory
    let pdfium_root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();

    // Link against bridge library
    // Try Release first (standard build), then Profile, then fall back to Optimized-Shared
    let lib_dir = if pdfium_root
        .join("out/Release/libpdfium_render_bridge.dylib")
        .exists()
    {
        pdfium_root.join("out/Release")
    } else if pdfium_root
        .join("out/Profile/libpdfium_render_bridge.dylib")
        .exists()
    {
        pdfium_root.join("out/Profile")
    } else {
        pdfium_root.join("out/Optimized-Shared")
    };
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=dylib=pdfium_render_bridge");
    println!("cargo:rustc-link-lib=dylib=pdfium");

    // Add rpath so the binary can find the libraries at runtime (macOS/Linux)
    #[cfg(target_os = "macos")]
    println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib_dir.display());
    #[cfg(target_os = "linux")]
    println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib_dir.display());
}
