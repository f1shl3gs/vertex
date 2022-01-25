mod compression;
pub mod metrics;
pub mod partition;

pub use compression::*;

/// Marker trait for types that can hold a batch of events
pub trait ElementCount {
    fn element_count(&self) -> usize;
}

impl<T> ElementCount for Vec<T> {
    fn element_count(&self) -> usize {
        self.len()
    }
}

impl ElementCount for serde_json::Value {
    fn element_count(&self) -> usize {
        1
    }
}
