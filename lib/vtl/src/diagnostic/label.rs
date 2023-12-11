use crate::compiler::Span;

pub struct Label {
    pub message: String,
    pub span: Span,
}

impl Label {
    pub fn new(msg: impl Into<String>, span: impl Into<Span>) -> Label {
        Label {
            message: msg.into(),
            span: span.into(),
        }
    }
}
