use proc_macro2::TokenStream;
use quote::quote;
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
        syn::Data::Struct(ds) => impl_from_struct(type_attrs, ds),
        syn::Data::Enum(de) => impl_from_enum(type_attrs, de),
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

fn impl_from_struct(type_attrs: &TypeAttrs, ds: &syn::DataStruct) -> Result<TokenStream> {
    let content = generate_struct_like(type_attrs, &ds.fields, None)?;

    Ok(quote!(
        fn generate_schema(schema_gen: &mut ::configurable::schema::SchemaGenerator) -> ::configurable::schema::SchemaObject {
            #content
        }
    ))
}

// generate Struct or a Named Variant in enum
fn generate_struct_like(
    type_attrs: &TypeAttrs,
    fields: &Fields,
    maybe_tag: Option<(&LitStr, String)>,
) -> Result<TokenStream> {
    let fields = match &fields {
        Fields::Named(fields) => fields,
        Fields::Unnamed(_) => {
            return Err(syn::Error::new(
                fields.span(),
                "`#[configurable_component(...)]` is not currently supported on tuple structs",
            ))
        }
        Fields::Unit => {
            // struct Empty
            return Ok(quote!(
                ::configurable::schema::generate_empty_struct_schema()
            ));
        }
    };

    let maybe_description = type_attrs
        .description
        .as_ref()
        .map(|desc| quote!( Some(#desc) ))
        .unwrap_or_else(|| quote!(None));

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

    let maybe_tag_schema = match maybe_tag {
        None => quote! {},
        Some((tag_name, value)) => quote! {
            properties.insert(#tag_name, ::configurable::schema::generate_const_string_schema(
                #value.to_string(),
            ));
            required.insert(#tag_name);
        },
    };

    let generated = if any_flatten {
        quote!(
            let mut flattened_subschemas = ::std::vec::Vec::new();

            #(#mapped_fields)*

            #maybe_tag_schema

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

            schema
        )
    } else {
        quote!(
            #(#mapped_fields)*

            #maybe_tag_schema

            ::configurable::schema::generate_struct_schema(
                properties,
                required,
                #maybe_description
            )
        )
    };

    Ok(quote!(
        let mut properties = ::configurable::IndexMap::new();
        let mut required = ::std::collections::BTreeSet::new();

        #generated
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
        let mut subschema = schema_gen.subschema_for::<#field_typ>();

        #maybe_format

        #maybe_description
        #maybe_deprecated
        #maybe_default
        #maybe_example

        #maybe_required

        #insert_fields
    })
}

fn impl_from_enum(type_attrs: &TypeAttrs, de: &syn::DataEnum) -> Result<TokenStream> {
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
            -> ::configurable::schema::SchemaObject
        {
            let mut subschemas = ::std::vec::Vec::new();

            #(#mapped_variants)*

            let mut schema = ::configurable::schema::generate_one_of_schema(subschemas);

            #maybe_description

            schema
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

// Named {
//     internal: String
// }
fn generate_enum_struct_named_variant_schema(
    type_attrs: &TypeAttrs,
    variant: &syn::Variant,
) -> Result<TokenStream> {
    let maybe_tag = type_attrs
        .tag
        .as_ref()
        .map(|tag| (tag, generate_variant_name(variant, &type_attrs.rename_all)));

    generate_struct_like(type_attrs, &variant.fields, maybe_tag)
}

fn generate_enum_variant_schema(
    type_attrs: &TypeAttrs,
    variant: &syn::Variant,
) -> Result<TokenStream> {
    let maybe_tag = &type_attrs.tag.clone();
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

            match maybe_tag {
                Some(tag_name) => {
                    quote! {
                        let mut properties = ::configurable::IndexMap::new();
                        let mut required = ::std::collections::BTreeSet::new();

                        properties.insert(#tag_name, ::configurable::schema::generate_const_string_schema(
                            #ident.to_string(),
                        ));
                        required.insert(#tag_name);

                        ::configurable::schema::generate_struct_schema(
                            properties,
                            required,
                            None
                        )
                    }
                }
                None => {
                    quote! { ::configurable::schema::generate_const_string_schema( #ident.to_string() ) }
                }
            }
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
            let mut subschema = schema_gen.subschema_for::<#field_type>();

            #maybe_description

            subschema
        });
    }

    let field_schema = if type_attrs.tag.is_none() {
        let variant_name = generate_variant_name(variant, &type_attrs.rename_all);

        quote!(
            let mut properties = ::configurable::IndexMap::new();
            let mut required = ::std::collections::BTreeSet::new();
            let subschema = schema_gen.subschema_for::<#field_type>();

            properties.insert(#variant_name, subschema);
            required.insert(#variant_name);

            ::configurable::schema::generate_struct_schema(
                properties,
                required,
                None
            )
        )
    } else {
        quote!(
            schema_gen.subschema_for::<#field_type>()
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
