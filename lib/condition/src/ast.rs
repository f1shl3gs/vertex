use crate::lexer::Lexer;
use crate::Error;

#[derive(Debug, PartialEq)]
enum Operator {
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
enum Node {
    Float(f64),
    String(String),
    Path(String),
    Regex(regex::Regex),

    BinaryExpr {
        lhs: Box<Node>,
        op: Operator,
        rhs: Box<Node>,
    },
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Node::Float(a), Node::Float(b)) => a.eq(b),
            (Node::String(a), Node::String(b)) => a.eq(b),
            (Node::Path(a), Node::Path(b)) => a.eq(b),
            (Node::Regex(a), Node::Regex(b)) => a.as_str().eq(b.as_str()),
            (
                Node::BinaryExpr {
                    lhs: al,
                    op: ao,
                    rhs: ar,
                },
                Node::BinaryExpr {
                    lhs: bl,
                    op: bo,
                    rhs: br,
                },
            ) => al.eq(bl) && ao.eq(bo) && ar.eq(br),
            _ => false,
        }
    }
}

impl Node {
    fn boxed(self) -> Box<Self> {
        Box::new(self)
    }
}

struct Parser<'a> {
    lexer: Lexer<'a>,
}

impl<'a> Parser<'a> {
    fn parse(&mut self) -> Result<Node, Error> {
        let mut node = self.parse_expr()?;

        loop {
            let op = match self.try_parse_operator() {
                Some(Ok(op)) => op,
                Some(Err(err)) => return Err(err),
                None => return Ok(node),
            };

            let rhs = self.parse_expr()?;

            node = Node::BinaryExpr {
                lhs: node.boxed(),
                op,
                rhs: rhs.boxed(),
            }
        }
    }

    fn try_parse_operator(&mut self) -> Option<Result<Operator, Error>> {
        let next = self.lexer.next()?;
        Some(next.try_into())
    }

    fn parse_expr(&mut self) -> Result<Node, Error> {
        let (pos, token) = self.lexer.next().ok_or(Error::EarlyEOF {
            pos: self.lexer.pos(),
        })?;

        if token == "(" {
            self.parse_expr()
        } else if token.starts_with('.') {
            let lhs = Box::new(Node::Path(token.to_string()));
            let op = self.parse_operator()?;
            let rhs = Box::new(self.parse_rhs()?);

            Ok(Node::BinaryExpr { lhs, op, rhs })
        } else {
            Err(Error::ExpectPathOrLeftParentheses {
                pos,
                found: token.into(),
            })
        }
    }

    fn parse_operator(&mut self) -> Result<Operator, Error> {
        let next = self.lexer.next().ok_or(Error::EarlyEOF {
            pos: self.lexer.pos(),
        })?;

        next.try_into()
    }

    fn parse_rhs(&mut self) -> Result<Node, Error> {
        let (_pos, token) = self.lexer.next().ok_or(Error::EarlyEOF {
            pos: self.lexer.pos(),
        })?;

        if token == "(" {
            return self.parse_expr();
        }

        let c = token.chars().next().unwrap();
        if c.is_ascii_digit() {
            if let Ok(v) = token.parse::<f64>() {
                return Ok(Node::Float(v));
            }
        }

        Ok(Node::String(token.into()))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::print_stdout)]

    use super::*;

    #[test]
    fn parse_and_print() {
        let input = ".message contains info and (.upper gt 10 or .lower lt -1)";

        let lexer = Lexer::new(input);
        let mut parser = Parser { lexer };

        let node = parser.parse().unwrap();
        println!("{:?}", node);
    }

    #[test]
    fn parse() {
        let tests = [
            (
                ".foo lt 10.1",
                Node::BinaryExpr {
                    lhs: Box::new(Node::Path(".foo".into())),
                    op: Operator::LessThan,
                    rhs: Box::new(Node::Float(10.1)),
                },
            ),
            (
                ".foo lt 10 and .bar gt 2",
                Node::BinaryExpr {
                    lhs: Box::new(Node::BinaryExpr {
                        lhs: Box::new(Node::Path(".foo".into())),
                        op: Operator::LessThan,
                        rhs: Box::new(Node::Float(10.0)),
                    }),
                    op: Operator::And,
                    rhs: Box::new(Node::BinaryExpr {
                        lhs: Node::Path(".bar".into()).boxed(),
                        op: Operator::GreaterThan,
                        rhs: Node::Float(2.0).boxed(),
                    }),
                },
            ),
            (
                " .foo lt 10 or .bar eq 3",
                Node::BinaryExpr {
                    lhs: Node::BinaryExpr {
                        lhs: Node::Path(".foo".into()).boxed(),
                        op: Operator::LessThan,
                        rhs: Node::Float(10.0).boxed(),
                    }
                    .boxed(),
                    op: Operator::Or,
                    rhs: Node::BinaryExpr {
                        lhs: Node::Path(".bar".into()).boxed(),
                        op: Operator::Equal,
                        rhs: Node::Float(3.0).boxed(),
                    }
                    .boxed(),
                },
            ),
            (
                " .foo contains abc or .bar eq 3",
                Node::BinaryExpr {
                    lhs: Node::BinaryExpr {
                        lhs: Node::Path(".foo".into()).boxed(),
                        op: Operator::Contains,
                        rhs: Node::String("abc".into()).boxed(),
                    }
                    .boxed(),
                    op: Operator::Or,
                    rhs: Node::BinaryExpr {
                        lhs: Node::Path(".bar".into()).boxed(),
                        op: Operator::Equal,
                        rhs: Node::Float(3.0).boxed(),
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
                "input: {}\nwant: {:?}\ngot: {:?}",
                input, want, got
            )
        }
    }
}
