use std::marker::PhantomData;

use snafu::Snafu;

pub trait GenerateConfig {
    fn generate_config() -> serde_yaml::Value;
}

#[macro_export]
macro_rules! impl_generate_config_from_default {
    ($type:ty) => {
        impl $crate::config::GenerateConfig for $type {
            fn generate_config() -> serde_yaml::Value {
                serde_yaml::to_value(&Self::default()).unwrap()
            }
        }
    };
}

#[macro_export]
macro_rules! register_source_config {
    ($name:expr, $type:ty) => {
        inventory::submit! {
            $crate::config::SourceDescription::new::<$type>($name)
        }
    };
    (name = $name:expr, typ =  $type:ty) => {
        inventory::submit! {
            $crate::config::SourceDescription::new::<$type>($name)
        }
    };
}

#[derive(Debug, Snafu, Clone, PartialEq)]
pub enum ExampleError {
    #[snafu(display("unable to create an example for this component"))]
    MissingExample,
    #[snafu(display("type '{}' does not exist", type_str))]
    DoesNotExist { type_str: String },
}

/// Describes a component plugin storing its type name, an example config,
/// and other useful information about the plugin
pub struct ComponentDescription<T: Sized> {
    pub type_str: &'static str,
    example: serde_yaml::Value,
    component_type: PhantomData<T>,
}

impl<T> ComponentDescription<T>
where
    T: 'static + Sized,
    inventory::iter<ComponentDescription<T>>:
        std::iter::IntoIterator<Item = &'static ComponentDescription<T>>,
{
    /// Creates a new component plugin description.
    /// Configuration example is generated by the `GenerateConfig` trait.
    pub fn new<B: GenerateConfig>(type_str: &'static str) -> Self {
        Self {
            type_str,
            example: B::generate_config(),
            component_type: PhantomData,
        }
    }

    /// Returns an example config for a plugin identified by tis type
    pub fn example(type_str: &str) -> Result<serde_yaml::Value, ExampleError> {
        inventory::iter::<ComponentDescription<T>>
            .into_iter()
            .find(|t| t.type_str == type_str)
            .ok_or_else(|| ExampleError::DoesNotExist {
                type_str: type_str.to_string(),
            })
            .map(|t| t.example.clone())
    }

    /// Returns a sorted Vec of all plugins registered of a type
    pub fn types() -> Vec<&'static str> {
        let mut types = Vec::new();
        for definition in inventory::iter::<ComponentDescription<T>> {
            types.push(definition.type_str);
        }

        types.sort_unstable();
        types
    }
}

#[cfg(test)]
pub fn test_generate_config<T>()
where
    for<'de> T: GenerateConfig + serde::Deserialize<'de>,
{
    let cfg = serde_yaml::to_string(&T::generate_config())
        .expect("Invalid config generated when stringify");

    serde_yaml::from_str::<T>(&cfg).expect("Invalid config generated");
}
