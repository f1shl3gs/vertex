use value::Value;

use super::block::Block;
use super::expr::Expr;
use super::{Expression, Spanned, TypeDef};
use super::{ExpressionError, Kind, ValueKind};
use crate::context::Context;

/// The `key/value` is temporary defined(insert when start, and remove when end),
/// and they are limited to be identifier, so we defined them with `String`.
#[derive(Clone)]
pub struct ForStatement {
    /// The key or index to set to the value of each item being iterated.
    pub key: String,
    /// The value to set to the value of each item being iterated.
    pub value: String,
    /// The expression to evaluate to get the iterator.
    pub iterator: Spanned<Expr>,
    /// The block of statements to be ran every item.
    pub block: Block,
}

impl Expression for ForStatement {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let iterator = self.iterator.resolve(cx)?;

        // This looks viable, but it's not, "multiple mutable borrow" will fail this.
        //
        // let key_target = cx
        //     .variables
        //     .get_mut(&self.key)
        //     .expect("variable should be registered already");
        // let value_target = cx
        //     .variables
        //     .get_mut(&self.value)
        //     .expect("variable should be registered already");

        // avoid overwrite variable
        let prev_key = cx.variables.remove(&self.key);
        let prev_value = cx.variables.remove(&self.value);

        match iterator {
            Value::Array(array) => {
                for (index, item) in array.into_iter().enumerate() {
                    cx.variables
                        .insert(self.key.clone(), Value::Integer(index as i64));
                    cx.variables.insert(self.value.clone(), item);

                    // *key_target = Value::Integer(i as i64);
                    // *value_target = v;

                    match self.block.resolve(cx) {
                        Ok(_) => {}
                        Err(err) => match err {
                            ExpressionError::Continue => continue,
                            ExpressionError::Break => break,
                            err => return Err(err),
                        },
                    }
                }
            }
            Value::Object(map) => {
                for (key, value) in map {
                    cx.variables.insert(self.key.clone(), key.into());
                    cx.variables.insert(self.value.clone(), value);

                    // *key_target = Value::Bytes(k.into());
                    // *value_target = v;

                    match self.block.resolve(cx) {
                        Ok(_) => {}
                        Err(err) => match err {
                            ExpressionError::Continue => continue,
                            ExpressionError::Break => break,
                            err => return Err(err),
                        },
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

        // avoid overwrite variable
        if let Some(key) = prev_key {
            cx.variables.insert(self.key.clone(), key);
        }
        if let Some(value) = prev_value {
            cx.variables.insert(self.value.clone(), value);
        }

        Ok(Value::Null)
    }

    fn type_def(&self) -> TypeDef {
        TypeDef {
            fallible: false,
            kind: Kind::UNDEFINED,
        }
    }
}
