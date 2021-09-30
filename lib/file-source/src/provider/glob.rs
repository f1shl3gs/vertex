use std::path::PathBuf;
use glob::Pattern;
use crate::events::InternalEvents;
use crate::provider::Provider;

/// A glob-based path provider
///
/// Provides the paths to the files on the file system that
/// match include patterns and don't match the exclude patterns.
pub struct Glob<E: InternalEvents> {
    include: Vec<String>,
    exclude: Vec<Pattern>,
    emitter: E,
}

impl<E: InternalEvents> Glob<E> {
    /// Create a new [`Glob`]
    ///
    /// Returns `None` if patterns aren't valid
    pub fn new(
        include: &[PathBuf],
        exclude: &[PathBuf],
        emitter: E,
    ) -> Option<Self> {
        let include = include.iter()
            .map(|path| path.to_str().map(ToOwned::to_owned))
            .collect::<Option<_>>()?;
        let exclude = exclude.iter()
            .map(|path| path.to_str().map(|path| Pattern::new(path).ok()))
            .flatten()
            .collect::<Option<_>>()?;

        Some(Self {
            include,
            exclude,
            emitter,
        })
    }
}

impl<E: InternalEvents> Provider for Glob<E> {
    type IntoIter = Vec<PathBuf>;

    fn paths(&self) -> Self::IntoIter {
        self.include
            .iter()
            .flat_map(|include| {
                glob::glob(include.as_str())
                    .expect("failed to read glob pattern")
                    .flat_map(|val| {
                        val.map_err(|err| {
                            self.emitter
                                .emit_path_globbing_failed(err.path(), err.error());
                        })
                            .ok()
                    })
            })
            .filter(|candidate: &PathBuf| -> bool {
                !self.exclude.iter().any(|pattern| {
                    let s = candidate.to_str().unwrap();
                    pattern.matches(s)
                })
            })
            .collect()
    }
}