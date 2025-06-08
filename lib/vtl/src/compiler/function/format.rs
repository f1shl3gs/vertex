use bytes::BytesMut;
use value::Value;

use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Function, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::parser::SyntaxError;
use crate::compiler::state::TypeState;
use crate::compiler::template::Segment;
use crate::compiler::{Expression, ExpressionError, Kind, Spanned, Template, TypeDef};
use crate::context::Context;

pub struct Format;

impl Function for Format {
    fn identifier(&self) -> &'static str {
        "format"
    }

    fn parameters(&self) -> &'static [Parameter] {
        &[Parameter {
            name: "format",
            kind: Kind::BYTES,
            required: true,
        }]
    }

    fn compile(&self, mut arguments: ArgumentList) -> Result<FunctionCall, SyntaxError> {
        let format = arguments.get_string()?;
        let template = Template::parse(&format.node)
            .map_err(|_err| SyntaxError::InvalidTemplate { span: format.span })?;

        let arguments = arguments.inner();
        if arguments.len() != template.placeholders() {
            let mut span = format.span;
            if let Some(last) = arguments.last() {
                span.end = last.span.end;
            }

            return Err(SyntaxError::FunctionArgumentsArityMismatch {
                function: self.identifier(),
                takes: 1 + template.placeholders(),
                got: 1 + arguments.len(),
                span,
            });
        }

        Ok(FunctionCall {
            function: Box::new(FormatFunc {
                template,
                arguments,
            }),
        })
    }
}

#[derive(Clone)]
struct FormatFunc {
    template: Template,
    arguments: Vec<Spanned<Expr>>,
}

impl Expression for FormatFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let mut buf = BytesMut::new();
        let segments = self.template.segments();
        let mut arguments = self.arguments.iter();

        for segment in segments {
            match segment {
                Segment::Literal(s) => buf.extend_from_slice(s.as_bytes()),
                Segment::Placeholder => {
                    let expr = arguments.next().expect("checked at compile-time");
                    let value = expr.resolve(cx)?;

                    buf.extend_from_slice(value.to_string_lossy().as_bytes());
                }
            }
        }

        Ok(Value::Bytes(buf.freeze()))
    }

    fn type_def(&self, state: &TypeState) -> TypeDef {
        let fallible = self
            .arguments
            .iter()
            .any(|argument| argument.type_def(state).fallible);

        TypeDef {
            fallible,
            kind: Kind::BYTES,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::function::compile_and_run;

    #[test]
    fn empty() {
        compile_and_run(
            vec!["foo".into()],
            Format,
            TypeDef::bytes(),
            Ok("foo".into()),
        )
    }

    #[test]
    fn empty_string() {
        compile_and_run(
            vec!["foo{}bar".into(), "".into()],
            Format,
            TypeDef::bytes(),
            Ok("foobar".into()),
        )
    }

    #[test]
    fn integer() {
        compile_and_run(
            vec!["foo{}bar".into(), 1.into()],
            Format,
            TypeDef::bytes(),
            Ok("foo1bar".into()),
        )
    }

    #[test]
    fn array() {
        compile_and_run(
            vec!["foo{}bar".into(), vec![1.into(), 2.into()].into()],
            Format,
            TypeDef::bytes(),
            Ok("foo[1,2]bar".into()),
        )
    }
}
