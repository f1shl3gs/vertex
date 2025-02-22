use value::Value;

use super::{Function, FunctionCompileContext};
use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::parser::SyntaxError;
use crate::compiler::state::TypeState;
use crate::compiler::{Expression, ExpressionError, Kind, Spanned, TypeDef};
use crate::context::Context;

pub struct ToBool;

impl Function for ToBool {
    fn identifier(&self) -> &'static str {
        "to_bool"
    }

    fn parameters(&self) -> &'static [Parameter] {
        &[Parameter {
            name: "value",
            kind: Kind::ANY,
            required: true,
        }]
    }

    fn compile(
        &self,
        cx: FunctionCompileContext,
        mut arguments: ArgumentList,
    ) -> Result<FunctionCall, SyntaxError> {
        let expr = arguments.get();

        Ok(FunctionCall {
            function: Box::new(ToBoolFunc { expr }),
            span: cx.span,
        })
    }
}

#[derive(Clone)]
struct ToBoolFunc {
    expr: Spanned<Expr>,
}

impl Expression for ToBoolFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let value = match self.expr.resolve(cx)? {
            Value::Boolean(b) => b,
            Value::Integer(i) => i != 0,
            Value::Float(f) => f != 0.0,
            Value::Null => false,
            Value::Bytes(b) => match String::from_utf8_lossy(b.as_ref()).as_ref() {
                "true" | "yes" | "on" => true,
                "false" | "no" | "off" => false,
                s => {
                    if let Ok(n) = s.parse::<isize>() {
                        n != 0
                    } else {
                        match s.to_lowercase().as_str() {
                            "true" | "yes" | "on" => true,
                            "false" | "no" | "off" => false,
                            _ => {
                                return Err(ExpressionError::Error {
                                    message: "convert string value to bool".to_string(),
                                    span: self.expr.span,
                                });
                            }
                        }
                    }
                }
            },
            value => {
                return Err(ExpressionError::UnexpectedType {
                    want: Kind::BOOLEAN | Kind::INTEGER | Kind::FLOAT | Kind::NULL | Kind::BYTES,
                    got: value.kind(),
                    span: self.expr.span,
                });
            }
        };

        Ok(value.into())
    }

    fn type_def(&self, state: &TypeState) -> TypeDef {
        let kind = self.expr.type_def(state).kind;
        let fallible = kind.contains(Kind::BYTES)
            || kind.contains(Kind::TIMESTAMP)
            || kind.contains(Kind::ARRAY)
            || kind.contains(Kind::OBJECT);

        TypeDef {
            fallible,
            kind: Kind::BOOLEAN,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::function::compile_and_run;

    #[test]
    fn bool() {
        compile_and_run(
            vec![true.into()],
            ToBool,
            TypeDef::boolean(),
            Ok(true.into()),
        );
        compile_and_run(
            vec![false.into()],
            ToBool,
            TypeDef::boolean(),
            Ok(false.into()),
        );
    }

    #[test]
    fn integer() {
        compile_and_run(vec![1.into()], ToBool, TypeDef::boolean(), Ok(true.into()));
        compile_and_run(vec![0.into()], ToBool, TypeDef::boolean(), Ok(false.into()));
    }

    #[test]
    fn float() {
        compile_and_run(
            vec![1.2.into()],
            ToBool,
            TypeDef::boolean(),
            Ok(true.into()),
        );
        compile_and_run(vec![0.into()], ToBool, TypeDef::boolean(), Ok(false.into()));
    }

    #[test]
    fn null() {
        compile_and_run(
            vec![Expr::Null],
            ToBool,
            TypeDef::boolean(),
            Ok(false.into()),
        );
    }

    #[test]
    fn string() {
        // yes
        for s in ["true", "yes", "on", "1"] {
            compile_and_run(
                vec![s.into()],
                ToBool,
                TypeDef::boolean().fallible(),
                Ok(true.into()),
            )
        }

        // no
        for s in ["false", "no", "off", "0"] {
            compile_and_run(
                vec![s.into()],
                ToBool,
                TypeDef::boolean().fallible(),
                Ok(false.into()),
            )
        }
    }
}
