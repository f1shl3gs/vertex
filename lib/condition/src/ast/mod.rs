mod field;
mod lexer;

use std::str::FromStr;

use event::LogRecord;
use field::{FieldExpr, FieldOp, OrderingOp};
use lexer::Lexer;
use value::path::parse_target_path;

use crate::Error;

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, Debug, PartialEq)]
pub enum CombiningOp {
    And,
    Or,
}

impl FromStr for CombiningOp {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "and" | "&&" => Ok(CombiningOp::And),
            "or" | "||" => Ok(CombiningOp::Or),
            _ => Err(()),
        }
    }
}

pub trait Evaluator {
    fn eval(&self, log: &LogRecord) -> Result<bool, Error>;
}

#[derive(Clone, Debug)]
pub enum Expression {
    Field(FieldExpr),

    Binary {
        op: CombiningOp,
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
            ) => ao.eq(bo) && al.eq(bl) && ar.eq(br),
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
                CombiningOp::And => Ok(lhs.eval(log)? && rhs.eval(log)?),
                CombiningOp::Or => Ok(lhs.eval(log)? || rhs.eval(log)?),
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

        let lhs = parse_target_path(var).map_err(|_err| Error::InvalidPath {
            path: var.to_string(),
        })?;

        Ok(Expression::Field(FieldExpr { lhs, op }))
    }

    fn term(&mut self) -> Result<Expression, Error> {
        let lhs = self.primary()?;

        let op = match self.lexer.next() {
            Some((pos, token)) => {
                if token == ")" {
                    return Ok(lhs);
                }

                CombiningOp::from_str(token).map_err(|_| Error::UnknownCombiningOp {
                    pos,
                    token: token.to_string(),
                })?
            }
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

        while let Some((pos, token)) = self.lexer.next() {
            if token == ")" {
                break;
            }

            let op = CombiningOp::from_str(token).map_err(|_| Error::UnknownCombiningOp {
                pos,
                token: token.to_string(),
            })?;

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
    use event::fields;

    use super::*;

    #[test]
    fn test_eval() {
        let log = LogRecord::from(fields!(
            "message" => "info warn error",
            "upper" => 8,
            "lower" => 0,
        ));

        let tests: Vec<(&str, Result<bool, Error>)> = vec![
            (".message contains info", Ok(true)),
            (".message contains abc", Ok(false)),
            (".upper >= 8", Ok(true)),
            (".upper <= 8", Ok(true)),
            (".upper == 8", Ok(true)),
            (".upper > 8", Ok(false)),
            (".message contains info && .upper >= 8", Ok(true)),
            (".message contains info && .upper > 8", Ok(false)),
            (".message contains info || .upper >= 8", Ok(true)),
            (".message contains abc && .upper >= 8", Ok(false)),
            (".message contains abc && .upper < 8", Ok(false)),
            (
                ".message contains info and (.upper gt 10 or .lower lt -1)",
                Ok(false),
            ),
            (".message contains info && (.upper >= 8)", Ok(true)),
            (".message contains abc && ( .upper >= 8 )", Ok(false)),
            (
                ".message contains abc && ( .upper >= 8 && .lower == 0 )",
                Ok(false),
            ),
            (".abc contains abc", Err(Error::MissingField(".abc".into()))),
            (
                ".message contains info && .abc < 8",
                Err(Error::MissingField(".abc".into())),
            ),
        ];

        for (input, want) in tests {
            let lexer = Lexer::new(input);
            let mut parser = Parser { lexer };

            let expr = parser
                .parse()
                .unwrap_or_else(|err| panic!("input: {}\nerr: {:?}", input, err));
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
        let foo = parse_target_path(".foo").unwrap();
        let bar = parse_target_path(".bar").unwrap();
        let message = parse_target_path(".message").unwrap();
        let upper = parse_target_path(".upper").unwrap();
        let lower = parse_target_path(".lower").unwrap();

        let tests = [
            // Ordering
            (
                ".foo lt 10.1",
                Ok(Expression::Field(FieldExpr {
                    lhs: foo.clone(),
                    op: FieldOp::Ordering {
                        op: OrderingOp::LessThan,
                        rhs: 10.1,
                    },
                })),
            ),
            (
                ".foo lt 10 and .bar gt 2",
                Ok(Expression::Binary {
                    op: CombiningOp::And,
                    lhs: Expression::Field(FieldExpr {
                        lhs: foo.clone(),
                        op: FieldOp::Ordering {
                            op: OrderingOp::LessThan,
                            rhs: 10.0,
                        },
                    })
                    .boxed(),
                    rhs: Expression::Field(FieldExpr {
                        lhs: bar.clone(),
                        op: FieldOp::Ordering {
                            op: OrderingOp::GreaterThan,
                            rhs: 2.0,
                        },
                    })
                    .boxed(),
                }),
            ),
            (
                " .foo lt 10 or .bar eq 3",
                Ok(Expression::Binary {
                    op: CombiningOp::Or,
                    lhs: Expression::Field(FieldExpr {
                        lhs: foo.clone(),
                        op: FieldOp::Ordering {
                            op: OrderingOp::LessThan,
                            rhs: 10.0,
                        },
                    })
                    .boxed(),
                    rhs: Expression::Field(FieldExpr {
                        lhs: bar.clone(),
                        op: FieldOp::Ordering {
                            op: OrderingOp::Equal,
                            rhs: 3.0,
                        },
                    })
                    .boxed(),
                }),
            ),
            (
                " .foo contains abc or .bar eq 3",
                Ok(Expression::Binary {
                    op: CombiningOp::Or,
                    lhs: Expression::Field(FieldExpr {
                        lhs: foo,
                        op: FieldOp::Contains("abc".into()),
                    })
                    .boxed(),
                    rhs: Expression::Field(FieldExpr {
                        lhs: bar,
                        op: FieldOp::Ordering {
                            op: OrderingOp::Equal,
                            rhs: 3.0,
                        },
                    })
                    .boxed(),
                }),
            ),
            (
                ".message contains info and (.upper gt 10 or .lower lt -1)",
                Ok(Expression::Binary {
                    op: CombiningOp::And,
                    lhs: Expression::Field(FieldExpr {
                        lhs: message,
                        op: FieldOp::Contains("info".into()),
                    })
                    .boxed(),
                    rhs: Expression::Binary {
                        op: CombiningOp::Or,
                        lhs: Expression::Field(FieldExpr {
                            lhs: upper,
                            op: FieldOp::Ordering {
                                op: OrderingOp::GreaterThan,
                                rhs: 10.0,
                            },
                        })
                        .boxed(),
                        rhs: Expression::Field(FieldExpr {
                            lhs: lower,
                            op: FieldOp::Ordering {
                                op: OrderingOp::LessThan,
                                rhs: -1.0,
                            },
                        })
                        .boxed(),
                    }
                    .boxed(),
                }),
            ),
            ("abc", Err(Error::PathExpected { pos: 0 })),
            (".abc", Err(Error::EarlyEOF)),
            (".abc gt", Err(Error::EarlyEOF)),
            (
                ".abc gt 1.1a",
                Err(Error::InvalidNumber {
                    pos: 8,
                    token: "1.1a".to_string(),
                    err: "1.1a".parse::<f64>().unwrap_err(),
                }),
            ),
        ];

        for (input, want) in tests {
            let lexer = Lexer::new(input);
            let mut p = Parser { lexer };

            let got = p.parse();
            assert_eq!(
                got, want,
                "input: {}\nwant: {:?}\ngot:  {:?}",
                input, want, got
            )
        }
    }
}
