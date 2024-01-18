use value::Value;

use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Function, FunctionCompileContext, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::parser::SyntaxError;
use crate::compiler::query::Query;
use crate::compiler::state::TypeState;
use crate::compiler::{Expression, ExpressionError, Kind, Spanned, TypeDef};
use crate::context::Context;

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
        let Spanned { node, span } = arguments.get();
        let query = match node {
            Expr::Query(query) => Spanned::new(query, span),
            _ => {
                return Err(SyntaxError::InvalidFunctionArgumentType {
                    function: self.identifier(),
                    argument: "path",
                    want: Kind::ANY,
                    got: Kind::UNDEFINED, // todo: fix this
                    span,
                });
            }
        };

        let compact = arguments.get_bool_opt()?.unwrap_or(false);

        Ok(FunctionCall {
            function: Box::new(DelFunc { query, compact }),
            span: cx.span,
        })
    }
}

#[derive(Clone)]
struct DelFunc {
    query: Spanned<Query>,
    compact: bool,
}

impl Expression for DelFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        match &self.query.node {
            Query::Internal(index, path) => {
                let value = cx
                    .get_mut(*index)
                    .remove(path, self.compact)
                    .unwrap_or(Value::Null);

                Ok(value)
            }
            Query::External(path) => cx
                .target
                .remove(path, self.compact)
                .map_err(|err| ExpressionError::Error {
                    message: err.to_string(),
                    span: self.query.span,
                })
                .map(|value| value.unwrap_or(Value::Null)),
        }
    }

    fn type_def(&self, state: &TypeState) -> TypeDef {
        self.query.type_def(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use value::{parse_target_path, value};

    use crate::compiler::function::compile_and_run;

    #[test]
    fn exists() {
        compile_and_run(
            vec![parse_target_path(".key").unwrap().into()],
            Del,
            TypeDef::any(),
            Ok("value".into()),
        )
    }

    #[test]
    fn not_exists() {
        compile_and_run(
            vec![parse_target_path(".foo").unwrap().into()],
            Del,
            TypeDef::any(),
            Ok(Value::Null),
        )
    }

    #[test]
    fn array_field() {
        compile_and_run(
            vec![parse_target_path(".array").unwrap().into()],
            Del,
            TypeDef::any(),
            Ok(value!([1, 2, 3])),
        )
    }

    #[test]
    fn null_field() {
        compile_and_run(
            vec![parse_target_path(".null").unwrap().into()],
            Del,
            TypeDef::any(),
            Ok(Value::Null),
        )
    }

    #[test]
    fn map_field() {
        compile_and_run(
            vec![parse_target_path(".map").unwrap().into()],
            Del,
            TypeDef::any(),
            Ok(value!({"k1": "v1"})),
        )
    }

    #[test]
    fn array_item() {
        compile_and_run(
            vec![parse_target_path(".array[1]").unwrap().into()],
            Del,
            TypeDef::any(),
            Ok(Value::Integer(2)),
        )
    }
}
