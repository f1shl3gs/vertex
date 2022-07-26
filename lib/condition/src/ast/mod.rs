mod field;

use std::ops::Deref;
use std::str::FromStr;

use event::attributes::Value;
use event::LogRecord;

use crate::lexer::Lexer;
use crate::Error;

#[derive(Debug)]
pub enum CombiningOp {
    And,
    Or
}

impl FromStr for CombiningOp {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "and" | "&&" => Ok(CombiningOp::And),
            "or" | "||" => Ok(CombiningOp::Or),
            _ => Err(Error::UnexpectedCombiningOp(s.into()))
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Operator {
    // Logical
    And,
    Or,

    // Numbers
    LessThan,
    LessEqual,
    Equal,
    GreaterEqual,
    GreaterThan,

    // String
    Contains,
    Match,
}

impl TryFrom<(usize, &str)> for Operator {
    type Error = Error;

    fn try_from(value: (usize, &str)) -> Result<Self, Self::Error> {
        let pos = value.0;
        let value = value.1;

        match value {
            "and" => Ok(Operator::And),
            "or" => Ok(Operator::Or),

            "lt" => Ok(Operator::LessThan),
            "le" => Ok(Operator::LessEqual),
            "eq" => Ok(Operator::Equal),
            "ge" => Ok(Operator::GreaterEqual),
            "gt" => Ok(Operator::GreaterThan),

            "contains" => Ok(Operator::Contains),
            "match" => Ok(Operator::Match),

            _ => Err(Error::UnknownOperator {
                pos,
                found: value.into(),
            }),
        }
    }
}

#[derive(Debug)]
pub enum Expression {
    Float(f64),
    String(String),
    Path(String),
    Regex(regex::Regex),

    Binary {
        op: Operator,
        lhs: Box<Expression>,
        rhs: Box<Expression>,
    },

    // support Unary !?
}

impl PartialEq for Expression {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Expression::Float(a), Expression::Float(b)) => a.eq(b),
            (Expression::String(a), Expression::String(b)) => a.eq(b),
            (Expression::Path(a), Expression::Path(b)) => a.eq(b),
            (Expression::Regex(a), Expression::Regex(b)) => a.as_str().eq(b.as_str()),
            (
                Expression::Binary {
                    lhs: al,
                    op: ao,
                    rhs: ar,
                },
                Expression::Binary {
                    lhs: bl,
                    op: bo,
                    rhs: br,
                },
            ) => al.eq(bl) && ao.eq(bo) && ar.eq(br),
            _ => false,
        }
    }
}

impl Expression {
    fn boxed(self) -> Box<Self> {
        Box::new(self)
    }

    fn eval(&self, log: &LogRecord) -> Result<bool, Error> {
        todo!()
    }
}

struct Parser<'a> {
    lexer: Lexer<'a>,
}

impl<'a> Parser<'a> {
    fn primary(&mut self) -> Result<Expression, Error> {
        let (_pos, token) = self.lexer.next().ok_or(Error::EarlyEOF)?;

        if  token.starts_with('.') {
            Ok(Expression::Path(token.to_string()))
        } else if token == "(" {
            let node = self.expr()?;

            Ok(node)
        } else {
            if let Ok(f) = token.parse::<f64>() {
                Ok(Expression::Float(f))
            } else {
                Ok(Expression::String(token.into()))
            }
        }
    }

    fn term(&mut self) -> Result<Expression, Error> {
        let lhs = self.primary()?;

        let op: Operator = match self.lexer.next() {
            Some(next) => {
                next.try_into()?
            },
            None => return Ok(lhs)
        };

        let rhs = self.primary()?;

        Ok(Expression::Binary {
            op,
            lhs: lhs.boxed(),
            rhs: rhs.boxed()
        })
    }

    fn expr(&mut self) -> Result<Expression, Error> {
        let mut node = self.term()?;

        loop {
            let (pos, token) = match self.lexer.next() {
                Some((pos, token)) => (pos, token),
                None => break
            };

            if token == ")" {
                break;
            }

            let op = (pos, token).try_into()?;

            let rhs = self.term()?;

            node = Expression::Binary {
                lhs: node.boxed(),
                op,
                rhs: rhs.boxed()
            }
        }

        Ok(node)
    }

    #[inline]
    pub fn parse(&mut self) -> Result<Expression, Error> {
        self.expr()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::print_stdout)]

    use event::{fields, tags};
    use crate::ast::Operator::Contains;
    use super::*;

    #[test]
    fn parse_and_print() {
        let input = ".message contains info and (.upper gt 10 or .lower lt -1)";

        let lexer = Lexer::new(input);
        let mut parser = Parser { lexer };

        let node = parser.parse().unwrap();
        println!("{:#?}", node);
    }

    #[test]
    fn test_eval() {
        let log = LogRecord::new(tags!(
            "foo" => "bar"
        ), fields!(
            "message" => "info warn error",
            "upper" => 8,
            "lower" => 0,
        ));

        let tests = [
            (".message contains info",
             true)
        ];

        for (input, want) in tests {
            let lexer = Lexer::new(input);
            let mut parser = Parser { lexer };

            let expr = parser.parse().unwrap();
            let got = expr.eval(&log).unwrap();
            assert_eq!(got, want, "input: {}\nwant: {:?}\ngot:  {:?}", input, want, got)
        }
    }

    #[test]
    fn parse() {
        let tests = [
            (
                ".foo lt 10.1",
                Expression::Binary {
                    lhs: Box::new(Expression::Path(".foo".into())),
                    op: Operator::LessThan,
                    rhs: Box::new(Expression::Float(10.1)),
                },
            ),
            (
                ".foo lt 10 and .bar gt 2",
                Expression::Binary {
                    lhs: Box::new(Expression::Binary {
                        lhs: Box::new(Expression::Path(".foo".into())),
                        op: Operator::LessThan,
                        rhs: Box::new(Expression::Float(10.0)),
                    }),
                    op: Operator::And,
                    rhs: Box::new(Expression::Binary {
                        lhs: Expression::Path(".bar".into()).boxed(),
                        op: Operator::GreaterThan,
                        rhs: Expression::Float(2.0).boxed(),
                    }),
                },
            ),
            (
                " .foo lt 10 or .bar eq 3",
                Expression::Binary {
                    lhs: Expression::Binary {
                        lhs: Expression::Path(".foo".into()).boxed(),
                        op: Operator::LessThan,
                        rhs: Expression::Float(10.0).boxed(),
                    }
                        .boxed(),
                    op: Operator::Or,
                    rhs: Expression::Binary {
                        lhs: Expression::Path(".bar".into()).boxed(),
                        op: Operator::Equal,
                        rhs: Expression::Float(3.0).boxed(),
                    }
                        .boxed(),
                },
            ),
            (
                " .foo contains abc or .bar eq 3",
                Expression::Binary {
                    lhs: Expression::Binary {
                        lhs: Expression::Path(".foo".into()).boxed(),
                        op: Operator::Contains,
                        rhs: Expression::String("abc".into()).boxed(),
                    }
                        .boxed(),
                    op: Operator::Or,
                    rhs: Expression::Binary {
                        lhs: Expression::Path(".bar".into()).boxed(),
                        op: Operator::Equal,
                        rhs: Expression::Float(3.0).boxed(),
                    }
                        .boxed(),
                },
            ),
            (
                ".message contains info and (.upper gt 10 or .lower lt -1)",
                Expression::Binary {
                    op: Operator::And,
                    lhs: Expression::Binary {
                        op: Contains,
                        lhs: Expression::Path(".message".into()).boxed(),
                        rhs: Expression::String("info".into()).boxed()
                    }.boxed(),
                    rhs: Expression::Binary {
                        op: Operator::Or,
                        lhs: Expression::Binary {
                            op: Operator::GreaterThan,
                            lhs: Expression::Path(".upper".into()).boxed(),
                            rhs: Expression::Float(10.0).boxed()
                        }.boxed(),
                        rhs: Expression::Binary {
                            op: Operator::LessThan,
                            lhs: Expression::Path(".lower".into()).boxed(),
                            rhs: Expression::Float(-1.0).boxed()
                        }.boxed()
                    }.boxed()
                }
            )
        ];

        for (input, want) in tests {
            let lexer = Lexer::new(input);
            let mut p = Parser { lexer };

            let got = p.parse().unwrap();
            assert_eq!(
                got, want,
                "input: {}\nwant: {:?}\ngot:  {:?}",
                input, want, got
            )
        }
    }
}

