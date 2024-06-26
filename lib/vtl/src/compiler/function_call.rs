use value::Value;

use super::state::TypeState;
use super::{Expression, ExpressionError, Span, TypeDef};
use crate::context::Context;

#[derive(Clone)]
pub struct FunctionCall {
    pub function: Box<dyn Expression>,
    pub span: Span,
}

impl Expression for FunctionCall {
    #[inline]
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        self.function.resolve(cx).map_err(|err| match err {
            ExpressionError::Error { message, .. } => ExpressionError::Error {
                message,
                span: self.span,
            },
            err => err,
        })
    }

    #[inline]
    fn type_def(&self, state: &TypeState) -> TypeDef {
        self.function.type_def(state)
    }
}
