use value::Value;

use super::assignment::Assignment;
use super::for_statement::ForStatement;
use super::function_call::FunctionCall;
use super::if_statement::IfStatement;
use super::parser::Expr;
use super::ExpressionError;
use crate::compiler::{Expression, Kind, TypeDef};
use crate::context::Context;

#[derive(Clone)]
pub enum Statement {
    /// An if else statement
    ///
    /// ```text
    /// if contains(.foo, "bar") {
    ///     do_something()
    /// } else {
    ///     do_something_else()
    /// }
    /// ```
    If(IfStatement),

    /// For loop
    ///
    /// ```text
    /// for k, v in map {
    ///     .map[k] = v
    /// }
    /// ```
    ///
    /// # Todo
    ///
    /// maybe implement something like
    /// ```text
    /// for i in 0..10 {
    ///     call(i)
    /// }
    /// ```
    For(ForStatement),

    /// `continue` control for iteration
    Continue,

    /// `break` control for iteration
    Break,

    /// An assignment operator.
    ///
    /// # Example
    /// ```text
    /// a = 1
    /// a, b = "foo", true
    /// ```
    Assign(Assignment),

    /// A function call
    ///
    /// ```text
    /// delete(.path)
    /// ```
    Call(FunctionCall),

    /// Returns form the script
    ///
    /// if the value is not present, then the external variable "." is returned
    ///
    /// if the value is present, then the value express is returned, e.g.
    /// ```text
    /// return {
    ///   foo: "bar"
    /// }
    /// ```
    /// or
    /// ```text
    /// return .message
    /// ```
    Return(Option<Expr>),

    Expression(Expr),
}

impl Expression for Statement {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        match self {
            Statement::If(stmt) => stmt.resolve(cx),
            Statement::Assign(assign) => assign.resolve(cx),
            Statement::Expression(expr) => expr.resolve(cx),
            Statement::Call(call) => call.resolve(cx),
            Statement::For(stmt) => stmt.resolve(cx),
            Statement::Continue => Err(ExpressionError::Continue),
            Statement::Break => Err(ExpressionError::Break),
            Statement::Return(expr) => {
                let value = match expr {
                    Some(expr) => Some(expr.resolve(cx)?),
                    None => None,
                };

                Err(ExpressionError::Return { value })
            }
        }
    }

    fn type_def(&self) -> TypeDef {
        match self {
            Statement::If(if_statement) => if_statement.type_def(),
            Statement::For(for_statement) => for_statement.type_def(),
            Statement::Continue => TypeDef {
                fallible: false,
                kind: Kind::UNDEFINED,
            },
            Statement::Break => TypeDef {
                fallible: false,
                kind: Kind::UNDEFINED,
            },
            Statement::Assign(assignment) => assignment.type_def(),
            Statement::Call(call) => call.type_def(),
            Statement::Return(ret) => match ret {
                Some(expr) => expr.type_def(),
                None => TypeDef {
                    fallible: false,
                    kind: Kind::UNDEFINED,
                },
            },
            Statement::Expression(expr) => expr.type_def(),
        }
    }
}
