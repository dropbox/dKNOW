//! Execution context for plugin operations

use serde::{Deserialize, Serialize};

/// Execution mode that determines optimization priorities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionMode {
    /// Debug mode - verbose logging, intermediate outputs
    Debug,

    /// Performance mode - streaming results, minimum latency
    Performance,

    /// Bulk mode - maximum throughput, batch processing
    Bulk,
}

/// Context passed to plugins during execution
#[derive(Debug, Clone)]
pub struct Context {
    /// Execution mode
    pub mode: ExecutionMode,

    /// Whether to save intermediate outputs
    pub save_intermediates: bool,

    /// Whether verbose logging is enabled
    pub verbose: bool,
}

impl Context {
    /// Create a debug context
    pub fn debug() -> Self {
        Self {
            mode: ExecutionMode::Debug,
            save_intermediates: true,
            verbose: true,
        }
    }

    /// Create a performance context
    pub fn performance() -> Self {
        Self {
            mode: ExecutionMode::Performance,
            save_intermediates: false,
            verbose: false,
        }
    }

    /// Create a bulk context
    pub fn bulk() -> Self {
        Self {
            mode: ExecutionMode::Bulk,
            save_intermediates: false,
            verbose: false,
        }
    }

    /// Create a custom context
    pub fn new(mode: ExecutionMode) -> Self {
        match mode {
            ExecutionMode::Debug => Self::debug(),
            ExecutionMode::Performance => Self::performance(),
            ExecutionMode::Bulk => Self::bulk(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_context() {
        let ctx = Context::debug();
        assert_eq!(ctx.mode, ExecutionMode::Debug);
        assert!(ctx.save_intermediates);
        assert!(ctx.verbose);
    }

    #[test]
    fn test_performance_context() {
        let ctx = Context::performance();
        assert_eq!(ctx.mode, ExecutionMode::Performance);
        assert!(!ctx.save_intermediates);
        assert!(!ctx.verbose);
    }

    #[test]
    fn test_bulk_context() {
        let ctx = Context::bulk();
        assert_eq!(ctx.mode, ExecutionMode::Bulk);
        assert!(!ctx.save_intermediates);
        assert!(!ctx.verbose);
    }
}
