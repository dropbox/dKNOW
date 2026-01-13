//! Pipeline executors for different execution modes

use crate::cache::PipelineCache;
use crate::error::PluginError;
use crate::plugin::{PluginData, PluginRequest};
use crate::{Context, Pipeline, PipelineStage};
use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

/// Result from pipeline execution
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// Final output data
    pub output: PluginData,

    /// Intermediate results from each stage (if saved)
    pub intermediates: Vec<StageResult>,

    /// Total execution time
    pub total_duration: std::time::Duration,

    /// Any warnings collected during execution
    pub warnings: Vec<String>,
}

/// Result from a single pipeline stage
#[derive(Debug, Clone)]
pub struct StageResult {
    /// Stage index
    pub stage_index: usize,

    /// Plugin name
    pub plugin_name: String,

    /// Operation performed
    pub operation_name: Cow<'static, str>,

    /// Output from this stage
    pub output: PluginData,

    /// Duration of this stage
    pub duration: std::time::Duration,

    /// Warnings from this stage
    pub warnings: Vec<String>,
}

/// Debug mode executor - verbose logging, intermediate outputs
pub struct DebugExecutor {
    /// Execution context
    context: Context,

    /// Where to save intermediate outputs (if any)
    output_dir: Option<PathBuf>,

    /// Cache for intermediate results (eliminates duplicate work)
    cache: Option<PipelineCache>,

    /// Timeout for stage execution (None = no timeout)
    /// Default: 5 minutes for each stage
    stage_timeout: Option<Duration>,
}

impl DebugExecutor {
    /// Create a new debug executor
    pub fn new() -> Self {
        Self {
            context: Context::debug(),
            output_dir: None,
            cache: None,
            stage_timeout: Some(Duration::from_secs(300)), // 5 minutes default
        }
    }

    /// Enable result caching (eliminates duplicate work within pipeline)
    pub fn with_cache(mut self) -> Self {
        self.cache = Some(PipelineCache::new());
        self
    }

    /// Enable result caching with a memory limit (bytes)
    pub fn with_cache_limit(mut self, max_memory_bytes: usize) -> Self {
        self.cache = Some(PipelineCache::with_memory_limit(max_memory_bytes));
        self
    }

    /// Set output directory for intermediate results
    pub fn with_output_dir(mut self, dir: PathBuf) -> Self {
        self.output_dir = Some(dir);
        self
    }

    /// Set timeout for each stage execution
    /// Use `None` to disable timeout (not recommended for production)
    pub fn with_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.stage_timeout = timeout;
        self
    }

    /// Execute a pipeline with debug logging
    pub async fn execute(
        &self,
        pipeline: &Pipeline,
        initial_input: PluginData,
    ) -> Result<ExecutionResult, PluginError> {
        info!("=== Debug Executor Started ===");
        info!("Pipeline: {} stages", pipeline.stages.len());

        let start_time = Instant::now();
        let mut current_input = initial_input;
        let mut intermediates = Vec::with_capacity(pipeline.stages.len());
        let mut all_warnings = Vec::with_capacity(pipeline.stages.len());

        // Execute each stage sequentially
        for (idx, stage) in pipeline.stages.iter().enumerate() {
            info!(
                "--- Stage {}/{}: {} ---",
                idx + 1,
                pipeline.stages.len(),
                stage.plugin.name()
            );
            info!("  Input type: {}", stage.input_type);
            info!("  Output type: {}", stage.output_type);
            info!("  Operation: {:?}", stage.operation);

            // Check cache first (if enabled)
            let mut stage_result = if let Some(ref cache) = self.cache {
                if let Some(cached_output) =
                    cache.get(stage.plugin.name(), stage.operation.name(), &current_input)
                {
                    info!("  ✅ Cache hit - skipping execution");
                    // Create stage result from cached output
                    StageResult {
                        stage_index: idx,
                        plugin_name: stage.plugin.name().to_string(),
                        operation_name: Cow::Borrowed(stage.operation.name()),
                        output: cached_output,
                        duration: std::time::Duration::ZERO,
                        warnings: Vec::new(),
                    }
                } else {
                    // Cache miss - execute stage
                    let result = self
                        .execute_stage(idx, stage, current_input.clone())
                        .await?;

                    // Save to cache
                    cache.put(
                        stage.plugin.name(),
                        stage.operation.name(),
                        &current_input,
                        &result.output,
                    );

                    result
                }
            } else {
                // No cache - execute normally
                self.execute_stage(idx, stage, current_input.clone())
                    .await?
            };

            // Log stage completion
            if stage_result.duration.as_secs_f64() > 0.0 {
                info!(
                    "  ✓ Completed in {:.2}s",
                    stage_result.duration.as_secs_f64()
                );
            }

            // Save warnings
            if !stage_result.warnings.is_empty() {
                // Log warnings first
                for warning in &stage_result.warnings {
                    warn!("  Warning: {}", warning);
                }
                // Take ownership of warnings Vec (avoids per-element clones)
                all_warnings.extend(std::mem::take(&mut stage_result.warnings));
            }

            // Save intermediate if debug mode
            if self.context.save_intermediates {
                if let Some(ref output_dir) = self.output_dir {
                    self.save_intermediate(output_dir, idx, stage, &stage_result)
                        .await?;
                }
            }

            // Update input for next stage (swap to avoid clone)
            current_input =
                std::mem::replace(&mut stage_result.output, PluginData::Bytes(Vec::new()));
            intermediates.push(stage_result);
        }

        let total_duration = start_time.elapsed();
        info!("=== Debug Executor Completed ===");
        info!("Total time: {:.2}s", total_duration.as_secs_f64());
        info!("Stages: {}", intermediates.len());

        Ok(ExecutionResult {
            output: current_input,
            intermediates,
            total_duration,
            warnings: all_warnings,
        })
    }

    /// Execute a single stage
    async fn execute_stage(
        &self,
        stage_index: usize,
        stage: &PipelineStage,
        input: PluginData,
    ) -> Result<StageResult, PluginError> {
        debug!("Executing stage {}: {}", stage_index, stage.plugin.name());

        let request = PluginRequest {
            operation: stage.operation.clone(),
            input,
        };

        let stage_start = Instant::now();

        // Execute with timeout if configured
        let response = if let Some(timeout_duration) = self.stage_timeout {
            match tokio::time::timeout(
                timeout_duration,
                stage.plugin.execute(&self.context, &request),
            )
            .await
            {
                Ok(result) => result?,
                Err(_) => {
                    return Err(PluginError::Timeout(format!(
                        "Stage '{}' timed out after {} seconds. \
                         This usually indicates a corrupted or malformed file. \
                         Try using ffprobe to validate the file first.",
                        stage.plugin.name(),
                        timeout_duration.as_secs()
                    )));
                }
            }
        } else {
            // No timeout - execute directly
            stage.plugin.execute(&self.context, &request).await?
        };

        let duration = stage_start.elapsed();

        Ok(StageResult {
            stage_index,
            plugin_name: stage.plugin.name().to_string(),
            operation_name: Cow::Borrowed(stage.operation.name()),
            output: response.output,
            duration,
            warnings: response.warnings,
        })
    }

    /// Save intermediate output to disk
    async fn save_intermediate(
        &self,
        output_dir: &PathBuf,
        stage_index: usize,
        stage: &PipelineStage,
        result: &StageResult,
    ) -> Result<(), PluginError> {
        // Create output directory if it doesn't exist
        tokio::fs::create_dir_all(output_dir).await?;

        let filename = format!(
            "stage_{:02}_{}.intermediate",
            stage_index,
            stage.plugin.name()
        );
        let output_path = output_dir.join(filename);

        match &result.output {
            PluginData::Bytes(bytes) => {
                tokio::fs::write(&output_path, bytes).await?;
                info!(
                    "  Saved intermediate: {} ({} bytes)",
                    output_path.display(),
                    bytes.len()
                );
            }
            PluginData::FilePath(path) => {
                // Copy file to output directory
                let dest_path = output_dir.join(format!(
                    "stage_{:02}_{}.{}",
                    stage_index,
                    stage.plugin.name(),
                    path.extension().unwrap_or_default().to_string_lossy()
                ));
                tokio::fs::copy(path, &dest_path).await?;
                info!(
                    "  Saved intermediate: {} (copied from {})",
                    dest_path.display(),
                    path.display()
                );
            }
            PluginData::Json(value) => {
                let json_str = serde_json::to_string_pretty(value)?;
                tokio::fs::write(&output_path.with_extension("json"), json_str).await?;
                info!("  Saved intermediate: {}.json", output_path.display());
            }
            PluginData::Multiple(items) => {
                info!(
                    "  Intermediate contains {} items (not saved individually)",
                    items.len()
                );
            }
        }

        Ok(())
    }
}

impl Default for DebugExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// Streaming result for performance mode
#[derive(Debug, Clone)]
pub enum StreamingResult {
    /// Partial result from a stage (e.g., progress update)
    Partial {
        stage_index: usize,
        plugin_name: String,
        data: crate::plugin::PartialResult,
    },

    /// Complete result from a stage
    Complete(StageResult),

    /// Final pipeline result
    Final(ExecutionResult),
}

/// Performance mode executor - streaming results, minimum latency
pub struct PerformanceExecutor {
    /// Execution context
    context: Context,

    /// Maximum number of parallel stages
    max_parallelism: usize,

    /// Cache for intermediate results (eliminates duplicate work)
    cache: Option<PipelineCache>,

    /// Timeout for stage execution (None = no timeout)
    /// Default: 5 minutes for each stage
    /// TODO: Implement timeout support in execute_streaming (currently unused)
    #[allow(dead_code)]
    stage_timeout: Option<Duration>,
}

impl PerformanceExecutor {
    /// Create a new performance executor
    pub fn new() -> Self {
        Self {
            context: Context::performance(),
            max_parallelism: num_cpus::get(),
            cache: None,
            stage_timeout: Some(Duration::from_secs(300)), // 5 minutes default
        }
    }

    /// Set maximum parallelism
    pub fn with_max_parallelism(mut self, max: usize) -> Self {
        self.max_parallelism = max;
        self
    }

    /// Enable result caching (eliminates duplicate work within pipeline)
    pub fn with_cache(mut self) -> Self {
        self.cache = Some(PipelineCache::new());
        self
    }

    /// Enable result caching with a memory limit (bytes)
    pub fn with_cache_limit(mut self, max_memory_bytes: usize) -> Self {
        self.cache = Some(PipelineCache::with_memory_limit(max_memory_bytes));
        self
    }

    /// Execute a pipeline with streaming results
    pub async fn execute_streaming(
        &self,
        pipeline: &Pipeline,
        initial_input: PluginData,
    ) -> Result<tokio::sync::mpsc::Receiver<StreamingResult>, PluginError> {
        let (tx, rx) = tokio::sync::mpsc::channel(100);

        // Clone data for async task
        let pipeline = pipeline.clone();
        let context = self.context.clone();
        let initial_input = initial_input.clone();
        let cache = self.cache.clone();

        // Spawn task to execute pipeline
        tokio::spawn(async move {
            let result = Self::execute_pipeline_internal(
                &context,
                &pipeline,
                initial_input,
                cache.as_ref(),
                tx.clone(),
            )
            .await;

            // Send final result or error
            match result {
                Ok(execution_result) => {
                    let _ = tx.send(StreamingResult::Final(execution_result)).await;
                }
                Err(e) => {
                    debug!("Pipeline execution failed: {:?}", e);
                }
            }
        });

        Ok(rx)
    }

    /// Internal pipeline execution with streaming
    async fn execute_pipeline_internal(
        context: &Context,
        pipeline: &Pipeline,
        initial_input: PluginData,
        cache: Option<&PipelineCache>,
        tx: tokio::sync::mpsc::Sender<StreamingResult>,
    ) -> Result<ExecutionResult, PluginError> {
        info!("=== Performance Executor Started ===");
        info!("Pipeline: {} stages", pipeline.stages.len());

        let start_time = Instant::now();
        let mut stage_results = Vec::with_capacity(pipeline.stages.len());
        let mut all_warnings = Vec::with_capacity(pipeline.stages.len());

        // Track outputs by type for dependency resolution
        // Key: output_type, Value: (stage_index, output_data)
        let mut output_map: std::collections::HashMap<String, (usize, PluginData)> =
            std::collections::HashMap::with_capacity(pipeline.stages.len());

        // Initial input type (e.g., "mp4" for video file)
        // Determine from first stage's input type
        if let Some(first_stage) = pipeline.stages.first() {
            output_map.insert(
                first_stage.input_type.clone(),
                (usize::MAX, initial_input.clone()),
            );
        }

        // Group stages by dependency level
        let stage_groups = Self::group_stages_by_dependency(pipeline);

        info!("Identified {} dependency groups", stage_groups.len());

        for (group_idx, group) in stage_groups.iter().enumerate() {
            info!(
                "Executing group {} with {} stages",
                group_idx + 1,
                group.len()
            );

            if group.len() == 1 {
                // Single stage - execute directly
                let stage_idx = group[0];
                let stage = &pipeline.stages[stage_idx];

                // Find input for this stage from output_map
                let input = output_map
                    .get(&stage.input_type)
                    .ok_or_else(|| {
                        PluginError::ExecutionFailed(format!(
                            "No output found for input type '{}' required by stage {} ({})",
                            stage.input_type,
                            stage_idx,
                            stage.plugin.name()
                        ))
                    })?
                    .1
                    .clone();

                let mut result =
                    Self::execute_single_stage(context, stage_idx, stage, input, cache, &tx)
                        .await?;

                // Take ownership of warnings (avoids clone)
                all_warnings.extend(std::mem::take(&mut result.warnings));

                // Update output map with this stage's output, using mem::replace to avoid double-clone
                let output_for_map =
                    std::mem::replace(&mut result.output, PluginData::Bytes(Vec::new()));
                output_map.insert(
                    stage.output_type.clone(),
                    (stage_idx, output_for_map.clone()),
                );

                // Send complete result (warnings and output already moved, minimal clone overhead)
                let _ = tx.send(StreamingResult::Complete(result.clone())).await;

                // Restore output for stage_results (swap back)
                result.output = output_for_map;
                stage_results.push(result);
            } else {
                // Multiple stages - execute in parallel
                // Each stage gets its input from output_map based on input_type
                let mut results = Self::execute_parallel_stages_with_map(
                    context,
                    group,
                    pipeline,
                    &output_map,
                    cache,
                    &tx,
                )
                .await?;

                // Update output map with all results
                for result in &mut results {
                    let stage = &pipeline.stages[result.stage_index];
                    output_map.insert(
                        stage.output_type.clone(),
                        (result.stage_index, result.output.clone()),
                    );
                    // Take ownership of warnings (avoids clone)
                    all_warnings.extend(std::mem::take(&mut result.warnings));
                }

                stage_results.extend(results);
            }
        }

        let total_duration = start_time.elapsed();
        info!("=== Performance Executor Completed ===");
        info!("Total time: {:.2}s", total_duration.as_secs_f64());

        // Return the output of the last stage executed
        let final_output = stage_results
            .last()
            .map(|r| r.output.clone())
            .unwrap_or(initial_input);

        Ok(ExecutionResult {
            output: final_output,
            intermediates: stage_results,
            total_duration,
            warnings: all_warnings,
        })
    }

    /// Execute a single stage with streaming support
    async fn execute_single_stage(
        context: &Context,
        stage_index: usize,
        stage: &PipelineStage,
        input: PluginData,
        cache: Option<&PipelineCache>,
        tx: &tokio::sync::mpsc::Sender<StreamingResult>,
    ) -> Result<StageResult, PluginError> {
        info!(
            "▶️  Stage {}: {} ({} → {})",
            stage_index + 1,
            stage.plugin.name(),
            stage.input_type,
            stage.output_type
        );

        // Check cache first (if enabled)
        if let Some(cache) = cache {
            if let Some(cached_output) =
                cache.get(stage.plugin.name(), stage.operation.name(), &input)
            {
                info!("  ✅ Cache hit - skipping execution");
                return Ok(StageResult {
                    stage_index,
                    plugin_name: stage.plugin.name().to_string(),
                    operation_name: Cow::Borrowed(stage.operation.name()),
                    output: cached_output,
                    duration: std::time::Duration::ZERO,
                    warnings: Vec::new(),
                });
            }
        }

        let request = PluginRequest {
            operation: stage.operation.clone(),
            input: input.clone(),
        };

        let stage_start = Instant::now();

        // Try streaming execution if supported
        let streaming_response = stage.plugin.execute_streaming(context, &request).await?;

        let (output, warnings) = match streaming_response {
            crate::plugin::PluginStreamingResponse::Partial(partial) => {
                // Send partial result
                let _ = tx
                    .send(StreamingResult::Partial {
                        stage_index,
                        plugin_name: stage.plugin.name().to_string(),
                        data: partial,
                    })
                    .await;

                // Continue execution to get complete result
                let response = stage.plugin.execute(context, &request).await?;
                (response.output, response.warnings)
            }
            crate::plugin::PluginStreamingResponse::Complete(response) => {
                (response.output, response.warnings)
            }
        };

        let duration = stage_start.elapsed();
        info!("  ✓ Completed in {:.2}s", duration.as_secs_f64());

        // Save to cache (if enabled)
        if let Some(cache) = cache {
            cache.put(
                stage.plugin.name(),
                stage.operation.name(),
                &request.input,
                &output,
            );
        }

        Ok(StageResult {
            stage_index,
            plugin_name: stage.plugin.name().to_string(),
            operation_name: Cow::Borrowed(stage.operation.name()),
            output,
            duration,
            warnings,
        })
    }

    /// Execute multiple stages in parallel with different inputs based on output_map
    /// Each stage looks up its required input from the output_map based on input_type
    async fn execute_parallel_stages_with_map(
        context: &Context,
        stage_indices: &[usize],
        pipeline: &Pipeline,
        output_map: &std::collections::HashMap<String, (usize, PluginData)>,
        cache: Option<&PipelineCache>,
        tx: &tokio::sync::mpsc::Sender<StreamingResult>,
    ) -> Result<Vec<StageResult>, PluginError> {
        let mut tasks = Vec::with_capacity(stage_indices.len());

        for &stage_idx in stage_indices {
            let stage = pipeline.stages[stage_idx].clone();
            let context = context.clone();
            let tx = tx.clone();
            let cache = cache.cloned();

            // Look up input for this stage from output_map
            let input = output_map
                .get(&stage.input_type)
                .ok_or_else(|| {
                    PluginError::ExecutionFailed(format!(
                        "No output found for input type '{}' required by stage {} ({})",
                        stage.input_type,
                        stage_idx,
                        stage.plugin.name()
                    ))
                })?
                .1
                .clone();

            let task = tokio::spawn(async move {
                Self::execute_single_stage(&context, stage_idx, &stage, input, cache.as_ref(), &tx)
                    .await
            });

            tasks.push(task);
        }

        // Wait for all tasks to complete
        let mut results = Vec::with_capacity(tasks.len());
        for task in tasks {
            match task.await {
                Ok(Ok(result)) => results.push(result),
                Ok(Err(e)) => return Err(e),
                Err(e) => {
                    return Err(PluginError::ExecutionFailed(format!(
                        "Task panicked: {:?}",
                        e
                    )))
                }
            }
        }

        Ok(results)
    }

    /// Group stages by dependency level
    /// Stages in the same group can be executed in parallel
    ///
    /// Algorithm: Topological sort with level grouping (Kahn's algorithm)
    /// - Builds dependency graph based on input/output type matching
    /// - Groups stages by dependency level (BFS layers)
    /// - Stages in same group have no dependencies on each other
    fn group_stages_by_dependency(pipeline: &Pipeline) -> Vec<Vec<usize>> {
        let num_stages = pipeline.stages.len();

        if num_stages == 0 {
            return vec![];
        }

        if num_stages == 1 {
            return vec![vec![0]];
        }

        // Build dependency graph
        // dependencies[i] = list of stages that depend on stage i (forward edges)
        // in_degree[i] = number of stages that stage i depends on (backward edges)
        let mut dependencies: Vec<Vec<usize>> = (0..num_stages)
            .map(|_| Vec::with_capacity(2)) // Most stages have 1-2 dependents
            .collect();
        let mut in_degree: Vec<usize> = vec![0; num_stages];

        // For each stage, find what it depends on
        for (i, stage_i) in pipeline.stages.iter().enumerate() {
            // Check if this stage depends on any previous stage's output
            for (j, stage_j) in pipeline.stages[..i].iter().enumerate() {
                // Stage i depends on stage j if stage_i's input matches stage_j's output
                if stage_i.input_type == stage_j.output_type {
                    dependencies[j].push(i); // j -> i edge
                    in_degree[i] += 1;
                }
            }
        }

        // Topological sort with level grouping (Kahn's algorithm)
        // Pre-allocate for typical pipeline depth (2-5 levels: input, processing stages, output)
        let mut groups = Vec::with_capacity(4);

        // Start with stages that have no dependencies
        let zero_degree_count = in_degree.iter().filter(|&&d| d == 0).count();
        let mut queue: Vec<usize> = Vec::with_capacity(zero_degree_count);
        queue.extend(in_degree.iter().enumerate().filter_map(|(i, &degree)| {
            if degree == 0 {
                Some(i)
            } else {
                None
            }
        }));

        let mut processed = vec![false; num_stages];

        while !queue.is_empty() {
            // All stages in current queue can run in parallel
            let current_group = std::mem::take(&mut queue);

            // Mark these as processed
            for &stage_idx in &current_group {
                processed[stage_idx] = true;
            }

            // Find next level - stages whose dependencies are now all processed
            // Conservative estimate: allocate capacity based on current group size
            let mut next_queue = Vec::with_capacity(current_group.len());
            let mut next_queue_set = std::collections::HashSet::with_capacity(current_group.len());
            for &stage_idx in &current_group {
                // For each stage that depends on this one
                for &dependent in &dependencies[stage_idx] {
                    if !processed[dependent] && next_queue_set.insert(dependent) {
                        // Check if all of dependent's dependencies are now processed
                        let all_deps_processed = (0..num_stages)
                            .filter(|&j| dependencies[j].contains(&dependent))
                            .all(|j| processed[j]);

                        if all_deps_processed {
                            next_queue.push(dependent);
                        } else {
                            // Remove from set if not ready yet
                            next_queue_set.remove(&dependent);
                        }
                    }
                }
            }

            // Move current_group into groups (no clone needed)
            groups.push(current_group);

            queue = next_queue;
        }

        // Verify all stages were processed (no cycles)
        if groups.iter().map(|g| g.len()).sum::<usize>() != num_stages {
            warn!("Dependency cycle detected, falling back to sequential execution");
            let mut fallback: Vec<Vec<usize>> = Vec::with_capacity(num_stages);
            fallback.extend((0..num_stages).map(|i| vec![i]));
            return fallback;
        }

        info!(
            "Dependency analysis complete: {} stages grouped into {} parallel levels",
            num_stages,
            groups.len()
        );
        for (level, group) in groups.iter().enumerate() {
            info!(
                "  Level {}: {} stages can run in parallel",
                level,
                group.len()
            );
        }

        groups
    }
}

impl Default for PerformanceExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// Bulk result for a single file
#[derive(Debug, Clone)]
pub struct BulkFileResult {
    /// Input file path
    pub input_path: PathBuf,

    /// Execution result (Ok or Err)
    pub result: Result<ExecutionResult, String>,

    /// Processing time for this file
    pub processing_time: std::time::Duration,
}

/// Result from a bulk fast-path execution (keyframes + object detection)
#[derive(Debug, Clone)]
pub struct BulkFastPathResult {
    /// Input file path
    pub input_path: PathBuf,

    /// Detections with frame information (Ok or Err)
    pub result: Result<Vec<crate::fast_path::DetectionWithFrame>, String>,

    /// Processing time for this file
    pub processing_time: std::time::Duration,
}

/// Bulk mode executor - maximum throughput, parallel processing
pub struct BulkExecutor {
    /// Execution context
    context: Context,

    /// Maximum concurrent files to process
    max_concurrent_files: usize,

    /// Optional cache for operation results (thread-safe for parallel access)
    cache: Option<PipelineCache>,
}

impl BulkExecutor {
    /// Create a new bulk executor with cache enabled by default
    pub fn new() -> Self {
        Self {
            context: Context::bulk(),
            max_concurrent_files: num_cpus::get(),
            cache: Some(PipelineCache::new()),
        }
    }

    /// Set maximum concurrent files
    pub fn with_max_concurrent_files(mut self, max: usize) -> Self {
        self.max_concurrent_files = max;
        self
    }

    /// Enable cache with default settings
    pub fn with_cache(mut self) -> Self {
        self.cache = Some(PipelineCache::new());
        self
    }

    /// Enable cache with custom memory limit
    pub fn with_cache_memory_limit(mut self, max_memory_bytes: usize) -> Self {
        self.cache = Some(PipelineCache::with_memory_limit(max_memory_bytes));
        self
    }

    /// Disable cache
    pub fn without_cache(mut self) -> Self {
        self.cache = None;
        self
    }

    /// Execute a pipeline on multiple files concurrently
    /// Returns a channel that streams results as files complete
    pub async fn execute_bulk(
        &self,
        pipeline: &Pipeline,
        input_files: Vec<PathBuf>,
    ) -> Result<tokio::sync::mpsc::Receiver<BulkFileResult>, PluginError> {
        let (tx, rx) = tokio::sync::mpsc::channel(100);

        // Create semaphore to limit concurrent file processing
        let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(self.max_concurrent_files));

        info!("=== Bulk Executor Started ===");
        info!("Total files: {}", input_files.len());
        info!("Max concurrent: {}", self.max_concurrent_files);
        if self.cache.is_some() {
            info!("Cache: enabled (shared across parallel workers)");
        } else {
            info!("Cache: disabled");
        }

        // Clone data for async tasks
        let pipeline = pipeline.clone();
        let context = self.context.clone();
        let cache = self.cache.clone(); // Arc-based, safe to share across threads

        // Spawn task for each file
        for input_path in input_files {
            let tx = tx.clone();
            let semaphore = semaphore.clone();
            let pipeline = pipeline.clone();
            let context = context.clone();
            let cache = cache.clone();

            tokio::spawn(async move {
                // Acquire semaphore permit
                let _permit = semaphore.acquire().await;

                let start_time = Instant::now();

                // Execute pipeline for this file
                let result =
                    Self::execute_single_file(&context, &pipeline, &input_path, cache.as_ref())
                        .await;

                let processing_time = start_time.elapsed();

                // Convert error to string for serialization
                let result = result.map_err(|e| format!("{:?}", e));

                // Send result
                let _ = tx
                    .send(BulkFileResult {
                        input_path,
                        result,
                        processing_time,
                    })
                    .await;
            });
        }

        Ok(rx)
    }

    /// Execute keyframes + object detection pipeline on multiple files concurrently
    ///
    /// This method provides a direct fast-path for bulk video processing that bypasses
    /// the plugin system for maximum performance. It uses zero-copy decoding and shared
    /// ONNX model sessions across all workers.
    ///
    /// # Performance
    ///
    /// - **2-3x speedup** for 10+ files (vs sequential processing)
    /// - **3-5x speedup** for 50+ files (linear scaling with cores)
    /// - Shared ONNX model session (saves 200-500ms per file)
    /// - Thread-safe FFmpeg decoding (multiple files in parallel)
    /// - Zero disk I/O (no intermediate JPEG writes)
    ///
    /// # Arguments
    ///
    /// * `input_files` - Paths to video files to process
    /// * `confidence_threshold` - Object detection confidence threshold (0.0-1.0)
    /// * `classes` - Optional filter for specific object classes (e.g., vec!["person", "car"])
    ///
    /// # Returns
    ///
    /// Channel that streams results as files complete. Each result contains:
    /// - Input file path
    /// - Detections with frame numbers, timestamps, and bounding boxes
    /// - Processing time for that file
    ///
    /// # Thread Safety
    ///
    /// - FFmpeg initialization is serialized via global mutex (FFMPEG_INIT_LOCK)
    /// - ONNX model session is shared across all workers (OnceLock + Mutex)
    /// - Each file is decoded independently (fully parallel after initialization)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use video_extract_core::executor::BulkExecutor;
    /// # use std::path::PathBuf;
    /// # tokio_test::block_on(async {
    /// let executor = BulkExecutor::new()
    ///     .with_max_concurrent_files(8);
    ///
    /// let files = vec![
    ///     PathBuf::from("video1.mp4"),
    ///     PathBuf::from("video2.mp4"),
    /// ];
    ///
    /// let mut rx = executor.execute_bulk_fast_path(
    ///     files,
    ///     0.25,  // 25% confidence
    ///     None,  // All object classes
    /// ).await.unwrap();
    ///
    /// while let Some(result) = rx.recv().await {
    ///     println!("Processed {:?} in {:?}", result.input_path, result.processing_time);
    /// }
    /// # });
    /// ```
    pub async fn execute_bulk_fast_path(
        &self,
        input_files: Vec<PathBuf>,
        confidence_threshold: f32,
        classes: Option<Vec<String>>,
    ) -> Result<tokio::sync::mpsc::Receiver<BulkFastPathResult>, PluginError> {
        let (tx, rx) = tokio::sync::mpsc::channel(100);

        // Create semaphore to limit concurrent file processing
        let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(self.max_concurrent_files));

        info!("=== Bulk Fast Path Executor Started ===");
        info!("Total files: {}", input_files.len());
        info!("Max concurrent: {}", self.max_concurrent_files);
        info!("Confidence threshold: {}", confidence_threshold);
        if let Some(ref cls) = classes {
            info!("Filtering classes: {:?}", cls);
        }

        // Spawn task for each file
        for input_path in input_files {
            let tx = tx.clone();
            let semaphore = semaphore.clone();
            let classes = classes.clone();

            tokio::spawn(async move {
                // Acquire semaphore permit
                let _permit = semaphore.acquire().await;

                let start_time = Instant::now();

                // Call fast path (zero-copy, ONNX session from pool)
                let result = crate::fast_path::extract_and_detect_zero_copy(
                    &input_path,
                    confidence_threshold,
                    classes,
                );

                let processing_time = start_time.elapsed();

                // Convert error to string for serialization
                let result = result.map_err(|e| format!("{:?}", e));

                // Send result
                let _ = tx
                    .send(BulkFastPathResult {
                        input_path,
                        result,
                        processing_time,
                    })
                    .await;
            });
        }

        Ok(rx)
    }

    /// Execute pipeline for a single file
    async fn execute_single_file(
        context: &Context,
        pipeline: &Pipeline,
        input_path: &Path,
        cache: Option<&PipelineCache>,
    ) -> Result<ExecutionResult, PluginError> {
        let start_time = Instant::now();
        let file_name = input_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        debug!(
            "[FILE {}] Starting pipeline execution ({} stages)",
            file_name,
            pipeline.stages.len()
        );

        // Initial input is the file path
        let mut current_input = PluginData::FilePath(input_path.to_path_buf());
        let mut stage_results = Vec::with_capacity(pipeline.stages.len());
        let mut all_warnings = Vec::with_capacity(pipeline.stages.len());

        // Execute each stage sequentially for this file
        for (idx, stage) in pipeline.stages.iter().enumerate() {
            debug!(
                "[FILE {}] Stage {}/{}: {} ({}->{})",
                file_name,
                idx + 1,
                pipeline.stages.len(),
                stage.plugin.name(),
                stage.input_type,
                stage.output_type
            );

            // Check cache first (if enabled)
            if let Some(cache) = cache {
                if let Some(cached_output) =
                    cache.get(stage.plugin.name(), stage.operation.name(), &current_input)
                {
                    debug!("  [FILE {}] ✅ Cache hit - skipping execution", file_name);

                    // Store cached result (avoid clone by swapping)
                    let mut stage_result = StageResult {
                        stage_index: idx,
                        plugin_name: stage.plugin.name().to_string(),
                        operation_name: Cow::Borrowed(stage.operation.name()),
                        output: cached_output,
                        duration: std::time::Duration::ZERO,
                        warnings: Vec::new(),
                    };

                    // Use cached output as input for next stage (take ownership, replace with placeholder)
                    current_input =
                        std::mem::replace(&mut stage_result.output, PluginData::Bytes(Vec::new()));

                    stage_results.push(stage_result);
                    continue;
                }
            }

            let stage_start = Instant::now();

            // Build request
            let request = crate::plugin::PluginRequest {
                input: current_input.clone(),
                operation: stage.operation.clone(),
            };

            // Execute stage
            let response = stage.plugin.execute(context, &request).await?;

            let stage_duration = stage_start.elapsed();

            debug!(
                "[FILE {}] Stage {}/{} completed in {:.2}s",
                file_name,
                idx + 1,
                pipeline.stages.len(),
                stage_duration.as_secs_f64()
            );

            // Save to cache (if enabled)
            if let Some(cache) = cache {
                cache.put(
                    stage.plugin.name(),
                    stage.operation.name(),
                    &request.input,
                    &response.output,
                );
            }

            // Store stage result first (take ownership of output to avoid clone)
            let mut stage_result = StageResult {
                stage_index: idx,
                plugin_name: stage.plugin.name().to_string(),
                operation_name: Cow::Borrowed(stage.operation.name()),
                output: response.output,
                duration: stage_duration,
                warnings: response.warnings,
            };

            // Take ownership of warnings (avoids Vec clone + String element clones)
            all_warnings.extend(std::mem::take(&mut stage_result.warnings));

            // Take output for next stage, replace with placeholder
            current_input =
                std::mem::replace(&mut stage_result.output, PluginData::Bytes(Vec::new()));

            stage_results.push(stage_result);
        }

        let total_duration = start_time.elapsed();

        debug!(
            "[FILE {}] Pipeline execution completed in {:.2}s",
            file_name,
            total_duration.as_secs_f64()
        );

        Ok(ExecutionResult {
            output: current_input,
            intermediates: stage_results,
            total_duration,
            warnings: all_warnings,
        })
    }
}

impl Default for BulkExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operation::{Operation, WhisperModel};
    use crate::plugin::{
        CacheConfig, PerformanceConfig, Plugin, PluginConfig, PluginResponse, RuntimeConfig,
    };
    use crate::Context;
    use std::sync::Arc;
    use std::time::{Duration, SystemTime};

    // Mock plugin for testing
    struct MockPlugin {
        name: String,
        config: PluginConfig,
        output_data: PluginData,
    }

    #[async_trait::async_trait]
    impl Plugin for MockPlugin {
        fn name(&self) -> &str {
            &self.name
        }

        fn config(&self) -> &PluginConfig {
            &self.config
        }

        fn supports_input(&self, input_type: &str) -> bool {
            self.config.inputs.iter().any(|s| s == input_type)
        }

        fn produces_output(&self, output_type: &str) -> bool {
            self.config.outputs.iter().any(|s| s == output_type)
        }

        async fn execute(
            &self,
            _ctx: &Context,
            _request: &PluginRequest,
        ) -> Result<PluginResponse, PluginError> {
            Ok(PluginResponse {
                output: self.output_data.clone(),
                duration: Duration::from_millis(100),
                warnings: vec![],
            })
        }
    }

    fn create_mock_plugin(
        name: &str,
        inputs: Vec<&str>,
        outputs: Vec<&str>,
        output_data: PluginData,
    ) -> Arc<dyn Plugin> {
        let mut input_strings = Vec::with_capacity(inputs.len());
        input_strings.extend(inputs.iter().map(|s| s.to_string()));
        let mut output_strings = Vec::with_capacity(outputs.len());
        output_strings.extend(outputs.iter().map(|s| s.to_string()));
        Arc::new(MockPlugin {
            name: name.to_string(),
            config: PluginConfig {
                name: name.to_string(),
                description: "Mock plugin".to_string(),
                inputs: input_strings,
                outputs: output_strings,
                config: RuntimeConfig {
                    max_file_size_mb: 1000,
                    requires_gpu: false,
                    experimental: false,
                },
                performance: PerformanceConfig {
                    avg_processing_time_per_gb: "30s".to_string(),
                    memory_per_file_mb: 512,
                    supports_streaming: false,
                },
                cache: CacheConfig {
                    enabled: true,
                    version: 1,
                    invalidate_before: SystemTime::UNIX_EPOCH,
                },
            },
            output_data,
        })
    }

    #[tokio::test]
    async fn test_debug_executor_single_stage() {
        let executor = DebugExecutor::new();

        // Create a simple pipeline with one stage
        let audio_plugin = create_mock_plugin(
            "audio_extract",
            vec!["mp4"],
            vec!["Audio"],
            PluginData::Bytes(vec![1, 2, 3, 4]),
        );

        let pipeline = Pipeline {
            stages: vec![PipelineStage {
                plugin: audio_plugin,
                input_type: "mp4".to_string(),
                output_type: "Audio".to_string(),
                operation: Operation::Audio {
                    sample_rate: 16000,
                    channels: 1,
                },
            }],
        };

        let initial_input = PluginData::FilePath(PathBuf::from("test.mp4"));
        let result = executor.execute(&pipeline, initial_input).await.unwrap();

        assert_eq!(result.intermediates.len(), 1);
        assert_eq!(result.intermediates[0].plugin_name, "audio_extract");
    }

    #[tokio::test]
    async fn test_debug_executor_multi_stage() {
        let executor = DebugExecutor::new();

        // Create a two-stage pipeline: video -> audio -> transcription
        let audio_plugin = create_mock_plugin(
            "audio_extract",
            vec!["mp4"],
            vec!["Audio"],
            PluginData::FilePath(PathBuf::from("audio.wav")),
        );

        let transcription_plugin = create_mock_plugin(
            "transcription",
            vec!["Audio", "wav"],
            vec!["Transcription"],
            PluginData::Json(serde_json::json!({"text": "Hello world"})),
        );

        let pipeline = Pipeline {
            stages: vec![
                PipelineStage {
                    plugin: audio_plugin,
                    input_type: "mp4".to_string(),
                    output_type: "Audio".to_string(),
                    operation: Operation::Audio {
                        sample_rate: 16000,
                        channels: 1,
                    },
                },
                PipelineStage {
                    plugin: transcription_plugin,
                    input_type: "Audio".to_string(),
                    output_type: "Transcription".to_string(),
                    operation: Operation::Transcription {
                        language: None,
                        model: WhisperModel::Base,
                    },
                },
            ],
        };

        let initial_input = PluginData::FilePath(PathBuf::from("test.mp4"));
        let result = executor.execute(&pipeline, initial_input).await.unwrap();

        assert_eq!(result.intermediates.len(), 2);
        assert_eq!(result.intermediates[0].plugin_name, "audio_extract");
        assert_eq!(result.intermediates[1].plugin_name, "transcription");

        // Check final output is JSON from transcription
        match result.output {
            PluginData::Json(_) => {}
            _ => panic!("Expected JSON output"),
        }
    }

    #[tokio::test]
    async fn test_performance_executor_streaming() {
        let executor = PerformanceExecutor::new();

        // Create a simple pipeline
        let audio_plugin = create_mock_plugin(
            "audio_extract",
            vec!["mp4"],
            vec!["Audio"],
            PluginData::FilePath(PathBuf::from("audio.wav")),
        );

        let pipeline = Pipeline {
            stages: vec![PipelineStage {
                plugin: audio_plugin,
                input_type: "mp4".to_string(),
                output_type: "Audio".to_string(),
                operation: Operation::Audio {
                    sample_rate: 16000,
                    channels: 1,
                },
            }],
        };

        let initial_input = PluginData::FilePath(PathBuf::from("test.mp4"));
        let mut rx = executor
            .execute_streaming(&pipeline, initial_input)
            .await
            .unwrap();

        // Collect streaming results
        let mut complete_count = 0;
        let mut final_count = 0;

        while let Some(result) = rx.recv().await {
            match result {
                StreamingResult::Partial { .. } => {}
                StreamingResult::Complete(_) => {
                    complete_count += 1;
                }
                StreamingResult::Final(_) => {
                    final_count += 1;
                }
            }
        }

        // Should have 1 complete result and 1 final result
        assert_eq!(complete_count, 1);
        assert_eq!(final_count, 1);
    }

    #[tokio::test]
    async fn test_performance_executor_multi_stage() {
        let executor = PerformanceExecutor::new();

        // Create a two-stage pipeline
        let audio_plugin = create_mock_plugin(
            "audio_extract",
            vec!["mp4"],
            vec!["Audio"],
            PluginData::FilePath(PathBuf::from("audio.wav")),
        );

        let transcription_plugin = create_mock_plugin(
            "transcription",
            vec!["Audio", "wav"],
            vec!["Transcription"],
            PluginData::Json(serde_json::json!({"text": "Hello world"})),
        );

        let pipeline = Pipeline {
            stages: vec![
                PipelineStage {
                    plugin: audio_plugin,
                    input_type: "mp4".to_string(),
                    output_type: "Audio".to_string(),
                    operation: Operation::Audio {
                        sample_rate: 16000,
                        channels: 1,
                    },
                },
                PipelineStage {
                    plugin: transcription_plugin,
                    input_type: "Audio".to_string(),
                    output_type: "Transcription".to_string(),
                    operation: Operation::Transcription {
                        language: None,
                        model: WhisperModel::Base,
                    },
                },
            ],
        };

        let initial_input = PluginData::FilePath(PathBuf::from("test.mp4"));
        let mut rx = executor
            .execute_streaming(&pipeline, initial_input)
            .await
            .unwrap();

        let mut stage_results = Vec::with_capacity(pipeline.stages.len());
        let mut final_result = None;

        while let Some(result) = rx.recv().await {
            match result {
                StreamingResult::Complete(stage_result) => {
                    stage_results.push(stage_result);
                }
                StreamingResult::Final(exec_result) => {
                    final_result = Some(exec_result);
                }
                _ => {}
            }
        }

        assert_eq!(stage_results.len(), 2);
        assert!(final_result.is_some());

        let final_result = final_result.unwrap();
        assert_eq!(final_result.intermediates.len(), 2);

        // Check final output is JSON from transcription
        match final_result.output {
            PluginData::Json(_) => {}
            _ => panic!("Expected JSON output"),
        }
    }

    #[test]
    fn test_dependency_grouping_sequential() {
        // Test case: Simple sequential pipeline (A -> B -> C)
        // Expected: 3 groups, each with 1 stage
        let plugin_a = create_mock_plugin(
            "plugin_a",
            vec!["mp4"],
            vec!["TypeA"],
            PluginData::Bytes(vec![1]),
        );
        let plugin_b = create_mock_plugin(
            "plugin_b",
            vec!["TypeA"],
            vec!["TypeB"],
            PluginData::Bytes(vec![2]),
        );
        let plugin_c = create_mock_plugin(
            "plugin_c",
            vec!["TypeB"],
            vec!["TypeC"],
            PluginData::Bytes(vec![3]),
        );

        let pipeline = Pipeline {
            stages: vec![
                PipelineStage {
                    plugin: plugin_a,
                    input_type: "mp4".to_string(),
                    output_type: "TypeA".to_string(),
                    operation: Operation::Audio {
                        sample_rate: 16000,
                        channels: 1,
                    },
                },
                PipelineStage {
                    plugin: plugin_b,
                    input_type: "TypeA".to_string(),
                    output_type: "TypeB".to_string(),
                    operation: Operation::Audio {
                        sample_rate: 16000,
                        channels: 1,
                    },
                },
                PipelineStage {
                    plugin: plugin_c,
                    input_type: "TypeB".to_string(),
                    output_type: "TypeC".to_string(),
                    operation: Operation::Audio {
                        sample_rate: 16000,
                        channels: 1,
                    },
                },
            ],
        };

        let groups = PerformanceExecutor::group_stages_by_dependency(&pipeline);

        // Verify: 3 groups (sequential)
        assert_eq!(groups.len(), 3);
        assert_eq!(groups[0], vec![0]); // Stage 0 runs first
        assert_eq!(groups[1], vec![1]); // Stage 1 runs after 0
        assert_eq!(groups[2], vec![2]); // Stage 2 runs after 1
    }

    #[test]
    fn test_dependency_grouping_parallel_simple() {
        // Test case: Two independent branches from same input
        // mp4 -> audio (TypeA)
        // mp4 -> keyframes (TypeB)
        // Expected: 1 group with 2 stages (both can run in parallel)
        let plugin_audio = create_mock_plugin(
            "audio",
            vec!["mp4"],
            vec!["TypeA"],
            PluginData::Bytes(vec![1]),
        );
        let plugin_keyframes = create_mock_plugin(
            "keyframes",
            vec!["mp4"],
            vec!["TypeB"],
            PluginData::Bytes(vec![2]),
        );

        let pipeline = Pipeline {
            stages: vec![
                PipelineStage {
                    plugin: plugin_audio,
                    input_type: "mp4".to_string(),
                    output_type: "TypeA".to_string(),
                    operation: Operation::Audio {
                        sample_rate: 16000,
                        channels: 1,
                    },
                },
                PipelineStage {
                    plugin: plugin_keyframes,
                    input_type: "mp4".to_string(),
                    output_type: "TypeB".to_string(),
                    operation: Operation::Keyframes {
                        max_frames: None,
                        min_interval_sec: 1.0,
                    },
                },
            ],
        };

        let groups = PerformanceExecutor::group_stages_by_dependency(&pipeline);

        // Verify: 1 group with both stages (parallel)
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].len(), 2);
        assert!(groups[0].contains(&0));
        assert!(groups[0].contains(&1));
    }

    #[test]
    fn test_dependency_grouping_complex_pipeline() {
        // Test case: Complex real-world pipeline
        // Level 0: mp4 -> keyframes (Keyframes), mp4 -> audio (Audio)
        // Level 1: Keyframes -> object_detection (ObjectDetection),
        //          Keyframes -> vision_embeddings (VisionEmbeddings),
        //          Audio -> transcription (Transcription)
        // Level 2: Transcription -> text_embeddings (TextEmbeddings)
        //
        // Expected groups:
        // Group 0: [0, 1] (keyframes + audio in parallel)
        // Group 1: [2, 3, 4] (object_detection + vision_embeddings + transcription in parallel)
        // Group 2: [5] (text_embeddings sequential, depends on transcription)

        let keyframes = create_mock_plugin(
            "keyframes",
            vec!["mp4"],
            vec!["Keyframes"],
            PluginData::Bytes(vec![1]),
        );
        let audio = create_mock_plugin(
            "audio",
            vec!["mp4"],
            vec!["Audio"],
            PluginData::Bytes(vec![2]),
        );
        let object_detection = create_mock_plugin(
            "object_detection",
            vec!["Keyframes"],
            vec!["ObjectDetection"],
            PluginData::Bytes(vec![3]),
        );
        let vision_embeddings = create_mock_plugin(
            "vision_embeddings",
            vec!["Keyframes"],
            vec!["VisionEmbeddings"],
            PluginData::Bytes(vec![4]),
        );
        let transcription = create_mock_plugin(
            "transcription",
            vec!["Audio"],
            vec!["Transcription"],
            PluginData::Bytes(vec![5]),
        );
        let text_embeddings = create_mock_plugin(
            "text_embeddings",
            vec!["Transcription"],
            vec!["TextEmbeddings"],
            PluginData::Bytes(vec![6]),
        );

        let pipeline = Pipeline {
            stages: vec![
                PipelineStage {
                    plugin: keyframes,
                    input_type: "mp4".to_string(),
                    output_type: "Keyframes".to_string(),
                    operation: Operation::Keyframes {
                        max_frames: None,
                        min_interval_sec: 1.0,
                    },
                },
                PipelineStage {
                    plugin: audio,
                    input_type: "mp4".to_string(),
                    output_type: "Audio".to_string(),
                    operation: Operation::Audio {
                        sample_rate: 16000,
                        channels: 1,
                    },
                },
                PipelineStage {
                    plugin: object_detection,
                    input_type: "Keyframes".to_string(),
                    output_type: "ObjectDetection".to_string(),
                    operation: Operation::ObjectDetection {
                        model: crate::operation::ObjectDetectionModel::YoloV8n,
                        confidence_threshold: 0.3,
                        classes: None,
                    },
                },
                PipelineStage {
                    plugin: vision_embeddings,
                    input_type: "Keyframes".to_string(),
                    output_type: "VisionEmbeddings".to_string(),
                    operation: Operation::VisionEmbeddings {
                        model: crate::operation::VisionModel::ClipVitB32,
                    },
                },
                PipelineStage {
                    plugin: transcription,
                    input_type: "Audio".to_string(),
                    output_type: "Transcription".to_string(),
                    operation: Operation::Transcription {
                        language: None,
                        model: WhisperModel::Base,
                    },
                },
                PipelineStage {
                    plugin: text_embeddings,
                    input_type: "Transcription".to_string(),
                    output_type: "TextEmbeddings".to_string(),
                    operation: Operation::TextEmbeddings {
                        model: crate::operation::TextModel::AllMiniLmL6V2,
                    },
                },
            ],
        };

        let groups = PerformanceExecutor::group_stages_by_dependency(&pipeline);

        // Verify: 3 groups with correct parallelism
        assert_eq!(groups.len(), 3, "Expected 3 dependency levels");

        // Group 0: keyframes (0) + audio (1) in parallel
        assert_eq!(groups[0].len(), 2, "Group 0 should have 2 stages");
        assert!(groups[0].contains(&0), "Group 0 should contain keyframes");
        assert!(groups[0].contains(&1), "Group 0 should contain audio");

        // Group 1: object_detection (2) + vision_embeddings (3) + transcription (4) in parallel
        assert_eq!(groups[1].len(), 3, "Group 1 should have 3 stages");
        assert!(
            groups[1].contains(&2),
            "Group 1 should contain object_detection"
        );
        assert!(
            groups[1].contains(&3),
            "Group 1 should contain vision_embeddings"
        );
        assert!(
            groups[1].contains(&4),
            "Group 1 should contain transcription"
        );

        // Group 2: text_embeddings (5) sequential
        assert_eq!(groups[2].len(), 1, "Group 2 should have 1 stage");
        assert!(
            groups[2].contains(&5),
            "Group 2 should contain text_embeddings"
        );
    }

    #[test]
    fn test_dependency_grouping_empty_pipeline() {
        let pipeline = Pipeline { stages: vec![] };

        let groups = PerformanceExecutor::group_stages_by_dependency(&pipeline);

        assert_eq!(groups.len(), 0);
    }

    #[test]
    fn test_dependency_grouping_single_stage() {
        let plugin = create_mock_plugin(
            "audio",
            vec!["mp4"],
            vec!["Audio"],
            PluginData::Bytes(vec![1]),
        );

        let pipeline = Pipeline {
            stages: vec![PipelineStage {
                plugin,
                input_type: "mp4".to_string(),
                output_type: "Audio".to_string(),
                operation: Operation::Audio {
                    sample_rate: 16000,
                    channels: 1,
                },
            }],
        };

        let groups = PerformanceExecutor::group_stages_by_dependency(&pipeline);

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0], vec![0]);
    }
}
