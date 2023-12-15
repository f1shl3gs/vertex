use value::Value;

use super::{Expression, ExpressionError, Span, TypeDef};
use crate::Context;

#[derive(Clone)]
pub struct FunctionCall {
    pub function: Box<dyn Expression>,
    pub span: Span,
}

impl Expression for FunctionCall {
    #[inline]
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        self.function.resolve(cx)
    }

    #[inline]
    fn type_def(&self) -> TypeDef {
        self.function.type_def()
    }
}
