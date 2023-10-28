use std::fmt::Formatter;

use serde::de::{Error, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::ast::{CombiningOp, FieldOp};
use crate::Expression;

impl<'de> Deserialize<'de> for Expression {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ExpressionVisitor;

        impl<'de> Visitor<'de> for ExpressionVisitor {
            type Value = Expression;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("expect string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Expression::parse(v).map_err(|err| Error::custom(err))
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: Error,
            {
                self.visit_str(&v)
            }
        }

        deserializer.deserialize_str(ExpressionVisitor)
    }
}

impl Serialize for Expression {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut buf = String::new();
        walk(self, &mut buf);

        serializer.serialize_str(&buf)
    }
}

fn walk(expr: &Expression, buf: &mut String) {
    match expr {
        Expression::Binary { op, lhs, rhs } => {
            walk(lhs, buf);
            match op {
                CombiningOp::And => buf.push_str(" and "),
                CombiningOp::Or => buf.push_str(" or "),
            }
            walk(rhs, buf);
        }
        Expression::Field(field) => {
            buf.push_str(&field.lhs.to_string());
            buf.push(' ');

            match &field.op {
                FieldOp::Ordering { op, rhs } => {
                    buf.push_str(op.as_str());
                    buf.push(' ');
                    buf.push_str(&rhs.to_string())
                }
                FieldOp::Contains(s) => {
                    buf.push_str("contains ");
                    buf.push_str(s)
                }
                FieldOp::Matches(re) => {
                    buf.push_str("match ");
                    buf.push_str(re.as_str())
                }

                FieldOp::StartsWith(s) => {
                    buf.push_str("starts_with ");
                    buf.push_str(s);
                }
                FieldOp::EndsWith(s) => {
                    buf.push_str("ends_with ");
                    buf.push_str(s);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize() {
        let tests = [
            ".foo contains abc",
            ".foo contains abc and .bar gt 10",
            ".message contains info and .upper gt 10 or .lower lt -1",
        ];

        for input in tests {
            let expr = Expression::parse(input).unwrap();
            let mut buf = String::new();

            walk(&expr, &mut buf);

            assert_eq!(input, buf)
        }
    }
}
