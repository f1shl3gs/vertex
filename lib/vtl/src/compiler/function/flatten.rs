use std::collections::{BTreeMap, btree_map};

use value::Value;

use crate::SyntaxError;
use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Function, FunctionCompileContext, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::state::TypeState;
use crate::compiler::{Expression, ExpressionError, Kind, Spanned, TypeDef};
use crate::context::Context;

pub struct Flatten;

impl Function for Flatten {
    fn identifier(&self) -> &'static str {
        "flatten"
    }

    fn parameters(&self) -> &'static [Parameter] {
        &[
            Parameter {
                name: "value",
                kind: Kind::ARRAY_OR_OBJECT,
                required: true,
            },
            Parameter {
                name: "separator",
                kind: Kind::BYTES,
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
        let separator = arguments
            .get_string_opt()?
            .map(|s| s.node)
            .unwrap_or(".".to_string());

        Ok(FunctionCall {
            function: Box::new(FlattenFunc { value, separator }),
            span: cx.span,
        })
    }
}

#[derive(Clone)]
struct FlattenFunc {
    value: Spanned<Expr>,
    separator: String,
}

impl Expression for FlattenFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let value = match self.value.resolve(cx)? {
            Value::Array(array) => {
                let af = ArrayFlatten {
                    values: array.iter(),
                    inner: None,
                };

                af.cloned().collect::<Vec<_>>().into()
            }
            Value::Object(object) => MapFlatten::new(object.iter(), &self.separator)
                .map(|(k, v)| (k, v.clone()))
                .collect::<BTreeMap<_, _>>()
                .into(),
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
        TypeDef {
            fallible: false,
            kind: self.value.type_def(state).kind,
        }
    }
}

/// Create an iterator that can walk a tree of Array values. This can be
/// used to flatten the array.
struct ArrayFlatten<'a> {
    values: std::slice::Iter<'a, Value>,
    inner: Option<Box<ArrayFlatten<'a>>>,
}

impl<'a> ArrayFlatten<'a> {
    fn new(values: std::slice::Iter<'a, Value>) -> Self {
        ArrayFlatten {
            values,
            inner: None,
        }
    }
}

impl<'a> Iterator for ArrayFlatten<'a> {
    type Item = &'a Value;

    fn next(&mut self) -> Option<Self::Item> {
        // Iterate over our inner list first.
        if let Some(ref mut inner) = self.inner {
            let next = inner.next();

            match next {
                Some(_) => return next,
                None => {
                    // The inner list has been exhausted.
                    self.inner = None;
                }
            }
        }

        // Then iterate over our values.
        let next = self.values.next();
        match next {
            Some(Value::Array(next)) => {
                // Create a new iterator for this child list.
                self.inner = Some(Box::new(ArrayFlatten::new(next.iter())));
                self.next()
            }
            _ => next,
        }
    }
}

/// An iterator to walk over maps allowing us to flatten nested maps
/// to a single level.
struct MapFlatten<'a> {
    values: btree_map::Iter<'a, String, Value>,
    separator: &'a str,
    inner: Option<Box<MapFlatten<'a>>>,
    parent: Option<String>,
}

impl<'a> MapFlatten<'a> {
    fn new(values: btree_map::Iter<'a, String, Value>, separator: &'a str) -> Self {
        Self {
            values,
            separator,
            inner: None,
            parent: None,
        }
    }

    fn new_from_parent(
        parent: String,
        values: btree_map::Iter<'a, String, Value>,
        separator: &'a str,
    ) -> Self {
        Self {
            values,
            separator,
            inner: None,
            parent: Some(parent),
        }
    }

    fn new_key(&self, key: &str) -> String {
        match self.parent {
            None => key.to_string(),
            Some(ref parent) => format!("{parent}{}{key}", self.separator),
        }
    }
}

impl<'a> Iterator for MapFlatten<'a> {
    type Item = (String, &'a Value);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(ref mut inner) = self.inner {
            let next = inner.next();
            match next {
                Some(_) => return next,
                None => self.inner = None,
            }
        }

        let next = self.values.next();
        match next {
            Some((key, Value::Object(value))) => {
                self.inner = Some(Box::new(MapFlatten::new_from_parent(
                    self.new_key(key),
                    value.iter(),
                    self.separator,
                )));

                self.next()
            }
            Some((key, value)) => Some((self.new_key(key), value)),
            None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::function::compile_and_run;
    use value::value;

    #[test]
    fn empty() {
        compile_and_run(
            vec![vec![].into()],
            Flatten,
            TypeDef::array(),
            Ok(value!([])),
        )
    }

    #[test]
    fn one_element() {
        compile_and_run(
            vec![vec![1.into()].into()],
            Flatten,
            TypeDef::array(),
            Ok(value!([1])),
        )
    }

    #[test]
    fn nested_array() {
        compile_and_run(
            vec![vec![1.into(), vec![2.into(), 3.into()].into()].into()],
            Flatten,
            TypeDef::array(),
            Ok(value!([1, 2, 3])),
        )
    }

    #[test]
    fn nested_empty_array() {
        compile_and_run(
            vec![vec![1.into(), vec![].into(), vec![2.into(), 3.into()].into()].into()],
            Flatten,
            TypeDef::array(),
            Ok(value!([1, 2, 3])),
        )
    }

    #[test]
    fn double_nested_array() {
        compile_and_run(
            vec![
                vec![
                    1.into(),
                    vec![2.into(), 3.into(), vec![4.into(), 5.into()].into()].into(),
                ]
                .into(),
            ],
            Flatten,
            TypeDef::array(),
            Ok(value!([1, 2, 3, 4, 5])),
        )
    }

    #[test]
    fn two_array() {
        compile_and_run(
            vec![
                vec![
                    vec![1.into(), 2.into()].into(),
                    vec![3.into(), 4.into()].into(),
                ]
                .into(),
            ],
            Flatten,
            TypeDef::array(),
            Ok(value!([1, 2, 3, 4])),
        )
    }

    #[test]
    fn map() {
        compile_and_run(
            vec![
                value!({
                    parent: "child",
                })
                .into(),
            ],
            Flatten,
            TypeDef::object(),
            Ok(value!({
                parent: "child"
            })),
        )
    }

    #[test]
    fn nested_map() {
        compile_and_run(
            vec![
                value!({
                    parent: {
                        child1: 1,
                        child2: 2
                    },
                    key: "val"
                })
                .into(),
            ],
            Flatten,
            TypeDef::object(),
            Ok(value!({
                "parent.child1": 1,
                "parent.child2": 2,
                key: "val"
            })),
        )
    }

    #[test]
    fn nested_map_with_separator() {
        compile_and_run(
            vec![
                value!({
                    parent: {
                        child1: 1,
                        child2: 2
                    },
                    key: "val"
                })
                .into(),
                "_".into(),
            ],
            Flatten,
            TypeDef::object(),
            Ok(value!({
                "parent_child1": 1,
                "parent_child2": 2,
                key: "val"
            })),
        )
    }

    #[test]
    fn double_nested_map() {
        compile_and_run(
            vec![
                value!({
                    parent: {
                        child1: 1,
                        child2: { grandchild1: 1, grandchild2: 2 },
                    },
                    key: "val",
                })
                .into(),
            ],
            Flatten,
            TypeDef::object(),
            Ok(value!({
                "parent.child1": 1,
                "parent.child2.grandchild1": 1,
                "parent.child2.grandchild2": 2,
                key: "val",
            })),
        )
    }

    #[test]
    fn map_and_array() {
        compile_and_run(
            vec![
                value!({
                    parent: {
                        child1: [1, [2, 3]],
                        child2: {grandchild1: 1, grandchild2: [1, [2, 3], 4]},
                    },
                    key: "val",
                })
                .into(),
            ],
            Flatten,
            TypeDef::object(),
            Ok(value!({
                "parent.child1": [1, [2, 3]],
                "parent.child2.grandchild1": 1,
                "parent.child2.grandchild2": [1, [2, 3], 4],
                key: "val",
            })),
        )
    }

    #[test]
    fn map_and_array_with_separator() {
        compile_and_run(
            vec![
                value!({
                    parent: {
                        child1: [1, [2, 3]],
                        child2: {grandchild1: 1, grandchild2: [1, [2, 3], 4]},
                    },
                    key: "val",
                })
                .into(),
                "_".into(),
            ],
            Flatten,
            TypeDef::object(),
            Ok(value!({
                "parent_child1": [1, [2, 3]],
                "parent_child2_grandchild1": 1,
                "parent_child2_grandchild2": [1, [2, 3], 4],
                key: "val",
            })),
        )
    }

    // If the root object is an array, child maps are not flattened.
    #[test]
    fn root_array() {
        compile_and_run(
            vec![
                value!([
                    { parent1: { child1: 1, child2: 2 } },
                    [
                        { parent2: { child3: 3, child4: 4 } },
                        { parent3: { child5: 5 } },
                    ],
                ])
                .into(),
            ],
            Flatten,
            TypeDef::array(),
            Ok(value!([
                { parent1: { child1: 1, child2: 2 } },
                { parent2: { child3: 3, child4: 4 } },
                { parent3: { child5: 5 } },
            ])),
        )
    }

    #[test]
    fn triple_nested_map() {
        compile_and_run(
            vec![
                value!({
                    parent1: {
                        child1: { grandchild1: 1 },
                        child2: { grandchild2: 2, grandchild3: 3 },
                    },
                    parent2: 4,
                })
                .into(),
            ],
            Flatten,
            TypeDef::object(),
            Ok(value!({
                "parent1.child1.grandchild1": 1,
                "parent1.child2.grandchild2": 2,
                "parent1.child2.grandchild3": 3,
                parent2: 4,
            })),
        )
    }
}
