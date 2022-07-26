mod field;

use std::str::FromStr;

use event::LogRecord;

use crate::ast::field::{FieldExpr, FieldOp, OrderingOp};
use crate::lexer::Lexer;
use crate::Error;

#[derive(Debug)]
pub enum CombiningOp {
    And,
    Or,
}

impl FromStr for CombiningOp {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "and" | "&&" => Ok(CombiningOp::And),
            "or" | "||" => Ok(CombiningOp::Or),
            _ => Err(Error::UnexpectedCombiningOp(s.into())),
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

pub trait Evaluator {
    fn eval(&self, log: &LogRecord) -> Result<bool, Error>;
}

#[derive(Debug)]
pub enum Expression {
    Field(FieldExpr),

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
            (Expression::Field(a), Expression::Field(b)) => a.eq(b),
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

    pub fn eval(&self, log: &LogRecord) -> Result<bool, Error> {
        match self {
            Expression::Field(f) => f.eval(log),
            Expression::Binary { op, lhs, rhs } => match op {
                Operator::And => Ok(lhs.eval(log)? && rhs.eval(log)?),
                Operator::Or => Ok(lhs.eval(log)? || rhs.eval(log)?),
                _ => unreachable!(),
            },
        }
    }
}

pub struct Parser<'a> {
    lexer: Lexer<'a>,
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a str) -> Self {
        let lexer = Lexer::new(input);

        Self { lexer }
    }

    fn primary(&mut self) -> Result<Expression, Error> {
        let (pos, token) = self.lexer.next().ok_or(Error::EarlyEOF)?;

        if token.starts_with('.') {
            self.field(token)
        } else if token == "(" {
            let node = self.expr()?;

            Ok(node)
        } else {
            Err(Error::PathExpected { pos })
        }
    }

    fn field(&mut self, var: &str) -> Result<Expression, Error> {
        let (op_pos, op) = self.lexer.next().ok_or(Error::EarlyEOF)?;
        let (rhs_pos, rhs) = self.lexer.next().ok_or(Error::EarlyEOF)?;

        let op = match op {
            "contains" => FieldOp::Contains(rhs.into()),
            "match" | "~" => {
                let re = regex::bytes::Regex::new(rhs).map_err(|err| Error::InvalidRegex {
                    pos: rhs_pos,
                    token: rhs.into(),
                    err,
                })?;

                FieldOp::Matches(re)
            }
            _ => {
                let op = OrderingOp::from_str(op).map_err(|_| Error::UnknownFieldOp {
                    pos: op_pos,
                    token: op.into(),
                })?;

                let rhs = rhs.parse().map_err(|err| Error::InvalidNumber {
                    err,
                    pos: rhs_pos,
                    token: rhs.into(),
                })?;

                FieldOp::Ordering { op, rhs }
            }
        };

        Ok(Expression::Field(FieldExpr {
            lhs: var.strip_prefix('.').unwrap().into(),
            op,
        }))
    }

    fn term(&mut self) -> Result<Expression, Error> {
        let lhs = self.primary()?;

        let op: Operator = match self.lexer.next() {
            Some(next) => next.try_into()?,
            None => return Ok(lhs),
        };

        let rhs = self.primary()?;

        Ok(Expression::Binary {
            op,
            lhs: lhs.boxed(),
            rhs: rhs.boxed(),
        })
    }

    fn expr(&mut self) -> Result<Expression, Error> {
        let mut node = self.term()?;

        loop {
            let (pos, token) = match self.lexer.next() {
                Some((pos, token)) => (pos, token),
                None => break,
            };

            if token == ")" {
                break;
            }

            let op = (pos, token).try_into()?;

            let rhs = self.term()?;

            node = Expression::Binary {
                lhs: node.boxed(),
                op,
                rhs: rhs.boxed(),
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

    use super::*;
    use event::{fields, tags};

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
        let log = LogRecord::new(
            tags!(
                "foo" => "bar"
            ),
            fields!(
                "message" => "info warn error",
                "upper" => 8,
                "lower" => 0,
            ),
        );

        let tests: Vec<(&str, Result<bool, Error>)> = vec![
            (".message contains info", Ok(true)),
            (".message contains abc", Ok(false)),
            (".upper >= 8", Ok(true)),
            (".upper <= 8", Ok(true)),
            (".upper == 8", Ok(true)),
            (".upper > 8", Ok(false)),
        ];

        for (input, want) in tests {
            let lexer = Lexer::new(input);
            let mut parser = Parser { lexer };

            let expr = parser.parse().unwrap();
            let got = expr.eval(&log);

            assert_eq!(
                want, got,
                "input: {}\nwant: {:?}\ngot:  {:?}",
                input, want, got
            );
        }
    }

    #[test]
    fn parse() {
        let tests = [
            // Ordering
            (
                ".foo lt 10.1",
                Expression::Field(FieldExpr {
                    lhs: ".foo".to_string(),
                    op: FieldOp::Ordering {
                        op: OrderingOp::LessThan,
                        rhs: 10.1,
                    },
                }),
            ),
            (
                ".foo lt 10 and .bar gt 2",
                Expression::Binary {
                    op: Operator::And,
                    lhs: Expression::Field(FieldExpr {
                        lhs: ".foo".into(),
                        op: FieldOp::Ordering {
                            op: OrderingOp::LessThan,
                            rhs: 10.0,
                        },
                    })
                    .boxed(),
                    rhs: Expression::Field(FieldExpr {
                        lhs: ".bar".into(),
                        op: FieldOp::Ordering {
                            op: OrderingOp::GreaterThan,
                            rhs: 2.0,
                        },
                    })
                    .boxed(),
                },
            ),
            (
                " .foo lt 10 or .bar eq 3",
                Expression::Binary {
                    op: Operator::Or,
                    lhs: Expression::Field(FieldExpr {
                        lhs: ".foo".to_string(),
                        op: FieldOp::Ordering {
                            op: OrderingOp::LessThan,
                            rhs: 10.0,
                        },
                    })
                    .boxed(),
                    rhs: Expression::Field(FieldExpr {
                        lhs: ".bar".into(),
                        op: FieldOp::Ordering {
                            op: OrderingOp::Equal,
                            rhs: 3.0,
                        },
                    })
                    .boxed(),
                },
            ),
            (
                " .foo contains abc or .bar eq 3",
                Expression::Binary {
                    op: Operator::Or,
                    lhs: Expression::Field(FieldExpr {
                        lhs: ".foo".to_string(),
                        op: FieldOp::Contains("abc".into()),
                    })
                    .boxed(),
                    rhs: Expression::Field(FieldExpr {
                        lhs: ".bar".into(),
                        op: FieldOp::Ordering {
                            op: OrderingOp::Equal,
                            rhs: 3.0,
                        },
                    })
                    .boxed(),
                },
            ),
            (
                ".message contains info and (.upper gt 10 or .lower lt -1)",
                Expression::Binary {
                    op: Operator::And,
                    lhs: Expression::Field(FieldExpr {
                        lhs: ".message".to_string(),
                        op: FieldOp::Contains("info".into()),
                    })
                    .boxed(),
                    rhs: Expression::Binary {
                        op: Operator::Or,
                        lhs: Expression::Field(FieldExpr {
                            lhs: ".upper".into(),
                            op: FieldOp::Ordering {
                                op: OrderingOp::GreaterThan,
                                rhs: 10.0,
                            },
                        })
                        .boxed(),
                        rhs: Expression::Field(FieldExpr {
                            lhs: ".lower".into(),
                            op: FieldOp::Ordering {
                                op: OrderingOp::LessThan,
                                rhs: -1.0,
                            },
                        })
                        .boxed(),
                    }
                    .boxed(),
                },
            ),
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
