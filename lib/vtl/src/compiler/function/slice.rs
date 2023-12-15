use value::Value;

use crate::compiler::function::{ArgumentList, Function, FunctionCompileContext, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::parser::Expr;
use crate::compiler::{Expression, ExpressionError, Kind, Spanned, TypeDef, ValueKind};
use crate::{Context, SyntaxError};

pub struct Slice;

impl Function for Slice {
    fn identifier(&self) -> &'static str {
        "slice"
    }

    fn parameters(&self) -> &'static [Parameter] {
        &[
            Parameter {
                name: "value",
                kind: Kind::ARRAY_OR_BYTES,
                required: true,
            },
            Parameter {
                name: "start",
                kind: Kind::INTEGER,
                required: true,
            },
            Parameter {
                name: "end",
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
        let start = arguments.get();
        let end = arguments.get_opt();

        Ok(FunctionCall {
            function: Box::new(SliceFunc { value, start, end }),
            span: cx.span,
        })
    }
}

#[derive(Clone)]
struct SliceFunc {
    value: Spanned<Expr>,
    start: Spanned<Expr>,
    end: Option<Spanned<Expr>>,
}

impl Expression for SliceFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let value = self.value.resolve(cx)?;
        let len = match &value {
            Value::Bytes(b) => b.len() as i64,
            Value::Array(array) => array.len() as i64,
            _ => {
                return Err(ExpressionError::UnexpectedType {
                    want: Kind::ARRAY | Kind::BYTES,
                    got: value.kind(),
                    span: self.value.span,
                })
            }
        };

        // build the range
        let start = match self.start.resolve(cx)? {
            Value::Integer(original @ start) => {
                let start = if start < 0 { start + len } else { start };
                if start < 0 || start > len {
                    return Err(ExpressionError::UnexpectedValue {
                        msg: format!(
                            "start \"{}\" must be between \"{}\" and \"{}\"",
                            original, -len, len
                        ),
                        span: self.start.span,
                    });
                }

                start
            }
            value => {
                return Err(ExpressionError::UnexpectedType {
                    want: Kind::INTEGER,
                    got: value.kind(),
                    span: self.start.span,
                })
            }
        };

        let end = match &self.end {
            Some(expr) => match expr.resolve(cx)? {
                Value::Integer(original @ end) => {
                    let end = if end < 0 { end + len } else { end };
                    if end < start {
                        return Err(ExpressionError::UnexpectedValue {
                            msg: format!(
                                "end {} must be greater or equal to start {}",
                                original, start
                            ),
                            span: expr.span,
                        });
                    }

                    if end > len {
                        len
                    } else {
                        end
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
            None => len,
        };

        let range = start as usize..end as usize;
        match value {
            Value::Bytes(b) => Ok(b.slice(range).into()),
            Value::Array(mut array) => {
                let array = array.drain(range).collect::<Vec<_>>();
                Ok(array.into())
            }
            _ => unreachable!(),
        }
    }

    fn type_def(&self) -> TypeDef {
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
    use crate::compiler::Span;

    #[test]
    fn bytes_zero() {
        compile_and_run(
            vec!["foo".into(), 0.into()],
            Slice,
            TypeDef::bytes(),
            Ok("foo".into()),
        )
    }

    #[test]
    fn bytes_one() {
        compile_and_run(
            vec!["foo".into(), 1.into()],
            Slice,
            TypeDef::bytes(),
            Ok("oo".into()),
        )
    }

    #[test]
    fn bytes_two() {
        compile_and_run(
            vec!["foo".into(), 2.into()],
            Slice,
            TypeDef::bytes(),
            Ok("o".into()),
        )
    }

    #[test]
    fn bytes_three() {
        compile_and_run(
            vec!["foo".into(), 3.into()],
            Slice,
            TypeDef::bytes(),
            Ok("".into()),
        )
    }

    #[test]
    fn bytes_neg_two() {
        compile_and_run(
            vec!["foo".into(), (-2).into()],
            Slice,
            TypeDef::bytes(),
            Ok("oo".into()),
        )
    }

    #[test]
    fn bytes_two_to_two() {
        compile_and_run(
            vec!["foo".into(), 2.into(), 2.into()],
            Slice,
            TypeDef::bytes(),
            Ok("".into()),
        )
    }

    #[test]
    fn bytes_zero_to_four() {
        compile_and_run(
            vec!["foo".into(), 0.into(), 4.into()],
            Slice,
            TypeDef::bytes(),
            Ok("foo".into()),
        )
    }

    #[test]
    fn bytes_one_to_five() {
        compile_and_run(
            vec!["foo".into(), 1.into(), 5.into()],
            Slice,
            TypeDef::bytes(),
            Ok("oo".into()),
        )
    }

    #[test]
    fn bytes_neg_seven() {
        compile_and_run(
            vec!["abcdefghijklmno".into(), (-7).into()],
            Slice,
            TypeDef::bytes(),
            Ok("ijklmno".into()),
        )
    }

    #[test]
    fn bytes_five_to_nine() {
        compile_and_run(
            vec!["abcdefghijklmno".into(), 5.into(), 9.into()],
            Slice,
            TypeDef::bytes(),
            Ok("fghi".into()),
        )
    }

    #[test]
    fn array_zero() {
        compile_and_run(
            vec![
                vec![Expr::from(0), Expr::from(1), Expr::from(2)].into(),
                0.into(),
            ],
            Slice,
            TypeDef::bytes(),
            Ok(vec![Value::from(0), Value::from(1), Value::from(2)].into()),
        )
    }

    #[test]
    fn array_one() {
        compile_and_run(
            vec![
                vec![Expr::from(0), Expr::from(1), Expr::from(2)].into(),
                1.into(),
            ],
            Slice,
            TypeDef::bytes(),
            Ok(vec![Value::from(1), Value::from(2)].into()),
        )
    }

    #[test]
    fn array_two() {
        compile_and_run(
            vec![
                vec![Expr::from(0), Expr::from(1), Expr::from(2)].into(),
                2.into(),
            ],
            Slice,
            TypeDef::bytes(),
            Ok(vec![Value::from(2)].into()),
        )
    }

    #[test]
    fn array_three() {
        compile_and_run(
            vec![
                vec![Expr::from(0), Expr::from(1), Expr::from(2)].into(),
                3.into(),
            ],
            Slice,
            TypeDef::bytes(),
            Ok(Value::from(Vec::<Value>::new())),
        )
    }

    #[test]
    fn array_neg_two() {
        compile_and_run(
            vec![
                vec![Expr::from(0), Expr::from(1), Expr::from(2)].into(),
                0.into(),
            ],
            Slice,
            TypeDef::bytes(),
            Ok(vec![Value::from(0), Value::from(1), Value::from(2)].into()),
        )
    }

    #[test]
    fn array_mixed_type() {
        compile_and_run(
            vec![vec![1.into(), true.into(), "foo".into()].into(), 1.into()],
            Slice,
            TypeDef::bytes(),
            Ok(vec![Value::Boolean(true), "foo".into()].into()),
        )
    }

    #[test]
    fn error_start_after_end() {
        compile_and_run(
            vec!["foo".into(), 4.into()],
            Slice,
            TypeDef::bytes(),
            Err(ExpressionError::UnexpectedValue {
                msg: "start \"4\" must be between \"-3\" and \"3\"".to_string(),
                span: Span::empty(),
            }),
        )
    }

    #[test]
    fn error_minus_before_start() {
        compile_and_run(
            vec!["foo".into(), (-4).into()],
            Slice,
            TypeDef::bytes(),
            Err(ExpressionError::UnexpectedValue {
                msg: "start \"-4\" must be between \"-3\" and \"3\"".to_string(),
                span: Span::empty(),
            }),
        )
    }

    #[test]
    fn error_after_end() {
        compile_and_run(
            vec!["foo".into(), 4.into()],
            Slice,
            TypeDef::bytes(),
            Err(ExpressionError::UnexpectedValue {
                msg: "start \"4\" must be between \"-3\" and \"3\"".to_string(),
                span: Span::empty(),
            }),
        )
    }

    #[test]
    fn error_start_end() {
        compile_and_run(
            vec!["foo".into(), 2.into(), 1.into()],
            Slice,
            TypeDef::bytes(),
            Err(ExpressionError::UnexpectedValue {
                msg: "end 1 must be greater or equal to start 2".to_string(),
                span: Span::empty(),
            }),
        )
    }
}
