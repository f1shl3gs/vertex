use base64::Engine;
use value::{Kind, Value};

use crate::SyntaxError;
use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Function, FunctionCompileContext, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::state::TypeState;
use crate::compiler::{Expression, ExpressionError, Spanned, TypeDef};
use crate::context::Context;

pub struct DecodeBase64;

impl Function for DecodeBase64 {
    fn identifier(&self) -> &'static str {
        "decode_base64"
    }

    fn parameters(&self) -> &'static [Parameter] {
        &[
            Parameter {
                name: "value",
                kind: Kind::BYTES,
                required: true,
            },
            Parameter {
                name: "charset",
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
        let charset = arguments.get_opt();

        Ok(FunctionCall {
            function: Box::new(DecodeBase64Func { value, charset }),
            span: cx.span,
        })
    }
}

#[derive(Clone)]
struct DecodeBase64Func {
    value: Spanned<Expr>,
    charset: Option<Spanned<Expr>>,
}

impl Expression for DecodeBase64Func {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let value = self.value.resolve(cx)?;
        let Value::Bytes(value) = value else {
            return Err(ExpressionError::UnexpectedType {
                want: Kind::BYTES,
                got: value.kind(),
                span: self.value.span,
            });
        };

        let alphabet = match &self.charset {
            Some(charset) => {
                let value = charset.resolve(cx)?;
                let Value::Bytes(value) = value else {
                    return Err(ExpressionError::UnexpectedType {
                        want: Kind::BYTES,
                        got: value.kind(),
                        span: charset.span,
                    });
                };

                match String::from_utf8_lossy(&value).as_ref() {
                    "standard" => base64::alphabet::STANDARD,
                    "url_safe" => base64::alphabet::URL_SAFE,
                    _ => {
                        return Err(ExpressionError::UnexpectedValue {
                            msg: "charset of decode_base64 is unexpected".to_string(),
                            span: charset.span,
                        });
                    }
                }
            }
            None => base64::alphabet::STANDARD,
        };

        let config = base64::engine::general_purpose::GeneralPurposeConfig::new()
            .with_decode_padding_mode(base64::engine::DecodePaddingMode::Indifferent);
        let engine = base64::engine::GeneralPurpose::new(&alphabet, config);

        engine
            .decode(value)
            .map(|value| Value::Bytes(value.into()))
            .map_err(|err| ExpressionError::Error {
                message: format!("decode base64 failed, {}", err),
                span: self.value.span,
            })
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
    fn with_defaults() {
        compile_and_run(
            vec!["c29tZSs9c3RyaW5nL3ZhbHVl".into()],
            DecodeBase64,
            TypeDef::bytes().fallible(),
            Ok(Value::Bytes("some+=string/value".into())),
        )
    }

    #[test]
    fn with_standard_charset() {
        compile_and_run(
            vec!["c29tZSs9c3RyaW5nL3ZhbHVl".into(), "standard".into()],
            DecodeBase64,
            TypeDef::bytes().fallible(),
            Ok(Value::Bytes("some+=string/value".into())),
        )
    }

    #[test]
    fn with_urlsafe_charset() {
        compile_and_run(
            vec!["c29tZSs9c3RyaW5nL3ZhbHVl".into(), "url_safe".into()],
            DecodeBase64,
            TypeDef::bytes().fallible(),
            Ok(Value::Bytes("some+=string/value".into())),
        )
    }

    #[test]
    fn empty_with_default() {
        compile_and_run(
            vec!["".into()],
            DecodeBase64,
            TypeDef::bytes().fallible(),
            Ok(Value::Bytes("".into())),
        )
    }

    #[test]
    fn empty_standard() {
        compile_and_run(
            vec!["".into(), "standard".into()],
            DecodeBase64,
            TypeDef::bytes().fallible(),
            Ok(Value::Bytes("".into())),
        )
    }

    #[test]
    fn empty_url_safe() {
        compile_and_run(
            vec!["".into(), "url_safe".into()],
            DecodeBase64,
            TypeDef::bytes().fallible(),
            Ok(Value::Bytes("".into())),
        )
    }

    #[test]
    fn no_padding() {
        compile_and_run(
            vec![
                "eyJzY2hlbWEiOiJpZ2x1OmNvbS5zbm93cGxvd2FuYWx5dGljcy5zbm93cGxvdy91bnN0cnVjdF9ldmVudC9qc29uc2NoZW1hLzEtMC0wIiwiZGF0YSI6eyJzY2hlbWEiOiJpZ2x1OmNvbS5zbm93cGxvd2FuYWx5dGljcy5zbm93cGxvdy9saW5rX2NsaWNrL2pzb25zY2hlbWEvMS0wLTEiLCJkYXRhIjp7InRhcmdldFVybCI6Imh0dHBzOi8vaWRwLWF1dGguZ2FyLmVkdWNhdGlvbi5mci9kb21haW5lR2FyP2lkRU5UPVNqQT0maWRTcmM9WVhKck9pODBPRFUyTmk5d2RERTRNREF3TVE9PSIsImVsZW1lbnRJZCI6IiIsImVsZW1lbnRDbGFzc2VzIjpbImxpbmstYnV0dG9uIiwidHJhY2tlZCJdLCJlbGVtZW50VGFyZ2V0IjoiX2JsYW5rIn19fQ".into(),
            ],
            DecodeBase64,
            TypeDef::bytes().fallible(),
            Ok(Value::Bytes(r#"{"schema":"iglu:com.snowplowanalytics.snowplow/unstruct_event/jsonschema/1-0-0","data":{"schema":"iglu:com.snowplowanalytics.snowplow/link_click/jsonschema/1-0-1","data":{"targetUrl":"https://idp-auth.gar.education.fr/domaineGar?idENT=SjA=&idSrc=YXJrOi80ODU2Ni9wdDE4MDAwMQ==","elementId":"","elementClasses":["link-button","tracked"],"elementTarget":"_blank"}}}"#.into()))
        )
    }
}
