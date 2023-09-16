use std::path::PathBuf;

use glob::Pattern;

use crate::provider::Provider;

/// A glob-based path provider
///
/// Provides the paths to the files on the file system that
/// match includes patterns and don't match the exclude patterns
pub struct Glob {
    includes: Vec<String>,
    excludes: Vec<Pattern>,
}

impl Glob {
    /// Create a new [`Glob`]
    ///
    /// Returns `None` if patterns aren't valid
    pub fn new(includes: &[PathBuf], excludes: &[PathBuf]) -> Option<Self> {
        let includes = includes
            .iter()
            .map(|path| path.to_str().map(ToOwned::to_owned))
            .collect::<Option<_>>()?;
        let excludes = excludes
            .iter()
            .filter_map(|path| path.to_str().map(|path| Pattern::new(path).ok()))
            .collect::<Option<_>>()?;

        Some(Self { includes, excludes })
    }
}

impl Provider for Glob {
    fn scan(&self) -> Vec<PathBuf> {
        self.includes
            .iter()
            .flat_map(|include| {
                glob::glob(include.as_str())
                    .expect("failed to read flob pattern")
                    .flat_map(|result| result.ok())
            })
            .filter(|candidate: &PathBuf| -> bool {
                !self.excludes.iter().any(|pattern| {
                    let s = candidate.to_str().unwrap();
                    pattern.matches(s)
                })
            })
            .collect()
    }
}
