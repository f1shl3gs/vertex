#![recursion_limit = "256"]

mod configurable;
mod configurable_component;
mod errors;
mod parse_attrs;

use proc_macro::TokenStream;

#[proc_macro_derive(Configurable, attributes(configurable))]
pub fn derive_configurable(input: TokenStream) -> TokenStream {
    configurable::derive_configurable_impl(input)
}

#[proc_macro_attribute]
pub fn configurable_component(attrs: TokenStream, item: TokenStream) -> TokenStream {
    configurable_component::configurable_component_impl(attrs, item)
}
