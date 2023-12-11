use crate::compiler::Kind;

/// Properties for a given expression that express the expected outcome of the
/// expression.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeDef {
    /// True, if an expression can return an error.
    ///
    /// Some expressions are infallible (e.g. some custom functions are designed to be infallible).
    pub fallible: bool,

    /// The `Kind` this definition represents.
    pub kind: Kind,
}

impl From<Kind> for TypeDef {
    fn from(kind: Kind) -> Self {
        TypeDef {
            fallible: false,
            kind,
        }
    }
}

impl TypeDef {
    #[inline]
    pub fn is_null(&self) -> bool {
        self.kind.contains(Kind::NULL)
    }

    #[inline]
    pub fn any() -> TypeDef {
        TypeDef {
            fallible: false,
            kind: Kind::ANY,
        }
    }

    #[inline]
    pub fn boolean() -> TypeDef {
        TypeDef {
            fallible: false,
            kind: Kind::BOOLEAN,
        }
    }

    #[inline]
    pub fn bytes() -> TypeDef {
        TypeDef {
            fallible: false,
            kind: Kind::BYTES,
        }
    }

    #[inline]
    pub fn float() -> TypeDef {
        TypeDef {
            fallible: false,
            kind: Kind::FLOAT,
        }
    }

    #[inline]
    pub fn integer() -> TypeDef {
        TypeDef {
            fallible: false,
            kind: Kind::INTEGER,
        }
    }

    #[inline]
    pub fn array() -> TypeDef {
        TypeDef {
            fallible: false,
            kind: Kind::ARRAY,
        }
    }

    #[inline]
    pub fn object() -> TypeDef {
        TypeDef {
            fallible: false,
            kind: Kind::OBJECT,
        }
    }

    #[inline]
    pub fn timestamp() -> TypeDef {
        TypeDef {
            fallible: false,
            kind: Kind::TIMESTAMP,
        }
    }

    #[inline]
    pub fn fallible(mut self) -> Self {
        self.fallible = true;
        self
    }
}
