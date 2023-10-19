use proc_macro2::Span;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::token::Comma;
use syn::{
    parse_quote, parse_quote_spanned, punctuated::Punctuated, DeriveInput, Expr, Lit, Meta,
    MetaNameValue, Path, Token,
};

struct Args {
    name: String,
    description: Option<String>,
    component_type: ComponentType,

    no_ser: bool,
    no_deser: bool,
}

impl Parse for Args {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let attrs = Punctuated::<Meta, Token![,]>::parse_terminated(input)?;
        let mut name = None;
        let mut component_type = ComponentType::default();
        let mut description = None;
        let mut no_ser = false;
        let mut no_deser = false;

        for meta in &attrs {
            let ident = if let Some(ident) = meta.path().get_ident() {
                ident.to_string()
            } else {
                continue;
            };

            match ident.as_str() {
                "name" => {
                    let nv = meta.require_name_value()?;
                    let value = extract_meta_string_value(nv)
                        .map_err(|span| syn::Error::new(span, "extract `name` value failed"))?;

                    name = Some(value);
                }

                "description" => {
                    let nv = meta.require_name_value()?;
                    let value = extract_meta_string_value(nv).map_err(|span| {
                        syn::Error::new(span, "parse `description` value failed")
                    })?;
                    description = Some(value);
                }

                "extension" => component_type = ComponentType::Extension,
                "provider" => component_type = ComponentType::Provider,
                "source" => component_type = ComponentType::Source,
                "transform" => component_type = ComponentType::Transform,
                "sink" => component_type = ComponentType::Sink,
                "no_ser" => no_ser = true,
                "no_deser" => no_deser = true,

                _ => {
                    return Err(syn::Error::new(meta.span(), "Expect `name`, `description`, `extension`, `provider`, `source`, `transforms` or `sink`"));
                }
            }
        }

        let name = if let Some(name) = name {
            name
        } else {
            return Err(syn::Error::new(
                input.span(),
                "`name` is required in `#[configurable_component]`",
            ));
        };

        Ok(Args {
            name,
            description,
            component_type,

            no_ser,
            no_deser,
        })
    }
}

fn extract_meta_string_value(nv: &MetaNameValue) -> Result<String, Span> {
    if let Expr::Lit(expr) = &nv.value {
        if let Lit::Str(s) = &expr.lit {
            Ok(s.value())
        } else {
            Err(expr.lit.span())
        }
    } else {
        Err(nv.value.span())
    }
}

pub fn configurable_component_impl(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let args = syn::parse_macro_input!(args as Args);
    let input = syn::parse_macro_input!(input as DeriveInput);

    let ident = &input.ident;
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

    let name = args.name;

    // inventory staff
    let desc_type: syn::Type = match args.component_type {
        ComponentType::Extension => parse_quote!(::configurable::component::ExtensionDescription),
        ComponentType::Provider => parse_quote! { ::configurable::component::ProviderDescription },
        ComponentType::Source => parse_quote! { ::configurable::component::SourceDescription },
        ComponentType::Transform => {
            parse_quote! { ::configurable::component::TransformDescription }
        }
        ComponentType::Sink => parse_quote! { ::configurable::component::SinkDescription },
    };

    // Generate and apply all of the necessary derives.
    let mut derives = Punctuated::<Path, Comma>::new();
    derives.push(parse_quote_spanned! {ident.span() =>
        Debug
    });
    derives.push(parse_quote_spanned! {ident.span()=>
        ::configurable::Configurable
    });
    if !args.no_ser {
        derives.push(parse_quote_spanned! {input.ident.span()=>
            ::serde::Serialize
        });
    }
    if !args.no_deser {
        derives.push(parse_quote_spanned! {input.ident.span()=>
            ::serde::Deserialize
        });
    }

    let maybe_description = args.description.map(|value| {
        quote! {
            #[configurable(description = #value)]
        }
    });

    let configurable_component = quote!(
        #[derive(#derives)]
        #maybe_description
        #input

        impl #ident {
            pub const NAME: &'static str = #name;
        }

        impl #impl_generics ::configurable::NamedComponent for #ident #type_generics #where_clause {
            fn component_name(&self) -> &'static str {
                #name
            }
        }

        ::inventory::submit! {
            #desc_type::new::<#ident>(#name)
        }
    );

    configurable_component.into()
}

#[derive(Clone, Debug, Default)]
pub enum ComponentType {
    Extension,
    Provider,
    #[default]
    Source,
    Transform,
    Sink,
}
