use crate::ast::Evaluator;
use crate::Error;
use event::{LogRecord, log::Value};
use regex::Regex;
use std::str::FromStr;

#[derive(Debug, PartialEq)]
pub enum OrderingOp {
    Equal,
    NotEqual,
    GreaterEqual,
    LessEqual,
    GreaterThan,
    LessThan,
}

impl FromStr for OrderingOp {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "eq" | "==" => Ok(OrderingOp::Equal),
            "ne" | "!=" => Ok(OrderingOp::NotEqual),
            "gt" | ">" => Ok(OrderingOp::GreaterThan),
            "ge" | ">=" => Ok(OrderingOp::GreaterEqual),
            "lt" | "<" => Ok(OrderingOp::LessThan),
            "le" | "<=" => Ok(OrderingOp::LessEqual),
            _ => Err(()),
        }
    }
}

#[derive(Debug)]
pub enum FieldOp {
    Ordering { op: OrderingOp, rhs: f64 },

    Contains(String),

    Matches(regex::bytes::Regex),
}

impl PartialEq for FieldOp {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (FieldOp::Ordering { op: ao, rhs: ar }, FieldOp::Ordering { op: bo, rhs: br }) => {
                ar == br && ao == bo
            }
            (FieldOp::Contains(a), FieldOp::Contains(b)) => a == b,
            (FieldOp::Matches(a), FieldOp::Matches(b)) => a.as_str() == b.as_str(),
            _ => false,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct FieldExpr {
    pub lhs: String,

    pub op: FieldOp,
}

impl Evaluator for FieldExpr {
    fn eval(&self, log: &LogRecord) -> Result<bool, Error> {
        match &self.op {
            FieldOp::Ordering { op, rhs } => {
                let value = log.get_field(self.lhs.as_str())
                    .ok_or(Error::MissingField)?;
                let value = match value {
                    Value::Float(f) => *f,
                    Value::Int64(i) => *i as f64,
                    _ => return Ok(false)
                };

                Ok(match op {
                    OrderingOp::Equal => value == *rhs,
                    OrderingOp::NotEqual => value != *rhs,
                    OrderingOp::GreaterThan => value > *rhs,
                    OrderingOp::GreaterEqual => value >= *rhs,
                    OrderingOp::LessThan => value < *rhs,
                    OrderingOp::LessEqual => value <= *rhs,
                })
            },
            FieldOp::Contains(s) => {
                let value = log.get_field(self.lhs.as_str())
                    .ok_or(Error::MissingField)?;


                match value {
                    Value::Bytes(b) => {
                        let result = b.windows(s.len())
                            .position(|window| window == s.as_bytes())
                            .is_some();

                        Ok(result)
                    },
                    _ => Ok(false)
                }
            },
            FieldOp::Matches(re) => {
                let value = log.get_field(self.lhs.as_str())
                    .ok_or(Error::MissingField)?;
                match value {
                    Value::Bytes(b) => {
                        Ok(re.is_match(&b))
                    }
                    _ => Ok(false)
                }
            }
        }
    }
}
