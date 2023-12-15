use std::collections::BTreeMap;

use value::Value;

use crate::compiler::function::{ArgumentList, Function, FunctionCompileContext, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::parser::Expr;
use crate::compiler::{Expression, ExpressionError, Kind, Spanned, TypeDef, ValueKind};
use crate::context::Context;
use crate::SyntaxError;

pub struct Compact;

impl Function for Compact {
    fn identifier(&self) -> &'static str {
        "compact"
    }

    fn parameters(&self) -> &'static [Parameter] {
        &[
            Parameter {
                name: "value",
                kind: Kind::CONTAINER,
                required: true,
            },
            Parameter {
                name: "recursive",
                kind: Kind::BOOLEAN,
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
        let recursive = arguments.get_bool_opt()?.unwrap_or(true);

        Ok(FunctionCall {
            function: Box::new(CompactFunc { value, recursive }),
            span: cx.span,
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
                    want: Kind::CONTAINER,
                    got: value.kind(),
                    span: self.value.span,
                })
            }
        };

        Ok(value)
    }

    fn type_def(&self) -> TypeDef {
        self.value.type_def()
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
        let mut input = BTreeMap::new();
        input.insert("foo".to_string(), 1.into());
        input.insert("null".to_string(), Expr::Null);
        input.insert("array".to_string(), Expr::Array(vec![]));

        compile_and_run(
            vec![input.into()],
            Compact,
            TypeDef::object(),
            Ok(value!({
                "foo": 1
            })),
        )
    }

    #[test]
    fn object_no_recursive() {
        let mut input = BTreeMap::new();
        input.insert("foo".to_string(), 1.into());
        input.insert("null".to_string(), Expr::Null);
        input.insert("array".to_string(), Expr::Array(vec![]));

        compile_and_run(
            vec![input.into(), false.into()],
            Compact,
            TypeDef::object(),
            Ok(value!({
                "foo": 1,
                "array": []
            })),
        )
    }
}
