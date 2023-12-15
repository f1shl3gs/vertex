mod compiler;
mod context;
mod diagnostic;

#[cfg(test)]
mod tests;

pub use compiler::{Program, SyntaxError};
pub use context::{Context, Error as ContextError, Target, TargetValue};
pub use diagnostic::Diagnostic;

pub fn compile(input: &'_ str) -> Result<Program, SyntaxError> {
    compiler::Compiler::compile(input)
}
