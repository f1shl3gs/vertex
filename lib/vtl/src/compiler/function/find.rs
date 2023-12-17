use value::Value;

use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Function, FunctionCompileContext, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::{Expression, ExpressionError, Kind, Spanned, TypeDef, ValueKind};
use crate::context::Context;
use crate::SyntaxError;

pub struct Find;

impl Function for Find {
    fn identifier(&self) -> &'static str {
        "find"
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
                name: "start",
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
        let start = arguments.get_opt();

        Ok(FunctionCall {
            function: Box::new(FindFunc {
                value,
                pattern,
                start,
            }),
            span: cx.span,
        })
    }
}

#[derive(Clone)]
struct FindFunc {
    value: Spanned<Expr>,
    pattern: Spanned<Expr>,
    start: Option<Spanned<Expr>>,
}

impl Expression for FindFunc {
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

        let start = match &self.start {
            Some(expr) => match expr.resolve(cx)? {
                Value::Integer(i) => {
                    if i < 0 {
                        return Err(ExpressionError::UnexpectedValue {
                            msg: "from must be a nonzero value".to_string(),
                            span: expr.span,
                        });
                    } else {
                        i as usize
                    }
                }
                value => {
                    return Err(ExpressionError::UnexpectedType {
                        want: Kind::INTEGER,
                        got: value.kind(),
                        span: expr.span,
                    })
                }
            },
            None => 0,
        };

        let value = if start != 0 {
            let (_first, second) = value.split_at(start);
            second
        } else {
            value.as_ref()
        };

        let pos = value
            .find(pattern.as_ref())
            .map(|pos| (pos + start) as i64)
            .unwrap_or(-1);

        Ok(pos.into())
    }

    fn type_def(&self) -> TypeDef {
        TypeDef {
            fallible: false,
            kind: Kind::INTEGER,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::function::compile_and_run;

    #[test]
    fn match_end() {
        compile_and_run(
            vec!["foobar".into(), "bar".into()],
            Find,
            TypeDef::integer(),
            Ok(3.into()),
        )
    }

    #[test]
    fn match_start() {
        compile_and_run(
            vec!["foobar".into(), "foo".into()],
            Find,
            TypeDef::integer(),
            Ok(0.into()),
        )
    }

    #[test]
    fn match_middle() {
        compile_and_run(
            vec!["foobar".into(), "oob".into()],
            Find,
            TypeDef::integer(),
            Ok(1.into()),
        )
    }

    #[test]
    fn long_pattern() {
        compile_and_run(
            vec!["foo".into(), "foobar".into()],
            Find,
            TypeDef::integer(),
            Ok((-1).into()),
        )
    }
}
