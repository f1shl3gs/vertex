use value::Value;

use crate::compiler::function::{ArgumentList, Function, FunctionCompileContext, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::parser::Expr;
use crate::compiler::{Expression, ExpressionError, Kind, Spanned, TypeDef, ValueKind};
use crate::{Context, SyntaxError};

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

    fn type_def(&self) -> TypeDef {
        TypeDef {
            fallible: false,
            kind: Kind::ARRAY,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::function::compile_and_run;
    use std::collections::BTreeMap;
    use value::value;

    #[test]
    fn empty() {
        let input = BTreeMap::new();

        compile_and_run(
            vec![Expr::Object(input)],
            Values,
            TypeDef::array(),
            Ok(value!([])),
        )
    }

    #[test]
    fn not_empty() {
        let mut input = BTreeMap::new();
        input.insert("foo".to_string(), 1.into());

        compile_and_run(
            vec![Expr::Object(input)],
            Values,
            TypeDef::array(),
            Ok(value!([1])),
        )
    }
}
