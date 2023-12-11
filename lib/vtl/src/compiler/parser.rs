#![allow(unused_variables)]

use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};

use value::{parse_target_path, parse_value_path, PathParseError, Value};

use super::assignment::Assignment;
use super::assignment::AssignmentTarget;
use super::binary::{Binary, BinaryOp};
use super::block::Block;
use super::for_statement::ForStatement;
use super::function::{builtin_functions, Function, FunctionCompileContext};
use super::function_call::FunctionCall;
use super::if_statement::IfStatement;
use super::lex::{LexError, Lexer, Token};
use super::statement::Statement;
use super::unary::{Unary, UnaryOp};
use super::Program;
use super::{ExpressionError, Kind};
use super::{Span, Spanned};
use crate::compiler::function::ArgumentList;
use crate::compiler::query::Query;
use crate::compiler::{Expression, TypeDef};
use crate::context::Context;

#[derive(Debug)]
pub enum SyntaxError {
    Lex {
        err: LexError,
        pos: usize,
    },

    EmptyBlock {
        span: Span,
    },
    UnexpectedEof,
    UnexpectedToken {
        got: String,
        want: Option<String>,
        span: Span,
    },

    // Path
    InvalidPath {
        err: PathParseError,
        span: Span,
    },

    // Variables
    VariableNeverUsed {
        name: String,
    },
    VariableAlreadyDefined {
        name: String,
        span: Span,
    },
    UndefinedVariable {
        name: String,
        span: Span,
    },

    // Assignment
    // err: "value = fallible()"
    UnnecessaryErrorAssignment {
        span: Span,
    },
    // err: "ok, err = now()"
    UnhandledFallibleAssignment {
        span: Span,
    },

    // Functions
    UndefinedFunction {
        name: String,
        span: Span,
    },
    FunctionArgumentsArityMismatch {
        function: &'static str,
        takes: usize,
        got: usize,
        span: Span,
    },
    FunctionNotFallible {
        function: &'static str,
        span: Span,
    },
    InvalidFunctionArgumentType {
        function: &'static str,
        argument: &'static str,
        want: Kind,
        got: Kind,
        span: Span,
    },
    InvalidValue {
        err: String,
        want: String,
        got: String,
        span: Span,
    },
    InfallibleAssignment {
        span: Span,
    },
}

impl Display for SyntaxError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SyntaxError::Lex { err, .. } => {
                write!(f, "lex error: {}", err)
            }
            SyntaxError::EmptyBlock { .. } => f.write_str("empty block is not allowed"),
            SyntaxError::UnexpectedEof => f.write_str("unexpected end of file"),
            SyntaxError::UnexpectedToken { got, want, .. } => match want {
                Some(want) => write!(f, "unexpected token: {}, want: {}", got, want),
                None => write!(f, "unexpected token: {}", got),
            },
            SyntaxError::UndefinedVariable { name, .. } => {
                write!(f, "undefined variable {}", name)
            }
            SyntaxError::UndefinedFunction { name, .. } => {
                write!(f, "unknown function {}", name)
            }
            SyntaxError::InvalidPath { err, .. } => {
                write!(f, "invalid target path {}", err)
            }
            SyntaxError::VariableNeverUsed { name } => {
                write!(f, "variable \"{}\" is never used", name)
            }
            SyntaxError::VariableAlreadyDefined { name, .. } => {
                write!(f, "variable \"{}\" already defined", name)
            }
            SyntaxError::FunctionArgumentsArityMismatch {
                function,
                takes,
                got,
                ..
            } => {
                write!(
                    f,
                    "function \"{}\" takes {} but got {}",
                    function, takes, got
                )
            }
            SyntaxError::FunctionNotFallible { function, .. } => {
                write!(f, "function \"{}\" is not fallible", function)
            }
            SyntaxError::InvalidFunctionArgumentType {
                function,
                argument,
                want,
                got,
                ..
            } => {
                write!(
                    f,
                    "function \"{}\"'s argument {} should be {} rather than {}",
                    function, argument, want, got
                )
            }
            SyntaxError::InvalidValue { got, want, .. } => {
                write!(f, "invalid value \"{}\", want: \"{}\"", got, want)
            }
            SyntaxError::InfallibleAssignment { .. } => f.write_str("infallible assignment"),

            _ => {
                todo!()
            }
        }
    }
}

impl From<LexError> for SyntaxError {
    fn from(err: LexError) -> Self {
        Self::Lex { err, pos: 0 }
    }
}

pub struct Variable {
    name: String,
    value: Value,
    // writes: usize,
}

pub enum Expr {
    /// The literal null value.
    Null,
    /// The literal boolean value.
    Boolean(bool),
    /// The literal integer.
    Integer(i64),
    /// The literal float.
    Float(f64),
    /// A literal string.
    String(String),

    // TODO: Add timestamp!?
    /// A reference to a stored value, an identifier.
    Identifier(String),
    /// A query
    ///
    /// ".", "%", ".foo", "%foo" or "foo.bar"
    Query(Query),

    /// An unary operation.
    Unary(Unary),
    /// A binary operation.
    Binary(Binary),
    /// A call expression of something.
    Call(FunctionCall),

    /// A literal Array
    ///
    /// ```text
    /// arr = [1, false, "foo", -1]
    /// ```
    Array(Vec<Expr>),
    /// A literal Object.
    ///
    /// ```text
    /// obj = {
    ///     foo: "bar"
    /// }
    /// ```
    Object(BTreeMap<String, Expr>),
}

impl Display for Expr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

// this mod is used for tests only
#[cfg(test)]
mod expr_convert {
    use super::Expr;
    use crate::compiler::parser::unescape_string;
    use crate::compiler::query::Query;
    use std::collections::BTreeMap;
    use value::OwnedTargetPath;

    impl From<&str> for Expr {
        fn from(value: &str) -> Self {
            Expr::String(unescape_string(value))
        }
    }

    impl From<bool> for Expr {
        fn from(value: bool) -> Self {
            Expr::Boolean(value)
        }
    }

    impl From<i64> for Expr {
        fn from(value: i64) -> Self {
            Expr::Integer(value)
        }
    }

    impl From<f64> for Expr {
        fn from(value: f64) -> Self {
            Expr::Float(value)
        }
    }

    impl From<String> for Expr {
        fn from(value: String) -> Self {
            Expr::String(value)
        }
    }

    impl From<Vec<Expr>> for Expr {
        fn from(array: Vec<Expr>) -> Self {
            Expr::Array(array)
        }
    }

    impl From<OwnedTargetPath> for Expr {
        fn from(value: OwnedTargetPath) -> Self {
            Expr::Query(Query::External(value))
        }
    }

    impl From<BTreeMap<String, Expr>> for Expr {
        fn from(value: BTreeMap<String, Expr>) -> Self {
            Expr::Object(value)
        }
    }
}

impl Expression for Expr {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        match self {
            Expr::Null => Ok(Value::Null),
            Expr::Boolean(b) => Ok(Value::Boolean(*b)),
            Expr::Integer(i) => Ok(Value::Integer(*i)),
            Expr::Float(f) => Ok(Value::Float(*f)),
            Expr::String(s) => Ok(Value::from(s.as_str())),
            Expr::Identifier(s) => {
                let value = cx
                    .variables
                    .get(s)
                    .expect("variable must be checked at compile time");
                Ok(value.clone())
            }
            Expr::Query(query) => query.resolve(cx),
            Expr::Array(array) => {
                let array = array
                    .iter()
                    .map(|expr| expr.resolve(cx))
                    .collect::<Result<Vec<_>, ExpressionError>>()?;
                Ok(array.into())
            }
            Expr::Binary(b) => b.resolve(cx),
            Expr::Unary(u) => u.resolve(cx),
            Expr::Object(map) => {
                let object = map
                    .iter()
                    .map(|(key, expr)| {
                        let value = expr.resolve(cx)?;
                        Ok((key.to_string(), value))
                    })
                    .collect::<Result<BTreeMap<String, Value>, ExpressionError>>()?;

                Ok(Value::Object(object))
            }

            Expr::Call(call) => call.function.resolve(cx),
        }
    }

    fn type_def(&self) -> TypeDef {
        match self {
            Expr::Identifier(_) => {
                // TODO: fix this
                Kind::ANY.into()
            }
            Expr::Null => Kind::NULL.into(),
            Expr::Boolean(_) => Kind::BOOLEAN.into(),
            Expr::Integer(_) => Kind::INTEGER.into(),
            Expr::Float(_) => Kind::FLOAT.into(),
            Expr::String(_) => Kind::BYTES.into(),
            Expr::Array(_) => Kind::ARRAY.into(),
            Expr::Object(_) => Kind::OBJECT.into(),
            Expr::Call(call) => call.type_def(),
            Expr::Binary(b) => b.type_def(),
            Expr::Unary(u) => u.type_def(),

            _ => TypeDef {
                fallible: false,
                kind: Kind::ANY,
            },
        }
    }
}

/// ```text
/// .foo = .bar
/// delete(.bar)
///
/// if is_object(.msg) {
///   return
/// }
///
/// .msg = parse_json(.msg)?
///
/// if .host == null {
///   .host = get_hostname()
/// }
///
/// # iterate a map
/// for k, v in .map {
///   .arr = append(.arr, v)
/// }
///
/// # iterate an array
/// for index, item in .arr {
///   .arr[index] = item + 1
/// }
/// ```
pub struct Compiler<'input> {
    lexer: Lexer<'input>,
    functions: Vec<Box<dyn Function>>,

    // for parse state
    // increase 1 when entering the `for` loop, and decrease when leave,
    // so if iterating is zero, `continue` and `break` is not allowed.
    iterating: usize,

    variables: Vec<Variable>,
}

impl Compiler<'_> {
    pub fn compile(input: &'_ str) -> Result<Program, SyntaxError> {
        let lexer = Lexer::new(input);
        let mut compiler = Compiler {
            lexer,
            functions: builtin_functions(),
            iterating: 0,
            variables: vec![],
        };

        let block = compiler.parse_block()?;

        // todo: check variables
        //   if the variables are never changed, return error

        Ok(Program {
            statements: block,
            variables: compiler
                .variables
                .into_iter()
                .map(|var| (var.name, var.value))
                .collect(),
            target_queries: vec![],
            target_assignments: vec![],
        })
    }

    fn parse_block(&mut self) -> Result<Block, SyntaxError> {
        let mut statements = vec![];
        let start = self.lexer.pos();

        while let Some((token, span)) =
            self.lexer
                .peek()
                .transpose()
                .map_err(|err| SyntaxError::Lex {
                    err,
                    pos: self.lexer.pos(),
                })?
        {
            match token {
                Token::If => statements.push(self.parse_if()?),
                Token::For => statements.push(self.parse_for()?),
                Token::Return => statements.push(Statement::Return(None)),
                // end of block
                Token::RightBrace => {
                    break;
                }
                // assign to a variable
                Token::Identifier(_) | Token::PathField(_) => statements.push(self.parse_assign()?),

                Token::FunctionCall(name) => {
                    if let Expr::Call(call) = self.parse_function_call()? {
                        statements.push(Statement::Call(call));
                    }
                }
                Token::Break => {
                    if self.iterating == 0 {
                        return Err(SyntaxError::UnexpectedToken {
                            got: "break".to_string(),
                            want: None,
                            span,
                        });
                    }

                    self.lexer.next();
                    statements.push(Statement::Break);
                }
                Token::Continue => {
                    if self.iterating == 0 {
                        return Err(SyntaxError::UnexpectedToken {
                            got: "continue".to_string(),
                            want: None,
                            span,
                        });
                    }

                    self.lexer.next();
                    statements.push(Statement::Continue);
                }

                _ => {
                    // code example
                    //
                    // foo + bar
                    //
                    // it's very useful, like
                    // foo = if true {
                    //     a = 1 + 1
                    //     a + 1
                    // } else {
                    //     0
                    // }
                    //
                    // foo is 3
                    let expr = self.parse_expr()?;
                    statements.push(Statement::Expression(expr))
                }
            }
        }

        if statements.is_empty() {
            return Err(SyntaxError::EmptyBlock {
                span: Span {
                    start,
                    end: self.lexer.pos(),
                },
            });
        }

        Ok(Block::new(statements))
    }

    fn parse_assign_target(&mut self) -> Result<AssignmentTarget, SyntaxError> {
        match self.lexer.next().transpose()? {
            Some((token, span)) => match token {
                Token::Identifier(s) => {
                    self.register_variable(s.to_string());

                    Ok(AssignmentTarget::Internal(s.to_string(), None))
                }
                Token::PathField(path) => {
                    // ".", ".foo", "%" or "%foo"
                    if path.starts_with(|c| c == '.' || c == '%') {
                        let path = parse_target_path(path)
                            .map_err(|err| SyntaxError::InvalidPath { err, span })?;

                        return Ok(AssignmentTarget::External(path));
                    }

                    // "foo" or "foo.bar"
                    match path.split_once(|c| c == '.' || c == '[') {
                        Some((name, path)) => {
                            // at this case, the variable must exists already
                            let exists =
                                self.variables.iter().any(|variable| variable.name == name);
                            if !exists {
                                return Err(SyntaxError::UndefinedVariable {
                                    name: name.to_string(),
                                    span,
                                });
                            }

                            let path = parse_value_path(path)
                                .map_err(|err| SyntaxError::InvalidPath { err, span })?;

                            Ok(AssignmentTarget::Internal(name.to_string(), Some(path)))
                        }
                        None => Err(SyntaxError::InvalidPath {
                            err: PathParseError::InvalidPathSyntax {
                                path: path.to_string(),
                            },
                            span,
                        }),
                    }
                }
                _ => Err(SyntaxError::UnexpectedToken {
                    got: token.to_string(),
                    want: Some("ident or path".to_string()),
                    span,
                }),
            },
            None => Err(SyntaxError::UnexpectedEof),
        }
    }

    fn parse_assign(&mut self) -> Result<Statement, SyntaxError> {
        let target = self.parse_assign_target()?;

        match self.lexer.next().transpose()? {
            Some((token, span)) => {
                // comma or assign is expected
                let assignment = match token {
                    // ok, err = fallible()
                    Token::Comma => {
                        let err = self.parse_assign_target()?;

                        self.expect(Token::Assign)?;

                        // it must be a fallible function
                        let expr = self.parse_function_call()?;
                        if let Expr::Call(call) = &expr {
                            if !call.function.type_def().fallible {
                                // something like "ok, err = get_hostname()" is not allowed
                                return Err(SyntaxError::FunctionNotFallible {
                                    function: "todo",
                                    span,
                                });
                            }
                        } else {
                            // not function call, maybe "ok, err = 123"
                            return Err(SyntaxError::InfallibleAssignment { span });
                        }

                        Assignment::Infallible {
                            ok: target,
                            err,
                            expr,
                            default: Value::Null,
                        }
                    }

                    // a = 1 + 2
                    // a = fallible()?
                    Token::Assign => {
                        let expr = self.parse_expr()?;
                        if let Expr::Call(call) = &expr {
                            let fallible = call.function.type_def().fallible;
                            if fallible {
                                self.expect(Token::Question)?;
                            }
                        }

                        Assignment::Single { target, expr }
                    }
                    _ => {
                        return Err(SyntaxError::UnexpectedToken {
                            got: token.to_string(),
                            want: Some("comma or equal".to_string()),
                            span,
                        })
                    }
                };

                Ok(Statement::Assign(assignment))
            }
            None => Err(SyntaxError::UnexpectedEof),
        }
    }

    fn parse_expr(&mut self) -> Result<Expr, SyntaxError> {
        // maybe array or object
        self.parse_expr_or()
    }

    fn parse_expr_or(&mut self) -> Result<Expr, SyntaxError> {
        let mut expr = self.parse_expr_and()?;

        while let Some(result) = self.next_if(|token| Token::Or == token) {
            let _ = result?;
            expr = Expr::Binary(Binary {
                lhs: Box::new(expr),
                rhs: Box::new(self.parse_expr_and()?),
                op: BinaryOp::Or,
            })
        }

        Ok(expr)
    }

    fn parse_expr_and(&mut self) -> Result<Expr, SyntaxError> {
        let mut expr = self.parse_expr_comparison()?;

        while let Some(result) = self.next_if(|token| Token::And == token) {
            let _ = result?;
            expr = Expr::Binary(Binary {
                lhs: Box::new(expr),
                rhs: Box::new(self.parse_expr_comparison()?),
                op: BinaryOp::And,
            })
        }

        Ok(expr)
    }

    fn parse_expr_comparison(&mut self) -> Result<Expr, SyntaxError> {
        let mut expr = self.parse_expr_term()?;

        while let Some((token, span)) = self.lexer.peek().transpose()? {
            let op = match token {
                Token::Equal => BinaryOp::Equal,
                Token::NotEqual => BinaryOp::NotEqual,
                Token::GreatThan => BinaryOp::GreatThan,
                Token::GreatEqual => BinaryOp::GreatEqual,
                Token::LessThan => BinaryOp::LessThan,
                Token::LessEqual => BinaryOp::LessEqual,
                _ => break,
            };

            let _ = self.lexer.next().expect("must valid")?;
            expr = Expr::Binary(Binary {
                lhs: Box::new(expr),
                rhs: Box::new(self.parse_expr_term()?),
                op,
            })
        }

        Ok(expr)
    }

    fn parse_expr_term(&mut self) -> Result<Expr, SyntaxError> {
        let mut expr = self.parse_expr_factor()?;

        while let Some((token, span)) = self.lexer.peek().transpose()? {
            let op = match token {
                Token::Add => BinaryOp::Add,
                Token::Subtract => BinaryOp::Subtract,
                _ => break,
            };

            let _ = self.lexer.next().expect("must exist")?;
            expr = Expr::Binary(Binary {
                lhs: Box::new(expr),
                rhs: Box::new(self.parse_expr_factor()?),
                op,
            })
        }

        Ok(expr)
    }

    fn parse_expr_factor(&mut self) -> Result<Expr, SyntaxError> {
        let mut expr = self.parse_expr_unary()?;

        while let Some((token, span)) = self.lexer.peek().transpose()? {
            let op = match token {
                Token::Multiply => BinaryOp::Multiply,
                Token::Divide => BinaryOp::Divide,
                _ => break,
            };

            let _ = self.lexer.next().expect("must exist")?;
            expr = Expr::Binary(Binary {
                lhs: Box::new(expr),
                rhs: Box::new(self.parse_expr_unary()?),
                op,
            });
        }

        Ok(expr)
    }

    fn parse_expr_unary(&mut self) -> Result<Expr, SyntaxError> {
        match self.lexer.peek().transpose()? {
            Some((token, span)) => {
                let op = match token {
                    Token::Not => UnaryOp::Not,
                    Token::Subtract => UnaryOp::Negate,
                    _ => return self.parse_expr_exponent(),
                };

                self.lexer.next();
                let operand = self.parse_expr_unary()?;

                Ok(match (op, operand) {
                    // A little optimize
                    (UnaryOp::Negate, Expr::Float(f)) => Expr::Float(-f),
                    (UnaryOp::Negate, Expr::Integer(i)) => Expr::Integer(-i),
                    (UnaryOp::Not, Expr::Boolean(b)) => Expr::Boolean(!b),
                    (op, operand) => Expr::Unary(Unary {
                        op,
                        operand: Box::new(operand),
                    }),
                })
            }
            None => self.parse_expr_exponent(),
        }
    }

    fn parse_expr_exponent(&mut self) -> Result<Expr, SyntaxError> {
        let mut expr = self.parse_expr_primary()?;

        while let Some((Token::Exponent, span)) = self.lexer.peek().transpose()? {
            self.lexer.next();

            expr = Expr::Binary(Binary {
                lhs: Box::new(expr),
                rhs: Box::new(self.parse_expr_exponent()?),
                op: BinaryOp::Exponent,
            })
        }

        Ok(expr)
    }

    fn parse_expr_primary(&mut self) -> Result<Expr, SyntaxError> {
        match self.lexer.peek().transpose()? {
            Some((token, span)) => match token {
                Token::Identifier(s) => {
                    let actor = Expr::Identifier(s.to_string());

                    self.lexer.next();

                    self.parse_expr_inner(actor)
                }
                Token::FunctionCall(name) => self.parse_function_call(),
                Token::PathField(path) => {
                    // ".", ".foo", "%" or "%foo"
                    let query = if path.starts_with(|c| c == '.' || c == '%') {
                        let path = parse_target_path(path)
                            .map_err(|err| SyntaxError::InvalidPath { err, span })?;

                        Query::External(path)
                    } else {
                        // "foo", "foo.bar" or "foo[1]"
                        match path.find(|c| c == '.' || c == '[') {
                            Some(index) => {
                                // at this case, the variable must exists already
                                let (name, path) = path.split_at(index);
                                let exists =
                                    self.variables.iter().any(|variable| variable.name == name);
                                if !exists {
                                    return Err(SyntaxError::UndefinedVariable {
                                        name: name.to_string(),
                                        span,
                                    });
                                }

                                let path = parse_value_path(path)
                                    .map_err(|err| SyntaxError::InvalidPath { err, span })?;

                                Query::Internal(name.to_string(), path)
                            }
                            None => {
                                return Err(SyntaxError::InvalidPath {
                                    err: PathParseError::InvalidPathSyntax {
                                        path: path.to_string(),
                                    },
                                    span,
                                })
                            }
                        }
                    };

                    let _ = self.lexer.next();

                    Ok(Expr::Query(query))
                }
                // Simple literals
                Token::Integer(i) => {
                    let _ = self.lexer.next();
                    Ok(Expr::Integer(i))
                }
                Token::Float(f) => {
                    let _ = self.lexer.next();
                    Ok(Expr::Float(f))
                }
                Token::String(s) => {
                    let _ = self.lexer.next();
                    Ok(Expr::String(unescape_string(s)))
                }
                Token::Null => {
                    let _ = self.lexer.next();
                    Ok(Expr::Null)
                }
                Token::True => {
                    let _ = self.lexer.next();
                    Ok(Expr::Boolean(true))
                }
                Token::False => {
                    let _ = self.lexer.next();
                    Ok(Expr::Boolean(false))
                }
                // array
                Token::LeftBracket => self.parse_array(),
                // object
                Token::LeftBrace => self.parse_object(),
                // ( 1 + 2 ) * 2
                Token::LeftParen => {
                    self.lexer.next();
                    let expr = self.parse_expr()?;
                    self.expect(Token::RightParen)?;
                    self.parse_expr_inner(expr)
                }
                _ => Err(SyntaxError::UnexpectedToken {
                    got: token.to_string(),
                    want: None,
                    span,
                }),
            },
            None => Err(SyntaxError::UnexpectedEof),
        }
    }

    fn parse_function_call(&mut self) -> Result<Expr, SyntaxError> {
        let (token, mut span) = self.lexer.next().transpose()?.expect("must exist");
        let name = if let Token::FunctionCall(name) = token {
            name
        } else {
            panic!("parse_function_call must be called when Token::FunctionCall found")
        };

        self.expect(Token::LeftParen)?;

        let (name, parameters) = self
            .functions
            .iter()
            .find(|func| func.identifier() == name)
            .map(|func| (func.identifier(), func.parameters()))
            .ok_or(SyntaxError::UndefinedFunction {
                name: name.to_string(),
                span,
            })?;
        let mut arguments = ArgumentList::new(name, parameters);

        loop {
            // want argument or RightParen
            match self.lexer.peek().transpose()? {
                Some((token, span)) => match token {
                    Token::RightParen => {
                        self.lexer.next();
                        break;
                    }
                    _ => {
                        let start = self.lexer.pos();
                        let argument = self.parse_expr()?;
                        let end = self.lexer.pos();
                        if let Expr::Identifier(s) = &argument {
                            self.variables.iter().find(|var| &var.name == s).ok_or(
                                SyntaxError::UndefinedVariable {
                                    name: s.to_string(),
                                    span,
                                },
                            )?;

                            self.register_variable(s.to_string());
                        }

                        arguments.push(Spanned::new(argument, Span { start, end }))?;
                    }
                },
                None => return Err(SyntaxError::UnexpectedEof),
            }

            // want comma or RightParen
            match self.lexer.peek().transpose()? {
                Some((token, span)) => match token {
                    Token::Comma => {
                        self.lexer.next();
                        continue;
                    }
                    Token::RightParen => {
                        self.lexer.next();
                        break;
                    }
                    _ => {
                        return Err(SyntaxError::UnexpectedToken {
                            got: token.to_string(),
                            want: Some("comma or right paren".to_string()),
                            span,
                        })
                    }
                },
                None => return Err(SyntaxError::UnexpectedEof),
            }
        }

        span.end = self.lexer.pos();

        let func = self
            .functions
            .iter()
            .find(|func| func.identifier() == name)
            .ok_or(SyntaxError::UndefinedFunction {
                name: name.to_string(),
                span,
            })?;

        // Check function arity
        let at_least =
            func.parameters().iter().fold(
                0usize,
                |acc, param| {
                    if param.required {
                        acc + 1
                    } else {
                        acc
                    }
                },
            );
        if arguments.len() < at_least {
            return Err(SyntaxError::FunctionArgumentsArityMismatch {
                function: func.identifier(),
                takes: func.parameters().len(),
                got: arguments.len(),
                span,
            });
        }

        let compiled = func.compile(FunctionCompileContext { span }, arguments)?;

        Ok(Expr::Call(compiled))
    }

    fn parse_expr_inner(&mut self, actor: Expr) -> Result<Expr, SyntaxError> {
        match self.lexer.peek().transpose()? {
            Some((token, span)) => {
                match token {
                    // actor()
                    Token::LeftParen => {
                        let call = self.parse_function_call()?;
                        self.parse_expr_inner(call)
                    }
                    // actor "string"
                    Token::String(s) => {
                        // let arguments = vec![Expression::String(s.to_string())];
                        // let function = Box::new(actor);
                        // self.parse_expr_inner(Expression::Call {
                        //     function,
                        //     arguments,
                        // })
                        unimplemented!()
                    }
                    // actor array
                    Token::LeftBracket => {
                        unimplemented!()
                    }
                    // actor object
                    Token::LeftBrace => {
                        // start of for/if/else block
                        Ok(actor)
                    }
                    _ => Ok(actor),
                }
            }
            None => Ok(actor),
        }
    }

    fn parse_if(&mut self) -> Result<Statement, SyntaxError> {
        // in case
        self.expect(Token::If)?;

        let condition = self.parse_expr()?;
        self.expect(Token::LeftBrace)?;
        let then_block = self.parse_block()?;
        self.expect(Token::RightBrace)?;

        match self.lexer.peek().transpose()? {
            Some((token, _span)) => {
                if token == Token::Else {
                    self.lexer.next();

                    self.expect(Token::LeftBrace)?;
                    let else_block = self.parse_block()?;
                    self.expect(Token::RightBrace)?;

                    Ok(Statement::If(IfStatement {
                        condition,
                        then_block,
                        else_block: Some(else_block),
                    }))
                } else {
                    Ok(Statement::If(IfStatement {
                        condition,
                        then_block,
                        else_block: None,
                    }))
                }
            }
            None => Ok(Statement::If(IfStatement {
                condition,
                then_block,
                else_block: None,
            })),
        }
    }

    fn parse_for(&mut self) -> Result<Statement, SyntaxError> {
        // in case
        self.expect(Token::For)?;

        let key = match self.lexer.next().transpose()? {
            Some((token, span)) => match token {
                Token::Identifier(s) => {
                    // if self
                    //     .variables
                    //     .iter()
                    //     .find(|variable| variable.name == s)
                    //     .is_some()
                    // {
                    //     return Err(SyntaxError::VariableAlreadyDefined {
                    //         name: s.to_string(),
                    //         span
                    //     });
                    // }

                    s.to_string()
                }
                _ => {
                    return Err(SyntaxError::UnexpectedToken {
                        got: token.to_string(),
                        want: Some("identifier".to_string()),
                        span,
                    })
                }
            },
            None => return Err(SyntaxError::UnexpectedEof),
        };

        self.expect(Token::Comma)?;

        let value = match self.lexer.next().transpose()? {
            Some((token, span)) => match token {
                Token::Identifier(s) => {
                    // if self
                    //     .variables
                    //     .iter()
                    //     .find(|variable| variable.name == s)
                    //     .is_some()
                    // {
                    //     return Err(SyntaxError::VariableAlreadyDefined {
                    //         name: s.to_string(),
                    //         span
                    //     });
                    // }

                    s.to_string()
                }
                _ => {
                    return Err(SyntaxError::UnexpectedToken {
                        got: token.to_string(),
                        want: Some("identifier".to_string()),
                        span,
                    })
                }
            },
            None => return Err(SyntaxError::UnexpectedEof),
        };

        self.expect(Token::In)?;

        let iterator = self.parse_expr()?;

        self.expect(Token::LeftBrace)?;

        self.register_variable(key.clone());
        self.register_variable(value.clone());
        self.iterating += 1;
        let block = self.parse_block()?;
        self.iterating -= 1;

        // parse block does not consume '}'
        self.expect(Token::RightBrace)?;

        Ok(Statement::For(ForStatement {
            key,
            value,
            iterator,
            block,
        }))
    }

    fn parse_array(&mut self) -> Result<Expr, SyntaxError> {
        self.expect(Token::LeftBracket)?;

        let mut array = vec![];
        loop {
            match self.lexer.peek().transpose()? {
                Some((token, span)) => match token {
                    Token::RightBracket => {
                        let _ = self.lexer.next();
                        break;
                    }
                    Token::Comma => {
                        let _ = self.lexer.next();
                    }
                    _ => {
                        let item = self.parse_expr()?;
                        array.push(item);
                    }
                },
                None => return Err(SyntaxError::UnexpectedEof),
            }
        }

        Ok(Expr::Array(array))
    }

    fn parse_object(&mut self) -> Result<Expr, SyntaxError> {
        self.expect(Token::LeftBrace)?;

        let mut object = BTreeMap::new();
        loop {
            let (token, span) = self
                .lexer
                .next()
                .transpose()?
                .ok_or(SyntaxError::UnexpectedEof)?;

            let key = match token {
                Token::Colon => break,
                Token::Identifier(s) => s.to_string(),
                Token::RightBrace => break,
                _ => {
                    return Err(SyntaxError::UnexpectedToken {
                        got: token.to_string(),
                        want: Some("identifier".to_string()),
                        span,
                    })
                }
            };

            self.expect(Token::Colon)?;

            let value = self.parse_expr()?;

            object.insert(key, value);

            match self.lexer.peek().transpose()? {
                Some((token, span)) => match token {
                    Token::Comma => {
                        let _ = self.lexer.next();
                        continue;
                    }
                    Token::RightBrace => {
                        let _ = self.lexer.next();
                        break;
                    }
                    _ => {
                        return Err(SyntaxError::UnexpectedToken {
                            got: token.to_string(),
                            want: Some("comma or colon".to_string()),
                            span,
                        })
                    }
                },
                None => return Err(SyntaxError::UnexpectedEof),
            }
        }

        Ok(Expr::Object(object))
    }

    fn expect(&mut self, want: Token<&str>) -> Result<(), SyntaxError> {
        match self.lexer.next() {
            Some(result) => {
                let (got, span) = result?;
                if got == want {
                    Ok(())
                } else {
                    Err(SyntaxError::UnexpectedToken {
                        got: got.to_string(),
                        want: Some(want.to_string()),
                        span,
                    })
                }
            }
            None => Err(SyntaxError::UnexpectedEof),
        }
    }

    fn next_if(
        &mut self,
        f: impl FnOnce(Token<&str>) -> bool,
    ) -> Option<Result<(Token<&str>, Span), SyntaxError>> {
        let next = match self.lexer.peek() {
            Some(Ok((token, _span))) => {
                if !f(token) {
                    return None;
                }

                self.lexer.next()
            }
            Some(Err(_err)) => self.lexer.next(),
            None => None,
        };

        next.map(|result| {
            result.map_err(|err| SyntaxError::Lex {
                err,
                pos: self.lexer.pos(),
            })
        })
    }

    #[inline]
    fn register_variable(&mut self, name: String) {
        if !self.variables.iter().any(|var| var.name == name) {
            self.variables.push(Variable {
                name,
                value: Value::Null,
                // writes: 0,
            })
        }
    }
}

pub fn unescape_string(mut s: &str) -> String {
    let mut string = String::with_capacity(s.len());

    while let Some(i) = s.bytes().position(|b| b == b'\\') {
        let next = s.as_bytes()[i + 1];
        if next == b'\n' {
            // remote the \n and any ensuing spaces or tabs
            string.push_str(&s[..i]);
            let remaining = &s[i + 2..];
            let whitespace: usize = remaining
                .chars()
                .take_while(|c| c.is_whitespace())
                .map(char::len_utf8)
                .sum();
            s = &s[i + whitespace + 2..];
        } else {
            let c = match next {
                b'\'' => '\'',
                b'"' => '"',
                b'\\' => '\\',
                b'n' => '\n',
                b'r' => '\r',
                b't' => '\t',
                b'0' => '\0',
                b'{' => '{',
                _ => unimplemented!("invalid escape for {}", next as char),
            };

            string.push_str(&s[..i]);
            string.push(c);
            s = &s[i + 2..];
        }
    }

    string.push_str(s);
    string
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(dead_code)]
    fn unescape(s: &str) -> String {
        let mut buf = String::with_capacity(s.len());
        let text = s.as_bytes();
        let mut pos = 0;

        while pos < text.len() {
            let c = text[pos] as char;
            if c != '\\' {
                buf.push(c);
                pos += 1;
                continue;
            }

            if pos + 1 == text.len() {
                buf.push(c);
                break;
            }

            pos += 1;
            match text[pos] {
                b'\'' => buf.push('\''),
                b'"' => buf.push('"'),
                b'\\' => buf.push('\\'),
                b'n' => buf.push('\n'),
                b'r' => buf.push('\r'),
                b't' => buf.push('\t'),
                c => buf.push(c as char),
            }
        }

        buf
    }

    #[allow(clippy::print_stdout)]
    fn assert_compile(input: &str) {
        match Compiler::compile(input) {
            Ok(program) => {
                // todo
            }
            Err(err) => {
                let mut lexer = Lexer::new(input);
                while let Some(result) = lexer.next() {
                    match result {
                        Ok((token, span)) => {
                            println!("{:2}-{:2}:  {:?}", span.start, span.end, token)
                        }
                        Err(err) => {
                            panic!("lex error: {:?}", err);
                        }
                    }
                }

                panic!("compile failed: {:?}", err);
            }
        }
    }

    #[test]
    fn function_call() {
        let input = "now()";
        assert_compile(input);
    }

    #[test]
    fn function_call_with_arguments() {
        let input = r#"lowercase("FOO")"#;
        assert_compile(input);
    }

    #[test]
    fn assign_with_func() {
        let input = "ts = now()";
        assert_compile(input)
    }

    #[test]
    fn if_statement() {
        let text = r#"
        if foo {
            .timestamp = now()
        }
        "#;

        assert_compile(text)
    }

    #[test]
    fn if_else_statement() {
        let text = r#"
        if foo {
            .timestamp = now()
        } else {
            .ts = now()
        }
        "#;
    }

    #[test]
    fn for_statement() {
        let text = r#"
        for k, v in .map {
            a = a + 1
            k = k + "string"
        }
        "#;

        assert_compile(text);
    }

    #[test]
    fn function_with_arguments() {
        let text = r#"
        bar = "UP"
        foo = lowercase(bar)
        "#;
        assert_compile(text);
    }

    #[test]
    fn calc() {
        let text = "foo = 1+2-3*4/5";
        assert_compile(text);
    }

    #[test]
    fn if_bool() {
        let input = r#"if false {
        foo = "bar"
        }"#;
        assert_compile(input);
    }

    #[test]
    fn assign_object() {
        let input = r#"
        foo = {
            str: "value",
            int: 1,
            float: 1.1
        }
        "#;

        assert_compile(input)
    }

    #[test]
    fn assign_array() {
        let input = r#"
        arr = [
            1,
            "str",
            false,
            true,
            1.2,
            1 + 1,
        ]
        "#;

        assert_compile(input)
    }

    #[allow(clippy::print_stdout)]
    #[test]
    fn mixed() {
        let input = r#"
            if .index + 10 == 15 {
        log("15")
    }

    for index, item in .array {
        log("array:", index, item)
    }

    for key, value in .map {
        log("map:", key, value)
    }
        "#;

        let mut lexer = Lexer::new(input);
        while let Some(result) = lexer.next() {
            match result {
                Ok((token, span)) => {
                    println!("{:3}-{:3}:  {:?}", span.start, span.end, token)
                }
                Err(err) => {
                    panic!("lex error: {:?}", err);
                }
            }
        }
    }
}
