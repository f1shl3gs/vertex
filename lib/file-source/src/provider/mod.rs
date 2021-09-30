mod glob;

use std::path::PathBuf;
use std::slice::Iter;

/// Represents the ability to enumerate paths
///
/// For use at [`crate::Server`]
///
/// # Note
///
/// Ideally we'd use an iterator with bound lifetime here:
///
/// ```ignore
/// type Iter<'a>: Iterator<Item = PathBuf> + 'a;
/// fn paths(&self) -> Self::Iter<'_>;
/// ```
///
/// However, this currently unavailable at Rust
/// See: <https://github.com/rust-lang/rust/issues/44265>
///
/// We use an `IntoIter` here as a workaround.
pub trait Provider {
    /// Provides the iterator that returns paths
    type IntoIter: IntoIterator<Item = PathBuf>;

    /// Provides a set of paths
    fn paths(&self) -> Self::IntoIter;
}