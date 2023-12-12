use value::{OwnedTargetPath, OwnedValuePath, Value};

use crate::compiler::{Expression, ExpressionError, Kind, TypeDef};
use crate::Context;

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

    fn type_def(&self) -> TypeDef {
        TypeDef {
            fallible: false,
            kind: Kind::ANY,
        }
    }
}
