mod description;

pub use description::{ComponentDescription, ExampleError};

/// A provider component.
pub struct ProviderComponent;
/// A sink component.
pub struct SinkComponent;
/// A source component.
pub struct SourceComponent;
/// A transform component.
pub struct TransformComponent;
/// An extension component;
pub struct ExtensionComponent;

pub type SourceDescription = ComponentDescription<SourceComponent>;
pub type TransformDescription = ComponentDescription<TransformComponent>;
pub type SinkDescription = ComponentDescription<SinkComponent>;
pub type ProviderDescription = ComponentDescription<ProviderComponent>;
pub type ExtensionDescription = ComponentDescription<ExtensionComponent>;

inventory::collect!(SourceDescription);
inventory::collect!(TransformDescription);
inventory::collect!(SinkDescription);
inventory::collect!(ProviderDescription);
inventory::collect!(ExtensionDescription);
