#![allow(dead_code)]

use std::marker::PhantomData;

use crate::GenerateConfig;

/// Description of a component.
pub struct ComponentDescription<T: Sized> {
    component_name: &'static str,
    example_value: fn() -> Option<String>,
    _component_type: PhantomData<T>,
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
            example_value: || Some(C::generate_config()),
            _component_type: PhantomData,
        }
    }
}
