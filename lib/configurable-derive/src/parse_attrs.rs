use crate::errors::Errors;
use syn::{Lit, LitStr};

/// A description of a `#[configurable(...)]` struct.
///
/// Defaults to the docstring if one is present, or `#[configurable(description = "...")]`
/// if one is provided.
#[derive(Debug)]
pub struct Description {
    /// Whether the description was an explicit annotation or whether it was a doc string.
    pub explicit: bool,
    pub content: syn::LitStr,
}

/// Attributes applied to a field of a `#[configurable(...)]` struct.
#[derive(Default, Debug)]
pub struct FieldAttrs {
    pub default: Option<Lit>,
    pub format: Option<LitStr>,
    pub description: Option<Description>,
    pub example: Option<syn::Lit>,
    pub required: bool,
    pub deprecated: bool,
}

impl FieldAttrs {
    pub fn parse(errs: &Errors, field: &syn::Field) -> Self {
        let mut this = Self::default();

        for attr in &field.attrs {
            if is_doc_attr(attr) {
                parse_attr_doc(errs, attr, &mut this.description);
                continue;
            }

            let ml = if let Some(ml) = configurable_attr_to_meta_list(errs, attr) {
                ml
            } else {
                continue;
            };

            for meta in &ml.nested {
                let meta = if let Some(m) = errs.expect_nested_meta(meta) {
                    m
                } else {
                    continue;
                };
                let name = meta.path();

                if name.is_ident("required") {
                    this.required = true
                } else if name.is_ident("default") {
                    if let Some(m) = errs.expect_meta_name_value(meta) {
                        parse_attr_lit(errs, m, &mut this.default)
                    }
                } else if name.is_ident("example") {
                    if let Some(m) = errs.expect_meta_name_value(meta) {
                        parse_attr_lit(errs, m, &mut this.example)
                    }
                } else if name.is_ident("description") {
                    if let Some(m) = errs.expect_meta_name_value(meta) {
                        parse_attr_description(errs, m, &mut this.description)
                    }
                } else if name.is_ident("format") {
                    if let Some(m) = errs.expect_meta_name_value(meta) {
                        parse_attr_litstr(errs, m, &mut this.format)
                    }
                } else {
                    errs.err(
                        &meta,
                        concat!(
                            "Invalid `configurable` attribute\n",
                            "Expected one of: `default`, `description`, `required`, `examples`"
                        ),
                    )
                }
            }
        }

        this
    }
}

fn parse_attr_description(errs: &Errors, m: &syn::MetaNameValue, slot: &mut Option<Description>) {
    let lit_str = if let Some(lit_str) = errs.expect_lit_str(&m.lit) {
        lit_str
    } else {
        return;
    };

    // Don't allow multiple explicit (non doc-comment) descriptions
    if let Some(description) = slot {
        if description.explicit {
            errs.duplicate_attrs("description", &description.content, lit_str);
        }
    }

    *slot = Some(Description {
        explicit: true,
        content: lit_str.clone(),
    });
}

/// Represents a `#[derive(FromArgs)]` type's top-level attributes.
#[derive(Default)]
pub struct TypeAttrs {
    pub name: Option<syn::LitStr>,
    pub title: Option<syn::LitStr>,
    pub description: Option<Description>,
    pub component_type: Option<syn::Ident>,
}

impl TypeAttrs {
    pub fn parse(errs: &Errors, input: &syn::DeriveInput) -> Self {
        let mut this = TypeAttrs::default();

        for attr in &input.attrs {
            if is_doc_attr(attr) {
                parse_attr_doc(errs, attr, &mut this.description);
                continue;
            }

            let ml = if let Some(ml) = configurable_attr_to_meta_list(errs, attr) {
                ml
            } else {
                continue;
            };

            for meta in &ml.nested {
                let meta = if let Some(m) = errs.expect_nested_meta(meta) {
                    m
                } else {
                    continue;
                };

                let name = meta.path();
                if name.is_ident("description") {
                    if let Some(m) = errs.expect_meta_name_value(meta) {
                        parse_attr_description(errs, m, &mut this.description);
                    }
                } else if name.is_ident("name") {
                    if let Some(m) = errs.expect_meta_name_value(meta) {
                        parse_attr_litstr(errs, m, &mut this.name);
                    }
                } else if name.is_ident("title") {
                    if let Some(m) = errs.expect_meta_name_value(meta) {
                        parse_attr_litstr(errs, m, &mut this.title);
                    }
                } else if name.is_ident("source")
                    || name.is_ident("transform")
                    || name.is_ident("sink")
                    || name.is_ident("provider")
                {
                    this.component_type = name.get_ident().cloned();
                } else {
                    errs.err(
                        &meta,
                        concat!(
                            "Invalid type-level `configurable_component` attribute\n",
                            "Expected one of: `name`, `description`, `title`, `source`, ",
                            "`transform`, `sink`, `provider`",
                        ),
                    )
                }
            }
        }

        this
    }
}

fn parse_attr_doc(errors: &Errors, attr: &syn::Attribute, slot: &mut Option<Description>) {
    let nv = if let Some(nv) = attr_to_meta_name_value(errors, attr) {
        nv
    } else {
        return;
    };

    // Don't replace an existing description.
    if slot.as_ref().map(|d| d.explicit).unwrap_or(false) {
        return;
    }

    if let Some(lit_str) = errors.expect_lit_str(&nv.lit) {
        let lit_str = if let Some(previous) = slot {
            let previous = &previous.content;
            let previous_span = previous.span();

            LitStr::new(&(previous.value() + &*lit_str.value()), previous_span)
        } else {
            lit_str.clone()
        };

        *slot = Some(Description {
            explicit: false,
            content: lit_str,
        });
    }
}

/// Checks for `#[doc ...]`, which is generated by doc comments.
fn is_doc_attr(attr: &syn::Attribute) -> bool {
    is_matching_attr("doc", attr)
}

// Whether the attribute is one like `#[<name> ...]`
fn is_matching_attr(name: &str, attr: &syn::Attribute) -> bool {
    attr.path.segments.len() == 1 && attr.path.segments[0].ident == name
}

/// Checks for `#[configurable ...]`
fn is_configurable_attr(attr: &syn::Attribute) -> bool {
    is_matching_attr("configurable", attr)
}

fn attr_to_meta_subtype<R: Clone>(
    errors: &Errors,
    attr: &syn::Attribute,
    f: impl FnOnce(&syn::Meta) -> Option<&R>,
) -> Option<R> {
    match attr.parse_meta() {
        Ok(meta) => f(&meta).cloned(),
        Err(e) => {
            errors.push(e);
            None
        }
    }
}

fn attr_to_meta_list(errors: &Errors, attr: &syn::Attribute) -> Option<syn::MetaList> {
    attr_to_meta_subtype(errors, attr, |m| errors.expect_meta_list(m))
}

fn attr_to_meta_name_value(errors: &Errors, attr: &syn::Attribute) -> Option<syn::MetaNameValue> {
    attr_to_meta_subtype(errors, attr, |m| errors.expect_meta_name_value(m))
}

/// Filters out non-`#[configurable(...)]` attributes and converts to `syn::MetaList`.
fn configurable_attr_to_meta_list(errors: &Errors, attr: &syn::Attribute) -> Option<syn::MetaList> {
    if !is_configurable_attr(attr) {
        return None;
    }
    attr_to_meta_list(errors, attr)
}

pub fn parse_attr_litstr(errs: &Errors, m: &syn::MetaNameValue, slot: &mut Option<LitStr>) {
    let lit_str = if let Some(lit_str) = errs.expect_lit_str(&m.lit) {
        lit_str
    } else {
        return;
    };

    *slot = Some(lit_str.clone())
}

pub fn parse_attr_lit(_errs: &Errors, m: &syn::MetaNameValue, slot: &mut Option<Lit>) {
    *slot = Some(m.lit.clone());
}
