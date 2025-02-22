pub mod component;
mod configurable;
mod errors;
mod example;
mod external;
pub mod format;
mod named;
pub mod schema;

pub use errors::GenerateError;
pub use example::{generate_config, generate_config_with_schema};
pub use named::NamedComponent;

pub use configurable::{Configurable, ConfigurableString};

// re-export
pub use indexmap::IndexMap;

// Re-export of the `#[configurable_component]` and `#[derive(Configurable)]` proc macros.
pub use configurable_derive::{Configurable, configurable_component};
