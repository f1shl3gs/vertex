use value::{Kind, Value};

use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Function, FunctionCompileContext, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::state::TypeState;
use crate::compiler::{Expression, ExpressionError, Spanned, TypeDef};
use crate::context::Context;
use crate::SyntaxError;

pub struct KebabCase;

impl Function for KebabCase {
    fn identifier(&self) -> &'static str {
        "kebabcase"
    }

    fn parameters(&self) -> &'static [Parameter] {
        &[Parameter {
            name: "value",
            kind: Kind::BYTES,
            required: true,
        }]
    }

    fn compile(
        &self,
        cx: FunctionCompileContext,
        mut arguments: ArgumentList,
    ) -> Result<FunctionCall, SyntaxError> {
        let value = arguments.get();

        Ok(FunctionCall {
            function: Box::new(KebabCaseFunc { value }),
            span: cx.span,
        })
    }
}

#[derive(Clone)]
struct KebabCaseFunc {
    value: Spanned<Expr>,
}

impl Expression for KebabCaseFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let value = self.value.resolve(cx)?;
        let Value::Bytes(value) = value else {
            return Err(ExpressionError::UnexpectedType {
                want: Kind::BYTES,
                got: value.kind(),
                span: self.value.span,
            });
        };

        let cast = kebabcase(String::from_utf8_lossy(&value).as_ref());

        Ok(cast.into())
    }

    fn type_def(&self, _state: &TypeState) -> TypeDef {
        TypeDef::bytes()
    }
}

// copied from serde_derive
fn kebabcase(input: &str) -> String {
    let mut output = String::new();
    for (i, ch) in input.char_indices() {
        if i > 0 && ch.is_uppercase() {
            output.push('-');
        }
        output.push(ch.to_ascii_lowercase());
    }

    output.replace('_', "-")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::function::compile_and_run;

    #[test]
    fn casing() {
        for (input, want) in [
            ("Outcome", "outcome"),
            ("VeryTasty", "very-tasty"),
            ("A", "a"),
            ("Z42", "z42"),
            ("outcome", "outcome"),
            ("very_tasty", "very-tasty"),
            ("a", "a"),
            ("z42", "z42"),
        ] {
            let got = kebabcase(input);
            assert_eq!(got, want, "want: {}, got: {}\ninput: {}", want, got, input);
        }
    }

    #[test]
    fn simple() {
        compile_and_run(
            vec!["input_string".into()],
            KebabCase,
            TypeDef::bytes(),
            Ok(Value::Bytes("input-string".into())),
        )
    }
}
