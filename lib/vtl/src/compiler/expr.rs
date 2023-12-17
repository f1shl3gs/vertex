use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};

use value::Value;

use super::binary::Binary;
use super::function_call::FunctionCall;
use super::query::Query;
use super::unary::Unary;
use super::{Expression, ExpressionError, Spanned, TypeDef};
use super::{Kind, Span};
use crate::context::Context;

#[derive(Clone)]
pub enum Expr {
    /// The literal null value.
    Null,
    /// The literal boolean value.
    Boolean(bool),
    /// The literal integer.
    Integer(i64),
    /// The literal float.
    Float(f64),
    /// A literal string.
    String(String),

    /// A reference to a stored value, an identifier.
    Ident(String),
    /// A query
    ///
    /// ".", "%", ".foo", "%foo" or "foo.bar"
    Query(Query),

    /// An unary operation.
    Unary(Unary),

    /// A binary operation.
    Binary(Binary),

    /// A call expression of something.
    Call(FunctionCall),

    /// A literal Array
    ///
    /// ```text
    /// arr = [1, false, "foo", -1]
    /// ```
    Array(Vec<Spanned<Expr>>),

    /// A literal Object.
    ///
    /// ```text
    /// obj = {
    ///     foo: "bar"
    /// }
    /// ```
    Object(BTreeMap<String, Spanned<Expr>>),
}

impl Display for Expr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            Expr::Null => "null",
            Expr::Boolean(_) => "bool",
            Expr::Integer(_) => "integer",
            Expr::Float(_) => "float",
            Expr::String(_) => "string",
            Expr::Ident(_) => "identifier",
            Expr::Query(_) => "query",
            Expr::Unary(_) => "unary",
            Expr::Binary(_) => "binary",
            Expr::Call(_) => "function call",
            Expr::Array(_) => "array",
            Expr::Object(_) => "object",
        };

        f.write_str(text)
    }
}

impl Expr {
    pub fn with(self, span: Span) -> Spanned<Expr> {
        Spanned { node: self, span }
    }

    #[inline]
    pub fn is_bool(&self, b: bool) -> bool {
        match self {
            Expr::Boolean(value) => *value == b,
            _ => false,
        }
    }
}

// this mod is used for tests only
#[cfg(test)]
mod expr_convert {
    use std::collections::BTreeMap;

    use value::{OwnedTargetPath, Value};

    use super::Expr;
    use crate::compiler::parser::unescape_string;
    use crate::compiler::query::Query;
    use crate::compiler::{Span, Spanned};

    impl From<&str> for Expr {
        fn from(value: &str) -> Self {
            Expr::String(unescape_string(value))
        }
    }

    impl From<bool> for Expr {
        fn from(value: bool) -> Self {
            Expr::Boolean(value)
        }
    }

    impl From<bool> for Spanned<Expr> {
        fn from(value: bool) -> Self {
            Expr::Boolean(value).with(Span::empty())
        }
    }

    impl From<i64> for Expr {
        fn from(value: i64) -> Self {
            Expr::Integer(value)
        }
    }

    impl From<i64> for Spanned<Expr> {
        fn from(value: i64) -> Self {
            Expr::Integer(value).with(Span::empty())
        }
    }

    impl From<f64> for Expr {
        fn from(value: f64) -> Self {
            Expr::Float(value)
        }
    }

    impl From<&str> for Spanned<Expr> {
        fn from(value: &str) -> Self {
            Expr::String(value.into()).with(Span::empty())
        }
    }

    impl From<String> for Expr {
        fn from(value: String) -> Self {
            Expr::String(value)
        }
    }

    impl From<Vec<Expr>> for Expr {
        fn from(array: Vec<Expr>) -> Self {
            Expr::Array(
                array
                    .into_iter()
                    .map(|expr| expr.with(Span::empty()))
                    .collect::<Vec<_>>(),
            )
        }
    }

    impl From<Value> for Expr {
        fn from(value: Value) -> Self {
            match value {
                Value::Bytes(s) => Expr::String(String::from_utf8_lossy(&s).to_string()),
                Value::Float(f) => Expr::Float(f),
                Value::Integer(i) => Expr::Integer(i),
                Value::Boolean(b) => Expr::Boolean(b),
                Value::Timestamp(ts) => Expr::String(ts.to_string()),
                Value::Object(obj) => {
                    let map = obj
                        .into_iter()
                        .map(|(k, v)| (k, Expr::from(v).with(Span::empty())))
                        .collect::<BTreeMap<String, Spanned<Expr>>>();
                    Expr::Object(map)
                }
                Value::Array(arr) => {
                    let arr = arr
                        .into_iter()
                        .map(|item| Expr::from(item).with(Span::empty()))
                        .collect::<Vec<_>>();
                    Expr::Array(arr)
                }
                Value::Null => Expr::Null,
            }
        }
    }

    impl From<OwnedTargetPath> for Expr {
        fn from(value: OwnedTargetPath) -> Self {
            Expr::Query(Query::External(value))
        }
    }

    impl From<BTreeMap<String, Expr>> for Expr {
        fn from(value: BTreeMap<String, Expr>) -> Self {
            Expr::Object(
                value
                    .into_iter()
                    .map(|(k, v)| (k, v.with(Span::empty())))
                    .collect::<BTreeMap<String, Spanned<Expr>>>(),
            )
        }
    }
}

impl Expression for Expr {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        match self {
            Expr::Null => Ok(Value::Null),
            Expr::Boolean(b) => Ok(Value::Boolean(*b)),
            Expr::Integer(i) => Ok(Value::Integer(*i)),
            Expr::Float(f) => Ok(Value::Float(*f)),
            Expr::String(s) => Ok(Value::from(s.as_str())),
            Expr::Ident(s) => {
                let value = cx
                    .variables
                    .get(s)
                    .expect("variable must be checked at compile time");
                Ok(value.clone())
            }
            Expr::Query(query) => query.resolve(cx),
            Expr::Array(array) => {
                let array = array
                    .iter()
                    .map(|expr| expr.resolve(cx))
                    .collect::<Result<Vec<_>, ExpressionError>>()?;
                Ok(array.into())
            }
            Expr::Binary(b) => b.resolve(cx),
            Expr::Unary(u) => u.resolve(cx),
            Expr::Object(map) => {
                let object = map
                    .iter()
                    .map(|(key, expr)| {
                        let value = expr.resolve(cx)?;
                        Ok((key.to_string(), value))
                    })
                    .collect::<Result<BTreeMap<String, Value>, ExpressionError>>()?;

                Ok(Value::Object(object))
            }

            Expr::Call(call) => call.function.resolve(cx),
        }
    }

    fn type_def(&self) -> TypeDef {
        match self {
            Expr::Ident(_) => {
                // TODO: fix this
                Kind::ANY.into()
            }
            Expr::Null => Kind::NULL.into(),
            Expr::Boolean(_) => Kind::BOOLEAN.into(),
            Expr::Integer(_) => Kind::INTEGER.into(),
            Expr::Float(_) => Kind::FLOAT.into(),
            Expr::String(_) => Kind::BYTES.into(),
            Expr::Array(_) => Kind::ARRAY.into(),
            Expr::Object(_) => Kind::OBJECT.into(),
            Expr::Call(call) => call.type_def(),
            Expr::Binary(b) => b.type_def(),
            Expr::Unary(u) => u.type_def(),

            _ => TypeDef {
                fallible: false,
                kind: Kind::ANY,
            },
        }
    }
}
