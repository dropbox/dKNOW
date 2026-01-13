// Build script for docling-backend
//
// This adds the necessary rpath for pdfium_fast to find libpdfium.dylib at runtime.

fn main() {
    // When pdf feature is enabled, add rpath for runtime library loading
    #[cfg(feature = "pdf")]
    {
        // Get home directory
        if let Ok(home) = std::env::var("HOME") {
            let lib_path = format!("{home}/pdfium_fast/out/Release");

            // Check if the pdfium library exists
            let pdfium_lib = format!("{lib_path}/libpdfium.dylib");
            if std::path::Path::new(&pdfium_lib).exists() {
                // Add rpath for macOS/Linux
                println!("cargo:rustc-link-arg=-Wl,-rpath,{lib_path}");
                println!("cargo:rerun-if-changed={pdfium_lib}");
            } else {
                println!(
                    "cargo:warning=pdfium_fast library not found at {lib_path}. \
                     Build pdfium_fast first or set DYLD_LIBRARY_PATH at runtime."
                );
            }
        }
    }
}
