use std::marker::PhantomData;

use super::GenerateConfig;

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Clone, PartialEq)]
pub enum ExampleError {
    // #[error("unable to create an example for this component")]
    MissingExample,
    // #[error("type '{0}' does not exist")]
    DoesNotExist(String),
}

/// Description of a component.
pub struct ComponentDescription<T: Sized> {
    component_name: &'static str,
    example: fn() -> Option<String>,
    component_type: PhantomData<T>,
}

impl<T> ComponentDescription<T>
where
    T: Sized + 'static,
    inventory::iter<ComponentDescription<T>>: IntoIterator<Item = &'static ComponentDescription<T>>,
{
    /// Creates a new `ComponentDescription`.
    ///
    /// This creates a component description for a component identified both by the given component
    /// type `T` and the component name. As such, if `T` is `SourceComponent`, and the name is
    /// `stdin`, you would say that the component is a "source called `stdin`".
    ///
    /// The type parameter `C` must be the component's configuration type that implements `GenerateConfig`.
    pub const fn new<C: GenerateConfig>(component_name: &'static str) -> Self {
        ComponentDescription {
            component_name,
            example: || Some(C::generate_config()),
            component_type: PhantomData,
        }
    }

    /// TODO: deprecate this once we can generate example from JSON Schema
    ///
    /// Returns an example config for a plugin identified by tis type
    pub fn example(type_str: &str) -> Result<String, ExampleError> {
        inventory::iter::<ComponentDescription<T>>
            .into_iter()
            .find(|t| t.component_name == type_str)
            .ok_or_else(|| ExampleError::DoesNotExist(type_str.to_string()))
            .and_then(|t| (t.example)().ok_or(ExampleError::MissingExample))
    }

    /// Returns a sorted Vec of all plugins registered of a type
    pub fn types() -> Vec<&'static str> {
        let mut types = Vec::new();
        for definition in inventory::iter::<ComponentDescription<T>> {
            types.push(definition.component_name);
        }

        types.sort_unstable();
        types
    }
}
