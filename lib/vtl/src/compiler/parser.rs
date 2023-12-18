use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};

use bytes::Bytes;
use value::{parse_target_path, parse_value_path, OwnedTargetPath, PathParseError, Value};

use super::assignment::{Assignment, AssignmentTarget};
use super::binary::{Binary, BinaryOp};
use super::block::Block;
use super::expr::Expr;
use super::for_statement::ForStatement;
use super::function::{builtin_functions, ArgumentList, Function, FunctionCompileContext};
use super::if_statement::IfStatement;
use super::levenshtein::distance;
use super::lex::{LexError, Lexer, Token};
use super::query::Query;
use super::state::TypeState;
use super::statement::Statement;
use super::unary::{Unary, UnaryError, UnaryOp};
use super::Kind;
use super::Program;
use super::{BinaryError, Expression};
use super::{Span, Spanned};
use crate::diagnostic::{DiagnosticMessage, Label};

#[derive(Debug)]
pub enum SyntaxError {
    Lex(LexError),

    EmptyBlock {
        span: Span,
    },
    UnexpectedEof,
    UnexpectedToken {
        got: String,
        want: Option<String>,
        span: Span,
    },

    InvalidTemplate {
        span: Span,
    },

    //
    InvalidType {
        want: String,
        got: String,
        span: Span,
    },

    // If
    NonBooleanPrediction {
        got: Kind,
        span: Span,
    },
    FalliblePrediction {
        span: Span,
    },

    // Fallible things
    FallibleArgument {
        span: Span,
    },
    FallibleIterator {
        span: Span,
    },

    // Unary
    Unary(UnaryError),

    Binary {
        err: BinaryError,
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
        span: Span,
    },
    VariableAlreadyDefined {
        name: String,
        span: Span,
    },
    UndefinedVariable {
        name: String,
        maybe: Option<String>,
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
        maybe: Option<String>,
        span: Span,
    },
    FunctionArgumentsArityMismatch {
        function: &'static str,
        takes: usize,
        got: usize,
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
}

impl From<LexError> for SyntaxError {
    fn from(err: LexError) -> Self {
        SyntaxError::Lex(err)
    }
}

impl Display for SyntaxError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SyntaxError::Lex(err) => Display::fmt(err, f),
            SyntaxError::EmptyBlock { .. } => f.write_str("empty block is not allowed"),
            SyntaxError::UnexpectedEof => f.write_str("unexpected end of file"),
            SyntaxError::UnexpectedToken { got, want, .. } => match want {
                Some(want) => write!(f, "unexpected token: {}, want: {}", got, want),
                None => write!(f, "unexpected token: \"{}\"", got),
            },
            SyntaxError::UndefinedVariable { name, maybe, .. } => match maybe {
                Some(maybe) => write!(
                    f,
                    "undefined variable \"{}\", do you mean \"{}\"?",
                    name, maybe
                ),
                None => write!(f, "undefined variable {}", name),
            },
            SyntaxError::UndefinedFunction { name, .. } => {
                write!(f, "unknown function {}", name)
            }

            SyntaxError::Unary(err) => Display::fmt(err, f),
            SyntaxError::Binary { err, .. } => Display::fmt(err, f),

            // Fallible
            SyntaxError::FalliblePrediction { .. } => f.write_str("fallible prediction"),
            SyntaxError::FallibleArgument { .. } => f.write_str("fallible argument"),

            SyntaxError::InvalidPath { err, .. } => {
                write!(f, "invalid target path {}", err)
            }
            SyntaxError::VariableNeverUsed { name, .. } => {
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
            SyntaxError::UnnecessaryErrorAssignment { .. } => {
                f.write_str("unnecessary error assignment")
            }
            SyntaxError::UnhandledFallibleAssignment { .. } => {
                f.write_str("unhandled fallible assignment")
            }

            SyntaxError::InvalidType { .. } => f.write_str("invalid type"),
            SyntaxError::FallibleIterator { .. } => f.write_str("fallible iterator"),
            SyntaxError::NonBooleanPrediction { .. } => f.write_str("non-boolean prediction"),

            SyntaxError::InvalidTemplate { .. } => f.write_str("invalid template"),
        }
    }
}

impl Error for SyntaxError {}

impl DiagnosticMessage for SyntaxError {
    fn labels(&self) -> Vec<Label> {
        match self {
            SyntaxError::Lex(err) => err.labels(),
            SyntaxError::EmptyBlock { span } => {
                vec![
                    Label::new(
                        "block start",
                        Span {
                            start: span.start,
                            end: span.start + 1,
                        },
                    ),
                    Label::new(
                        "block end",
                        Span {
                            start: span.end - 1,
                            end: span.end,
                        },
                    ),
                ]
            }
            SyntaxError::UnexpectedEof => vec![],
            SyntaxError::UnexpectedToken { got, want, span } => {
                let msg = match want {
                    Some(want) => format!("got {}, want {}", got, want),
                    None => format!("got \"{}\"", got),
                };

                vec![Label::new(msg, span)]
            }
            SyntaxError::InvalidPath { err, span } => {
                vec![Label::new(err.to_string(), span)]
            }
            SyntaxError::VariableNeverUsed { span, .. } => {
                vec![Label::new("variable defined here", span)]
            }
            SyntaxError::VariableAlreadyDefined { span, .. } => {
                // todo:
                vec![Label::new("variable already defined", span)]
            }
            SyntaxError::UndefinedVariable { span, maybe, .. } => {
                let msg = match maybe {
                    Some(maybe) => {
                        format!("undefined variable, do you mean \"{}\"?", maybe)
                    }
                    None => "undefined variable".to_string(),
                };

                vec![Label::new(msg, span)]
            }
            SyntaxError::UnnecessaryErrorAssignment { span } => {
                vec![Label::new("this assignment is not necessary", span)]
            }
            SyntaxError::UnhandledFallibleAssignment { span } => {
                vec![Label::new("this expression is fallible", span)]
            }
            SyntaxError::UndefinedFunction { maybe, span, .. } => {
                let msg = match maybe {
                    Some(maybe) => {
                        format!("undefined function, do you mean \"{}\"?", maybe)
                    }
                    None => "undefined function".to_string(),
                };
                vec![Label::new(msg, span)]
            }
            SyntaxError::FunctionArgumentsArityMismatch {
                function,
                takes,
                got,
                span,
            } => {
                vec![Label::new(
                    format!("{} takes {}, got {}", function, takes, got),
                    span,
                )]
            }
            SyntaxError::InvalidFunctionArgumentType {
                want, got, span, ..
            } => {
                vec![Label::new(format!("want: {}, got {}", want, got), span)]
            }
            SyntaxError::InvalidValue {
                want, got, span, ..
            } => {
                vec![Label::new(format!("want: {}, got: {}", want, got), span)]
            }

            SyntaxError::FalliblePrediction { span } => {
                vec![Label::new("fallible prediction", span)]
            }
            SyntaxError::FallibleArgument { span } => {
                vec![Label::new("fallible argument is not allowed", span)]
            }

            SyntaxError::Unary(err) => err.labels(),
            SyntaxError::Binary { err, .. } => err.labels(),

            SyntaxError::InvalidType { want, got, span } => {
                vec![Label::new(format!("want: {}, got: {}", want, got), span)]
            }
            SyntaxError::FallibleIterator { span } => {
                vec![Label::new("fallible iterator is not allowed", span)]
            }

            SyntaxError::NonBooleanPrediction { got, span } => {
                vec![Label::new(
                    format!("prediction must be resolved to boolean, instead of {}", got),
                    span,
                )]
            }
            SyntaxError::InvalidTemplate { span } => {
                vec![Label::new("unescape template failed", span)]
            }
        }
    }
}

struct Variable {
    name: String,
    value: Value,
    // maybe we should track usage
    // reads: usize,
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
    type_state: TypeState,

    target_queries: Vec<OwnedTargetPath>,
}

impl Compiler<'_> {
    pub fn compile(input: &'_ str) -> Result<Program, SyntaxError> {
        let lexer = Lexer::new(input);
        let mut compiler = Compiler {
            lexer,
            functions: builtin_functions(),
            iterating: 0,
            variables: vec![],
            type_state: TypeState::default(),
            target_queries: vec![],
        };

        let block = compiler.parse_block()?;

        // todo: check variables
        //   if the variables are never changed, return error

        Ok(Program {
            statements: block,
            target_queries: compiler.target_queries,
            variables: compiler
                .variables
                .into_iter()
                .map(|var| (var.name, var.value))
                .collect(),
        })
    }

    fn parse_block(&mut self) -> Result<Block, SyntaxError> {
        let mut statements = vec![];

        while let Some((token, span)) = self.lexer.peek().transpose()? {
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

                Token::FunctionCall(_name) => {
                    if let Expr::Call(call) = self.parse_function_call()?.node {
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
                    statements.push(Statement::Expression(expr.node))
                }
            }
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
                        let path = if path == "." {
                            OwnedTargetPath::event_root()
                        } else if path == "%" {
                            OwnedTargetPath::metadata_root()
                        } else {
                            parse_target_path(path)
                                .map_err(|err| SyntaxError::InvalidPath { err, span })?
                        };

                        return Ok(AssignmentTarget::External(path));
                    }

                    // "foo" or "foo.bar"
                    match path.split_once(|c| c == '.' || c == '[') {
                        Some((name, path)) => {
                            // at this case, the variable must exists already
                            let exists =
                                self.variables.iter().any(|variable| variable.name == name);
                            if !exists {
                                let maybe = self
                                    .variables
                                    .iter()
                                    .map(|var| {
                                        (&var.name, distance(var.name.as_bytes(), name.as_bytes()))
                                    })
                                    .min_by_key(|(_var, score)| *score)
                                    .map(|(var, _score)| var.to_string());

                                return Err(SyntaxError::UndefinedVariable {
                                    name: name.to_string(),
                                    maybe,
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

                        let expr = self.parse_expr()?;
                        let expr_type = expr.type_def(&self.type_state);
                        if !expr_type.fallible {
                            return Err(SyntaxError::UnnecessaryErrorAssignment {
                                span: expr.span,
                            });
                        }

                        self.type_state.apply(&target, expr_type.kind);
                        self.type_state.apply(&err, Kind::BYTES);

                        Assignment::Infallible {
                            ok: target,
                            err,
                            expr,
                        }
                    }

                    // a = 1 + 2
                    // a = fallible()?
                    Token::Assign => {
                        let expr = self.parse_expr()?;
                        if expr.type_def(&self.type_state).fallible {
                            match self.lexer.peek().transpose()? {
                                Some((Token::Question, _span)) => {
                                    // it's ok
                                    self.lexer.next();
                                }
                                _ => {
                                    return Err(SyntaxError::UnhandledFallibleAssignment {
                                        span: expr.span,
                                    });
                                }
                            }
                        }

                        self.type_state
                            .apply(&target, expr.type_def(&self.type_state).kind);

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

    fn parse_expr(&mut self) -> Result<Spanned<Expr>, SyntaxError> {
        // maybe array or object
        self.parse_expr_or()
    }

    fn parse_expr_or(&mut self) -> Result<Spanned<Expr>, SyntaxError> {
        let mut expr = self.parse_expr_and()?;

        while let Some((token, _span)) = self.lexer.peek().transpose()? {
            if let Token::Or = token {
                let _ = self.lexer.next();

                let rhs = self.parse_expr_and()?;
                let span = expr.span.merge(rhs.span);
                expr = Binary::compile(expr, BinaryOp::Or, rhs)
                    .map_err(|err| SyntaxError::Binary { err, span })?
                    .with(span);
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn parse_expr_and(&mut self) -> Result<Spanned<Expr>, SyntaxError> {
        let mut expr = self.parse_expr_comparison()?;

        while let Some((token, _span)) = self.lexer.peek().transpose()? {
            if let Token::And = token {
                let _ = self.lexer.next();

                let rhs = self.parse_expr_comparison()?;
                let span = expr.span.merge(rhs.span);
                expr = Binary::compile(expr, BinaryOp::And, rhs)
                    .map_err(|err| SyntaxError::Binary { err, span })?
                    .with(span);
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn parse_expr_comparison(&mut self) -> Result<Spanned<Expr>, SyntaxError> {
        let mut expr = self.parse_expr_term()?;

        while let Some((token, _span)) = self.lexer.peek().transpose()? {
            let op = match token {
                Token::Equal => BinaryOp::Equal,
                Token::NotEqual => BinaryOp::NotEqual,
                Token::GreatThan => BinaryOp::GreatThan,
                Token::GreatEqual => BinaryOp::GreatEqual,
                Token::LessThan => BinaryOp::LessThan,
                Token::LessEqual => BinaryOp::LessEqual,
                _ => break,
            };

            let _ = self.lexer.next();
            let rhs = self.parse_expr_term()?;
            let span = expr.span.merge(rhs.span);
            expr = Binary::compile(expr, op, rhs)
                .map_err(|err| SyntaxError::Binary { err, span })?
                .with(span);
        }

        Ok(expr)
    }

    fn parse_expr_term(&mut self) -> Result<Spanned<Expr>, SyntaxError> {
        let mut expr = self.parse_expr_factor()?;

        while let Some((token, _span)) = self.lexer.peek().transpose()? {
            let op = match token {
                Token::Add => BinaryOp::Add,
                Token::Subtract => BinaryOp::Subtract,
                _ => break,
            };

            let _ = self.lexer.next();
            let rhs = self.parse_expr_factor()?;
            let span = expr.span.merge(rhs.span);
            expr = Binary::compile(expr, op, rhs)
                .map_err(|err| SyntaxError::Binary { err, span })?
                .with(span);
        }

        Ok(expr)
    }

    fn parse_expr_factor(&mut self) -> Result<Spanned<Expr>, SyntaxError> {
        let mut expr = self.parse_expr_unary()?;

        while let Some((token, _span)) = self.lexer.peek().transpose()? {
            let op = match token {
                Token::Multiply => BinaryOp::Multiply,
                Token::Divide => BinaryOp::Divide,
                _ => break,
            };

            let _ = self.lexer.next();
            let rhs = self.parse_expr_unary()?;
            let span = expr.span.merge(rhs.span);
            expr = Binary::compile(expr, op, rhs)
                .map_err(|err| SyntaxError::Binary { err, span })?
                .with(span);
        }

        Ok(expr)
    }

    fn parse_expr_unary(&mut self) -> Result<Spanned<Expr>, SyntaxError> {
        match self.lexer.peek().transpose()? {
            Some((token, span)) => {
                let op = match token {
                    Token::Not => UnaryOp::Not,
                    Token::Subtract => UnaryOp::Negate,
                    _ => return self.parse_expr_exponent(),
                };

                self.lexer.next();
                let operand = self.parse_expr_unary()?;
                let span = span.merge(operand.span);
                let expr = Unary::compile(op, operand, &self.type_state)
                    .map_err(SyntaxError::Unary)?
                    .with(span);
                Ok(expr)
            }
            None => self.parse_expr_exponent(),
        }
    }

    fn parse_expr_exponent(&mut self) -> Result<Spanned<Expr>, SyntaxError> {
        let mut expr = self.parse_expr_primary()?;

        while let Some((Token::Exponent, _span)) = self.lexer.peek().transpose()? {
            let _ = self.lexer.next();

            let rhs = self.parse_expr_exponent()?;
            let span = expr.span.merge(rhs.span);
            expr = Binary::compile(expr, BinaryOp::Exponent, rhs)
                .map_err(|err| SyntaxError::Binary { err, span })?
                .with(span);
        }

        Ok(expr)
    }

    fn parse_expr_primary(&mut self) -> Result<Spanned<Expr>, SyntaxError> {
        match self.lexer.peek().transpose()? {
            Some((token, span)) => match token {
                Token::Identifier(s) => {
                    let actor = Expr::Ident(s.to_string()).with(span);

                    self.lexer.next();

                    self.parse_expr_inner(actor)
                }
                Token::FunctionCall(_name) => self.parse_function_call(),
                Token::PathField(path) => {
                    // ".", ".foo", "%" or "%foo"
                    let query = if path.starts_with(|c| c == '.' || c == '%') {
                        let path = parse_target_path(path)
                            .map_err(|err| SyntaxError::InvalidPath { err, span })?;

                        self.target_queries.push(path.clone());

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
                                    let maybe = self
                                        .variables
                                        .iter()
                                        .map(|var| {
                                            (
                                                &var.name,
                                                distance(var.name.as_bytes(), name.as_bytes()),
                                            )
                                        })
                                        .min_by_key(|(_var, score)| *score)
                                        .map(|(var, _score)| var.to_string());

                                    return Err(SyntaxError::UndefinedVariable {
                                        name: name.to_string(),
                                        maybe,
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

                    Ok(Expr::Query(query).with(span))
                }
                // Simple literals
                Token::Integer(i) => {
                    let _ = self.lexer.next();
                    Ok(Expr::Integer(i).with(span))
                }
                Token::Float(f) => {
                    let _ = self.lexer.next();
                    Ok(Expr::Float(f).with(span))
                }
                Token::String(s) => {
                    let _ = self.lexer.next();
                    let unescaped = unescape_string(s);
                    Ok(Expr::String(Bytes::from(unescaped.into_bytes())).with(span))
                }
                Token::Null => {
                    let _ = self.lexer.next();
                    Ok(Expr::Null.with(span))
                }
                Token::True => {
                    let _ = self.lexer.next();
                    Ok(Expr::Boolean(true).with(span))
                }
                Token::False => {
                    let _ = self.lexer.next();
                    Ok(Expr::Boolean(false).with(span))
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

    fn parse_function_call(&mut self) -> Result<Spanned<Expr>, SyntaxError> {
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
            .ok_or_else(|| {
                let maybe = self
                    .functions
                    .iter()
                    .map(|func| {
                        (
                            func.identifier(),
                            distance(func.identifier().as_bytes(), name.as_bytes()),
                        )
                    })
                    .min_by_key(|(_var, score)| *score)
                    .map(|(var, _score)| var.to_string());

                SyntaxError::UndefinedFunction {
                    name: name.to_string(),
                    maybe,
                    span,
                }
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

                        if argument.type_def(&self.type_state).fallible {
                            return Err(SyntaxError::FallibleArgument {
                                span: Span { start, end },
                            });
                        }

                        if let Expr::Ident(s) = &argument.node {
                            let exists = self.variables.iter().any(|var| &var.name == s);
                            if !exists {
                                let maybe = self
                                    .variables
                                    .iter()
                                    .map(|var| {
                                        (&var.name, distance(var.name.as_bytes(), name.as_bytes()))
                                    })
                                    .min_by_key(|(_var, score)| *score)
                                    .map(|(var, _score)| var.to_string());

                                return Err(SyntaxError::UndefinedVariable {
                                    name: s.to_string(),
                                    maybe,
                                    span,
                                });
                            }

                            self.register_variable(s.to_string());
                        }

                        arguments.push(argument, &self.type_state)?;
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
            .ok_or_else(|| {
                let maybe = self
                    .functions
                    .iter()
                    .map(|func| {
                        (
                            func.identifier(),
                            distance(func.identifier().as_bytes(), name.as_bytes()),
                        )
                    })
                    .min_by_key(|(_var, score)| *score)
                    .map(|(var, _score)| var.to_string());

                SyntaxError::UndefinedFunction {
                    name: name.to_string(),
                    maybe,
                    span,
                }
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

        Ok(Expr::Call(compiled).with(span))
    }

    fn parse_expr_inner(&mut self, actor: Spanned<Expr>) -> Result<Spanned<Expr>, SyntaxError> {
        match self.lexer.peek().transpose()? {
            Some((token, _span)) => {
                match token {
                    // actor()
                    Token::LeftParen => {
                        let call = self.parse_function_call()?;
                        self.parse_expr_inner(call)
                    }
                    // actor "string"
                    Token::String(_s) => {
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
        self.expect(Token::If)?;

        let condition = self.parse_expr()?;
        if condition.type_def(&self.type_state).fallible {
            return Err(SyntaxError::FalliblePrediction {
                span: condition.span,
            });
        }

        if condition.type_def(&self.type_state).kind != Kind::BOOLEAN {
            return Err(SyntaxError::NonBooleanPrediction {
                got: condition.type_def(&self.type_state).kind,
                span: condition.span,
            });
        }

        let start_span = self.expect(Token::LeftBrace)?;
        let then_block = self.parse_block()?;
        let end_span = self.expect(Token::RightBrace)?;

        if then_block.is_empty() {
            return Err(SyntaxError::EmptyBlock {
                span: start_span.merge(end_span),
            });
        }

        match self.lexer.peek().transpose()? {
            Some((token, _span)) => {
                if token == Token::Else {
                    self.lexer.next();

                    let start_span = self.expect(Token::LeftBrace)?;
                    let else_block = self.parse_block()?;
                    let end_span = self.expect(Token::RightBrace)?;

                    if else_block.is_empty() {
                        return Err(SyntaxError::EmptyBlock {
                            span: start_span.merge(end_span),
                        });
                    }

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
        self.expect(Token::For)?;

        let key = match self.lexer.next().transpose()? {
            Some((token, span)) => match token {
                Token::Identifier(s) => {
                    // Override variable might happened
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
                    // Override variable might happened
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
        let td = iterator.type_def(&self.type_state);
        if td.fallible {
            return Err(SyntaxError::FallibleIterator {
                span: iterator.span,
            });
        }
        if td.kind.contains(Kind::ARRAY) {
            self.type_state.apply_variable(key.as_str(), Kind::INTEGER);
            self.type_state.apply_variable(value.as_str(), Kind::ANY);
        } else if td.kind.contains(Kind::OBJECT) {
            self.type_state.apply_variable(key.as_str(), Kind::BYTES);
            self.type_state.apply_variable(value.as_str(), Kind::ANY);
        } else {
            return Err(SyntaxError::InvalidType {
                want: "array or object".to_string(),
                got: td.kind.to_string(),
                span: iterator.span,
            });
        }

        let start_span = self.expect(Token::LeftBrace)?;

        self.register_variable(key.clone());
        self.register_variable(value.clone());

        self.iterating += 1;
        let block = self.parse_block()?;
        self.iterating -= 1;

        // parse block does not consume '}'
        let end_span = self.expect(Token::RightBrace)?;

        // check block
        if block.is_empty() {
            return Err(SyntaxError::EmptyBlock {
                span: start_span.merge(end_span),
            });
        }

        Ok(Statement::For(ForStatement {
            key,
            value,
            iterator,
            block,
        }))
    }

    fn parse_array(&mut self) -> Result<Spanned<Expr>, SyntaxError> {
        let mut arr_span = self.expect(Token::LeftBracket)?;

        let mut array = vec![];
        loop {
            match self.lexer.peek().transpose()? {
                Some((token, span)) => match token {
                    Token::RightBracket => {
                        let _ = self.lexer.next();
                        arr_span = arr_span.merge(span);
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

        Ok(Expr::Array(array).with(arr_span))
    }

    fn parse_object(&mut self) -> Result<Spanned<Expr>, SyntaxError> {
        let mut obj_span = self.expect(Token::LeftBrace)?;

        let mut object = BTreeMap::new();
        loop {
            let (token, span) = self
                .lexer
                .next()
                .transpose()?
                .ok_or(SyntaxError::UnexpectedEof)?;

            let key = match token {
                Token::Colon => break,
                // Token::Identifier(s) => s.to_string(),
                Token::String(s) => s.to_string(),
                Token::RightBrace => break,
                _ => {
                    return Err(SyntaxError::UnexpectedToken {
                        got: token.to_string(),
                        want: Some("string, colon or right brace".to_string()),
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
                        obj_span = obj_span.merge(span);
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

        Ok(Expr::Object(object).with(obj_span))
    }

    fn expect(&mut self, want: Token<&str>) -> Result<Span, SyntaxError> {
        match self.lexer.next() {
            Some(result) => {
                let (got, span) = result?;
                if got == want {
                    Ok(span)
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

    fn register_variable(&mut self, name: String) {
        if !self.variables.iter().any(|var| var.name == name) {
            self.variables.push(Variable {
                name,
                value: Value::Null,
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
                b'}' => '}',
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
            Ok(_program) => {
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
    fn metadata() {
        let input = r#"
        % = {
            "foo": 1
        }
        "#;

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
        if true {
            .timestamp = now()
        }
        "#;

        assert_compile(text)
    }

    #[test]
    fn if_else_statement() {
        let text = r#"
        if false {
            .timestamp = now()
        } else {
            .ts = now()
        }
        "#;

        assert_compile(text);
    }

    #[test]
    fn for_statement() {
        let text = r#"
        a = 1
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
    fn fallible_function_call() {
        let input = r#"
        parsed = parse_url("https://example.io/some/path?foo=bar")
        "#;

        match Compiler::compile(input) {
            Ok(_program) => panic!("should fail"),
            Err(err) => match err {
                SyntaxError::UnhandledFallibleAssignment {
                    span: Span { start: 18, end: 67 },
                } => {
                    // ok
                }
                err => panic!("invalid error, {}", err),
            },
        }
    }

    #[test]
    fn assign_object() {
        let input = r#"
        foo = {
            "str": "value",
            "int": 1,
            "float": 1.1,
            "array": [1, 2.3, true],
            "map": {
                "key": "value"
            }
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
