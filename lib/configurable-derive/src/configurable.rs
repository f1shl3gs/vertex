use proc_macro2::{Ident, TokenStream};
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::Fields;

use crate::errors::Errors;
use crate::parse_attrs::{FieldAttrs, TypeAttrs};

pub fn derive_configurable_impl(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = &syn::parse_macro_input!(input as syn::DeriveInput);

    let errs = &Errors::default();
    let name = &input.ident;

    let type_attrs = &TypeAttrs::parse(errs, input);

    let generics = &input.generics;
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let ref_name = name.to_string();

    let generate_schema = match &input.data {
        syn::Data::Struct(ds) => {
            impl_from_struct(errs, &input.ident, type_attrs, &input.generics, ds)
        }
        syn::Data::Enum(de) => impl_from_enum(errs, &input.ident, type_attrs, &input.generics, de),
        syn::Data::Union(_) => {
            errs.err(
                &input,
                "#[configurable_component(...)] cannot be applied to unions",
            );
            TokenStream::new()
        }
    };

    let configurable_impl = quote!(
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl #impl_generics ::configurable::Configurable for #name #type_generics #where_clause {
            fn reference() -> Option<&'static str> {
                let self_type_name = ::std::any::type_name::<Self>();

                if !self_type_name.starts_with(std::module_path!()) {
                    Some(std::concat!(std::module_path!(), "::", #ref_name))
                } else {
                    Some(self_type_name)
                }
            }

            #generate_schema
        }
    );

    configurable_impl.into()
}

fn impl_from_struct(
    errs: &Errors,
    _name: &Ident,
    type_attrs: &TypeAttrs,
    _generic_args: &syn::Generics,
    ds: &syn::DataStruct,
) -> TokenStream {
    let fields = match &ds.fields {
        Fields::Named(fields) => fields,
        Fields::Unnamed(_) => {
            errs.err(
                &ds.struct_token,
                "`#[configurable_component(...)]` is not currently supported on tuple structs",
            );

            return TokenStream::new();
        }
        Fields::Unit => {
            errs.err(
                &ds.struct_token,
                "`#[configurable_component(...)]` cannot be applied to unit structs",
            );

            return TokenStream::new();
        }
    };
    let maybe_description = type_attrs.description.as_ref().map(|desc| {
        let value = desc.content.value();
        quote!( metadata.description = Some(#value.to_string()); )
    });

    let mapped_fields = fields.named.iter()
        .map(|field| {
            let field_key = field.ident.clone().expect("filed has a name").to_string();
            let field_typ = &field.ty;

            let field_attrs = FieldAttrs::parse(errs, field);

            let maybe_field_required = if field_attrs.required {
                Some(quote!(
                    required.insert(#field_key.to_string());
                ))
            } else {
                None
            };

            let maybe_description = field_attrs.description.map(|desc| {
                let value = desc.content.value();

                quote!( metadata.description = Some(#value.to_string()); )
            });

            let maybe_deprecated = if field_attrs.deprecated {
                quote!( metadata.deprecated = true; )
            } else {
                quote!()
            };

            let maybe_default = if let Some(default_fn) = field_attrs.default_fn {
                let default_fn: Ident = Ident::new_raw(&default_fn.value(), default_fn.span());
                quote!( metadata.default = Some(::serde_json::Value::from( #default_fn() )); )
            } else if let Some(value) = field_attrs.default {
                quote!( metadata.default = Some(::serde_json::Value::from( #value )); )
            } else {
                quote!()
            };
            let maybe_format = field_attrs
                .format
                .map(|ls| quote!( subschema.format = Some(#ls.to_string()); ));
            let maybe_example = field_attrs.example.map(|example| {
                quote!( metadata.examples = vec![ ::serde_json::Value::from( #example ) ]; )
            });

            quote!(
                {
                    let mut subschema = ::configurable::schema::get_or_generate_schema::<#field_typ>(schema_gen)?;

                    #maybe_format

                    let metadata = subschema.metadata();

                    #maybe_description
                    #maybe_deprecated
                    #maybe_default
                    #maybe_example

                    #maybe_field_required
                    properties.insert(#field_key.to_string(), subschema);
                }
            )
        });

    let generate_schema = quote!(
        fn generate_schema(schema_gen: &mut ::configurable::schemars::gen::SchemaGenerator)
            -> std::result::Result<::configurable::schemars::schema::SchemaObject, ::configurable::GenerateError>
        {
            let mut properties = ::configurable::IndexMap::new();
            let mut required = ::std::collections::BTreeSet::new();
            let mut flattened_subschemas = ::std::vec::Vec::new();

            #(#mapped_fields)*

            let had_unflatted_properties = !properties.is_empty();
            let mut schema = ::configurable::schema::generate_struct_schema(
                properties,
                required,
                None,
            );

            if !flattened_subschemas.is_empty() {
                if !had_unflatted_properties {
                    schema = flattened_subschemas.remove(0)
                }

                ::configurable::schema::convert_to_flattened_schema(
                    &mut schema,
                    flattened_subschemas,
                )
            }

            let metadata = schema.metadata();
            #maybe_description

            Ok(schema)
        }
    );

    generate_schema
}

fn generate_struct_field(field: &syn::Field) -> TokenStream {
    let field_type = &field.ty;
    let spanned_generate_schema = quote_spanned! {field.span() =>
        ::configurable::schema::get_or_generate_schema::<#field_type>(schema_gen)?
    };

    quote!(
        let mut subschema = #spanned_generate_schema;
    )
}

fn impl_from_enum(
    _errs: &Errors,
    _name: &Ident,
    type_attrs: &TypeAttrs,
    _generic_args: &syn::Generics,
    de: &syn::DataEnum,
) -> TokenStream {
    let mapped_variants = de.variants.iter().map(generate_enum_variant_schema);

    let maybe_description = type_attrs.description.as_ref().map(|desc| {
        let desc = desc.content.value();
        quote!( metadata.description = Some(#desc.to_string()); )
    });

    quote! {
        fn generate_schema(schema_gen: &mut ::configurable::schemars::gen::SchemaGenerator)
            -> std::result::Result<::configurable::schemars::schema::SchemaObject, ::configurable::GenerateError>
        {
            let mut subschemas = ::std::vec::Vec::new();

            #(#mapped_variants)*

            let mut schema = ::configurable::schema::generate_one_of_schema(&subschemas);
            let metadata = schema.metadata();

            #maybe_description

            Ok(schema)
        }
    }
}

fn generate_enum_struct_named_variant_schema(variant: &syn::Variant) -> TokenStream {
    let mapped_fields = variant.fields.iter().map(generate_named_enum_field);

    quote! {
        {
            let mut properties = ::configurable::IndexMap::new();
            let mut required = ::std::collections::BTreeSet::new();

            #(#mapped_fields)*

            ::configurable::schema::generate_struct_schema(
                properties,
                required,
                None
            )
        }
    }
}

fn generate_enum_variant_schema(variant: &syn::Variant) -> TokenStream {
    //
    //     enum Variant {
    //         Unit,
    //         Named{
    //             internal: String
    //         },
    //         Unnamed(External),

    //     }
    //
    let variant_schema = match &variant.fields {
        Fields::Unit => {
            let ident = &variant.ident.to_string();
            quote! { ::configurable::schema::generate_const_string_schema( #ident.to_string() ) }
        }

        Fields::Named(_named) => generate_enum_struct_named_variant_schema(variant),

        Fields::Unnamed(_unnamed) => generate_enum_unamed_variant_schema(variant),
    };

    generate_enum_variant_subschema(variant, variant_schema)
}

fn generate_enum_unamed_variant_schema(variant: &syn::Variant) -> TokenStream {
    let field = variant.fields.iter().next().expect("must exist");
    let field_schema = generate_struct_field(field);

    quote! {
        {
            #field_schema
            subschema
        }
    }
}

fn generate_named_enum_field(field: &syn::Field) -> TokenStream {
    let field_name = field.ident.as_ref().expect("field should be named");
    let field_key = field_name.to_string();

    let errs = &Errors::default();
    let field_attrs = FieldAttrs::parse(errs, field);
    let field_schema = generate_struct_field(field);

    let maybe_required = if field_attrs.required {
        quote!( required.insert(#field_key.to_string()); )
    } else {
        quote!()
    };

    let maybe_description = if let Some(desc) = field_attrs.description {
        let value = desc.content.value();
        quote!( metadata.description = Some(#value.to_string()); )
    } else {
        quote!()
    };
    let maybe_default = if let Some(default_fn) = field_attrs.default_fn {
        let default_fn: Ident = Ident::new_raw(&default_fn.value(), default_fn.span());
        quote!( metadata.default = Some(::serde_json::Value::from( #default_fn() )); )
    } else if let Some(value) = field_attrs.default {
        quote!( metadata.default = Some(::serde_json::Value::from( #value )); )
    } else {
        quote!()
    };
    let maybe_format = field_attrs
        .format
        .map(|ls| quote!( subschema.format = Some(#ls.to_string()); ));

    let maybe_deprecated = if field_attrs.deprecated {
        quote!( metadata.deprecated = true; )
    } else {
        quote!()
    };

    quote!(
        {
            #field_schema

            let metadata = subschema.metadata();

            #maybe_description
            #maybe_required
            #maybe_default
            #maybe_format
            #maybe_deprecated

            properties.insert(#field_key.to_string(), subschema);
        }
    )
}

fn generate_enum_variant_subschema(
    _variant: &syn::Variant,
    variant_schema: TokenStream,
) -> TokenStream {
    quote! {
        {
            let mut subschema = { #variant_schema };

            subschemas.push(subschema);
        }
    }
}
