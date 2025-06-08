use value::Value;

use crate::SyntaxError;
use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Function, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::state::TypeState;
use crate::compiler::{Expression, ExpressionError, Kind, Spanned, TypeDef};
use crate::context::Context;

pub struct Includes;

impl Function for Includes {
    fn identifier(&self) -> &'static str {
        "includes"
    }

    fn parameters(&self) -> &'static [Parameter] {
        &[
            Parameter {
                name: "array",
                kind: Kind::ARRAY,
                required: true,
            },
            Parameter {
                name: "item",
                kind: Kind::ANY,
                required: true,
            },
        ]
    }

    fn compile(&self, mut arguments: ArgumentList) -> Result<FunctionCall, SyntaxError> {
        let array = arguments.get();
        let item = arguments.get();

        Ok(FunctionCall {
            function: Box::new(IncludesFunc { array, item }),
        })
    }
}

#[derive(Clone)]
struct IncludesFunc {
    array: Spanned<Expr>,
    item: Spanned<Expr>,
}

impl Expression for IncludesFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let array = match self.array.resolve(cx)? {
            Value::Array(array) => array,
            value => {
                return Err(ExpressionError::UnexpectedType {
                    want: Kind::ARRAY,
                    got: value.kind(),
                    span: self.array.span,
                });
            }
        };

        let item = self.item.resolve(cx)?;

        let found = array.iter().any(|value| value == &item);

        Ok(found.into())
    }

    fn type_def(&self, _state: &TypeState) -> TypeDef {
        TypeDef {
            fallible: false,
            kind: Kind::BOOLEAN,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::function::compile_and_run;

    #[test]
    fn empty_not_included() {
        compile_and_run(
            vec![Expr::Array(vec![]), "foo".into()],
            Includes,
            TypeDef::boolean(),
            Ok(false.into()),
        )
    }

    #[test]
    fn string_include() {
        compile_and_run(
            vec![Expr::Array(vec!["foo".into(), "bar".into()]), "foo".into()],
            Includes,
            TypeDef::boolean(),
            Ok(true.into()),
        )
    }

    #[test]
    fn string_not_include() {
        compile_and_run(
            vec![Expr::Array(vec!["foo".into(), "bar".into()]), "xyz".into()],
            Includes,
            TypeDef::boolean(),
            Ok(false.into()),
        )
    }
}
