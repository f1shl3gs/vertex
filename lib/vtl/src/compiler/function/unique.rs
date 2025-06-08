use std::collections::HashSet;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use value::Value;

use crate::SyntaxError;
use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Function, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::state::TypeState;
use crate::compiler::{Expression, ExpressionError, Kind, Spanned, TypeDef};
use crate::context::Context;

pub struct Unique;

impl Function for Unique {
    fn identifier(&self) -> &'static str {
        "unique"
    }

    fn parameters(&self) -> &'static [Parameter] {
        &[Parameter {
            name: "value",
            kind: Kind::ARRAY,
            required: true,
        }]
    }

    fn compile(&self, mut arguments: ArgumentList) -> Result<FunctionCall, SyntaxError> {
        let value = arguments.get();

        Ok(FunctionCall {
            function: Box::new(UniqueFunc { value }),
        })
    }
}

#[derive(Clone)]
struct UniqueFunc {
    value: Spanned<Expr>,
}

impl Expression for UniqueFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        match self.value.resolve(cx)? {
            Value::Array(mut array) => {
                let mut exists: HashSet<u64> = HashSet::with_capacity(array.len());

                array.retain(|item| {
                    let mut hasher = DefaultHasher::new();
                    item.hash(&mut hasher);
                    let h = hasher.finish();

                    if exists.contains(&h) {
                        false
                    } else {
                        exists.insert(h);
                        true
                    }
                });

                Ok(Value::Array(array))
            }
            value => Err(ExpressionError::UnexpectedType {
                want: Kind::ARRAY,
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
    use super::*;
    use crate::compiler::function::compile_and_run;
    use value::value;

    #[test]
    fn simple() {
        compile_and_run(
            vec![Expr::Array(vec!["foo".into(), "bar".into(), "foo".into()])],
            Unique,
            TypeDef::array(),
            Ok(value!(["foo", "bar"])),
        )
    }
}
