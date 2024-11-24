use proc_macro2::{Ident, TokenStream};
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{DeriveInput, Fields, LitStr, Result};

use crate::parse_attrs::{is_doc_attr, parse_attr_doc, Description, FieldAttrs, TypeAttrs};

pub fn derive_configurable_impl(input: proc_macro::TokenStream) -> Result<TokenStream> {
    let input = syn::parse::<DeriveInput>(input)?;
    let type_attrs = &TypeAttrs::parse(&input)?;

    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let ref_name = name.to_string();

    let generate_schema = match &input.data {
        syn::Data::Struct(ds) => impl_from_struct(type_attrs, &input.generics, ds),
        syn::Data::Enum(de) => impl_from_enum(&input.ident, type_attrs, &input.generics, de),
        syn::Data::Union(_) => Err(syn::Error::new(
            input.span(),
            "#[configurable_component(...)] cannot be applied to unions",
        )),
    }?;

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

    Ok(configurable_impl)
}

fn impl_from_struct(
    type_attrs: &TypeAttrs,
    _generic_args: &syn::Generics,
    ds: &syn::DataStruct,
) -> Result<TokenStream> {
    let fields = match &ds.fields {
        Fields::Named(fields) => fields,
        Fields::Unnamed(_) => {
            return Err(syn::Error::new(
                ds.struct_token.span(),
                "`#[configurable_component(...)]` is not currently supported on tuple structs",
            ))
        }
        Fields::Unit => {
            return Ok(quote!(
                fn generate_schema(
                    schema_gen: &mut ::configurable::schema::SchemaGenerator,
                ) -> std::result::Result<
                    ::configurable::schema::SchemaObject,
                    ::configurable::GenerateError,
                > {
                    Ok(::configurable::schema::generate_empty_struct_schema())
                }
            ));
        }
    };
    let maybe_description = type_attrs
        .description
        .as_ref()
        .map(|desc| quote!( schema.metadata().description = Some(#desc); ));

    let mut any_flatten = false;
    let mut mapped_fields = Vec::with_capacity(fields.named.len());
    for field in &fields.named {
        let field_attrs = FieldAttrs::parse(field)?;
        if field_attrs.skip {
            continue;
        }

        if field_attrs.flatten {
            any_flatten = true;
        }

        mapped_fields.push(generate_named_struct_field(field, field_attrs));
    }

    let generated = if any_flatten {
        quote!(
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
                );
            }
        )
    } else {
        quote!(
            #(#mapped_fields)*

            let mut schema = ::configurable::schema::generate_struct_schema(
                properties,
                required,
                None,
            );
        )
    };

    Ok(quote!(
        fn generate_schema(schema_gen: &mut ::configurable::schema::SchemaGenerator)
            -> std::result::Result<::configurable::schema::SchemaObject, ::configurable::GenerateError>
        {
            let mut properties = ::configurable::IndexMap::new();
            let mut required = ::std::collections::BTreeSet::new();

            #generated

            #maybe_description

            Ok(schema)
        }
    ))
}

fn generate_named_struct_field(field: &syn::Field, field_attrs: FieldAttrs) -> TokenStream {
    let field_typ = &field.ty;
    let maybe_default = field_attrs.maybe_default(field_typ);

    let field_key = if let Some(renamed) = field_attrs.rename {
        renamed.value()
    } else {
        field.ident.clone().expect("filed has a name").to_string()
    };

    // If the field is flattened, we store it into a different list of flattened
    // subschemas vs adding it directly as a field via `properties`/`required`.
    let insert_fields = if field_attrs.flatten {
        quote!( flattened_subschemas.push(subschema); )
    } else {
        quote!( properties.insert(#field_key, subschema); )
    };

    let maybe_required = field_attrs
        .required
        .then(|| quote!( required.insert(#field_key); ));

    let maybe_description = field_attrs
        .description
        .map(|desc| quote!( subschema.metadata().description = Some(#desc); ));

    let maybe_deprecated = field_attrs
        .deprecated
        .then(|| quote!( subschema.metadata().deprecated = true; ));

    let maybe_format = field_attrs
        .format
        .map(|ls| quote!( subschema.format = Some(#ls); ));
    let maybe_example = field_attrs.example.map(
        |example| quote!( subschema.metadata().examples = vec![ ::serde_json::Value::from( #example ) ]; ),
    );

    quote!({
        let mut subschema = ::configurable::schema::get_or_generate_schema::<#field_typ>(schema_gen)?;

        #maybe_format

        #maybe_description
        #maybe_deprecated
        #maybe_default
        #maybe_example

        #maybe_required

        #insert_fields
    })
}

fn impl_from_enum(
    _name: &Ident,
    type_attrs: &TypeAttrs,
    _generic_args: &syn::Generics,
    de: &syn::DataEnum,
) -> Result<TokenStream> {
    let mapped_variants = de
        .variants
        .iter()
        .map(|variant| generate_enum_variant_schema(type_attrs, variant))
        .collect::<Result<Vec<_>>>()?;

    let maybe_description = type_attrs
        .description
        .as_ref()
        .map(|desc| quote!( schema.metadata().description = Some(#desc); ));

    Ok(quote!(
        fn generate_schema(schema_gen: &mut ::configurable::schema::SchemaGenerator)
            -> std::result::Result<::configurable::schema::SchemaObject, ::configurable::GenerateError>
        {
            let mut subschemas = ::std::vec::Vec::new();

            #(#mapped_variants)*

            let mut schema = ::configurable::schema::generate_one_of_schema(&subschemas);

            #maybe_description

            Ok(schema)
        }
    ))
}

fn generate_variant_name(variant: &syn::Variant, rule: &Option<LitStr>) -> String {
    let original = variant.ident.to_string();
    match rule {
        None => original,
        Some(rule) => {
            let snake_case = || -> String {
                let mut snake = String::new();
                for (i, ch) in original.char_indices() {
                    if i > 0 && ch.is_uppercase() {
                        snake.push('_');
                    }
                    snake.push(ch.to_ascii_lowercase());
                }
                snake
            };

            match rule.value().as_str() {
                "lowercase" => original.to_ascii_lowercase(),
                "UPPERCASE" => original.to_ascii_uppercase(),
                "camelCase" => original[..1].to_ascii_lowercase() + &original[1..],
                "snake_case" => snake_case(),
                "SCREAMING_SNAKE_CASE" => snake_case().to_ascii_uppercase(),
                "kebab-case" => snake_case().replace('_', "-"),
                "SCREAMING-KEBAB-CASE" => snake_case().to_ascii_uppercase().replace('_', "-"),
                _ => original,
            }
        }
    }
}

fn generate_enum_struct_named_variant_schema(
    type_attrs: &TypeAttrs,
    variant: &syn::Variant,
) -> Result<TokenStream> {
    let mapped_fields = variant
        .fields
        .iter()
        .map(generate_named_enum_field)
        .collect::<Result<Vec<_>>>()?;

    let maybe_tag_schema = match &type_attrs.tag {
        Some(tag) => {
            let ident = generate_variant_name(variant, &type_attrs.rename_all);
            let mut description: Option<Description> = None;
            for attr in &variant.attrs {
                parse_attr_doc(attr, &mut description)?;
            }

            let maybe_tag_description = description
                .map(|description| quote!(tag_metadata.description = Some( #description );));

            quote! ({
                let mut tag_schema = ::configurable::schema::generate_const_string_schema( #ident.to_string() );
                let tag_metadata = tag_schema.metadata();

                #maybe_tag_description

                properties.insert(#tag, tag_schema);
                required.insert(#tag);
            })
        }
        None => quote!(),
    };

    Ok(quote! {
        let mut properties = ::configurable::IndexMap::new();
        let mut required = ::std::collections::BTreeSet::new();

        #maybe_tag_schema

        #(#mapped_fields)*

        ::configurable::schema::generate_struct_schema(
            properties,
            required,
            None
        )
    })
}

fn generate_enum_variant_schema(
    type_attrs: &TypeAttrs,
    variant: &syn::Variant,
) -> Result<TokenStream> {
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
            let ident = generate_variant_name(variant, &type_attrs.rename_all);

            quote! { ::configurable::schema::generate_const_string_schema( #ident.to_string() ) }
        }

        Fields::Named(_named) => generate_enum_struct_named_variant_schema(type_attrs, variant)?,

        Fields::Unnamed(_unnamed) => generate_enum_unamed_variant_schema(type_attrs, variant)?,
    };

    generate_enum_variant_subschema(variant, variant_schema)
}

fn generate_enum_unamed_variant_schema(
    type_attrs: &TypeAttrs,
    variant: &syn::Variant,
) -> Result<TokenStream> {
    let field = variant.fields.iter().next().expect("must exist");
    let field_type = &field.ty;

    if type_attrs.untagged {
        let field_attrs = FieldAttrs::parse(field)?;
        let maybe_description = field_attrs
            .description
            .map(|desc| quote!( subschema.metadata().description = Some(#desc); ));

        return Ok(quote! {
            let mut subschema = ::configurable::schema::get_or_generate_schema::<#field_type>(schema_gen)?;

            #maybe_description

            subschema
        });
    }

    let field_schema = if type_attrs.tag.is_none() {
        let variant_name = generate_variant_name(variant, &type_attrs.rename_all);

        quote!(
            let mut properties = ::configurable::IndexMap::new();
            let mut required = ::std::collections::BTreeSet::new();
            let subschema = ::configurable::schema::get_or_generate_schema::<#field_type>(schema_gen)?;
            properties.insert(#variant_name, subschema);
            required.insert(#variant_name);
            ::configurable::schema::generate_struct_schema(
                properties,
                required,
                None,
            )
        )
    } else {
        quote!(
            ::configurable::schema::get_or_generate_schema::<#field_type>(schema_gen)?
        )
    };

    let maybe_tag_schema = match &type_attrs.tag {
        Some(tag_name) => {
            let tag = generate_variant_name(variant, &type_attrs.rename_all);

            quote! {
                let tag_schema = ::configurable::schema::generate_internal_tagged_variant_schema(
                    #tag_name,
                    ::configurable::schema::generate_const_string_schema(#tag.to_string())
                );

                flattened_subschemas.push(tag_schema);
            }
        }
        None => quote!(),
    };

    Ok(quote! {
        let mut flattened_subschemas = ::std::vec::Vec::new();

        let mut subschema = {
            #field_schema
        };

        #maybe_tag_schema
        if !flattened_subschemas.is_empty() {
            ::configurable::schema::convert_to_flattened_schema(
                &mut subschema,
                flattened_subschemas
            );
        }

        subschema
    })
}

fn generate_named_enum_field(field: &syn::Field) -> Result<TokenStream> {
    let field_name = field.ident.as_ref().expect("field should be named");
    let field_key = field_name.to_string();
    let field_typ = &field.ty;

    let field_attrs = FieldAttrs::parse(field)?;
    if field_attrs.skip {
        return Ok(quote!());
    }

    let field_schema = {
        let field_type = &field.ty;
        let spanned_generate_schema = quote_spanned! {field.span() =>
            ::configurable::schema::get_or_generate_schema::<#field_type>(schema_gen)?
        };

        quote!(
            let mut subschema = #spanned_generate_schema;
        )
    };

    let maybe_default = field_attrs.maybe_default(field_typ);
    let maybe_required = field_attrs
        .required
        .then(|| quote!( required.insert(#field_key); ));
    let maybe_description = field_attrs
        .description
        .map(|desc| quote!( metadata.description = Some(#desc); ));
    let maybe_format = field_attrs
        .format
        .map(|ls| quote!( subschema.format = Some(#ls); ));
    let maybe_deprecated = field_attrs
        .deprecated
        .then(|| quote!( metadata.deprecated = true; ));

    Ok(quote!({
        #field_schema

        let metadata = subschema.metadata();

        #maybe_description
        #maybe_required
        #maybe_default
        #maybe_format
        #maybe_deprecated

        properties.insert(#field_key, subschema);
    }))
}

fn generate_enum_variant_subschema(
    variant: &syn::Variant,
    variant_schema: TokenStream,
) -> Result<TokenStream> {
    let mut desc: Option<Description> = None;

    for attr in &variant.attrs {
        if is_doc_attr(attr) {
            parse_attr_doc(attr, &mut desc)?;
        }
    }

    let maybe_description =
        desc.map(|desc| quote!( subschema.metadata().description = Some( #desc ); ));

    Ok(quote! ({
        let mut subschema = { #variant_schema };

        #maybe_description

        subschemas.push(subschema);
    }))
}
