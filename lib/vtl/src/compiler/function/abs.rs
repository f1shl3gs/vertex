use value::Value;

use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Function, FunctionCompileContext, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::parser::SyntaxError;
use crate::compiler::span::Spanned;
use crate::compiler::state::TypeState;
use crate::compiler::type_def::TypeDef;
use crate::compiler::{Expression, ExpressionError, Kind, ValueKind};
use crate::context::Context;

pub struct Abs;

impl Function for Abs {
    fn identifier(&self) -> &'static str {
        "abs"
    }

    fn parameters(&self) -> &'static [Parameter] {
        &[Parameter {
            name: "value",
            kind: Kind::NUMERIC,
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
            function: Box::new(AbsFunc { value }),
            span: cx.span,
        })
    }
}

#[derive(Clone)]
struct AbsFunc {
    value: Spanned<Expr>,
}

impl Expression for AbsFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let value = match self.value.resolve(cx)? {
            Value::Float(f) => Value::Float(f.abs()),
            Value::Integer(v) => Value::Integer(v.abs()),
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

    fn type_def(&self, state: &TypeState) -> TypeDef {
        let def = self.value.type_def(state);

        TypeDef {
            fallible: false,
            kind: def.kind,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::compiler::function::compile_and_run;

    #[test]
    fn negative_float() {
        compile_and_run(vec![(-1.1).into()], Abs, TypeDef::float(), Ok(1.1.into()))
    }

    #[test]
    fn negative_integer() {
        compile_and_run(vec![(-1).into()], Abs, TypeDef::integer(), Ok(1.into()))
    }

    #[test]
    fn positive_float() {
        compile_and_run(vec![1.2.into()], Abs, TypeDef::float(), Ok(1.2.into()))
    }

    #[test]
    fn positive_integer() {
        compile_and_run(vec![1.into()], Abs, TypeDef::integer(), Ok(1.into()))
    }
}
