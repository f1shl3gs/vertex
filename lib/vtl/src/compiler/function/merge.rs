use std::collections::BTreeMap;

use value::Value;

use crate::SyntaxError;
use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Function, FunctionCompileContext, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::state::TypeState;
use crate::compiler::{Expression, ExpressionError, Kind, Spanned, TypeDef};
use crate::context::Context;

pub struct Merge;

impl Function for Merge {
    fn identifier(&self) -> &'static str {
        "merge"
    }

    fn parameters(&self) -> &'static [Parameter] {
        &[
            Parameter {
                name: "from",
                kind: Kind::OBJECT,
                required: true,
            },
            Parameter {
                name: "to",
                kind: Kind::OBJECT,
                required: true,
            },
            Parameter {
                name: "deep",
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
        let from = arguments.get();
        let to = arguments.get();
        let deep = arguments.get_bool_opt()?.unwrap_or(false);

        Ok(FunctionCall {
            function: Box::new(MergeFunc { from, to, deep }),
            span: cx.span,
        })
    }
}

#[derive(Clone)]
struct MergeFunc {
    from: Spanned<Expr>,
    to: Spanned<Expr>,
    deep: bool,
}

impl Expression for MergeFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let from = match self.from.resolve(cx)? {
            Value::Object(map) => map,
            value => {
                return Err(ExpressionError::UnexpectedType {
                    want: Kind::OBJECT,
                    got: value.kind(),
                    span: self.from.span,
                });
            }
        };

        let mut to = match self.to.resolve(cx)? {
            Value::Object(map) => map,
            value => {
                return Err(ExpressionError::UnexpectedType {
                    want: Kind::OBJECT,
                    got: value.kind(),
                    span: self.to.span,
                });
            }
        };

        merge_maps(&mut to, &from, self.deep);

        Ok(to.into())
    }

    fn type_def(&self, _state: &TypeState) -> TypeDef {
        TypeDef {
            fallible: false,
            kind: Kind::OBJECT,
        }
    }
}

fn merge_maps(map1: &mut BTreeMap<String, Value>, map2: &BTreeMap<String, Value>, deep: bool) {
    for (key2, value2) in map2 {
        match (deep, map1.get_mut(key2), value2) {
            (true, Some(Value::Object(child1)), Value::Object(child2)) => {
                // We are doing a deep merge and both fields are maps.
                merge_maps(child1, child2, deep);
            }
            _ => {
                map1.insert(key2.clone(), value2.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use value::value;

    use super::*;
    use crate::compiler::function::compile_and_run;

    #[test]
    fn simple() {
        compile_and_run(
            vec![
                value!({
                    "key1": "val1",
                })
                .into(),
                value!({
                    "key2": "val2"
                })
                .into(),
            ],
            Merge,
            TypeDef::object(),
            Ok(value!({
                "key1": "val1",
                "key2": "val2"
            })),
        )
    }

    #[test]
    fn shallow() {
        compile_and_run(
            vec![
                value!({
                    "key2": "val2",
                    "child": {
                        "grandchild2": true
                    }
                })
                .into(),
                value!({
                    "key1": "val1",
                    "child": {
                        "grandchild1": "val1"
                    }
                })
                .into(),
            ],
            Merge,
            TypeDef::object(),
            Ok(value!({
                "key1": "val1",
                "key2": "val2",
                "child": {
                    "grandchild2": true
                }
            })),
        )
    }

    #[test]
    fn deep() {
        compile_and_run(
            vec![
                value!({
                    "key1": "val1",
                    "child": {
                        "grandchild1": "val1"
                    }
                })
                .into(),
                value!({
                    "key2": "val2",
                    "child": {
                        "grandchild2": true
                    }
                })
                .into(),
                true.into(),
            ],
            Merge,
            TypeDef::object(),
            Ok(value!({
                "key1": "val1",
                "key2": "val2",
                "child": {
                    "grandchild1": "val1",
                    "grandchild2": true
                }
            })),
        )
    }
}
