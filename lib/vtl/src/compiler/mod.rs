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
mod template;
mod type_def;
mod unary;

use std::collections::HashMap;

use value::{OwnedTargetPath, Value};

pub use binary::BinaryError;
pub use kind::{Kind, ValueKind};
pub use parser::{Compiler, SyntaxError, Variable};

use crate::context::Context;

pub use expression::{Expression, ExpressionError};
pub use span::{Span, Spanned};
pub use type_def::TypeDef;

pub struct Program {
    // program content
    statements: block::Block,

    pub variables: HashMap<String, Value>,

    /// A list of possible queries made to the external Target at runtime.
    pub target_queries: Vec<OwnedTargetPath>,
    /// A list of possible assignments made to the external Target at runtime.
    pub target_assignments: Vec<OwnedTargetPath>,
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
