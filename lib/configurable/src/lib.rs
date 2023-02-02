pub mod component;
mod configurable;
mod errors;
mod named;
pub mod schema;

pub use errors::GenerateError;
pub use named::NamedComponent;

// re-export
pub use component::generate::GenerateConfig;
pub use configurable::Configurable;
pub use indexmap::IndexMap;
pub use schemars;

// Re-export of the `#[configurable_component]` and `#[derive(Configurable)]` proc macros.
pub use configurable_derive::Configurable;
