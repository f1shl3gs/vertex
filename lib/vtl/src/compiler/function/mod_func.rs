use value::{Kind, Value};

use crate::SyntaxError;
use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Function, FunctionCompileContext, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::state::TypeState;
use crate::compiler::{Expression, ExpressionError, Spanned, TypeDef};
use crate::context::Context;

pub struct Mod;

impl Function for Mod {
    fn identifier(&self) -> &'static str {
        "mod"
    }

    fn parameters(&self) -> &'static [Parameter] {
        &[
            Parameter {
                name: "value",
                kind: Kind::NUMERIC,
                required: true,
            },
            Parameter {
                name: "modulus",
                kind: Kind::NUMERIC,
                required: true,
            },
        ]
    }

    fn compile(
        &self,
        cx: FunctionCompileContext,
        mut arguments: ArgumentList,
    ) -> Result<FunctionCall, SyntaxError> {
        let value = arguments.get();
        let modulus = arguments.get();

        Ok(FunctionCall {
            function: Box::new(RemainderFunc { value, modulus }),
            span: cx.span,
        })
    }
}

#[derive(Clone)]
struct RemainderFunc {
    value: Spanned<Expr>,
    modulus: Spanned<Expr>,
}

impl Expression for RemainderFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let value = self.value.resolve(cx)?;

        match value {
            Value::Integer(value) => {
                let modulus = self.modulus.resolve(cx)?;
                let Value::Integer(modulus) = modulus else {
                    return Err(ExpressionError::UnexpectedType {
                        want: Kind::INTEGER,
                        got: modulus.kind(),
                        span: self.modulus.span,
                    });
                };

                Ok((value % modulus).into())
            }
            Value::Float(value) => {
                let modulus = self.modulus.resolve(cx)?;
                let Value::Float(modulus) = modulus else {
                    return Err(ExpressionError::UnexpectedType {
                        want: Kind::FLOAT,
                        got: modulus.kind(),
                        span: self.modulus.span,
                    });
                };

                Ok((value % modulus).into())
            }
            value => Err(ExpressionError::UnexpectedType {
                want: Kind::INTEGER | Kind::FLOAT,
                got: value.kind(),
                span: self.value.span,
            }),
        }
    }

    fn type_def(&self, state: &TypeState) -> TypeDef {
        self.value.type_def(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::function::compile_and_run;

    #[test]
    fn int() {
        compile_and_run(
            vec![5.into(), 2.into()],
            Mod,
            TypeDef::integer(),
            Ok(Value::Integer(1)),
        )
    }

    #[test]
    fn float() {
        compile_and_run(
            vec![Expr::Float(5.0), Expr::Float(2.0)],
            Mod,
            TypeDef::float(),
            Ok(Value::Float(1.0)),
        )
    }
}
