use std::str::FromStr;

use event::{log::Value, LogRecord};
use value::OwnedTargetPath;

use crate::Error;

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, Debug, PartialEq)]
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

impl OrderingOp {
    pub fn as_str(&self) -> &'static str {
        match self {
            OrderingOp::Equal => "eq",
            OrderingOp::NotEqual => "ne",
            OrderingOp::GreaterEqual => "ge",
            OrderingOp::LessEqual => "le",
            OrderingOp::GreaterThan => "gt",
            OrderingOp::LessThan => "lt",
        }
    }
}

#[derive(Clone, Debug)]
pub enum FieldOp {
    Ordering { op: OrderingOp, rhs: f64 },

    Matches(regex::bytes::Regex),

    Contains(String),
    StartsWith(String),
    EndsWith(String),
}

impl PartialEq for FieldOp {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (FieldOp::Ordering { op: ao, rhs: ar }, FieldOp::Ordering { op: bo, rhs: br }) => {
                ar == br && ao == bo
            }
            (FieldOp::Matches(a), FieldOp::Matches(b)) => a.as_str() == b.as_str(),
            (FieldOp::Contains(a), FieldOp::Contains(b)) => a == b,
            (FieldOp::StartsWith(a), FieldOp::StartsWith(b)) => a == b,
            (FieldOp::EndsWith(a), FieldOp::EndsWith(b)) => a == b,
            _ => false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FieldExpr {
    pub lhs: OwnedTargetPath,

    pub op: FieldOp,
}

impl FieldExpr {
    pub fn eval(&self, log: &LogRecord) -> Result<bool, Error> {
        let value = log
            .get(&self.lhs)
            .ok_or_else(|| Error::MissingField(self.lhs.to_string()))?;

        let result = match &self.op {
            FieldOp::Ordering { op, rhs } => {
                let value = match value {
                    Value::Float(f) => *f,
                    Value::Integer(i) => *i as f64,
                    _ => return Ok(false),
                };

                match op {
                    OrderingOp::Equal => value == *rhs,
                    OrderingOp::NotEqual => value != *rhs,
                    OrderingOp::GreaterThan => value > *rhs,
                    OrderingOp::GreaterEqual => value >= *rhs,
                    OrderingOp::LessThan => value < *rhs,
                    OrderingOp::LessEqual => value <= *rhs,
                }
            }
            FieldOp::Matches(re) => match value {
                Value::Bytes(b) => re.is_match(b),
                _ => false,
            },
            FieldOp::Contains(s) => match value {
                Value::Bytes(b) => b.windows(s.len()).any(|window| window == s.as_bytes()),
                _ => false,
            },
            FieldOp::StartsWith(s) => value.to_string_lossy().starts_with(s),
            FieldOp::EndsWith(s) => value.to_string_lossy().ends_with(s),
        };

        Ok(result)
    }
}
