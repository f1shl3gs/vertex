use value::Value;

use crate::SyntaxError;
use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Function, FunctionCompileContext, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::state::TypeState;
use crate::compiler::{Expression, ExpressionError, Kind, Spanned, TypeDef};
use crate::context::Context;

pub struct Values;

impl Function for Values {
    fn identifier(&self) -> &'static str {
        "values"
    }

    fn parameters(&self) -> &'static [Parameter] {
        &[Parameter {
            name: "values",
            kind: Kind::OBJECT,
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
            function: Box::new(ValuesFunc { value }),
            span: cx.span,
        })
    }
}

#[derive(Clone)]
struct ValuesFunc {
    value: Spanned<Expr>,
}

impl Expression for ValuesFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        match self.value.resolve(cx)? {
            Value::Object(object) => {
                let value = object.into_values().collect::<Vec<_>>().into();

                Ok(value)
            }
            value => Err(ExpressionError::UnexpectedType {
                want: Kind::OBJECT,
                got: value.kind(),
                span: self.value.span,
            }),
        }
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
    use value::value;

    use super::*;
    use crate::compiler::function::compile_and_run;

    #[test]
    fn empty() {
        compile_and_run(
            vec![value!({}).into()],
            Values,
            TypeDef::array(),
            Ok(value!([])),
        )
    }

    #[test]
    fn not_empty() {
        compile_and_run(
            vec![
                value!({
                    "foo": 1
                })
                .into(),
            ],
            Values,
            TypeDef::array(),
            Ok(value!([1])),
        )
    }
}
