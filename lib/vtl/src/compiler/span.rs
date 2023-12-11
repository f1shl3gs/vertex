use std::ops::Deref;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn merge(mut self, other: Span) -> Self {
        self.start = self.start.min(other.start);
        self.end = self.end.max(other.end);
        self
    }

    #[inline]
    pub fn with<T>(self, node: T) -> Spanned<T> {
        Spanned::new(node, self)
    }
}

impl From<&Span> for Span {
    fn from(span: &Span) -> Self {
        *span
    }
}

pub struct Spanned<T> {
    pub node: T,
    pub span: Span,
}

impl<T> Deref for Spanned<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.node
    }
}

impl<T> Spanned<T> {
    #[inline]
    pub fn new(node: T, span: Span) -> Spanned<T> {
        Spanned { span, node }
    }
}
