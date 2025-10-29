mod description;

pub use description::{ComponentDescription, ExampleError};

pub struct SecretComponent;
/// A provider component.
pub struct ProviderComponent;
/// An extension component;
pub struct ExtensionComponent;
/// A source component.
pub struct SourceComponent;
/// A transform component.
pub struct TransformComponent;
/// A sink component.
pub struct SinkComponent;

pub type SecretDescription = ComponentDescription<SecretComponent>;
pub type ProviderDescription = ComponentDescription<ProviderComponent>;
pub type ExtensionDescription = ComponentDescription<ExtensionComponent>;
pub type SourceDescription = ComponentDescription<SourceComponent>;
pub type TransformDescription = ComponentDescription<TransformComponent>;
pub type SinkDescription = ComponentDescription<SinkComponent>;

inventory::collect!(SecretDescription);
inventory::collect!(ProviderDescription);
inventory::collect!(ExtensionDescription);
inventory::collect!(SourceDescription);
inventory::collect!(TransformDescription);
inventory::collect!(SinkDescription);
