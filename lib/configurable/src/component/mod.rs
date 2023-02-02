mod description;
pub mod generate;

use description::ComponentDescription;

/// A provider component.
pub struct ProviderComponent;
/// A sink component.
pub struct SinkComponent;
/// A source component.
pub struct SourceComponent;
// A transform component.
pub struct TransformComponent;

pub type SourceDescription = ComponentDescription<SourceComponent>;
pub type TransformDescription = ComponentDescription<TransformComponent>;
pub type SinkDescription = ComponentDescription<SinkComponent>;
pub type ProviderDescription = ComponentDescription<ProviderComponent>;

inventory::collect!(SourceDescription);
inventory::collect!(TransformDescription);
inventory::collect!(SinkDescription);
inventory::collect!(ProviderDescription);
