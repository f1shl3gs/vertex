mod assignment;
mod binary;
mod block;
mod expression;
mod for_statement;
mod function;
mod function_call;
mod if_statement;
mod kind;
mod lex;
// mod literal;
mod parser;
mod query;
mod span;
mod statement;
// mod literal;
mod levenshtein;
mod template;
mod type_def;
mod unary;

use std::collections::HashMap;

use value::Value;

pub use binary::BinaryError;
pub use expression::{Expression, ExpressionError};
pub use kind::{Kind, ValueKind};
pub use parser::{Compiler, SyntaxError, Variable};
pub use span::{Span, Spanned};
pub use template::Template;
pub use type_def::TypeDef;

use crate::context::Context;

#[derive(Clone)]
pub struct Program {
    // program content
    statements: block::Block,

    pub variables: HashMap<String, Value>,
}

impl Program {
    #[inline]
    pub fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        self.statements.resolve(cx)
    }

    #[inline]
    pub fn type_def(&self) -> TypeDef {
        self.statements.type_def()
    }
}
