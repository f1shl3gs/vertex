use crate::compiler::function::{ArgumentList, Function, FunctionCompileContext, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::parser::Expr;
use crate::compiler::{Expression, ExpressionError, Kind, Spanned, TypeDef, ValueKind};
use crate::{Context, SyntaxError};
use std::collections::BTreeMap;
use value::Value;

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
                })
            }
        };

        let mut to = match self.to.resolve(cx)? {
            Value::Object(map) => map,
            value => {
                return Err(ExpressionError::UnexpectedType {
                    want: Kind::OBJECT,
                    got: value.kind(),
                    span: self.to.span,
                })
            }
        };

        merge_maps(&mut to, &from, self.deep);

        Ok(to.into())
    }

    fn type_def(&self) -> TypeDef {
        TypeDef {
            fallible: false,
            kind: Kind::OBJECT,
        }
    }
}

fn merge_maps(map1: &mut BTreeMap<String, Value>, map2: &BTreeMap<String, Value>, deep: bool) {
    for (key2, value2) in map2 {
        match (deep, map1.get_mut(key2), value2) {
            (true, Some(Value::Object(ref mut child1)), Value::Object(ref child2)) => {
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
    use super::*;
    use crate::compiler::function::compile_and_run;
    use crate::compiler::Span;
    use value::value;

    #[test]
    fn simple() {
        let mut to = BTreeMap::new();
        to.insert("key1".to_string(), "val1".into());

        let mut from = BTreeMap::new();
        from.insert("key2".to_string(), "val2".into());

        compile_and_run(
            vec![Expr::Object(from), Expr::Object(to)],
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
        let mut to = BTreeMap::new();
        to.insert("key1".to_string(), "val1".into());
        to.insert("child".to_string(), {
            let mut map = BTreeMap::new();
            map.insert("grandchild1".to_string(), "val1".into());
            Expr::Object(map).with(Span::empty())
        });

        let mut from = BTreeMap::new();
        from.insert("key2".to_string(), "val2".into());
        from.insert("child".to_string(), {
            let mut map = BTreeMap::new();
            map.insert("grandchild2".to_string(), true.into());
            Expr::Object(map).with(Span::empty())
        });

        compile_and_run(
            vec![Expr::Object(from), Expr::Object(to)],
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
        let mut to = BTreeMap::new();
        to.insert("key1".to_string(), "val1".into());
        to.insert("child".to_string(), {
            let mut map = BTreeMap::new();
            map.insert("grandchild1".to_string(), "val1".into());
            Expr::Object(map).with(Span::empty())
        });

        let mut from = BTreeMap::new();
        from.insert("key2".to_string(), "val2".into());
        from.insert("child".to_string(), {
            let mut map = BTreeMap::new();
            map.insert("grandchild2".to_string(), true.into());
            Expr::Object(map).with(Span::empty())
        });

        compile_and_run(
            vec![Expr::Object(from), Expr::Object(to), true.into()],
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
