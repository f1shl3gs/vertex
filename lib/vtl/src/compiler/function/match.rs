use regex::Regex;
use value::Value;

use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Function, FunctionCompileContext, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::state::TypeState;
use crate::compiler::{Expression, ExpressionError, Kind, Spanned, TypeDef, ValueKind};
use crate::context::Context;
use crate::SyntaxError;

pub struct Match;

impl Function for Match {
    fn identifier(&self) -> &'static str {
        "match"
    }

    fn parameters(&self) -> &'static [Parameter] {
        &[
            Parameter {
                name: "value",
                kind: Kind::BYTES,
                required: true,
            },
            Parameter {
                name: "pattern",
                kind: Kind::BYTES,
                required: true,
            },
        ]
    }

    fn compile(
        &self,
        cx: FunctionCompileContext,
        mut arguments: ArgumentList,
    ) -> Result<FunctionCall, SyntaxError> {
        let value = arguments.get();
        let pattern = arguments.get_string()?;
        let pattern = Regex::new(pattern.as_str()).map_err(|err| SyntaxError::InvalidValue {
            err: err.to_string(),
            want: "valid regex pattern".to_string(),
            got: pattern.node,
            span: pattern.span,
        })?;

        Ok(FunctionCall {
            function: Box::new(MatchFunc { value, pattern }),
            span: cx.span,
        })
    }
}

#[derive(Clone)]
struct MatchFunc {
    value: Spanned<Expr>,
    pattern: Regex,
}

impl Expression for MatchFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        match self.value.resolve(cx)? {
            Value::Bytes(b) => {
                let text = String::from_utf8_lossy(&b);
                Ok(self.pattern.is_match(text.as_ref()).into())
            }
            value => Err(ExpressionError::UnexpectedType {
                want: Kind::BYTES,
                got: value.kind(),
                span: self.value.span,
            }),
        }
    }

    fn type_def(&self, _state: &TypeState) -> TypeDef {
        TypeDef {
            fallible: false,
            kind: Kind::BOOLEAN,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::function::compile_and_run;

    #[test]
    fn yes() {
        compile_and_run(
            vec!["foobar".into(), r#"foo.*"#.into()],
            Match,
            TypeDef::boolean(),
            Ok(true.into()),
        )
    }

    #[test]
    fn yes_with_escape() {
        compile_and_run(
            vec!["foobar".into(), Expr::String(r"\w+".into())],
            Match,
            TypeDef::boolean(),
            Ok(true.into()),
        )
    }
}
