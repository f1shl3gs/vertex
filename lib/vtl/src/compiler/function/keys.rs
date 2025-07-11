use value::Value;

use crate::SyntaxError;
use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Function, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::state::TypeState;
use crate::compiler::{Expression, ExpressionError, Kind, Spanned, TypeDef};
use crate::context::Context;

pub struct Keys;

impl Function for Keys {
    fn identifier(&self) -> &'static str {
        "keys"
    }

    fn parameters(&self) -> &'static [Parameter] {
        &[Parameter {
            name: "value",
            kind: Kind::OBJECT,
            required: true,
        }]
    }

    fn compile(&self, mut arguments: ArgumentList) -> Result<FunctionCall, SyntaxError> {
        let value = arguments.get();

        Ok(FunctionCall {
            function: Box::new(KeysFunc { value }),
        })
    }
}

#[derive(Clone)]
struct KeysFunc {
    value: Spanned<Expr>,
}

impl Expression for KeysFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        match self.value.resolve(cx)? {
            Value::Object(object) => {
                let keys = object.into_keys().map(Value::from).collect();

                Ok(Value::Array(keys))
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
            Keys,
            TypeDef::array(),
            Ok(value!([])),
        )
    }

    #[test]
    fn not_empty() {
        compile_and_run(
            vec![
                value!({
                    "foo": 0
                })
                .into(),
            ],
            Keys,
            TypeDef::array(),
            Ok(value!(["foo"])),
        )
    }
}
