//! Plugin registry and pipeline routing

use crate::error::RegistryError;
use crate::{Operation, OutputSpec, Plugin, PluginConfig};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info};

/// Plugin registry for lookup and routing
pub struct Registry {
    /// All registered plugins by name
    plugins: HashMap<String, Arc<dyn Plugin>>,

    /// Plugins indexed by output type they produce
    by_output: HashMap<String, Vec<Arc<dyn Plugin>>>,

    /// Transitive closure of supported inputs (for pipeline composition)
    transitive_inputs: HashMap<String, HashSet<String>>,
}

impl Registry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            plugins: HashMap::with_capacity(20), // Typical registry has 15-20 plugins
            by_output: HashMap::with_capacity(25), // Each plugin may register multiple output types
            transitive_inputs: HashMap::with_capacity(20), // One entry per plugin for transitive inputs
        }
    }

    /// Register a plugin
    pub fn register(&mut self, plugin: Arc<dyn Plugin>) {
        let name = plugin.name().to_string();
        let config = plugin.config();

        info!("Registering plugin: {}", name);

        // Index by output types
        for output in &config.outputs {
            self.by_output
                .entry(output.clone())
                .or_default()
                .push(Arc::clone(&plugin));
        }

        self.plugins.insert(name, plugin);

        // Recompute transitive closure
        self.compute_transitive_inputs();
    }

    /// Load plugin configuration from YAML file
    pub fn load_plugin_config(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<PluginConfig, RegistryError> {
        let contents = std::fs::read_to_string(path)?;
        let config: PluginConfig = serde_yaml::from_str(&contents)?;
        Ok(config)
    }

    /// Find plugin(s) to satisfy an OutputSpec
    pub fn lookup(&self, spec: &OutputSpec) -> Result<Pipeline, RegistryError> {
        debug!("Looking up pipeline for operation: {:?}", spec.operation);

        // Pre-allocate stages Vec. Typical pipelines have 2-4 stages (source → decoder → plugin)
        let mut stages = Vec::with_capacity(4);

        // 1. Recursively resolve sources
        for source in &spec.sources {
            let source_pipeline = self.lookup(source)?;
            stages.extend(source_pipeline.stages);
        }

        // 2. Determine input type (output of last source stage, or raw file)
        let input_type = if let Some(last) = stages.last() {
            last.output_type.clone()
        } else {
            // No source pipeline - must be DataSource
            match &spec.operation {
                Operation::DataSource(ds) => ds.format_hint()?,
                _ => {
                    return Err(RegistryError::NoSource);
                }
            }
        };

        // 3. Find plugin that converts input_type → desired output
        let output_type = spec.operation.output_type_name().to_string();
        let candidates = self
            .by_output
            .get(&output_type)
            .ok_or_else(|| RegistryError::NoPluginForOutput(output_type.clone()))?;

        let plugin = candidates
            .iter()
            .find(|p| p.supports_input(&input_type))
            .ok_or_else(|| RegistryError::NoPluginForConversion {
                from: input_type.clone(),
                to: output_type.clone(),
            })?;

        debug!(
            "Found plugin '{}' for {} → {}",
            plugin.name(),
            input_type,
            output_type
        );

        // 4. Add stage to pipeline
        stages.push(PipelineStage {
            plugin: Arc::clone(plugin),
            input_type,
            output_type: output_type.clone(),
            operation: spec.operation.clone(),
        });

        Ok(Pipeline { stages })
    }

    /// Get all registered plugin names
    pub fn plugin_names(&self) -> Vec<String> {
        let mut names = Vec::with_capacity(self.plugins.len());
        for key in self.plugins.keys() {
            names.push(key.clone());
        }
        names
    }

    /// Get plugin by name
    pub fn get_plugin(&self, name: &str) -> Option<Arc<dyn Plugin>> {
        self.plugins.get(name).cloned()
    }

    /// Compute transitive closure of inputs
    fn compute_transitive_inputs(&mut self) {
        // For each output type, find all input types that can reach it
        // This enables automatic pipeline composition

        self.transitive_inputs.clear();

        for (output_type, plugins) in &self.by_output {
            // Conservative estimate: most plugins have 1-3 inputs
            let mut reachable = HashSet::with_capacity(plugins.len() * 2);

            // Direct inputs
            for plugin in plugins {
                let config = plugin.config();
                for input in &config.inputs {
                    reachable.insert(input.clone());
                }
            }

            // Transitive inputs (inputs that can be converted to our inputs)
            // This would require a graph traversal - simplified for now
            // TODO: Implement full transitive closure with graph algorithm

            self.transitive_inputs
                .insert(output_type.clone(), reachable);
        }
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

/// A pipeline of plugin stages to execute
#[derive(Debug, Clone)]
pub struct Pipeline {
    /// Stages to execute in order
    pub stages: Vec<PipelineStage>,
}

impl Pipeline {
    /// Get the output type of the final stage
    pub fn output_type(&self) -> Option<&str> {
        self.stages.last().map(|s| s.output_type.as_str())
    }

    /// Get total estimated processing time
    pub fn estimated_duration(&self) -> std::time::Duration {
        // Simple estimation based on stage count
        // TODO: Use plugin performance characteristics for better estimates
        std::time::Duration::from_secs(self.stages.len() as u64 * 2)
    }
}

/// A single stage in a pipeline
#[derive(Clone)]
pub struct PipelineStage {
    /// The plugin to execute
    pub plugin: Arc<dyn Plugin>,

    /// Input type this stage expects
    pub input_type: String,

    /// Output type this stage produces
    pub output_type: String,

    /// Operation parameters
    pub operation: Operation,
}

impl std::fmt::Debug for PipelineStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PipelineStage")
            .field("plugin_name", &self.plugin.name())
            .field("input_type", &self.input_type)
            .field("output_type", &self.output_type)
            .field("operation", &self.operation)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::PluginData;
    use crate::plugin::{CacheConfig, PerformanceConfig, RuntimeConfig};
    use crate::{Context, PluginRequest, PluginResponse};
    use std::time::{Duration, SystemTime};

    // Mock plugin for testing
    struct MockPlugin {
        config: PluginConfig,
    }

    #[async_trait::async_trait]
    impl Plugin for MockPlugin {
        fn name(&self) -> &str {
            &self.config.name
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
        ) -> Result<PluginResponse, crate::error::PluginError> {
            Ok(PluginResponse {
                output: PluginData::Bytes(vec![]),
                duration: Duration::from_secs(1),
                warnings: vec![],
            })
        }
    }

    fn create_test_config(name: &str, inputs: Vec<&str>, outputs: Vec<&str>) -> PluginConfig {
        let mut input_strings = Vec::with_capacity(inputs.len());
        input_strings.extend(inputs.iter().map(|s| s.to_string()));
        let mut output_strings = Vec::with_capacity(outputs.len());
        output_strings.extend(outputs.iter().map(|s| s.to_string()));
        PluginConfig {
            name: name.to_string(),
            description: "Test plugin".to_string(),
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
        }
    }

    #[test]
    fn test_registry_register_plugin() {
        let mut registry = Registry::new();

        let plugin = Arc::new(MockPlugin {
            config: create_test_config("test", vec!["mp4"], vec!["Audio"]),
        });

        registry.register(plugin);

        assert_eq!(registry.plugin_names().len(), 1);
        assert!(registry.get_plugin("test").is_some());
    }

    #[test]
    fn test_registry_lookup_by_output() {
        let mut registry = Registry::new();

        let plugin = Arc::new(MockPlugin {
            config: create_test_config("audio_extract", vec!["mp4", "mov"], vec!["Audio"]),
        });

        registry.register(plugin);

        // Should find plugin for Audio output
        assert!(registry.by_output.contains_key("Audio"));
        assert_eq!(registry.by_output.get("Audio").unwrap().len(), 1);
    }
}
