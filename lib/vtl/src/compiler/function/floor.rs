use value::Value;

use crate::compiler::expression::Expression;
use crate::compiler::function::{ArgumentList, Function, FunctionCompileContext, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::parser::Expr;
use crate::compiler::{ExpressionError, Kind, Spanned, TypeDef, ValueKind};
use crate::{Context, SyntaxError};

pub struct Floor;

impl Function for Floor {
    fn identifier(&self) -> &'static str {
        "floor"
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
            function: Box::new(FloorFunc { value, precision }),
            span: cx.span,
        })
    }
}

#[derive(Clone)]
struct FloorFunc {
    value: Spanned<Expr>,
    precision: Option<Spanned<Expr>>,
}

impl Expression for FloorFunc {
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
                Value::Float(f64::floor(f * multiplier) / multiplier)
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
    fn lower() {
        compile_and_run(vec![1.234.into()], Floor, TypeDef::float(), Ok(1.0.into()))
    }
}
