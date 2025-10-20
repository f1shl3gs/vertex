use value::Value;

use super::state::TypeState;
use super::statement::Statement;
use super::{Expression, ExpressionError, TypeDef};
use crate::context::Context;

#[derive(Clone)]
pub struct Block(Vec<Statement>);

impl Block {
    #[inline]
    pub fn new(statements: Vec<Statement>) -> Self {
        Self(statements)
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[cfg(test)]
    pub fn inner(&self) -> &[Statement] {
        &self.0
    }
}

impl Expression for Block {
    #[inline]
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let [previous @ .., last] = self.0.as_slice() else {
            unreachable!("checked already at compile time");
        };

        for statement in previous {
            if let Err(err) = statement.resolve(cx) {
                return match err {
                    ExpressionError::Return { value } => Ok(value.unwrap_or(Value::Null)),
                    err => Err(err),
                };
            }
        }

        last.resolve(cx)
    }

    fn type_def(&self, state: &TypeState) -> TypeDef {
        self.0
            .last()
            .expect("at least one expression")
            .type_def(state)
    }
}
