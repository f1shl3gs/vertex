use std::collections::BTreeMap;

use value::Value;

use crate::SyntaxError;
use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Function, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::state::TypeState;
use crate::compiler::{Expression, ExpressionError, Kind, Spanned, TypeDef};
use crate::context::Context;

pub struct Compact;

impl Function for Compact {
    fn identifier(&self) -> &'static str {
        "compact"
    }

    fn parameters(&self) -> &'static [Parameter] {
        &[
            Parameter {
                name: "value",
                kind: Kind::ARRAY_OR_OBJECT,
                required: true,
            },
            Parameter {
                name: "recursive",
                kind: Kind::BOOLEAN,
                required: false,
            },
        ]
    }

    fn compile(&self, mut arguments: ArgumentList) -> Result<FunctionCall, SyntaxError> {
        let value = arguments.get();
        let recursive = arguments.get_bool_opt()?.unwrap_or(true);

        Ok(FunctionCall {
            function: Box::new(CompactFunc { value, recursive }),
        })
    }
}

#[derive(Clone)]
struct CompactFunc {
    value: Spanned<Expr>,
    recursive: bool,
}

impl Expression for CompactFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let value = match self.value.resolve(cx)? {
            Value::Array(array) => compact_array(array, self.recursive).into(),
            Value::Object(object) => compact_object(object, self.recursive).into(),
            value => {
                return Err(ExpressionError::UnexpectedType {
                    want: Kind::ARRAY_OR_OBJECT,
                    got: value.kind(),
                    span: self.value.span,
                });
            }
        };

        Ok(value)
    }

    fn type_def(&self, state: &TypeState) -> TypeDef {
        self.value.type_def(state)
    }
}

fn compact_array(array: Vec<Value>, recursive: bool) -> Vec<Value> {
    array
        .into_iter()
        .filter_map(|item| match item {
            Value::Array(array) if recursive => {
                let compacted = compact_array(array, recursive);
                if compacted.is_empty() {
                    None
                } else {
                    Some(compacted.into())
                }
            }
            Value::Object(object) if recursive => {
                let compacted = compact_object(object, recursive);
                if compacted.is_empty() {
                    None
                } else {
                    Some(compacted.into())
                }
            }
            Value::Null => None,
            value => Some(value),
        })
        .collect()
}

fn compact_object(object: BTreeMap<String, Value>, recursive: bool) -> BTreeMap<String, Value> {
    object
        .into_iter()
        .filter_map(|(key, value)| match value {
            Value::Array(array) if recursive => {
                let compacted = compact_array(array, recursive);
                if compacted.is_empty() {
                    None
                } else {
                    Some((key, compacted.into()))
                }
            }
            Value::Object(object) if recursive => {
                let compacted = compact_object(object, recursive);
                if compacted.is_empty() {
                    None
                } else {
                    Some((key, compacted.into()))
                }
            }
            Value::Null => None,
            value => Some((key, value)),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::function::compile_and_run;
    use value::value;

    #[test]
    fn array() {
        compile_and_run(
            vec![vec![1.into(), Expr::Null, Expr::Array(vec![])].into()],
            Compact,
            TypeDef::array(),
            Ok(value!([1])),
        )
    }

    #[test]
    fn array_no_recursive() {
        compile_and_run(
            vec![
                vec![1.into(), Expr::Null, Expr::Array(vec![])].into(),
                false.into(),
            ],
            Compact,
            TypeDef::array(),
            Ok(value!([1, []])),
        )
    }

    #[test]
    fn object() {
        compile_and_run(
            vec![
                value!({
                    "foo": 1,
                    "null": null,
                    "array": []
                })
                .into(),
            ],
            Compact,
            TypeDef::object(),
            Ok(value!({
                "foo": 1
            })),
        )
    }

    #[test]
    fn object_no_recursive() {
        compile_and_run(
            vec![
                value!({
                    "foo": 1,
                    "null": null,
                    "array": []
                })
                .into(),
                false.into(),
            ],
            Compact,
            TypeDef::object(),
            Ok(value!({
                "foo": 1,
                "array": []
            })),
        )
    }
}
