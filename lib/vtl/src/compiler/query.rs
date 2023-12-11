use value::{OwnedTargetPath, OwnedValuePath, Value};

use crate::compiler::{Expression, ExpressionError, Kind, Span, TypeDef};
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
                .map_err(|err| ExpressionError::Error {
                    message: err.to_string(),
                    span: Span { start: 0, end: 0 },
                })?
                .clone(),
            Query::Internal(name, path) => cx
                .variables
                .get(name)
                .expect("variable checked already")
                .get(path)
                .ok_or(ExpressionError::NotFound {
                    path: format!("{}{}", name, path),
                    // TODO: fix
                    span: Span { start: 0, end: 0 },
                })?
                .clone(),
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
