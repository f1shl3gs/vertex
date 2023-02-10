pub mod component;
mod configurable;
mod errors;
pub mod example;
mod external;
pub mod format;
mod named;
pub mod schema;

pub use errors::GenerateError;
pub use example::generate_config;
pub use named::NamedComponent;

pub use crate::configurable::{Configurable, ConfigurableString};

// re-export
pub use indexmap::IndexMap;
pub use schemars;

// Re-export of the `#[configurable_component]` and `#[derive(Configurable)]` proc macros.
pub use configurable_derive::{configurable_component, Configurable};
