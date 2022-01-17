mod glob;

use std::path::PathBuf;

// Re-export
pub use self::glob::Glob;

/// Represents the ability to enumerate paths
pub trait Provider {
    fn scan(&self) -> Vec<PathBuf>;
}
