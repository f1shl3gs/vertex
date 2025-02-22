use value::Value;

use crate::SyntaxError;
use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Function, FunctionCompileContext, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::state::TypeState;
use crate::compiler::{Expression, ExpressionError, Kind, Spanned, TypeDef};
use crate::context::Context;

pub struct Split;

impl Function for Split {
    fn identifier(&self) -> &'static str {
        "split"
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
                name: "limit",
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
        let limit = arguments.get_opt();

        Ok(FunctionCall {
            function: Box::new(SplitFunc {
                value,
                pattern,
                limit,
            }),
            span: cx.span,
        })
    }
}

#[derive(Clone)]
struct SplitFunc {
    value: Spanned<Expr>,
    pattern: Spanned<Expr>,
    limit: Option<Spanned<Expr>>,
}

impl Expression for SplitFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let value = self.value.resolve(cx)?;
        let value = match &value {
            Value::Bytes(b) => String::from_utf8_lossy(b),
            value => {
                return Err(ExpressionError::UnexpectedType {
                    want: Kind::BYTES,
                    got: value.kind(),
                    span: self.value.span,
                });
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
                });
            }
        };

        let array = match &self.limit {
            None => value
                .as_ref()
                .split(pattern.as_ref())
                .map(Value::from)
                .collect::<Vec<_>>(),
            Some(expr) => match expr.resolve(cx)? {
                Value::Integer(i) => {
                    if i <= 0 {
                        return Err(ExpressionError::UnexpectedValue {
                            msg: "limit must be greater than 0".to_string(),
                            span: expr.span,
                        });
                    }

                    value
                        .as_ref()
                        .splitn(i as usize, pattern.as_ref())
                        .map(Value::from)
                        .collect::<Vec<_>>()
                }
                value => {
                    return Err(ExpressionError::UnexpectedType {
                        want: Kind::INTEGER,
                        got: value.kind(),
                        span: expr.span,
                    });
                }
            },
        };

        Ok(array.into())
    }

    fn type_def(&self, _state: &TypeState) -> TypeDef {
        TypeDef {
            fallible: false,
            kind: Kind::ARRAY,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::function::compile_and_run;

    #[test]
    fn yes() {
        compile_and_run(
            vec!["foo bar blah".into(), " ".into()],
            Split,
            TypeDef::array(),
            Ok(vec![Value::from("foo"), "bar".into(), "blah".into()].into()),
        )
    }

    #[test]
    fn yes_with_limit() {
        compile_and_run(
            vec!["foo bar blah".into(), " ".into(), 2.into()],
            Split,
            TypeDef::array(),
            Ok(vec![Value::from("foo"), "bar blah".into()].into()),
        )
    }

    #[test]
    fn start() {
        compile_and_run(
            vec!["foo bar blah".into(), "foo".into()],
            Split,
            TypeDef::array(),
            Ok(vec!["".into(), Value::from(" bar blah")].into()),
        )
    }

    #[test]
    fn end() {
        compile_and_run(
            vec!["foo bar blah".into(), "blah".into()],
            Split,
            TypeDef::array(),
            Ok(vec!["foo bar ".into(), Value::from("")].into()),
        )
    }
}
