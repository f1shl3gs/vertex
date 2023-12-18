use crate::compiler::Kind;
use value::{OwnedTargetPath, OwnedValuePath, Value};

use super::state::TypeState;
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
        let kind = state.get_query_kind(self);

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
