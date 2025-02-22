mod assignment;
mod binary;
mod block;
mod expr;
mod for_statement;
mod function;
mod function_call;
mod if_statement;
mod lex;
// mod literal;
mod levenshtein;
mod parser;
mod query;
mod span;
mod statement;
// mod literal;
mod state;
mod template;
mod type_def;
mod unary;

use std::error::Error;
use std::fmt::{Display, Formatter};

use dyn_clone::{DynClone, clone_trait_object};
use state::TypeState;
use value::{OwnedTargetPath, Value};

pub use binary::BinaryError;
pub use parser::{Compiler, SyntaxError};
pub use span::{Span, Spanned};
pub use template::Template;
pub use type_def::TypeDef;
pub use value::Kind;

use crate::Target;
use crate::context::Context;
use crate::diagnostic::{DiagnosticMessage, Label};

#[derive(Clone)]
pub struct Program {
    // program content
    statements: block::Block,
    type_def: TypeDef,

    // variables are used repeatedly
    variables: Vec<Value>,

    /// A list of possible queries made to the
    /// external Target at runtime.
    target_queries: Vec<OwnedTargetPath>,
}

impl Program {
    pub fn run<T: Target>(&self, target: &mut T) -> Result<Value, ExpressionError> {
        // TODO: find a better way to handle this
        let mut variables = self.variables.clone();

        let mut cx = Context {
            target,
            variables: &mut variables,
        };

        self.statements.resolve(&mut cx)
    }

    #[inline]
    pub fn target_queries(&self) -> &[OwnedTargetPath] {
        &self.target_queries
    }

    #[inline]
    pub fn type_def(&self) -> &TypeDef {
        &self.type_def
    }
}

#[derive(Debug, PartialEq)]
pub enum ExpressionError {
    Error { message: String, span: Span },
    NotFound { path: String, span: Span },
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
            ExpressionError::NotFound { path, .. } => write!(f, "path \"{}\" is not exist", path),
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

impl Error for ExpressionError {}

impl DiagnosticMessage for ExpressionError {
    fn labels(&self) -> Vec<Label> {
        match self {
            ExpressionError::Error { message, span } => {
                vec![Label::new(message, span)]
            }
            ExpressionError::NotFound { path, span } => {
                vec![Label::new(format!("{} path not found", path), span)]
            }
            ExpressionError::Binary { err, span } => {
                vec![Label::new(err.to_string(), span)]
            }
            ExpressionError::UnexpectedType { want, got, span } => {
                vec![Label::new(format!("got {}, want: {}", got, want), span)]
            }
            ExpressionError::UnexpectedValue { msg, span } => {
                vec![Label::new(msg, span)]
            }
            _ => unreachable!(),
        }
    }
}

pub trait Expression: Send + Sync + DynClone {
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
    fn type_def(&self, state: &TypeState) -> TypeDef;
}

clone_trait_object!(Expression);
