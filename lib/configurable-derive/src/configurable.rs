use proc_macro2::TokenStream;
use quote::quote;
use syn::spanned::Spanned;
use syn::{DeriveInput, Fields, LitStr, Result};

use crate::parse_attrs::{Description, FieldAttrs, TypeAttrs, is_doc_attr, parse_attr_doc};

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
    let maybe_desc = match &type_attrs.description {
        Some(desc) => quote! { Some(#desc) },
        None => quote! { None },
    };

    let content = generate_fields(&ds.fields, None, maybe_desc)?;

    Ok(quote!(
        fn generate_schema(schema_gen: &mut ::configurable::schema::SchemaGenerator) -> ::configurable::schema::SchemaObject {
            #content
        }
    ))
}

// generate Struct or a Named Variant in enum
fn generate_fields(
    fields: &Fields,
    maybe_tag: Option<(&LitStr, String)>,
    maybe_desc: TokenStream,
) -> Result<TokenStream> {
    let fields = match &fields {
        Fields::Named(fields) => fields,
        Fields::Unnamed(_) => {
            return Err(syn::Error::new(
                fields.span(),
                "`#[configurable_component(...)]` is not currently supported on tuple structs",
            ));
        }
        Fields::Unit => {
            // struct Empty
            return Ok(quote!(::configurable::schema::SchemaObject::new_object(
                None
            )));
        }
    };

    let mut mapped_fields = Vec::with_capacity(fields.named.len());
    for field in &fields.named {
        let field_attrs = FieldAttrs::parse(field)?;
        if field_attrs.skip {
            continue;
        }

        if field_attrs.flatten {
            let field_type = &field.ty;

            mapped_fields.push(quote!({
                schema.insert_flatten(
                    #field_type :: generate_schema( schema_gen ),
                );
            }));

            continue;
        }

        mapped_fields.push(generate_named_struct_field(field, field_attrs));
    }

    let maybe_tag = maybe_tag.map(|(key, value)| {
        quote!({
            schema.insert_tag(#key, #value);
        })
    });

    Ok(quote!(
        let mut schema = ::configurable::schema::SchemaObject::new_object( #maybe_desc );

        #maybe_tag

        #( #mapped_fields )*

        schema
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

    let maybe_required = if field_attrs.required {
        true
    } else {
        field_attrs.default.is_none() && field_attrs.default_fn.is_none()
    };

    let maybe_description = match field_attrs.description {
        None => quote! { None },
        Some(desc) => quote! { Some(#desc) },
    };

    let maybe_deprecated = field_attrs
        .deprecated
        .then(|| quote!( subschema.deprecated = true; ));

    let maybe_format = field_attrs
        .format
        .map(|format| quote!( subschema.set_format( #format ); ));

    let maybe_example = field_attrs
        .example
        .map(|example| quote!( subschema.add_example( ::serde_json::Value::from( #example ) ); ));

    if maybe_format.is_some()
        || maybe_deprecated.is_some()
        || maybe_default.is_some()
        || maybe_example.is_some()
    {
        quote!({
            let subschema = schema.insert_property(
                #field_key,
                #maybe_required,
                #maybe_description,
                schema_gen.subschema_for::<#field_typ>(),
            );

            #maybe_format
            #maybe_deprecated
            #maybe_default
            #maybe_example
        })
    } else {
        quote!({
            schema.insert_property(
                #field_key,
                #maybe_required,
                #maybe_description,
                schema_gen.subschema_for::<#field_typ>(),
            );
        })
    }
}

fn impl_from_enum(type_attrs: &TypeAttrs, de: &syn::DataEnum) -> Result<TokenStream> {
    let mapped_variants = de
        .variants
        .iter()
        .map(|variant| generate_enum_variant(type_attrs, variant))
        .collect::<Result<Vec<_>>>()?;

    let maybe_description = match type_attrs.description.as_ref() {
        None => quote! { None },
        Some(desc) => quote! { Some( #desc ) },
    };

    Ok(quote!(
        fn generate_schema(schema_gen: &mut ::configurable::schema::SchemaGenerator)
            -> ::configurable::schema::SchemaObject
        {
            let mut subschemas = ::std::vec::Vec::new();

            #(#mapped_variants)*

            ::configurable::schema::SchemaObject::one_of(subschemas, #maybe_description)
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
fn generate_named_variant(
    type_attrs: &TypeAttrs,
    variant: &syn::Variant,
    maybe_desc: TokenStream,
) -> Result<TokenStream> {
    let maybe_tag = type_attrs
        .tag
        .as_ref()
        .map(|tag| (tag, generate_variant_name(variant, &type_attrs.rename_all)));

    generate_fields(&variant.fields, maybe_tag, maybe_desc)
}

fn generate_enum_variant(type_attrs: &TypeAttrs, variant: &syn::Variant) -> Result<TokenStream> {
    let maybe_tag = type_attrs.tag.as_ref();

    let mut desc: Option<Description> = None;
    for attr in &variant.attrs {
        if is_doc_attr(attr) {
            parse_attr_doc(attr, &mut desc)?;
        }
    }

    let maybe_desc = desc
        .map(|desc| quote!( Some( #desc ) ))
        .unwrap_or(quote!(None));

    match &variant.fields {
        // enum Variants {
        //     Unit,
        // }
        Fields::Unit => {
            let ident = generate_variant_name(variant, &type_attrs.rename_all);

            match maybe_tag {
                Some(tag_name) => Ok(quote!({
                    subschemas.push({
                        let mut schema = ::configurable::schema::SchemaObject::new_object( #maybe_desc );

                        schema.insert_tag(#tag_name, #ident);

                        schema
                    });
                })),

                None => Ok(quote!({
                    subschemas.push(
                        ::configurable::schema::SchemaObject::const_value( #ident )
                    );
                })),
            }
        }

        // enum Variants {
        //     Named {
        //         foo: String,
        //     },
        // }
        Fields::Named(_named) => {
            let generated = generate_named_variant(type_attrs, variant, maybe_desc)?;

            Ok(quote! ({
                let mut subschema = { #generated };
                subschemas.push(subschema);
            }))
        }

        // enum Variants {
        //     Unnamed(String),
        // }
        Fields::Unnamed(_unnamed) => {
            if variant.fields.len() != 1 {
                return Err(syn::Error::new(
                    variant.span(),
                    "unnamed variant with multiple items is not support",
                ));
            }

            let field = variant.fields.iter().next().unwrap();
            let field_type = &field.ty;
            let variant_name = generate_variant_name(variant, &type_attrs.rename_all);

            if type_attrs.untagged {
                return Ok(quote!({
                    subschemas.push(
                        schema_gen.subschema_for::<#field_type>()
                    );
                }));
            }

            let insertion = match type_attrs.tag.as_ref() {
                None => quote!({
                    subschema.insert_property(
                        #variant_name,
                        false,
                        None,
                        schema_gen.subschema_for::<#field_type>(),
                    );
                }),
                Some(tag) => quote!(
                    subschema.insert_tag(#tag, #variant_name);
                    subschema.insert_flatten(
                        <#field_type>::generate_schema(schema_gen),
                    );
                ),
            };

            Ok(quote!({
                let mut subschema = ::configurable::schema::SchemaObject::new_object( None );

                #insertion

                subschemas.push(subschema);
            }))
        }
    }
}
