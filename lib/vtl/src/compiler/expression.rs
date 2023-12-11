use std::fmt::{Display, Formatter};

use value::Value;

use super::{BinaryError, Kind, Span};
use crate::compiler::TypeDef;
use crate::context::Context;

#[derive(Debug, PartialEq)]
pub enum ExpressionError {
    Error { message: String, span: Span },
    NotFound { path: String },
    Binary { err: BinaryError, span: Span },
    UnexpectedType { want: Kind, got: Kind, span: Span },
    UnexpectedValue { msg: String, span: Span },

    // actually, they are used to control steps, not a really error
    Break,
    Continue,
    Return { value: Option<Value> },
}

impl Display for ExpressionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ExpressionError::Error { message, .. } => f.write_str(message),
            ExpressionError::NotFound { path } => write!(f, "path \"{}\" is not exist", path),
            ExpressionError::Binary { err, .. } => write!(f, "binary operate \"{}\"", err),
            ExpressionError::UnexpectedType { want, got, .. } => {
                write!(f, "invalid type \"{}\" found, want: \"{}\"", got, want)
            }
            ExpressionError::UnexpectedValue { msg, .. } => f.write_str(msg),
            ExpressionError::Break => f.write_str("break"),
            ExpressionError::Continue => f.write_str("continue"),
            ExpressionError::Return { value } => match value {
                Some(value) => {
                    write!(f, "return {}", value)
                }
                None => f.write_str("return None"),
            },
        }
    }
}

pub trait Expression {
    /// Result<Option<Value>, Error>
    /// ```text
    /// if .a {
    ///     return .message    # Ok(Some(value))
    /// }
    ///
    /// .a = false
    ///
    /// # return .
    /// ```
    ///
    /// Resolve an expression to a concrete `Value`.
    ///
    /// This method is executed at runtime. An expression is allowed to fail,
    /// which aborts the running program.
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError>;

    /// Resolve an expression to its `TypeDef` type definition.
    /// This must be called with the _initial_ `TypeState`.
    ///
    /// Consider calling `type_info` instead if you want to capture changes in the type
    /// state from side-effects.
    fn type_def(&self) -> TypeDef;
}
