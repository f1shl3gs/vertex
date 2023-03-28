use proc_macro2::{Literal, TokenStream};
use quote::{quote, ToTokens, TokenStreamExt};
use syn::spanned::Spanned;
use syn::{Attribute, Expr, Lit, LitBool, LitStr, Path, Token, Type};

pub const DOC: &str = "doc";
pub const SERDE: &str = "serde";
pub const CONFIGURABLE: &str = "configurable";

/// A description of a `#[configurable(...)]` struct.
///
/// Defaults to the docstring if one is present, or `#[configurable(description = "...")]`
/// if one is provided.
pub struct Description {
    /// Whether the description was an explicit annotation or whether it was a doc string.
    explicit: bool,
    content: syn::LitStr,
}

impl Description {
    fn parse(attr: &Attribute) -> syn::Result<Description> {
        let nv = attr.meta.require_name_value()?;

        match &nv.value {
            Expr::Lit(lit) => {
                if let Lit::Str(ls) = &lit.lit {
                    return Ok(Description {
                        explicit: false,
                        content: ls.clone(),
                    });
                }

                Err(syn::Error::new(lit.span(), "value should be Lit::Str"))
            }
            _ => Err(syn::Error::new(nv.value.span(), "Lit expect")),
        }
    }

    fn merge(&mut self, attr: &Attribute) -> syn::Result<()> {
        let nv = attr.meta.require_name_value()?;
        match &nv.value {
            Expr::Lit(lit) => {
                if let Lit::Str(ls) = &lit.lit {
                    self.content =
                        LitStr::new(&(self.content.value() + "\n" + &*ls.value()), attr.span());

                    return Ok(());
                }

                Err(syn::Error::new(lit.span(), "value should be Lit::Str"))
            }
            _ => Err(syn::Error::new(nv.value.span(), "Lit expect")),
        }
    }
}

impl ToTokens for Description {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let content = self.content.value();
        tokens.append(Literal::string(content.trim_end()))
    }
}

/// Attributes applied to a field of a `#[configurable(...)]` struct.
#[derive(Default)]
pub struct FieldAttrs {
    pub skip: bool,
    pub required: bool,
    pub deprecated: bool,
    pub flatten: bool,

    pub rename: Option<LitStr>,
    pub default: Option<Lit>,
    // default_fn is set when #[serde(default)] is present
    pub default_fn: Option<LitStr>,
    pub format: Option<LitStr>,
    pub serde_with: Option<LitStr>,
    pub description: Option<Description>,
    pub example: Option<syn::Lit>,
}

impl FieldAttrs {
    pub fn parse(field: &syn::Field) -> syn::Result<FieldAttrs> {
        let mut this = Self::default();

        for attr in &field.attrs {
            if attr.path().is_ident(DOC) {
                parse_attr_doc(attr, &mut this.description)?;
                continue;
            }

            // handle serde attributes, e.g. default
            if attr.path().is_ident(SERDE) {
                this.parse_serde_attr(attr)?;
                continue;
            }

            if attr.path().is_ident(CONFIGURABLE) {
                attr.parse_nested_meta(|meta| {
                    let name = match meta.path.get_ident() {
                        Some(ident) => ident.to_string(),
                        None => return Ok(()),
                    };

                    match name.as_str() {
                        "skip" => this.skip = true,
                        "required" => {
                            if meta.input.peek(Token![=]) {
                                // #[configurable(require = true)] or #[configurable(require = false)]
                                let value = meta.value()?;
                                let value: LitBool = value.parse()?;
                                this.required = value.value;
                            } else {
                                // #[configurable(require = false)]
                                this.required = true;
                            }
                        },
                        "example" => {
                            let value = meta.value()?;
                            let value: Lit = value.parse()?;
                            this.example = Some(value)
                        }
                        "description" => {
                            let value = meta.value()?;
                            let content: LitStr = value.parse()?;

                            this.description = Some(Description{
                                explicit: true,
                                content,
                            });
                        }
                        "format" => {
                            let value = meta.value()?;
                            let value: LitStr = value.parse()?;
                            this.format = Some(value)
                        },
                        "default" => {
                            let value = meta.value()?;
                            let value: Lit = value.parse()?;
                            this.default = Some(value);
                        },
                        _ => {
                            return Err(syn::Error::new(
                                meta.path.span(),
                                format!("Invalid `configurable` attribute \"{}\"\n\
                                Expected one of: `default`, `description`, `required`, `examples`, `skip`",
                                        name),
                            ))
                        }
                    }

                    Ok(())
                })?;
            }
        }

        Ok(this)
    }

    /// This func does not check if the Attribute is serde,
    fn parse_serde_attr(&mut self, attr: &Attribute) -> syn::Result<()> {
        attr.parse_nested_meta(|meta| {
            let name = match meta.path.get_ident() {
                Some(ident) => ident.to_string(),
                None => return Ok(()),
            };

            match name.as_str() {
                "default" => {
                    if meta.input.peek(Token![=]) {
                        // #[serde(default = "...")]
                        let value = meta.value()?;
                        let value = value.parse()?;
                        self.default_fn = Some(value);
                    } else {
                        // #[serde(default)]
                        self.default_fn = Some(LitStr::new("", meta.path.span()))
                    }
                }
                "with" => {
                    let value = meta.value()?;
                    let value = value.parse()?;
                    self.serde_with = Some(value)
                }
                "rename" => {
                    let value = meta.value()?;
                    let value = value.parse()?;
                    self.rename = Some(value)
                }
                "flatten" => self.flatten = true,
                "skip" => self.skip = true,

                _ => {
                    // consume tokens by parse, so next iteration of parse_nested_meta() will not fail.
                    if meta.input.peek(Token![=]) {
                        let value = meta.value()?;
                        let _value: Lit = value.parse()?;
                    }
                }
            }

            Ok(())
        })
    }

    pub fn maybe_default(&self, field_typ: &Type) -> TokenStream {
        let default_value = if let Some(value) = &self.default {
            // #[configurable(default = 1111)
            // #[configurable(default = "abcd")
            quote!( let default_value = #value; )
        } else if let Some(default_fn) = &self.default_fn {
            if default_fn.value().is_empty() {
                // handle something like `#[serde(default)]`
                quote!( let default_value: #field_typ = Default::default(); )
            } else {
                let default_fn: Path = default_fn
                    .parse()
                    .expect("serde's default function cannot be transform to syn::Path");
                quote!( let default_value = #default_fn(); )
            }
        } else {
            return quote!();
        };

        let json_value = match &self.serde_with {
            Some(serde_with) => {
                let serde_with: syn::Path = serde_with.parse().expect("valid serde with value");

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
    }
}

/// Represents a `#[derive(FromArgs)]` type's top-level attributes.
#[derive(Default)]
pub struct TypeAttrs {
    pub name: Option<syn::LitStr>,
    pub title: Option<syn::LitStr>,
    pub description: Option<Description>,
    pub component_type: Option<syn::Ident>,

    // serde's attributes
    pub rename_all: Option<syn::LitStr>,
    pub tag: Option<LitStr>,
    pub untagged: bool,
}

pub fn parse_attr_doc(attr: &Attribute, slot: &mut Option<Description>) -> syn::Result<()> {
    if !attr.path().is_ident(DOC) {
        return Ok(());
    }

    if let Some(prev) = slot {
        if prev.explicit {
            return Ok(());
        }

        prev.merge(attr)?;

        Ok(())
    } else {
        let desc = Description::parse(attr)?;
        *slot = Some(desc);
        Ok(())
    }
}

impl TypeAttrs {
    fn parse_serde_attr(&mut self, attr: &Attribute) -> syn::Result<()> {
        attr.parse_nested_meta(|meta| {
            let name = if let Some(name) = meta.path.get_ident() {
                name
            } else {
                return Ok(());
            };

            match name.to_string().as_str() {
                "rename_all" => {
                    let value = meta.value()?;
                    let value: LitStr = value.parse()?;
                    self.rename_all = Some(value);
                }
                "tag" => {
                    let value = meta.value()?;
                    let value: LitStr = value.parse()?;
                    self.tag = Some(value);
                }
                "untagged" => self.untagged = true,
                _ => {
                    // consume tokens by parse, so next iteration of parse_nested_meta() will not fail.
                    if meta.input.peek(Token![=]) {
                        let value = meta.value()?;
                        let _value: Lit = value.parse()?;
                    }
                }
            }

            Ok(())
        })
    }

    pub fn parse(input: &syn::DeriveInput) -> syn::Result<TypeAttrs> {
        let mut this = TypeAttrs::default();

        for attr in &input.attrs {
            if attr.path().is_ident("doc") {
                parse_attr_doc(attr, &mut this.description)?;
                continue;
            }

            if is_serde_attr(attr) {
                this.parse_serde_attr(attr)?;
                continue;
            }

            // parse attributes of '#[configurable(..)]'
            if is_configurable_attr(attr) {
                attr.parse_nested_meta(|meta| {
                    let name = if let Some(name) = meta.path.get_ident() {
                        name
                    } else {
                        return Ok(());
                    };

                    match name.to_string().as_str() {
                        "name" => {
                            let value = meta.value()?;
                            let value: LitStr = value.parse()?;
                            this.name = Some(value);
                        }
                        "title" => {
                            let value = meta.value()?;
                            let value: LitStr = value.parse()?;
                            this.title = Some(value);
                        }
                        "description" => {
                            let value = meta.value()?;
                            let content: LitStr = value.parse()?;
                            this.description = Some(Description {
                                explicit: true,
                                content,
                            })
                        }
                        "extension" | "provider" | "source" | "transform" | "sink" => {
                            this.component_type = Some(name.clone());
                        }
                        "rename_all" => {
                            let value = meta.value()?;
                            let value: LitStr = value.parse()?;
                            this.rename_all = Some(value);
                        }
                        _ => {
                            return Err(syn::Error::new(
                                meta.path.span(),
                                concat!(
                                    "Invalid type-level `configurable_component` attribute\n",
                                    "Expected one of: `name`, `description`, `title`, `source`, ",
                                    "`transform`, `sink`, `provider`",
                                ),
                            ))
                        }
                    }

                    Ok(())
                })?;
            }
        }

        Ok(this)
    }
}

/// Checks for `#[doc ...]`, which is generated by doc comments.
pub fn is_doc_attr(attr: &Attribute) -> bool {
    attr.path().is_ident("doc")
}

/// Checks for `#[serde(...)]`
fn is_serde_attr(attr: &Attribute) -> bool {
    attr.path().is_ident("serde")
}

/// Checks for `#[configurable ...]`
fn is_configurable_attr(attr: &Attribute) -> bool {
    attr.path().is_ident("configurable")
}
