use std::borrow::Cow;
use std::iter::Cloned;
use std::slice::Iter;
use crate::Path;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BorrowedSegment<'a> {
    Field(Cow<'a, str>),
    Index(isize),
    CoalesceField(Cow<'a, str>),
    CoalesceEnd(Cow<'a, str>),
    Invalid
}

impl BorrowedSegment<'_> {
    pub fn field(value: &str) -> BorrowedSegment {
        BorrowedSegment::Field(Cow::Borrowed(value))
    }

    pub fn index(value: isize) -> BorrowedSegment<'static> {
        BorrowedSegment::Index(value)
    }

    pub fn is_field(&self) -> bool {
        matches!(self, BorrowedSegment::Field(_))
    }

    pub fn is_index(&self) -> bool {
        matches!(self, BorrowedSegment::Index(_))
    }

    pub fn is_invalid(&self) -> bool {
        matches!(self, BorrowedSegment::Invalid)
    }
}

impl<'a> From<&'a str> for BorrowedSegment<'a> {
    fn from(s: &'a str) -> Self {
        BorrowedSegment::field(s.as_str())
    }
}

impl<'a, 'b> Path<'a> for &'b Vec<BorrowedSegment<'a>> {
    type Iter = Cloned<Iter<'b, BorrowedSegment<'a>>>;

    fn segment_iter(&self) -> Self::Iter {
        self.as_slice().iter().cloned()
    }
}

