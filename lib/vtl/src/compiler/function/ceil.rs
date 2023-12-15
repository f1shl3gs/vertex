use value::Value;

use crate::compiler::expression::Expression;
use crate::compiler::function::{ArgumentList, Function, FunctionCompileContext, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::parser::Expr;
use crate::compiler::{ExpressionError, Kind, Spanned, TypeDef, ValueKind};
use crate::{Context, SyntaxError};

pub struct Ceil;

impl Function for Ceil {
    fn identifier(&self) -> &'static str {
        "ceil"
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
            function: Box::new(CeilFunc { value, precision }),
            span: cx.span,
        })
    }
}

#[derive(Clone)]
struct CeilFunc {
    value: Spanned<Expr>,
    precision: Option<Spanned<Expr>>,
}

impl Expression for CeilFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let value = match self.value.resolve(cx)? {
            Value::Integer(i) => return Ok(i.into()),
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

                let multiplier = 10.0f64.powf(precision as f64);
                Value::Float(f64::ceil(f * multiplier) / multiplier)
            }
            value => {
                return Err(ExpressionError::UnexpectedType {
                    want: Kind::INTEGER | Kind::FLOAT,
                    got: value.kind(),
                    span: self.value.span,
                })
            }
        };

        Ok(value)
    }

    fn type_def(&self) -> TypeDef {
        let kind = self.value.type_def().kind;
        let kind = if kind == Kind::INTEGER || kind == Kind::FLOAT {
            kind
        } else {
            Kind::NUMERIC
        };

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
    fn lower() {
        compile_and_run(vec![1.2.into()], Ceil, TypeDef::float(), Ok(2.0.into()))
    }

    #[test]
    fn higher() {
        compile_and_run(vec![1.8.into()], Ceil, TypeDef::float(), Ok(2.0.into()))
    }

    #[test]
    fn integer() {
        compile_and_run(vec![123.into()], Ceil, TypeDef::integer(), Ok(123.into()))
    }

    #[test]
    fn precision_one() {
        compile_and_run(
            vec![1.23.into(), 1.into()],
            Ceil,
            TypeDef::float(),
            Ok(1.3.into()),
        )
    }

    #[test]
    fn precision_four() {
        compile_and_run(
            vec![1.23456789.into(), 4.into()],
            Ceil,
            TypeDef::float(),
            Ok(1.2346.into()),
        )
    }
}
