/// Build script for video-audio-decoder crate
///
/// Links FFmpeg C libraries directly for zero-copy performance
/// Compiles C helper functions for stream group support
fn main() {
    // Compile C helper functions for stream group access (FFmpeg 6.1+)
    cc::Build::new()
        .file("src/stream_group_helpers.c")
        .include("/opt/homebrew/include") // FFmpeg headers on macOS
        .compile("stream_group_helpers");

    // Link FFmpeg C libraries (avcodec, avformat, avutil, swscale)
    // These are required for C FFI decoder with zero-copy memory buffers

    #[cfg(target_os = "macos")]
    {
        // macOS with Homebrew FFmpeg installation
        println!("cargo:rustc-link-search=native=/opt/homebrew/lib");
        println!("cargo:rustc-link-lib=dylib=avcodec");
        println!("cargo:rustc-link-lib=dylib=avformat");
        println!("cargo:rustc-link-lib=dylib=avutil");
        println!("cargo:rustc-link-lib=dylib=swscale");
    }

    #[cfg(target_os = "linux")]
    {
        // Linux with system FFmpeg installation
        println!("cargo:rustc-link-lib=dylib=avcodec");
        println!("cargo:rustc-link-lib=dylib=avformat");
        println!("cargo:rustc-link-lib=dylib=avutil");
        println!("cargo:rustc-link-lib=dylib=swscale");
    }

    // Rerun build script if FFmpeg libraries or C helpers change
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/stream_group_helpers.c");
}
