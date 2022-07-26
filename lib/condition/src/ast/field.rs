use regex::Regex;

#[derive(Debug)]
enum OrderingOp {
    Equal,
    NotEqual,
    GreaterThanEqual,
    LessThanEqual,
    GreaterThan,
    LessThan
}

#[derive(Debug)]
pub enum FieldOp {
    Ordering {
        op: OrderingOp,
        rhs: f64,
    },

    Contains(String),

    Matches(Regex),
}

#[derive(Debug)]
pub struct FieldExpr {
    lhs: String,

    op: FieldOp,
}