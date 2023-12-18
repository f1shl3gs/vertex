use std::fmt::Write;

use value::Value;

use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Function, FunctionCompileContext};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::parser::SyntaxError;
use crate::compiler::{Expression, ExpressionError, Kind, Spanned, TypeDef};
use crate::context::Context;

pub struct Log;

impl Function for Log {
    fn identifier(&self) -> &'static str {
        "log"
    }

    fn compile(
        &self,
        cx: FunctionCompileContext,
        mut arguments: ArgumentList,
    ) -> Result<FunctionCall, SyntaxError> {
        let format = arguments.get_string()?.node;
        let arguments = arguments.inner();

        Ok(FunctionCall {
            function: Box::new(LogFunc { format, arguments }),
            span: cx.span,
        })
    }
}

#[derive(Clone)]
struct LogFunc {
    format: String,
    arguments: Vec<Spanned<Expr>>,
}

impl Expression for LogFunc {
    #[allow(clippy::print_stdout)]
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let mut buf = String::new();
        for argument in &self.arguments {
            let value = argument.resolve(cx)?;
            buf.write_char(' ').unwrap();
            buf += value.to_string_lossy().as_ref();
        }

        println!("{}{}", self.format, buf);

        Ok(Value::Null)
    }

    #[inline]
    fn type_def(&self) -> TypeDef {
        TypeDef {
            fallible: false,
            kind: Kind::UNDEFINED,
        }
    }
}
