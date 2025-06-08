use value::Value;

use crate::compiler::function::{ArgumentList, Function};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::parser::SyntaxError;
use crate::compiler::state::TypeState;
use crate::compiler::{Expression, ExpressionError, Kind, Span, TypeDef};
use crate::context::Context;

pub struct GetHostname;

impl Function for GetHostname {
    fn identifier(&self) -> &'static str {
        "get_hostname"
    }

    fn compile(&self, _arguments: ArgumentList) -> Result<FunctionCall, SyntaxError> {
        Ok(FunctionCall {
            function: Box::new(GetHostnameFunc),
        })
    }
}

#[derive(Clone)]
struct GetHostnameFunc;

impl Expression for GetHostnameFunc {
    fn resolve(&self, _cx: &mut Context) -> Result<Value, ExpressionError> {
        let hostname = hostname::get()
            .map_err(|err| ExpressionError::Error {
                message: err.to_string(),
                span: Span { start: 0, end: 0 },
            })?
            .into();

        Ok(hostname)
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
    fn get() {
        let hostname = hostname::get().unwrap();

        compile_and_run(
            vec![],
            GetHostname,
            TypeDef::bytes().fallible(),
            Ok(hostname.into()),
        )
    }
}
