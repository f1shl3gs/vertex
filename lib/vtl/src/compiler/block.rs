use value::Value;

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
        let (last, others) = self.0.split_last().expect("at least one expression");

        for statement in others {
            statement.resolve(cx)?;
        }

        last.resolve(cx)
    }

    fn type_def(&self) -> TypeDef {
        self.0.last().expect("at least one expression").type_def()
    }
}
