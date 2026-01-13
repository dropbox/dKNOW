//! Plugins listing command

use super::registry_helper::register_all_plugins;
use anyhow::Result;
use video_extract_core::Registry;

pub async fn list_plugins() -> Result<()> {
    println!("Available plugins:");

    let mut registry = Registry::new();

    // Register all available plugins using shared helper
    register_all_plugins(&mut registry)?;

    // Display plugin information
    for plugin_name in registry.plugin_names() {
        if let Some(plugin) = registry.get_plugin(&plugin_name) {
            let config = plugin.config();
            println!("\n{}", plugin_name);
            println!("  Description: {}", config.description);
            println!("  Inputs: {}", config.inputs.join(", "));
            println!("  Outputs: {}", config.outputs.join(", "));
            println!(
                "  Performance: {} per GB, {} MB memory",
                config.performance.avg_processing_time_per_gb,
                config.performance.memory_per_file_mb
            );
            println!("  GPU required: {}", config.config.requires_gpu);
            println!("  Experimental: {}", config.config.experimental);
        }
    }

    Ok(())
}
