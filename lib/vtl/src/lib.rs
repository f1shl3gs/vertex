mod compiler;
mod context;
mod diagnostic;

pub use compiler::{ExpressionError, Program, SyntaxError};
pub use context::{Context, Error as ContextError, Target, TargetValue};
pub use diagnostic::Diagnostic;

#[inline]
pub fn compile(input: &'_ str) -> Result<Program, SyntaxError> {
    compiler::Compiler::compile(input)
}

#[inline]
pub fn compile_with(input: &'_ str, predefined: &[&str]) -> Result<Program, SyntaxError> {
    compiler::Compiler::compile_with_predefined(input, predefined)
}
