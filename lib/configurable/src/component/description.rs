use std::fmt::{Display, Formatter};
use std::marker::PhantomData;

use crate::{generate_config, Configurable};

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Clone, PartialEq)]
pub enum ExampleError {
    MissingExample,
    DoesNotExist(String),
}

impl Display for ExampleError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ExampleError::MissingExample => {
                f.write_str("unable to create an example for this component")
            }
            ExampleError::DoesNotExist(s) => write!(f, r#"type "{}" does not exist"#, s),
        }
    }
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
    /// The type parameter `C` must be the component's configuration type that implements `Configurable`.
    pub const fn new<C: Configurable>(component_name: &'static str) -> Self {
        ComponentDescription {
            component_name,
            example: || Some(generate_config::<C>()),
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
