use std::fmt::{Display, Formatter};
use std::ops::BitOr;

use super::Value;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Kind(u16);

impl BitOr for Kind {
    type Output = Kind;

    fn bitor(self, rhs: Self) -> Self::Output {
        Kind(self.0 | rhs.0)
    }
}

impl Display for Kind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if *self == Kind::ANY {
            return f.write_str("any");
        }

        let mut kinds = vec![];

        if self.contains(Kind::BYTES) {
            kinds.push("bytes");
        }

        if self.contains(Kind::INTEGER) {
            kinds.push("integer")
        }

        if self.contains(Kind::FLOAT) {
            kinds.push("float");
        }

        if self.contains(Kind::BOOLEAN) {
            kinds.push("boolean");
        }

        if self.contains(Kind::OBJECT) {
            kinds.push("object");
        }

        if self.contains(Kind::ARRAY) {
            kinds.push("array");
        }

        if self.contains(Kind::TIMESTAMP) {
            kinds.push("timestamp");
        }

        if self.contains(Kind::NULL) {
            kinds.push("null")
        }

        if self.contains(Kind::UNDEFINED) {
            kinds.push("undefined")
        }

        f.write_str(&kinds.join(", "))
    }
}

impl Kind {
    pub const BYTES: Kind = Kind(1 << 1);
    pub const INTEGER: Kind = Kind(1 << 2);
    pub const FLOAT: Kind = Kind(1 << 3);
    pub const BOOLEAN: Kind = Kind(1 << 4);
    pub const OBJECT: Kind = Kind(1 << 5);
    pub const ARRAY: Kind = Kind(1 << 6);
    pub const TIMESTAMP: Kind = Kind(1 << 7);
    pub const NULL: Kind = Kind(1 << 8);
    pub const UNDEFINED: Kind = Kind(1 << 9);

    pub const NUMERIC: Kind = Kind(1 << 2 | 1 << 3);
    pub const ARRAY_OR_OBJECT: Kind = Kind(1 << 5 | 1 << 6);
    pub const ARRAY_BYTES_OBJECT: Kind = Kind(1 << 1 | 1 << 5 | 1 << 6);
    pub const ARRAY_OR_BYTES: Kind = Kind(1 << 1 | 1 << 6);
    pub const BYTES_OR_INTEGER: Kind = Kind(1 << 1 | 1 << 2);

    pub const REDACTABLE: Kind = Kind(1 << 1 | 1 << 5 | 1 << 6);

    pub const ANY: Kind =
        Kind(1 << 1 | 1 << 2 | 1 << 3 | 1 << 4 | 1 << 5 | 1 << 6 | 1 << 7 | 1 << 8 | 1 << 9);

    #[inline]
    pub const fn new(kind: u16) -> Kind {
        Kind(kind)
    }

    #[inline]
    pub fn inner(&self) -> u16 {
        self.0
    }

    #[inline]
    pub const fn or(self, other: Kind) -> Kind {
        Kind(self.0 | other.0)
    }

    #[inline]
    pub fn contains(&self, other: Kind) -> bool {
        self.0 & other.0 == other.0
    }

    #[inline]
    pub fn intersects(&self, other: Kind) -> bool {
        self.0 & other.0 > 0
    }
}

impl Value {
    pub fn kind(&self) -> Kind {
        match self {
            Value::Bytes(_) => Kind::BYTES,
            Value::Float(_) => Kind::FLOAT,
            Value::Integer(_) => Kind::INTEGER,
            Value::Boolean(_) => Kind::BOOLEAN,
            Value::Timestamp(_) => Kind::TIMESTAMP,
            Value::Object(_) => Kind::OBJECT,
            Value::Array(_) => Kind::ARRAY,
            Value::Null => Kind::NULL,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains_any() {
        let any = Kind::ANY;

        assert!(any.contains(Kind::BYTES));
        assert!(any.contains(Kind::FLOAT));
        assert!(any.contains(Kind::INTEGER));
        assert!(any.contains(Kind::BOOLEAN));
        assert!(any.contains(Kind::TIMESTAMP));
        assert!(any.contains(Kind::OBJECT));
        assert!(any.contains(Kind::ARRAY));
        assert!(any.contains(Kind::NULL));
    }

    #[test]
    fn contains_some() {
        let kind = Kind::BYTES | Kind::TIMESTAMP;

        assert!(kind.contains(Kind::BYTES | Kind::TIMESTAMP));

        assert!(kind.contains(Kind::BYTES));
        assert!(!kind.contains(Kind::FLOAT));
        assert!(!kind.contains(Kind::INTEGER));
        assert!(!kind.contains(Kind::BOOLEAN));
        assert!(kind.contains(Kind::TIMESTAMP));
        assert!(!kind.contains(Kind::OBJECT));
        assert!(!kind.contains(Kind::ARRAY));
        assert!(!kind.contains(Kind::NULL));
    }

    #[test]
    fn contains_multiple() {
        let kind = Kind::BYTES | Kind::FLOAT | Kind::INTEGER;
        assert!(kind.contains(Kind::FLOAT | Kind::INTEGER))
    }

    #[test]
    fn intersects() {
        let lhs = Kind::BYTES | Kind::FLOAT;
        let rhs = Kind::FLOAT;

        assert!(lhs.intersects(rhs))
    }
}
