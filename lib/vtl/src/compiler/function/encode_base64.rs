use base64::Engine;
use value::{Kind, Value};

use crate::SyntaxError;
use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Function, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::state::TypeState;
use crate::compiler::{Expression, ExpressionError, Spanned, TypeDef};
use crate::context::Context;

pub struct EncodeBase64;

impl Function for EncodeBase64 {
    fn identifier(&self) -> &'static str {
        "encode_base64"
    }

    fn parameters(&self) -> &'static [Parameter] {
        &[
            Parameter {
                name: "value",
                kind: Kind::BYTES,
                required: true,
            },
            Parameter {
                name: "padding",
                kind: Kind::BOOLEAN,
                required: false,
            },
            Parameter {
                name: "charset",
                kind: Kind::BYTES,
                required: false,
            },
        ]
    }

    fn compile(&self, mut arguments: ArgumentList) -> Result<FunctionCall, SyntaxError> {
        let value = arguments.get();
        let padding = arguments.get_opt();
        let charset = arguments.get_opt();

        Ok(FunctionCall {
            function: Box::new(EncodeBase64Func {
                value,
                padding,
                charset,
            }),
        })
    }
}

#[derive(Clone)]
struct EncodeBase64Func {
    value: Spanned<Expr>,
    padding: Option<Spanned<Expr>>,
    charset: Option<Spanned<Expr>>,
}

impl Expression for EncodeBase64Func {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let value = self.value.resolve(cx)?;
        let Value::Bytes(value) = value else {
            return Err(ExpressionError::UnexpectedType {
                want: Kind::BYTES,
                got: value.kind(),
                span: self.value.span,
            });
        };

        let padding = match &self.padding {
            Some(spanned) => {
                let padding = spanned.resolve(cx)?;
                let Value::Boolean(padding) = padding else {
                    return Err(ExpressionError::UnexpectedType {
                        want: Kind::BOOLEAN,
                        got: padding.kind(),
                        span: spanned.span,
                    });
                };

                padding
            }
            None => true,
        };

        let charset = match &self.charset {
            Some(spanned) => {
                let charset = spanned.resolve(cx)?;
                let Value::Bytes(value) = charset else {
                    return Err(ExpressionError::UnexpectedType {
                        want: Kind::BYTES,
                        got: charset.kind(),
                        span: spanned.span,
                    });
                };

                let value = String::from_utf8_lossy(&value);
                match value.as_ref() {
                    "standard" => base64::alphabet::STANDARD,
                    "url_safe" => base64::alphabet::URL_SAFE,
                    value => {
                        return Err(ExpressionError::UnexpectedValue {
                            msg: format!(
                                "base64's charset should be \"standard\" or \"url_safe\", but found \"{value}\"",
                            ),
                            span: spanned.span,
                        });
                    }
                }
            }
            None => base64::alphabet::STANDARD,
        };

        let engine = base64::engine::GeneralPurpose::new(
            &charset,
            base64::engine::general_purpose::GeneralPurposeConfig::default()
                .with_encode_padding(padding),
        );

        Ok(engine.encode(&value).into())
    }

    fn type_def(&self, _state: &TypeState) -> TypeDef {
        TypeDef {
            fallible: false,
            kind: Kind::BYTES,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::Span;
    use crate::compiler::function::compile_and_run;

    #[test]
    fn with_default() {
        compile_and_run(
            vec!["some+=string/value".into()],
            EncodeBase64,
            TypeDef::bytes(),
            Ok(Value::Bytes("c29tZSs9c3RyaW5nL3ZhbHVl".into())),
        )
    }

    #[test]
    fn with_padding() {
        compile_and_run(
            vec!["some+=string/value".into(), true.into()],
            EncodeBase64,
            TypeDef::bytes(),
            Ok(Value::Bytes("c29tZSs9c3RyaW5nL3ZhbHVl".into())),
        )
    }

    #[test]
    fn with_padding_standard() {
        compile_and_run(
            vec!["some+=string/value".into(), true.into(), "standard".into()],
            EncodeBase64,
            TypeDef::bytes(),
            Ok(Value::Bytes("c29tZSs9c3RyaW5nL3ZhbHVl".into())),
        )
    }

    #[test]
    fn with_padding_url_safe() {
        compile_and_run(
            vec!["some+=string/value".into(), true.into(), "url_safe".into()],
            EncodeBase64,
            TypeDef::bytes(),
            Ok(Value::Bytes("c29tZSs9c3RyaW5nL3ZhbHVl".into())),
        )
    }

    #[test]
    fn empty_with_default() {
        compile_and_run(
            vec!["".into(), true.into()],
            EncodeBase64,
            TypeDef::bytes(),
            Ok(Value::Bytes("".into())),
        )
    }

    #[test]
    fn empty_with_standard() {
        compile_and_run(
            vec!["".into(), true.into(), "standard".into()],
            EncodeBase64,
            TypeDef::bytes(),
            Ok(Value::Bytes("".into())),
        )
    }

    #[test]
    fn empty_with_url_safe() {
        compile_and_run(
            vec!["".into(), true.into(), "url_safe".into()],
            EncodeBase64,
            TypeDef::bytes(),
            Ok(Value::Bytes("".into())),
        )
    }

    #[test]
    fn invalid_charset() {
        compile_and_run(
            vec!["some string value".into(), true.into(), "blah".into()],
            EncodeBase64,
            TypeDef::bytes(),
            Err(ExpressionError::UnexpectedValue {
                msg: "base64's charset should be \"standard\" or \"url_safe\", but found \"blah\""
                    .to_string(),
                span: Span::empty(),
            }),
        )
    }
}
