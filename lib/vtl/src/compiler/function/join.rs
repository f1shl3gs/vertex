use value::{Kind, Value};

use crate::SyntaxError;
use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Function, FunctionCompileContext, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::state::TypeState;
use crate::compiler::{Expression, ExpressionError, Spanned, TypeDef};
use crate::context::Context;

pub struct Join;

impl Function for Join {
    fn identifier(&self) -> &'static str {
        "join"
    }

    fn parameters(&self) -> &'static [Parameter] {
        &[
            Parameter {
                name: "array",
                kind: Kind::ARRAY,
                required: true,
            },
            Parameter {
                name: "separator",
                kind: Kind::BYTES,
                required: false,
            },
        ]
    }

    fn compile(
        &self,
        cx: FunctionCompileContext,
        mut arguments: ArgumentList,
    ) -> Result<FunctionCall, SyntaxError> {
        let array = arguments.get();
        let separator = arguments.get_opt();

        Ok(FunctionCall {
            function: Box::new(JoinFunc { array, separator }),
            span: cx.span,
        })
    }
}

#[derive(Clone)]
struct JoinFunc {
    array: Spanned<Expr>,
    separator: Option<Spanned<Expr>>,
}

impl Expression for JoinFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let array = self.array.resolve(cx)?;
        let Value::Array(array) = array else {
            return Err(ExpressionError::UnexpectedType {
                want: Kind::ARRAY,
                got: array.kind(),
                span: self.array.span,
            });
        };

        let array = array.into_iter().map(|item| {
            if let Value::Bytes(s) = item {
                Ok(String::from_utf8_lossy(&s).into_owned())
            } else {
                Err(ExpressionError::UnexpectedValue {
                    msg: format!("item must be string instead of {}", item.kind()),
                    span: self.array.span,
                })
            }
        });

        let separator = match &self.separator {
            Some(separator) => {
                let value = separator.resolve(cx)?;
                let Value::Bytes(separator) = value else {
                    return Err(ExpressionError::UnexpectedType {
                        want: Kind::BYTES,
                        got: value.kind(),
                        span: separator.span,
                    });
                };

                String::from_utf8_lossy(separator.as_ref()).to_string()
            }
            None => String::new(),
        };

        let joined = array
            .collect::<Result<Vec<_>, ExpressionError>>()?
            .join(separator.as_ref());

        Ok(Value::Bytes(joined.into()))
    }

    fn type_def(&self, _state: &TypeState) -> TypeDef {
        TypeDef::bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::Span;
    use crate::compiler::function::compile_and_run;

    #[test]
    fn comma_separator() {
        compile_and_run(
            vec![
                Expr::Array(vec!["one".into(), "two".into(), "three".into()]),
                ", ".into(),
            ],
            Join,
            TypeDef::bytes(),
            Ok(Value::Bytes("one, two, three".into())),
        )
    }

    #[test]
    fn space_separator() {
        compile_and_run(
            vec![
                Expr::Array(vec!["one".into(), "two".into(), "three".into()]),
                " ".into(),
            ],
            Join,
            TypeDef::bytes(),
            Ok(Value::Bytes("one two three".into())),
        )
    }

    #[test]
    fn without_separator() {
        compile_and_run(
            vec![Expr::Array(vec![
                "one".into(),
                "two".into(),
                "three".into(),
            ])],
            Join,
            TypeDef::bytes(),
            Ok(Value::Bytes("onetwothree".into())),
        )
    }

    #[test]
    fn non_string_array_item() {
        compile_and_run(
            vec![
                Expr::Array(vec!["one".into(), "two".into(), 3.into()]),
                " ".into(),
            ],
            Join,
            TypeDef::bytes(),
            Err(ExpressionError::UnexpectedValue {
                msg: "item must be string instead of integer".to_string(),
                span: Span::empty(),
            }),
        )
    }
}
