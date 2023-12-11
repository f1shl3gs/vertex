use value::Value;

use crate::compiler::expression::Expression;
use crate::compiler::function::{ArgumentList, Function, FunctionCompileContext, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::parser::{Expr, SyntaxError};
use crate::compiler::query::Query;
use crate::compiler::{ExpressionError, Kind, Spanned, TypeDef};
use crate::Context;

pub struct Del;

impl Function for Del {
    fn identifier(&self) -> &'static str {
        "del"
    }

    fn parameters(&self) -> &'static [Parameter] {
        &[
            Parameter {
                name: "target",
                kind: Kind::ANY,
                required: true,
            },
            Parameter {
                name: "compact",
                kind: Kind::BOOLEAN,
                required: false,
            },
        ]
    }

    fn compile(
        &self,
        cx: FunctionCompileContext,
        mut arguments: ArgumentList,
    ) -> Result<FunctionCall, SyntaxError> {
        let expr = arguments.get();
        let span = expr.span;
        let query = match expr.node {
            Expr::Query(query) => span.with(query),
            expr => {
                return Err(SyntaxError::InvalidFunctionArgumentType {
                    function: self.identifier(),
                    argument: "path",
                    want: Kind::ANY,
                    got: expr.type_def().kind,
                    span,
                })
            }
        };

        let compact = arguments.get_bool_opt()?.unwrap_or(false);

        Ok(FunctionCall {
            function: Box::new(DelFunc { query, compact }),
            span: cx.span,
        })
    }
}

struct DelFunc {
    query: Spanned<Query>,
    compact: bool,
}

impl Expression for DelFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        match &self.query.node {
            Query::Internal(name, path) => {
                let value = cx
                    .variables
                    .get_mut(name)
                    .expect("variable should exist already")
                    .remove(path, self.compact)
                    .unwrap_or(Value::Null);

                Ok(value)
            }
            Query::External(path) => {
                cx.target
                    .remove(path, self.compact)
                    .map_err(|err| ExpressionError::Error {
                        message: err.to_string(),
                        span: self.query.span,
                    })
            }
        }
    }

    fn type_def(&self) -> TypeDef {
        TypeDef {
            fallible: true,
            kind: Kind::ANY,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use value::{parse_target_path, value};

    use crate::compiler::function::compile_and_run;
    use crate::compiler::Span;

    #[test]
    fn exists() {
        compile_and_run(
            vec![parse_target_path(".key").unwrap().into()],
            Del,
            TypeDef::any().fallible(),
            Ok("value".into()),
        )
    }

    #[test]
    fn not_exists() {
        compile_and_run(
            vec![parse_target_path(".foo").unwrap().into()],
            Del,
            TypeDef::any().fallible(),
            Err(ExpressionError::Error {
                message: "not found".to_string(),
                span: Span { start: 0, end: 0 },
            }),
        )
    }

    #[test]
    fn array_field() {
        compile_and_run(
            vec![parse_target_path(".array").unwrap().into()],
            Del,
            TypeDef::any().fallible(),
            Ok(value!([1, 2, 3])),
        )
    }

    #[test]
    fn null_field() {
        compile_and_run(
            vec![parse_target_path(".null").unwrap().into()],
            Del,
            TypeDef::any().fallible(),
            Ok(Value::Null),
        )
    }

    #[test]
    fn map_field() {
        compile_and_run(
            vec![parse_target_path(".map").unwrap().into()],
            Del,
            TypeDef::any().fallible(),
            Ok(value!({"k1": "v1"})),
        )
    }

    #[test]
    fn array_item() {
        compile_and_run(
            vec![parse_target_path(".array[1]").unwrap().into()],
            Del,
            TypeDef::any().fallible(),
            Ok(Value::Integer(2)),
        )
    }
}
