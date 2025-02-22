use bytes::Buf;
use value::Value;

use crate::SyntaxError;
use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Function, FunctionCompileContext, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::state::TypeState;
use crate::compiler::{Expression, ExpressionError, Kind, Spanned, TypeDef};
use crate::context::Context;

pub struct Trim;

impl Function for Trim {
    fn identifier(&self) -> &'static str {
        "trim"
    }

    fn parameters(&self) -> &'static [Parameter] {
        &[Parameter {
            name: "value",
            kind: Kind::BYTES,
            required: true,
        }]
    }

    fn compile(
        &self,
        cx: FunctionCompileContext,
        mut arguments: ArgumentList,
    ) -> Result<FunctionCall, SyntaxError> {
        let value = arguments.get();

        Ok(FunctionCall {
            function: Box::new(TrimFunc { value }),
            span: cx.span,
        })
    }
}

#[derive(Clone)]
struct TrimFunc {
    value: Spanned<Expr>,
}

impl Expression for TrimFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        match self.value.resolve(cx)? {
            Value::Bytes(mut b) => {
                // start
                loop {
                    match b.first() {
                        Some(ch) => {
                            if ch.is_ascii_whitespace() {
                                b.advance(1);
                                continue;
                            } else {
                                break;
                            }
                        }
                        None => return Ok(Value::Bytes(b)),
                    }
                }

                // end
                loop {
                    match b.last() {
                        Some(ch) => {
                            if ch.is_ascii_whitespace() {
                                b.truncate(b.len() - 1);
                            } else {
                                break;
                            }
                        }
                        None => return Ok(Value::Bytes(b)),
                    }
                }

                Ok(b.into())
            }
            value => Err(ExpressionError::UnexpectedType {
                want: Kind::BYTES,
                got: value.kind(),
                span: self.value.span,
            }),
        }
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
    fn no_trim() {
        compile_and_run(vec!["foo".into()], Trim, TypeDef::bytes(), Ok("foo".into()))
    }

    #[test]
    fn trim_start() {
        compile_and_run(
            vec![" foo".into()],
            Trim,
            TypeDef::bytes(),
            Ok("foo".into()),
        )
    }

    #[test]
    fn trim_end() {
        compile_and_run(
            vec!["foo ".into()],
            Trim,
            TypeDef::bytes(),
            Ok("foo".into()),
        )
    }

    #[test]
    fn trim() {
        compile_and_run(
            vec![" foo ".into()],
            Trim,
            TypeDef::bytes(),
            Ok("foo".into()),
        )
    }
}
