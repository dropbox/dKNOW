#[test]
#[ignore = "test-corpus/pptx directory does not exist - test file never created"]
fn test_debug_pptx_output() {
    use docling_backend::{DocumentBackend, PptxBackend};

    let backend = PptxBackend;
    let test_file = "../../test-corpus/pptx/powerpoint_sample.pptx";
    let result = backend
        .parse_file(test_file, &Default::default())
        .expect("Failed to parse PPTX");

    println!(
        "\n=== RUST OUTPUT ===\n{}\n=== END RUST OUTPUT ===\n",
        result.markdown
    );
}
