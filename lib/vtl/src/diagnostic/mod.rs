mod label;

use std::error::Error;
use std::fmt::Write;

use crate::compiler::Span;
pub use label::Label;

pub trait DiagnosticMessage: Error {
    fn title(&self) -> String {
        self.to_string()
    }

    fn labels(&self) -> Vec<Label>;
}

pub struct Diagnostic<'a> {
    source: &'a str,
}

impl Diagnostic<'_> {
    pub fn new(source: &str) -> Diagnostic<'_> {
        Diagnostic { source }
    }

    pub fn snippets<T: DiagnosticMessage>(&self, msg: T) -> String {
        let mut buf = format!("Error: {}", msg.title());
        let labels = msg.labels();
        let total = labels.len();
        for (index, label) in labels.into_iter().enumerate() {
            self.render_label(&mut buf, label);
            if total > 1 && index + 1 != total {
                let _ = buf.write_str("\n...");
            }
        }

        buf
    }

    fn render_label(&self, buf: &mut String, label: Label) {
        let Label { message, span } = label;
        let Span { start, end } = span;

        let mut consumed = 0;
        let mut offset = 0;
        let mut lines = vec![];
        let mut lo_width = 0;
        for (lo, line) in self.source.lines().enumerate() {
            if consumed + line.len() < start {
                consumed += line.len() + 1;
                continue;
            }

            if consumed > end && lines.len() > 3 {
                break;
            }

            lo_width = lo_width.max((lo + 1).to_string().len());
            lines.push((lo + 1, line));
            if lines.len() == 1 {
                offset = start - consumed;
            }
            consumed += line.len() + 1;
        }

        for (index, (lo, line)) in lines.iter().enumerate() {
            let mut lo_part = lo.to_string();
            if lo_part.len() < lo_width {
                lo_part.push(' ');
            }

            buf.write_fmt(format_args!("\n{lo_part} | {line}")).unwrap();
            if index == 0 {
                let padding = " ".repeat(lo_width + 3).to_string();
                buf.write_fmt(format_args!(
                    "\n{padding}{}{} {}",
                    " ".repeat(offset),
                    "-".repeat(end - start),
                    message
                ))
                .unwrap();
            }
        }
    }
}
