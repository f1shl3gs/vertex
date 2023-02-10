use quote::{quote, ToTokens};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::Comma;
use syn::{parse_quote, parse_quote_spanned, AttributeArgs, DeriveInput, Path};

use crate::errors::Errors;
use crate::parse_attrs::parse_attr_litstr;

pub fn configurable_component_impl(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let attrs = syn::parse_macro_input!(args as AttributeArgs);
    let input = syn::parse_macro_input!(input as DeriveInput);

    let errs = &Errors::default();
    let ident = &input.ident;
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

    let struct_attrs = StructAttrs::parse(errs, &attrs);
    let name = match struct_attrs.name {
        Some(name) => name,
        _ => {
            errs.err(
                &input.span(),
                "`name` is required in `#[configurable_component]`",
            );

            return errs.into_token_stream().into();
        }
    };
    let description = struct_attrs.description.map(|d| d.value());
    let component_type = match struct_attrs.component_type {
        Some(ct) => ct,
        _ => {
            errs.err(&input.span(), "`provider`, `source`, `transform` or `sink` is required in `#[configurable_component]`");
            return errs.into_token_stream().into();
        }
    };
    if !errs.is_empty() {
        return errs.into_token_stream().into();
    }

    // inventory staff
    let desc_type: syn::Type = match component_type {
        ComponentType::Provider => parse_quote! { ::configurable::component::ProviderDescription },
        ComponentType::Source => parse_quote! { ::configurable::component::SourceDescription },
        ComponentType::Transform => {
            parse_quote! { ::configurable::component::TransformDescription }
        }
        ComponentType::Sink => parse_quote! { ::configurable::component::SinkDescription },
    };

    // Generate and apply all of the necessary derives.
    let mut derives = Punctuated::<Path, Comma>::new();
    derives.push(parse_quote_spanned! {ident.span()=>
        ::configurable::Configurable
    });
    if !struct_attrs.no_ser {
        derives.push(parse_quote_spanned! {input.ident.span()=>
            ::serde::Serialize
        });
    }
    if !struct_attrs.no_deser {
        derives.push(parse_quote_spanned! {input.ident.span()=>
            ::serde::Deserialize
        });
    }

    let maybe_description = description.map(|value| {
        quote! {
            #[configurable(description = #value)]
        }
    });

    let mut configurable_component = quote!(
        #[derive(#derives)]
        #maybe_description
        #input

        impl #impl_generics ::configurable::NamedComponent for #ident #type_generics #where_clause {
            // const NAME: &'static str = #name;
            fn component_name(&self) -> &'static str {
                #name
            }
        }

        // TODO: this should be removed, once configurable_component used for all config.
        impl #impl_generics ::configurable::GenerateConfig for #ident #type_generics #where_clause {
            fn generate_config() -> String {
                ::configurable::generate_example::<#ident>()
            }
        }

        ::inventory::submit! {
            #desc_type::new::<#ident>(#name)
        }
    );

    errs.to_tokens(&mut configurable_component);

    configurable_component.into()
}

#[derive(Clone, Debug, Default)]
pub enum ComponentType {
    Provider,
    #[default]
    Source,
    Transform,
    Sink,
}

#[derive(Default)]
struct StructAttrs {
    name: Option<syn::LitStr>,
    description: Option<syn::LitStr>,
    component_type: Option<ComponentType>,

    no_ser: bool,
    no_deser: bool,
}

impl StructAttrs {
    fn parse(errs: &Errors, args: &AttributeArgs) -> Self {
        let mut this = Self::default();

        for nm in args {
            let meta = if let Some(m) = errs.expect_nested_meta(nm) {
                m
            } else {
                continue;
            };

            let name = meta.path();
            if name.is_ident("name") {
                if let Some(m) = errs.expect_meta_name_value(meta) {
                    parse_attr_litstr(errs, m, &mut this.name);
                }
            } else if name.is_ident("description") {
                if let Some(m) = errs.expect_meta_name_value(meta) {
                    parse_attr_litstr(errs, m, &mut this.description);
                }
            } else if name.is_ident("provider") {
                this.component_type = Some(ComponentType::Provider);
            } else if name.is_ident("source") {
                this.component_type = Some(ComponentType::Source);
            } else if name.is_ident("transform") {
                this.component_type = Some(ComponentType::Transform);
            } else if name.is_ident("sink") {
                this.component_type = Some(ComponentType::Sink);
            } else if name.is_ident("no_ser") {
                this.no_ser = true
            } else if name.is_ident("no_deser") {
                this.no_deser = true
            } else {
                errs.err(
                    &name.span(),
                    "Expect `name`, `description`, `provider`, `source`, `transforms` or `sink`",
                )
            }
        }

        this
    }
}
