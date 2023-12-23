use value::path::PathPrefix;
use value::{OwnedTargetPath, OwnedValuePath, Value};

use super::state::TypeState;
use super::Kind;
use super::{Expression, ExpressionError, TypeDef};
use crate::context::Context;

#[derive(Clone)]
pub enum Query {
    // .foo or %foo
    External(OwnedTargetPath),

    // url.host
    Internal(String, OwnedValuePath),
}

impl Expression for Query {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let value = match self {
            Query::External(path) => cx
                .target
                .get(path)
                .unwrap_or_default()
                .cloned()
                .unwrap_or(Value::Null),
            Query::Internal(name, path) => cx
                .variables
                .get(name)
                .expect("variable checked already at compile-time")
                .get(path)
                .cloned()
                .unwrap_or(Value::Null),
        };

        Ok(value)
    }

    fn type_def(&self, state: &TypeState) -> TypeDef {
        let kind = match self {
            Query::Internal(name, value_path) => {
                let variable = state.variable(name).expect("must exists");
                variable.kind(Some(value_path))
            }

            Query::External(target_path) => {
                let value_path = &target_path.path;

                match target_path.prefix {
                    PathPrefix::Event => state.target.kind(Some(value_path)),
                    PathPrefix::Metadata => state.metadata.kind(Some(value_path)),
                }
            }
        };

        if kind == Kind::UNDEFINED {
            // path is not valid
            TypeDef {
                fallible: true,
                kind: Kind::ANY,
            }
        } else {
            TypeDef {
                fallible: false,
                kind,
            }
        }
    }
}
