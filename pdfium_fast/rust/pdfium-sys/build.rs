use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const PDFIUM_VERSION: &str = "2.1.0";

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=PDFIUM_LIB_DIR");

    let target = env::var("TARGET").unwrap();
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // Priority 1: Manual override via PDFIUM_LIB_DIR
    if let Ok(lib_dir) = env::var("PDFIUM_LIB_DIR") {
        println!("cargo:warning=Using PDFIUM_LIB_DIR: {}", lib_dir);
        setup_linking(&PathBuf::from(&lib_dir), &target);
        generate_bindings_with_lib_dir(&PathBuf::from(&lib_dir));
        return;
    }

    // Priority 2: Check for out/Release in repo (development mode)
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let pdfium_root = manifest_dir.parent().unwrap().parent().unwrap();
    let release_dir = pdfium_root.join("out/Release");

    if release_dir.join("libpdfium.dylib").exists() || release_dir.join("libpdfium.so").exists() {
        println!("cargo:warning=Using development build: {}", release_dir.display());
        setup_linking(&release_dir, &target);
        generate_bindings_from_repo(pdfium_root);
        return;
    }

    // Priority 3: Check bundled releases/ directory
    let platform = get_platform(&target);
    let bundled_dir = pdfium_root.join(format!("releases/v{}/{}", PDFIUM_VERSION, platform));

    if bundled_dir.exists() {
        println!("cargo:warning=Using bundled release: {}", bundled_dir.display());
        setup_linking(&bundled_dir, &target);
        generate_bindings_from_repo(pdfium_root);
        return;
    }

    // Priority 4: Download from GitHub releases
    println!("cargo:warning=Downloading pdfium binaries for {}", platform);
    let download_dir = out_dir.join("pdfium");
    fs::create_dir_all(&download_dir).expect("Failed to create download directory");

    download_release(&platform, &download_dir);
    setup_linking(&download_dir, &target);

    // For downloaded binaries, we need headers too - download or use bundled
    let headers_dir = download_dir.join("include");
    if !headers_dir.exists() {
        // Generate minimal bindings or panic with helpful message
        panic!(
            "pdfium headers not found. Either:\n\
             1. Set PDFIUM_LIB_DIR to a directory with headers\n\
             2. Clone the full pdfium_fast repo\n\
             3. Use a pre-built release with headers included"
        );
    }
    generate_bindings_with_headers(&headers_dir);
}

fn get_platform(target: &str) -> &'static str {
    match target {
        t if t.contains("aarch64-apple") => "macos-arm64",
        t if t.contains("x86_64-apple") => "macos-x86_64",
        t if t.contains("x86_64-unknown-linux") => "linux-x86_64",
        t if t.contains("aarch64-unknown-linux") => "linux-arm64",
        t if t.contains("x86_64-pc-windows") => "windows-x86_64",
        _ => panic!("Unsupported platform: {}. Supported: macos-arm64, macos-x86_64, linux-x86_64, linux-arm64, windows-x86_64", target),
    }
}

fn setup_linking(lib_dir: &PathBuf, target: &str) {
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=dylib=pdfium");

    // Bridge library is optional (only needed for form callbacks)
    let bridge_name = if target.contains("windows") {
        "pdfium_render_bridge.dll"
    } else if target.contains("apple") {
        "libpdfium_render_bridge.dylib"
    } else {
        "libpdfium_render_bridge.so"
    };

    if lib_dir.join(bridge_name).exists() {
        println!("cargo:rustc-link-lib=dylib=pdfium_render_bridge");
    }

    // Add rpath for runtime library discovery (macOS/Linux)
    if cfg!(target_os = "macos") || cfg!(target_os = "linux") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib_dir.display());
    }
}

fn download_release(platform: &str, dest_dir: &PathBuf) {
    let url = format!(
        "https://github.com/ayates_dbx/pdfium_fast/releases/download/v{}/pdfium-{}.tar.gz",
        PDFIUM_VERSION, platform
    );

    println!("cargo:warning=Downloading from: {}", url);

    // Use curl for downloading (available on all platforms)
    let tar_path = dest_dir.join("pdfium.tar.gz");
    let status = Command::new("curl")
        .args(["-L", "-o", tar_path.to_str().unwrap(), &url])
        .status()
        .expect("Failed to run curl. Install curl or set PDFIUM_LIB_DIR");

    if !status.success() {
        panic!(
            "Failed to download pdfium from {}. \n\
             Either:\n\
             1. Set PDFIUM_LIB_DIR to point to pre-built libraries\n\
             2. Build from source: ninja -C out/Release pdfium\n\
             3. Check network connectivity",
            url
        );
    }

    // Extract tarball
    let status = Command::new("tar")
        .args(["-xzf", tar_path.to_str().unwrap(), "-C", dest_dir.to_str().unwrap()])
        .status()
        .expect("Failed to run tar");

    if !status.success() {
        panic!("Failed to extract pdfium tarball");
    }

    // Clean up tarball
    fs::remove_file(&tar_path).ok();
}

fn generate_bindings_from_repo(pdfium_root: &std::path::Path) {
    println!("cargo:rerun-if-changed={}/public", pdfium_root.display());

    let bindings = bindgen::Builder::default()
        // Main PDFium headers
        .header(pdfium_root.join("public/fpdfview.h").to_str().unwrap())
        .header(pdfium_root.join("public/fpdf_text.h").to_str().unwrap())
        .header(pdfium_root.join("public/fpdf_edit.h").to_str().unwrap())
        .header(pdfium_root.join("public/fpdf_formfill.h").to_str().unwrap())
        .header(pdfium_root.join("public/fpdf_progressive.h").to_str().unwrap())
        // Parallel rendering API (pdfium_fast extension)
        .header(pdfium_root.join("public/fpdf_parallel.h").to_str().unwrap())
        // Document metadata, page labels, bookmarks
        .header(pdfium_root.join("public/fpdf_doc.h").to_str().unwrap())
        // Tagged PDF structure tree
        .header(pdfium_root.join("public/fpdf_structtree.h").to_str().unwrap())
        // Annotations
        .header(pdfium_root.join("public/fpdf_annot.h").to_str().unwrap())
        // Thumbnail extraction
        .header(pdfium_root.join("public/fpdf_thumbnail.h").to_str().unwrap())
        // Embedded file attachments
        .header(pdfium_root.join("public/fpdf_attachment.h").to_str().unwrap())
        // Digital signatures
        .header(pdfium_root.join("public/fpdf_signature.h").to_str().unwrap())
        // JavaScript action detection
        .header(pdfium_root.join("public/fpdf_javascript.h").to_str().unwrap())
        // Extended search APIs
        .header(pdfium_root.join("public/fpdf_searchex.h").to_str().unwrap())
        // Batch text extraction API
        .header(pdfium_root.join("public/fpdf_text_batch.h").to_str().unwrap())
        // Catalog API
        .header(pdfium_root.join("public/fpdf_catalog.h").to_str().unwrap())
        // Page transformation API
        .header(pdfium_root.join("public/fpdf_transformpage.h").to_str().unwrap())
        // Document save API
        .header(pdfium_root.join("public/fpdf_save.h").to_str().unwrap())
        // Flatten API
        .header(pdfium_root.join("public/fpdf_flatten.h").to_str().unwrap())
        // Page operations API
        .header(pdfium_root.join("public/fpdf_ppo.h").to_str().unwrap())
        // Extension API
        .header(pdfium_root.join("public/fpdf_ext.h").to_str().unwrap())
        // Data Availability API
        .header(pdfium_root.join("public/fpdf_dataavail.h").to_str().unwrap())
        // System Font Info API
        .header(pdfium_root.join("public/fpdf_sysfontinfo.h").to_str().unwrap())
        // Include paths
        .clang_arg(format!("-I{}", pdfium_root.display()))
        .clang_arg(format!("-I{}/public", pdfium_root.display()))
        // Options
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .allowlist_function("FPDF.*")
        .allowlist_function("FORM_.*")
        .allowlist_function("FPDFDOC_.*")
        .allowlist_function("FPDFBookmark_.*")
        .allowlist_function("FPDFLink_.*")
        .allowlist_function("FPDFAction_.*")
        .allowlist_function("FPDFDest_.*")
        .allowlist_function("FPDFDoc_GetAttachment.*")
        .allowlist_function("FPDFAttachment_.*")
        .allowlist_function("FPDFAnnot_.*")
        .allowlist_function("FPDFPage_.*Annot.*")
        .allowlist_function("FPDFSignatureObj_.*")
        .allowlist_function("FPDFDoc_GetJavaScriptActionCount")
        .allowlist_function("FPDFDoc_GetJavaScriptAction")
        .allowlist_function("FPDFDoc_CloseJavaScriptAction")
        .allowlist_function("FPDFJavaScriptAction_.*")
        .allowlist_type("FPDF.*")
        .allowlist_type("FORM.*")
        .allowlist_type("IFSDK_PAUSE")
        .allowlist_type("FPDFANNOT_.*")
        .allowlist_var("FPDF.*")
        .allowlist_var("FORM.*")
        .allowlist_var("FPDF_ANNOT_.*")
        .allowlist_var("FPDF_FORMFLAG_.*")
        .allowlist_var("PDFACTION_.*")
        .allowlist_var("PDFDEST_.*")
        .allowlist_var("FPDF_INCREMENTAL")
        .allowlist_var("FPDF_NO_INCREMENTAL")
        .allowlist_var("FPDF_REMOVE_SECURITY")
        .allowlist_var("FLATTEN_.*")
        .allowlist_var("FLAT_.*")
        .allowlist_var("PAGEMODE_.*")
        .allowlist_var("FPDF_UNSP_.*")
        .allowlist_var("PDF_LINEARIZATION_.*")
        .allowlist_var("PDF_NOT_LINEARIZED")
        .allowlist_var("PDF_LINEARIZED")
        .allowlist_var("PDF_DATA_.*")
        .allowlist_var("PDF_FORM_.*")
        .allowlist_type("FX_FILEAVAIL")
        .allowlist_type("FX_DOWNLOADHINTS")
        .allowlist_type("FPDF_AVAIL")
        .allowlist_function("FPDFAvail_.*")
        .allowlist_var("FXFONT_.*")
        .allowlist_type("FPDF_SYSFONTINFO")
        .allowlist_type("FPDF_CharsetFontMap")
        .allowlist_function("FPDF_GetDefaultTTFMap.*")
        .allowlist_function("FPDF_AddInstalledFont")
        .allowlist_function("FPDF_SetSystemFontInfo")
        .allowlist_function("FPDF_GetDefaultSystemFontInfo")
        .allowlist_function("FPDF_FreeDefaultSystemFontInfo")
        .allowlist_function("FPDF_SaveAsCopy")
        .allowlist_function("FPDF_SaveWithVersion")
        .allowlist_function("FPDFPage_Flatten")
        .allowlist_function("FPDF_ImportPages")
        .allowlist_function("FPDF_ImportPagesByIndex")
        .allowlist_function("FPDF_ImportNPagesToOne")
        .allowlist_function("FPDF_CopyViewerPreferences")
        .allowlist_function("FPDF_NewXObjectFromPage")
        .allowlist_function("FPDF_CloseXObject")
        .allowlist_function("FPDF_NewFormObjectFromXObject")
        .allowlist_function("FPDFDoc_GetPageMode")
        .allowlist_function("FSDK_SetUnSpObjProcessHandler")
        .allowlist_function("FSDK_SetTimeFunction")
        .allowlist_function("FSDK_SetLocaltimeFunction")
        .allowlist_type("UNSUPPORT_INFO")
        .opaque_type("fpdf_.*")
        .opaque_type("FPDF_DOCUMENT__")
        .opaque_type("FPDF_PAGE__")
        .opaque_type("FPDF_TEXTPAGE__")
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    println!("cargo:warning=Generated PDFium bindings from repo");
}

fn generate_bindings_with_lib_dir(lib_dir: &PathBuf) {
    // Look for headers in lib_dir/include or lib_dir/../include
    let include_dir = if lib_dir.join("include").exists() {
        lib_dir.join("include")
    } else if lib_dir.parent().map(|p| p.join("include").exists()).unwrap_or(false) {
        lib_dir.parent().unwrap().join("include")
    } else {
        // Try parent's public directory (common repo structure)
        let pdfium_root = lib_dir.parent().unwrap().parent().unwrap();
        if pdfium_root.join("public").exists() {
            generate_bindings_from_repo(pdfium_root);
            return;
        }
        panic!("Cannot find PDFium headers. Expected in {}/include or {}/public", lib_dir.display(), pdfium_root.display());
    };

    generate_bindings_with_headers(&include_dir);
}

fn generate_bindings_with_headers(headers_dir: &PathBuf) {
    let bindings = bindgen::Builder::default()
        .header(headers_dir.join("fpdfview.h").to_str().unwrap())
        .header(headers_dir.join("fpdf_text.h").to_str().unwrap())
        .clang_arg(format!("-I{}", headers_dir.display()))
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .allowlist_function("FPDF.*")
        .allowlist_function("FORM_.*")
        .allowlist_type("FPDF.*")
        .allowlist_var("FPDF.*")
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    println!("cargo:warning=Generated minimal PDFium bindings from headers");
}
