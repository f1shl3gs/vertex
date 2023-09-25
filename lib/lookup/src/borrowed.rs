use std::borrow::Cow;
use std::iter::Cloned;
use std::slice::Iter;

use crate::{OwnedSegment, Path};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BorrowedSegment<'a> {
    Field(Cow<'a, str>),
    Index(isize),
    CoalesceField(Cow<'a, str>),
    CoalesceEnd(Cow<'a, str>),
    Invalid,
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
        BorrowedSegment::field(s)
    }
}

impl<'a> From<&'a String> for BorrowedSegment<'a> {
    fn from(field: &'a String) -> Self {
        BorrowedSegment::field(field.as_str())
    }
}

impl From<isize> for BorrowedSegment<'_> {
    fn from(i: isize) -> Self {
        BorrowedSegment::Index(i)
    }
}

impl<'a, 'b: 'a> From<&'b OwnedSegment> for BorrowedSegment<'a> {
    fn from(segment: &'b OwnedSegment) -> Self {
        match segment {
            OwnedSegment::Field(f) => BorrowedSegment::Field(f.as_str().into()),
            OwnedSegment::Index(i) => BorrowedSegment::Index(*i),
            OwnedSegment::Invalid => BorrowedSegment::Invalid,
            OwnedSegment::CoalesceField(f) => BorrowedSegment::CoalesceField(f.as_str().into()),
            OwnedSegment::CoalesceEnd(f) => BorrowedSegment::CoalesceEnd(f.as_str().into()),
        }
    }
}

impl<'a, 'b> Path<'a> for &'b Vec<BorrowedSegment<'a>> {
    type Iter = Cloned<Iter<'b, BorrowedSegment<'a>>>;

    fn segment_iter(&self) -> Self::Iter {
        self.as_slice().iter().cloned()
    }
}

impl<'a, 'b> Path<'a> for &'b [BorrowedSegment<'a>] {
    type Iter = Cloned<Iter<'b, BorrowedSegment<'a>>>;

    fn segment_iter(&self) -> Self::Iter {
        self.iter().cloned()
    }
}

impl<'a, 'b, const A: usize> Path<'a> for &'b [BorrowedSegment<'a>; A] {
    type Iter = Cloned<Iter<'b, BorrowedSegment<'a>>>;

    fn segment_iter(&self) -> Self::Iter {
        self.iter().cloned()
    }
}
