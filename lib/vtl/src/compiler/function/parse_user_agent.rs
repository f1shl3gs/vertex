use bytes::Bytes;
use value::{Kind, Value};

use crate::SyntaxError;
use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Function, FunctionCompileContext, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::state::TypeState;
use crate::compiler::{Expression, ExpressionError, Spanned, TypeDef};
use crate::context::Context;

pub struct ParseUserAgent;

impl Function for ParseUserAgent {
    fn identifier(&self) -> &'static str {
        "parse_user_agent"
    }

    fn parameters(&self) -> &'static [Parameter] {
        &[
            Parameter {
                name: "value",
                kind: Kind::BYTES,
                required: true,
            },
            // Parameter {
            //     name: "mode",
            //     kind: Kind::BYTES,
            //     required: false,
            // },
        ]
    }

    fn compile(
        &self,
        cx: FunctionCompileContext,
        mut arguments: ArgumentList,
    ) -> Result<FunctionCall, SyntaxError> {
        let value = arguments.get();
        // let mode = arguments.get_opt();

        Ok(FunctionCall {
            function: Box::new(ParseUserAgentFunc { value }),
            span: cx.span,
        })
    }
}

#[derive(Clone)]
struct ParseUserAgentFunc {
    value: Spanned<Expr>,
    // mode: Spanned<Expr>,
}
impl Expression for ParseUserAgentFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let value = self.value.resolve(cx)?;
        let parser = woothee::parser::Parser::new();

        match value {
            Value::Bytes(data) => match parser.parse(String::from_utf8_lossy(&data).as_ref()) {
                Some(parsed) => {
                    let mut value = Value::Object(Default::default());

                    // browser
                    value.insert("browser.family", maybe_none(parsed.name));
                    value.insert("browser.version", maybe_none(parsed.version));
                    // os
                    value.insert("os.family", maybe_none(parsed.os));
                    value.insert("os.version", maybe_none(parsed.os_version.as_ref()));
                    // device
                    value.insert("device.category", maybe_none(parsed.category));

                    Ok(value)
                }
                None => Ok(Value::Null),
            },
            _ => Err(ExpressionError::UnexpectedValue {
                msg: "value cannot be parse as user-agent".to_string(),
                span: self.value.span,
            }),
        }
    }

    fn type_def(&self, _state: &TypeState) -> TypeDef {
        TypeDef {
            fallible: true,
            kind: Kind::OBJECT,
        }
    }
}

#[inline]
fn maybe_none(value: &str) -> Value {
    if value.is_empty() || value == "UNKNOWN" {
        Value::Null
    } else {
        Value::Bytes(Bytes::from(value.to_string()))
    }
}

/*
// UserAgent
{
  "browser": {
    "family": "",
    "version": "",
    "major": "",
    "minor": "",
    "patch": ""
  },
  "os": {
    "family":  "",
    "version":  "",
    "major":  "",
    "minor":  "",
    "patch":  "",
    "patch_minor":  ""
  },
  "device": {
    "family":  "",
    "category":  "",
    "brand":  "",
    "model":  ""
  }
}
*/
