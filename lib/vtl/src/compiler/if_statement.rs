use value::Value;

use super::block::Block;
use super::parser::Expr;
use super::ExpressionError;
use crate::compiler::{Expression, Kind, Spanned, TypeDef, ValueKind};
use crate::context::Context;

pub struct IfStatement {
    /// The condition for the if statement.
    pub condition: Spanned<Expr>,

    /// The block of statements to be ran if the condition is met.
    pub then_block: Block,

    /// The block of statements to be ran if no other conditions were met.
    pub else_block: Option<Block>,
}

impl Expression for IfStatement {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let predicate = match self.condition.resolve(cx)? {
            Value::Boolean(b) => b,
            value => {
                return Err(ExpressionError::UnexpectedType {
                    want: Kind::BOOLEAN,
                    got: value.kind(),
                    span: self.condition.span,
                })
            }
        };

        if predicate {
            self.then_block.resolve(cx)
        } else {
            match &self.else_block {
                Some(block) => block.resolve(cx),
                None => Ok(Value::Null),
            }
        }
    }

    fn type_def(&self) -> TypeDef {
        TypeDef {
            fallible: false,
            kind: Kind::UNDEFINED,
        }
    }
}
