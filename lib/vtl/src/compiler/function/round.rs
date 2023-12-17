use value::Value;

use crate::compiler::function::{ArgumentList, Function, FunctionCompileContext, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::parser::Expr;
use crate::compiler::{Expression, ExpressionError, Kind, Spanned, TypeDef, ValueKind};
use crate::context::Context;
use crate::SyntaxError;

pub struct Round;

impl Function for Round {
    fn identifier(&self) -> &'static str {
        "round"
    }

    fn parameters(&self) -> &'static [Parameter] {
        &[
            Parameter {
                name: "value",
                kind: Kind::NUMERIC,
                required: true,
            },
            Parameter {
                name: "precision",
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
        let precision = arguments.get_opt();

        Ok(FunctionCall {
            function: Box::new(RoundFunc { value, precision }),
            span: cx.span,
        })
    }
}

#[derive(Clone)]
struct RoundFunc {
    value: Spanned<Expr>,
    precision: Option<Spanned<Expr>>,
}

impl Expression for RoundFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let value = match self.value.resolve(cx)? {
            value @ Value::Integer(_) => value,
            Value::Float(f) => {
                let precision = match &self.precision {
                    Some(expr) => match expr.resolve(cx)? {
                        Value::Integer(i) => i,
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

                let multiplier = 10f64.powf(precision as f64);
                Value::Float(f64::round(f * multiplier) / multiplier)
            }
            value => {
                return Err(ExpressionError::UnexpectedType {
                    want: Kind::NUMERIC,
                    got: value.kind(),
                    span: self.value.span,
                })
            }
        };

        Ok(value)
    }

    fn type_def(&self) -> TypeDef {
        let kind = self.value.type_def().kind;

        TypeDef {
            fallible: false,
            kind,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::function::compile_and_run;

    #[test]
    fn down() {
        compile_and_run(vec![1.2.into()], Round, TypeDef::float(), Ok(1.0.into()))
    }

    #[test]
    fn up() {
        compile_and_run(vec![1.6.into()], Round, TypeDef::float(), Ok(2.0.into()))
    }

    #[test]
    fn integer() {
        compile_and_run(vec![1.into()], Round, TypeDef::integer(), Ok(1.into()))
    }

    #[test]
    fn precision() {
        compile_and_run(
            vec![1.23456789.into(), 1.into()],
            Round,
            TypeDef::float(),
            Ok(1.2.into()),
        )
    }

    #[test]
    fn bigger_precision() {
        compile_and_run(
            vec![1.23456789.into(), 4.into()],
            Round,
            TypeDef::float(),
            Ok(1.2346.into()),
        )
    }

    #[test]
    fn huge() {
        compile_and_run(
            vec![
                9_876_543_210_123_456_789_098_765_432_101_234_567_890_987_654_321.987_654_321
                    .into(),
                5.into(),
            ],
            Round,
            TypeDef::float(),
            Ok(9_876_543_210_123_456_789_098_765_432_101_234_567_890_987_654_321.987_65.into()),
        )
    }
}
