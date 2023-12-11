use value::Value;

use super::expression::Expression;
use super::statement::Statement;
use super::ExpressionError;
use crate::compiler::TypeDef;
use crate::context::Context;

pub struct Block(Vec<Statement>);

impl Block {
    pub fn new(statements: Vec<Statement>) -> Self {
        Self(statements)
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
