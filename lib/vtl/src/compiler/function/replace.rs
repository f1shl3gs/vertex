use value::Value;

use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Function, FunctionCompileContext, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::state::TypeState;
use crate::compiler::{Expression, ExpressionError, Kind, Spanned, TypeDef, ValueKind};
use crate::context::Context;
use crate::SyntaxError;

pub struct Replace;

impl Function for Replace {
    fn identifier(&self) -> &'static str {
        "replace"
    }

    fn parameters(&self) -> &'static [Parameter] {
        &[
            Parameter {
                name: "value",
                kind: Kind::BYTES,
                required: true,
            },
            Parameter {
                name: "pattern",
                kind: Kind::BYTES,
                required: true,
            },
            Parameter {
                name: "with",
                kind: Kind::BYTES,
                required: true,
            },
            Parameter {
                name: "count",
                kind: Kind::INTEGER,
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
        let pattern = arguments.get();
        let with = arguments.get();
        let count = arguments.get_opt();

        Ok(FunctionCall {
            function: Box::new(ReplaceFunc {
                value,
                pattern,
                with,
                count,
            }),
            span: cx.span,
        })
    }
}

#[derive(Clone)]
struct ReplaceFunc {
    value: Spanned<Expr>,
    pattern: Spanned<Expr>,
    with: Spanned<Expr>,
    count: Option<Spanned<Expr>>,
}

impl Expression for ReplaceFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let value = self.value.resolve(cx)?;
        let value = match &value {
            Value::Bytes(b) => String::from_utf8_lossy(b),
            value => {
                return Err(ExpressionError::UnexpectedType {
                    want: Kind::BYTES,
                    got: value.kind(),
                    span: self.value.span,
                })
            }
        };

        let pattern = self.pattern.resolve(cx)?;
        let pattern = match &pattern {
            Value::Bytes(b) => String::from_utf8_lossy(b),
            value => {
                return Err(ExpressionError::UnexpectedType {
                    want: Kind::BYTES,
                    got: value.kind(),
                    span: self.pattern.span,
                })
            }
        };

        let to = self.with.resolve(cx)?;
        let to = match &to {
            Value::Bytes(b) => String::from_utf8_lossy(b),
            value => {
                return Err(ExpressionError::UnexpectedType {
                    want: Kind::BYTES,
                    got: value.kind(),
                    span: self.with.span,
                })
            }
        };

        let count = match &self.count {
            Some(count) => match count.resolve(cx)? {
                Value::Integer(i) => i as usize,
                value => {
                    return Err(ExpressionError::UnexpectedType {
                        want: Kind::INTEGER,
                        got: value.kind(),
                        span: count.span,
                    })
                }
            },
            None => usize::MAX,
        };

        let replaced = value.replacen(pattern.as_ref(), to.as_ref(), count);

        Ok(Value::Bytes(replaced.into()))
    }

    fn type_def(&self, _state: &TypeState) -> TypeDef {
        TypeDef {
            fallible: false,
            kind: Kind::BYTES,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::function::compile_and_run;

    #[test]
    fn ok() {
        compile_and_run(
            vec!["foobar".into(), "foo".into(), "FOO".into()],
            Replace,
            TypeDef::bytes(),
            Ok(Value::Bytes("FOObar".into())),
        )
    }

    #[test]
    fn multiple() {
        compile_and_run(
            vec!["foobar".into(), "o".into(), "O".into()],
            Replace,
            TypeDef::bytes(),
            Ok(Value::Bytes("fOObar".into())),
        )
    }

    #[test]
    fn one() {
        compile_and_run(
            vec!["foobar".into(), "o".into(), "O".into(), 1.into()],
            Replace,
            TypeDef::bytes(),
            Ok(Value::Bytes("fOobar".into())),
        )
    }

    #[test]
    fn not_exists() {
        compile_and_run(
            vec!["foobar".into(), "z".into(), "Z".into()],
            Replace,
            TypeDef::bytes(),
            Ok(Value::Bytes("foobar".into())),
        )
    }
}
