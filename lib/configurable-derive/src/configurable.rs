use proc_macro2::{Ident, TokenStream};
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{Fields, Path};

use crate::errors::Errors;
use crate::parse_attrs::{is_doc_attr, parse_attr_doc, Description, FieldAttrs, TypeAttrs};

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
        const _: () = {
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
        };
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

            return quote!(
                fn generate_schema(
                    schema_gen: &mut ::configurable::schemars::gen::SchemaGenerator,
                ) -> std::result::Result<
                    ::configurable::schemars::schema::SchemaObject,
                    ::configurable::GenerateError,
                > {
                    Ok(::configurable::schema::generate_empty_struct_schema())
                }
            );
        }
    };
    let maybe_description = type_attrs.description.as_ref().map(|desc| {
        let value = desc.content.value();
        quote!( metadata.description = Some(#value.to_string()); )
    });

    let mapped_fields = fields
        .named
        .iter()
        .map(|field| generate_named_struct_field(errs, type_attrs, field));

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

fn generate_named_struct_field(
    errs: &Errors,
    _type_attrs: &TypeAttrs,
    field: &syn::Field,
) -> TokenStream {
    let field_key = field.ident.clone().expect("filed has a name").to_string();
    let field_typ = &field.ty;

    let field_attrs = FieldAttrs::parse(errs, field);
    if field_attrs.skip {
        return quote!();
    }

    // If the field is flattened, we store it into a different list of flattened
    // subschemas vs adding it directly as a field via `properties`/`required`.
    let insert_fields = if field_attrs.flatten {
        quote!( flattened_subschemas.push(subschema); )
    } else {
        quote!( properties.insert(#field_key.to_string(), subschema); )
    };

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

    let maybe_default = if let Some(default_fn) = field_attrs.default {
        let default_value = if default_fn.value().is_empty() {
            quote!( let default_value: #field_typ = Default::default(); )
        } else {
            let default_fn: Path = default_fn.parse().expect("valid serde default value");
            quote!( let default_value = #default_fn(); )
        };

        let json_value = match field_attrs.serde_with {
            Some(serde_with) => {
                let serde_with: Path = serde_with.parse().expect("valid serde with value");

                quote! {
                    let value = #serde_with::serialize(&default_value, serde_json::value::Serializer)
                        .expect("serialize default value");
                }
            }
            None => {
                quote! {
                    let value = ::serde_json::to_value( & default_value )
                        .expect("transform default value to serde_json::Value");
                }
            }
        };

        quote! {
            #default_value
            #json_value

            metadata.default = Some(value);
        }
    } else {
        quote!()
    };

    let maybe_format = field_attrs
        .format
        .map(|ls| quote!( subschema.format = Some(#ls.to_string()); ));
    let maybe_example = field_attrs.example.map(
        |example| quote!( metadata.examples = vec![ ::serde_json::Value::from( #example ) ]; ),
    );

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

            #insert_fields
        }
    )
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
    errs: &Errors,
    _name: &Ident,
    type_attrs: &TypeAttrs,
    _generic_args: &syn::Generics,
    de: &syn::DataEnum,
) -> TokenStream {
    let mapped_variants = de
        .variants
        .iter()
        .map(|variant| generate_enum_variant_schema(errs, type_attrs, variant));

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

fn apply_rename(variant: &str, rule: &syn::LitStr) -> String {
    let snake_case = || -> String {
        let mut snake = String::new();
        for (i, ch) in variant.char_indices() {
            if i > 0 && ch.is_uppercase() {
                snake.push('_');
            }
            snake.push(ch.to_ascii_lowercase());
        }
        snake
    };

    match rule.value().as_str() {
        "lowercase" => variant.to_ascii_lowercase(),
        "UPPERCASE" => variant.to_ascii_uppercase(),
        "camelCase" => variant[..1].to_ascii_lowercase() + &variant[1..],
        "snake_case" => snake_case(),
        "SCREAMING_SNAKE_CASE" => snake_case().to_ascii_uppercase(),
        "kebab-case" => snake_case().replace('_', "-"),
        "SCREAMING-KEBAB-CASE" => snake_case().to_ascii_uppercase().replace('_', "-"),
        _ => variant.to_owned(),
    }
}

fn generate_enum_struct_named_variant_schema(
    type_attrs: &TypeAttrs,
    variant: &syn::Variant,
) -> TokenStream {
    let mapped_fields = variant.fields.iter().map(generate_named_enum_field);
    let maybe_tag_schema = match &type_attrs.tag {
        Some(tag) => {
            let ident = variant.ident.to_string();
            let ident = match &type_attrs.rename_all {
                Some(rule) => apply_rename(&ident, rule),
                None => ident,
            };

            quote! {
                {
                    let tag_schema = ::configurable::schema::generate_const_string_schema( #ident.to_string() );
                    properties.insert(#tag.to_string(), tag_schema);
                }
            }
        }
        None => quote!(),
    };

    quote! {
        {
            let mut properties = ::configurable::IndexMap::new();
            let mut required = ::std::collections::BTreeSet::new();

            #maybe_tag_schema
            #(#mapped_fields)*

            ::configurable::schema::generate_struct_schema(
                properties,
                required,
                None
            )
        }
    }
}

fn generate_enum_variant_schema(
    errs: &Errors,
    type_attrs: &TypeAttrs,
    variant: &syn::Variant,
) -> TokenStream {
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
            let ident = match &type_attrs.rename_all {
                Some(rule) => apply_rename(&variant.ident.to_string(), rule),
                None => variant.ident.to_string(),
            };

            quote! { ::configurable::schema::generate_const_string_schema( #ident.to_string() ) }
        }

        Fields::Named(_named) => generate_enum_struct_named_variant_schema(type_attrs, variant),

        Fields::Unnamed(_unnamed) => generate_enum_unamed_variant_schema(variant),
    };

    generate_enum_variant_subschema(errs, variant, variant_schema)
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
    let field_typ = &field.ty;

    let errs = &Errors::default();
    let field_attrs = FieldAttrs::parse(errs, field);
    if field_attrs.skip {
        return quote!();
    }

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

    let maybe_default = if let Some(default_fn) = field_attrs.default {
        let default_value = if default_fn.value().is_empty() {
            quote!( let default_value: #field_typ = Default::default(); )
        } else {
            let default_fn: Path = default_fn.parse().expect("valid serde default value");
            quote!( let default_value = #default_fn(); )
        };

        let json_value = match field_attrs.serde_with {
            Some(serde_with) => {
                let serde_with: Path = serde_with.parse().expect("valid serde with value");

                quote! {
                    let value = #serde_with::serialize(&default_value, serde_json::value::Serializer)
                        .expect("serialize default value");
                }
            }
            None => {
                quote! {
                    let value = ::serde_json::to_value( & default_value )
                        .expect("transform default value to serde_json::Value");
                }
            }
        };

        quote! {
            #default_value
            #json_value

            metadata.default = Some(value);
        }
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
    errs: &Errors,
    variant: &syn::Variant,
    variant_schema: TokenStream,
) -> TokenStream {
    let mut desc: Option<Description> = None;

    for attr in &variant.attrs {
        if is_doc_attr(attr) {
            parse_attr_doc(errs, attr, &mut desc);
        }
    }

    let maybe_description = match desc {
        Some(desc) => {
            let desc = desc.content;
            quote!( metadata.description = Some(#desc.to_string()); )
        }
        None => quote!(),
    };

    quote! {
        {
            let mut subschema = { #variant_schema };
            let metadata = subschema.metadata();

            #maybe_description

            subschemas.push(subschema);
        }
    }
}
