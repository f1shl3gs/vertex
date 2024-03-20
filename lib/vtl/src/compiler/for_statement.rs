use value::Value;

use super::block::Block;
use super::expr::Expr;
use super::state::TypeState;
use super::{Expression, Spanned, TypeDef};
use super::{ExpressionError, Kind};
use crate::context::Context;

#[derive(Clone)]
pub struct ForStatement {
    /// The index of variable for "key" or "index".
    pub key: usize,

    /// The index of variable for "value" or "item".
    pub value: usize,

    /// The expression to evaluate to get the iterator.
    pub iterator: Spanned<Expr>,

    /// The block of statements to be ran every item.
    pub block: Block,
}

impl Expression for ForStatement {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let iterator = self.iterator.resolve(cx)?;

        match iterator {
            Value::Array(array) => {
                for (index, item) in array.into_iter().enumerate() {
                    cx.set(self.key, Value::Integer(index as i64));
                    cx.set(self.value, item);

                    if let Err(err) = self.block.resolve(cx) {
                        match err {
                            ExpressionError::Continue => continue,
                            ExpressionError::Break => break,
                            err => return Err(err),
                        }
                    }
                }
            }
            Value::Object(map) => {
                for (key, value) in map {
                    cx.set(self.key, Value::Bytes(key.into()));
                    cx.set(self.value, value);

                    if let Err(err) = self.block.resolve(cx) {
                        match err {
                            ExpressionError::Continue => continue,
                            ExpressionError::Break => break,
                            err => return Err(err),
                        }
                    }
                }
            }
            _ => {
                return Err(ExpressionError::UnexpectedType {
                    want: Kind::CONTAINER,
                    got: iterator.kind(),
                    span: self.iterator.span,
                })
            }
        }

        Ok(Value::Null)
    }

    fn type_def(&self, _state: &TypeState) -> TypeDef {
        TypeDef {
            fallible: false,
            kind: Kind::NULL,
        }
    }
}
