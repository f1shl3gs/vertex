use value::Value;

use crate::compiler::function::{ArgumentList, Function, FunctionCompileContext, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::parser::Expr;
use crate::compiler::{Expression, ExpressionError, Kind, Spanned, TypeDef, ValueKind};
use crate::{Context, SyntaxError};

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

    fn compile(
        &self,
        cx: FunctionCompileContext,
        mut arguments: ArgumentList,
    ) -> Result<FunctionCall, SyntaxError> {
        let value = arguments.get();

        Ok(FunctionCall {
            function: Box::new(KeysFunc { value }),
            span: cx.span,
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

    fn type_def(&self) -> TypeDef {
        TypeDef {
            fallible: false,
            kind: Kind::ARRAY,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use value::value;

    use super::*;
    use crate::compiler::function::compile_and_run;

    #[test]
    fn empty() {
        let input = BTreeMap::new();

        compile_and_run(
            vec![Expr::Object(input)],
            Keys,
            TypeDef::array(),
            Ok(value!([])),
        )
    }

    #[test]
    fn not_empty() {
        let mut input = BTreeMap::new();
        input.insert("foo".into(), 0.into());

        compile_and_run(
            vec![Expr::Object(input)],
            Keys,
            TypeDef::array(),
            Ok(value!(["foo"])),
        )
    }
}
