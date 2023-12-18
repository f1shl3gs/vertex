use std::collections::BTreeMap;

use url::Url;
use value::Value;

use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Function, FunctionCompileContext, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::state::TypeState;
use crate::compiler::{Expression, ExpressionError, Kind, Spanned, TypeDef, ValueKind};
use crate::context::Context;
use crate::SyntaxError;

pub struct ParseUrl;

impl Function for ParseUrl {
    fn identifier(&self) -> &'static str {
        "parse_url"
    }

    fn parameters(&self) -> &'static [Parameter] {
        &[
            Parameter {
                name: "value",
                kind: Kind::BYTES,
                required: true,
            },
            Parameter {
                name: "default_known_ports",
                kind: Kind::BOOLEAN,
                required: false,
            },
        ]
    }

    fn compile(
        &self,
        cx: FunctionCompileContext,
        mut arguments: ArgumentList,
    ) -> Result<FunctionCall, SyntaxError> {
        let value = arguments.get();
        let default_known_ports = arguments.get_opt();

        Ok(FunctionCall {
            function: Box::new(ParseURLFunc {
                value,
                default_known_ports,
            }),
            span: cx.span,
        })
    }
}

#[derive(Clone)]
struct ParseURLFunc {
    value: Spanned<Expr>,
    default_known_ports: Option<Spanned<Expr>>,
}

impl Expression for ParseURLFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let value = match self.value.resolve(cx)? {
            Value::Bytes(b) => b,
            value => {
                return Err(ExpressionError::UnexpectedType {
                    want: Kind::OBJECT,
                    got: value.kind(),
                    span: self.value.span,
                })
            }
        };

        let default_known_ports = match &self.default_known_ports {
            Some(expr) => match expr.resolve(cx)? {
                Value::Boolean(b) => b,
                value => {
                    return Err(ExpressionError::UnexpectedType {
                        want: Kind::BOOLEAN,
                        got: value.kind(),
                        span: expr.span,
                    })
                }
            },
            None => false,
        };

        let text = String::from_utf8_lossy(&value);
        let url = Url::parse(text.as_ref()).map_err(|err| ExpressionError::Error {
            message: err.to_string(),
            span: self.value.span,
        })?;

        Ok(url_to_value(url, default_known_ports))
    }

    fn type_def(&self, _state: &TypeState) -> TypeDef {
        TypeDef {
            fallible: true,
            kind: Kind::OBJECT,
        }
    }
}

fn url_to_value(url: Url, default_known_ports: bool) -> Value {
    let mut map = BTreeMap::new();

    map.insert("scheme".to_string(), url.scheme().into());
    map.insert("username".to_string(), url.username().into());
    map.insert(
        "password".to_string(),
        url.password().unwrap_or_default().into(),
    );
    map.insert("path".to_string(), url.path().into());
    map.insert(
        "host".to_string(),
        url.host().map(|host| host.to_string()).into(),
    );

    let port = if default_known_ports {
        url.port_or_known_default()
    } else {
        url.port()
    };
    map.insert("port".to_string(), port.into());
    map.insert(
        "fragment".to_string(),
        url.fragment().map(ToOwned::to_owned).into(),
    );
    map.insert(
        "query".to_string(),
        url.query_pairs()
            .into_owned()
            .map(|(k, v)| (k, v.into()))
            .collect::<BTreeMap<String, Value>>()
            .into(),
    );

    Value::Object(map)
}

#[cfg(test)]
mod tests {
    use value::value;

    use super::*;
    use crate::compiler::function::compile_and_run;

    #[test]
    fn https() {
        compile_and_run(
            vec!["https://example.io".into()],
            ParseUrl,
            TypeDef::object().fallible(),
            Ok(value!({
                "fragment": null,
                "host": "example.io",
                "password": "",
                "path": "/",
                "port": null,
                "query": {},
                "scheme": "https",
                "username": "",
            })),
        )
    }

    #[test]
    fn https_with_query() {
        compile_and_run(
            vec!["https://example.io?foo=bar".into()],
            ParseUrl,
            TypeDef::object().fallible(),
            Ok(value!({
                "fragment": null,
                "host": "example.io",
                "password": "",
                "path": "/",
                "port": null,
                "query": {
                    "foo": "bar"
                },
                "scheme": "https",
                "username": "",
            })),
        )
    }

    #[test]
    fn https_with_port() {
        compile_and_run(
            vec!["https://example.io:443".into()],
            ParseUrl,
            TypeDef::object().fallible(),
            Ok(value!({
                "fragment": null,
                "host": "example.io",
                "password": "",
                "path": "/",
                "port": null,
                "query": {},
                "scheme": "https",
                "username": "",
            })),
        )
    }

    #[test]
    fn default_port() {
        compile_and_run(
            vec!["https://example.io:443".into(), true.into()],
            ParseUrl,
            TypeDef::object().fallible(),
            Ok(value!({
                "fragment": null,
                "host": "example.io",
                "password": "",
                "path": "/",
                "port": 443,
                "query": {},
                "scheme": "https",
                "username": "",
            })),
        )
    }
}
