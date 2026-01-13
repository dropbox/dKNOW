# docling-examples

**Internal crate for organizing example code and usage patterns.**

This crate serves as a structural wrapper to organize the example code located in the `examples/` directory. It is not intended for publication to crates.io (`publish = false`).

## Purpose

- Provides a single crate reference for all docling-rs examples
- Organizes examples with proper Cargo.toml configuration
- Enables consistent example discovery and compilation

## Examples

All examples are located in the `examples/` directory at the repository root. See [examples/README.md](../../examples/README.md) for comprehensive documentation.

### Quick Start

Run any example using:

```bash
cargo run --example basic_conversion -- path/to/document.pdf
```

### Available Examples

1. **basic_conversion** - Simple document conversion
2. **ocr_processing** - OCR text extraction
3. **batch_processing** - Parallel batch processing
4. **error_handling** - Robust error handling patterns
5. **metadata_extraction** - Document metadata extraction
6. **custom_serialization** - Markdown customization
7. **format_detection** - Multi-format handling
8. **streaming_api** - Progress tracking and streaming
9. **performance_bench** - Performance measurement
10. **cli_tool** - Complete CLI application

See [examples/README.md](../../examples/README.md) for detailed documentation, usage examples, and common patterns.

## Architecture

This crate defines example binaries via `[[example]]` sections in Cargo.toml:

```toml
[[example]]
name = "basic_conversion"
path = "../../examples/basic_conversion.rs"
```

This approach allows examples to:
- Reference docling-core and other crates cleanly
- Be compiled and run via standard `cargo run --example` commands
- Be tested via `cargo test --examples`
- Be documented via `cargo doc --examples`

## Not Published

This crate is marked `publish = false` because:
- It contains no library code, only example organization
- Examples are included in workspace documentation
- Users access examples via the repository, not crates.io
- The examples require the full repository context

## Related Crates

- **docling-core** - Main library demonstrated by these examples
- **docling-cli** - Production CLI tool (not an example)

## License

MIT License - See LICENSE file for details
