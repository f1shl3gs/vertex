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

use value::{OwnedTargetPath, Value};

pub use binary::BinaryError;
pub use expression::{Expression, ExpressionError};
pub use kind::{Kind, ValueKind};
pub use parser::{Compiler, SyntaxError, Variable};
pub use span::{Span, Spanned};
pub use template::Template;
pub use type_def::TypeDef;

use crate::context::Context;
use crate::Target;

#[derive(Clone)]
pub struct Program {
    // program content
    statements: block::Block,

    // variables are used, repeatedly
    variables: HashMap<String, Value>,

    /// A list of possible queries made to the
    /// external Target at runtime.
    target_queries: Vec<OwnedTargetPath>,
}

impl Program {
    pub fn run<T: Target>(&mut self, target: &mut T) -> Result<Value, ExpressionError> {
        let mut cx = Context {
            target,
            variables: &mut self.variables,
        };

        self.statements.resolve(&mut cx)
    }

    #[inline]
    pub fn target_queries(&self) -> &[OwnedTargetPath] {
        &self.target_queries
    }

    #[inline]
    pub fn type_def(&self) -> TypeDef {
        self.statements.type_def()
    }
}
