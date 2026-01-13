# docling-pipeline

Document processing pipeline orchestration for docling-rs.

## Overview

`docling-pipeline` provides pipeline orchestration and workflow management for document processing. This crate defines the structure and execution flow for processing documents through multiple stages.

## Status

**Current Status:** Placeholder

This crate is reserved for future implementation of pipeline orchestration features. Currently, pipeline functionality in docling-rs is handled within docling-backend and docling-core.

## Future Roadmap

When pipeline orchestration is implemented, this crate will provide:

### Planned Features

- **Pipeline Definition:** Define multi-stage processing pipelines
- **Stage Management:** Chain processing stages together
- **Error Handling:** Graceful error recovery and retry logic
- **Parallel Processing:** Process multiple documents concurrently
- **Progress Tracking:** Monitor pipeline execution progress
- **Caching:** Cache intermediate results for efficiency
- **Streaming:** Process large documents in chunks
- **Plugin System:** Add custom processing stages

### Example (Future API)

```rust
use docling_pipeline::{Pipeline, Stage};

// Define a processing pipeline
let pipeline = Pipeline::builder()
    .add_stage(Stage::Extract)       // Extract raw content
    .add_stage(Stage::Segment)       // Segment into blocks
    .add_stage(Stage::Classify)      // Classify content types
    .add_stage(Stage::Serialize)     // Serialize to output format
    .with_parallelism(4)             // Process 4 docs in parallel
    .with_caching(true)              // Enable result caching
    .build()?;

// Process a document through the pipeline
let result = pipeline.process("document.pdf")?;
```

## Potential Architecture

### Pipeline Stages

A document processing pipeline might consist of:

1. **Input Stage:** Read and validate input file
2. **Detection Stage:** Detect document format
3. **Parse Stage:** Parse document structure
4. **OCR Stage:** Extract text from images (if needed)
5. **Segmentation Stage:** Identify document blocks
6. **Classification Stage:** Classify content types
7. **Extraction Stage:** Extract tables, images, metadata
8. **Serialization Stage:** Convert to output format
9. **Output Stage:** Write results to destination

### Pipeline Configuration

```rust
struct PipelineConfig {
    // Parallelism
    max_concurrent: usize,

    // Caching
    enable_cache: bool,
    cache_dir: PathBuf,

    // Error handling
    retry_count: u32,
    fail_fast: bool,

    // Progress tracking
    progress_callback: Option<Box<dyn Fn(Progress)>>,

    // Stage configuration
    stages: Vec<StageConfig>,
}
```

### Stage Interface

```rust
trait ProcessingStage {
    fn name(&self) -> &str;
    fn process(&self, input: StageInput) -> Result<StageOutput>;
    fn can_skip(&self, input: &StageInput) -> bool;
}
```

## Current Usage

For document processing today, use the high-level API in docling-backend:

```rust
use docling_backend::DocumentConverter;  // Note: DocumentConverter is in docling-backend crate

// Simple conversion (internally uses a default pipeline)
let converter = DocumentConverter::new(Default::default())?;
let result = converter.convert("document.pdf")?;
```

## Use Cases

### Batch Processing

Process multiple documents efficiently:

```rust
// Future API
let pipeline = Pipeline::builder()
    .with_parallelism(8)  // Process 8 documents at once
    .build()?;

for doc in documents {
    pipeline.submit(doc)?;
}

let results = pipeline.wait_all()?;
```

### Custom Stages

Add custom processing stages:

```rust
// Future API
struct CustomAnalysisStage {
    // Custom configuration
}

impl ProcessingStage for CustomAnalysisStage {
    fn process(&self, input: StageInput) -> Result<StageOutput> {
        // Custom processing logic
        Ok(output)
    }
}

let pipeline = Pipeline::builder()
    .add_stage(Stage::Parse)
    .add_custom(CustomAnalysisStage::new())
    .add_stage(Stage::Serialize)
    .build()?;
```

### Streaming Processing

Process large documents in chunks:

```rust
// Future API
let pipeline = Pipeline::builder()
    .with_streaming(true)
    .with_chunk_size(10 * 1024 * 1024)  // 10MB chunks
    .build()?;

let stream = pipeline.process_stream("large_document.pdf")?;
for chunk in stream {
    handle_chunk(chunk)?;
}
```

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
docling-pipeline = "2.58.0"
```

## Related Crates

- **docling-core:** High-level document processing API (includes pipeline logic)
- **docling-backend:** Format-specific backends (pipeline stages)
- **docling-models:** ML models (used in analysis stages)
- **docling-py:** Python bridge (may be used in hybrid pipelines)

## Benefits of Pipeline Orchestration

### Performance

- **Parallelism:** Process multiple documents concurrently
- **Caching:** Avoid reprocessing unchanged documents
- **Streaming:** Handle documents larger than memory

### Reliability

- **Error Recovery:** Retry failed stages
- **Checkpointing:** Resume interrupted processing
- **Validation:** Verify output at each stage

### Flexibility

- **Custom Stages:** Add domain-specific processing
- **Conditional Execution:** Skip stages based on input
- **Multiple Outputs:** Generate different output formats

### Observability

- **Progress Tracking:** Monitor pipeline execution
- **Metrics:** Collect performance metrics
- **Logging:** Detailed execution logs

## Integration with docling-core

This crate is designed to integrate with docling-backend:

```rust
use docling_backend::DocumentConverter;  // Note: DocumentConverter is in docling-backend crate
use docling_pipeline::Pipeline;

// Future API: Use custom pipeline with DocumentConverter
let pipeline = Pipeline::custom()
    .add_stage(Stage::Parse)
    .add_stage(Stage::OCR)
    .build()?;

let converter = DocumentConverter::with_pipeline(pipeline)?;
let result = converter.convert("document.pdf")?;
```

## License

Licensed under the MIT License. See LICENSE file for details.

## Contributing

This crate is part of the docling-rs project. For contribution guidelines, see the main repository.

## References

- **Python docling:** https://github.com/docling-project/docling
- **docling-rs repository:** https://github.com/ayates_dbx/docling_rs

## Note on Pipeline Implementation

The docling-rs project uses a **simple sequential approach** currently:

**Current (Phase H):**
- Documents processed sequentially by DocumentConverter
- Each backend handles its own processing flow
- No explicit pipeline orchestration

**Future (Phase I+):**
- Explicit pipeline definition and orchestration
- Parallel document processing
- Custom stage support
- Progress tracking and metrics

This crate will be populated during Phase I or later when advanced pipeline features are needed.

## Comparison to Other Pipeline Frameworks

Similar concepts exist in other data processing frameworks:

| Framework | Language | Focus |
|-----------|----------|-------|
| docling-pipeline | Rust | Document processing |
| Apache Beam | Java/Python | Big data pipelines |
| Luigi | Python | Batch workflow |
| Airflow | Python | Workflow orchestration |
| Prefect | Python | Data workflow |

docling-pipeline will be specialized for document processing with features like:
- Document-specific stages (OCR, table extraction)
- Integration with docling backends
- Optimized for file-based workflows
