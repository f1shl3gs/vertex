use crate::Path;
use std::str::CharIndices;

#[derive(Clone)]
pub struct JitPath<'a> {
    path: &'a str,
}

impl JitPath<'_> {
    pub fn new(path: &str) -> Self {
        Self { path }
    }
}

/// This is essentially an iterator over a `JitPath`
pub struct JitLookup<'a> {
    path: &'a str,
    chars: CharIndices<'a>,
    state: JitState,
    escape_buffer: String,
    // keep track of the number of options in a coalesce to prevent size 1 coalesces
    coalesce_count: u32,
}

impl<'a> JitLookup<'a> {
    pub fn new(path: &'a str) -> Self {
        Self {
            chars: path.char_indices(),
            path,
            state: JitState::Start,
            escape_buffer: String::new(),
            coalesce_count: 0,
        }
    }
}

impl<'a> Path<'a> for JitPath<'a> {
    type Iter = JitLookup<'a>;

    fn segment_iter(&self) -> Self::Iter {
        JitLookup::new(self.path)
    }
}

enum JitState {
    EventRoot,
    Start,
    Continue,
    Dot,
    IndexStart,
    NegativeIndex(isize),
    Index(isize),
    Field(usize),
    Quote(usize),
    EscapedQuote,
    CoalesceStart,
    CoalesceField(usize),
    CoalesceFieldEnd { start: usize, end: usize },
    CoalesceEscapedFieldEnd,
    CoalesceQuote(usize),
    CoalesceEscapedQuote,
    End,
}
