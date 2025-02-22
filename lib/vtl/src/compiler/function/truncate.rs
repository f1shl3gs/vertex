use value::{Kind, Value};

use crate::SyntaxError;
use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Function, FunctionCompileContext, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::state::TypeState;
use crate::compiler::{Expression, ExpressionError, Spanned, TypeDef};
use crate::context::Context;

pub struct Truncate;

impl Function for Truncate {
    fn identifier(&self) -> &'static str {
        "truncate"
    }

    fn parameters(&self) -> &'static [Parameter] {
        &[
            Parameter {
                name: "value",
                kind: Kind::BYTES,
                required: true,
            },
            Parameter {
                name: "limit",
                kind: Kind::INTEGER,
                required: true,
            },
            Parameter {
                name: "suffix",
                kind: Kind::BYTES,
                required: false,
            },
        ]
    }

    fn compile(
        &self,
        cx: FunctionCompileContext,
        mut arguments: ArgumentList,
    ) -> Result<FunctionCall, SyntaxError> {
        let value = arguments.get();
        let limit = arguments.get();
        let suffix = arguments.get_opt();

        Ok(FunctionCall {
            function: Box::new(TruncateFunc {
                value,
                limit,
                suffix,
            }),
            span: cx.span,
        })
    }
}

#[derive(Clone)]
struct TruncateFunc {
    value: Spanned<Expr>,
    limit: Spanned<Expr>,
    suffix: Option<Spanned<Expr>>,
}

impl Expression for TruncateFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let value = self.value.resolve(cx)?;
        let Value::Bytes(value) = value else {
            return Err(ExpressionError::UnexpectedType {
                want: Kind::BYTES,
                got: value.kind(),
                span: self.value.span,
            });
        };

        let limit = self.limit.resolve(cx)?;
        let Value::Integer(limit) = limit else {
            return Err(ExpressionError::UnexpectedType {
                want: Kind::INTEGER,
                got: limit.kind(),
                span: self.limit.span,
            });
        };

        let mut value = String::from_utf8_lossy(&value).to_string();
        let limit = if limit < 0 { 0 } else { limit as usize };
        let pos = if let Some((pos, char)) = value.char_indices().take(limit).last() {
            // char_indices gives us the starting position of the character at limit,
            // we want the end position.
            pos + char.len_utf8()
        } else {
            // we have an empty string
            0
        };

        if value.len() > pos {
            value.truncate(pos);
        }

        if let Some(suffix) = self.suffix.as_ref() {
            let suffix = suffix.resolve(cx)?;
            value.push_str(suffix.to_string_lossy().as_ref());
        }

        Ok(value.into())
    }

    fn type_def(&self, _state: &TypeState) -> TypeDef {
        TypeDef {
            fallible: true,
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
            vec!["hello".into(), 0.into()],
            Truncate,
            TypeDef::bytes().fallible(),
            Ok(Value::Bytes("".into())),
        )
    }

    #[test]
    fn empty_with_suffix() {
        compile_and_run(
            vec!["hello".into(), 0.into(), "...".into()],
            Truncate,
            TypeDef::bytes().fallible(),
            Ok(Value::Bytes("...".into())),
        )
    }

    #[test]
    fn complete() {
        compile_and_run(
            vec!["hello".into(), 10.into()],
            Truncate,
            TypeDef::bytes().fallible(),
            Ok(Value::Bytes("hello".into())),
        );
    }

    #[test]
    fn complete_with_suffix() {
        compile_and_run(
            vec!["hello".into(), 10.into(), "...".into()],
            Truncate,
            TypeDef::bytes().fallible(),
            Ok(Value::Bytes("hello...".into())),
        )
    }

    #[test]
    fn big() {
        compile_and_run(
            vec!["abcdefghijklmnopqrstuvwxyz".into(), 10.into()],
            Truncate,
            TypeDef::bytes().fallible(),
            Ok(Value::Bytes("abcdefghij".into())),
        )
    }

    #[test]
    fn big_with_suffix() {
        compile_and_run(
            vec!["abcdefghijklmnopqrstuvwxyz".into(), 10.into(), "...".into()],
            Truncate,
            TypeDef::bytes().fallible(),
            Ok(Value::Bytes("abcdefghij...".into())),
        )
    }

    #[test]
    fn unicode() {
        compile_and_run(
            vec!["♔♕♖♗♘♙♚♛♜♝♞♟".into(), 6.into(), "...".into()],
            Truncate,
            TypeDef::bytes().fallible(),
            Ok(Value::Bytes("♔♕♖♗♘♙...".into())),
        )
    }
}
