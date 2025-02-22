use std::ops::Deref;

use value::{Kind, Value};

use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Function, FunctionCompileContext, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::query::Query;
use crate::compiler::state::TypeState;
use crate::compiler::{Expression, ExpressionError, Spanned, TypeDef};
use crate::context::Context;
use crate::{ContextError, SyntaxError};

pub struct Exists;

impl Function for Exists {
    fn identifier(&self) -> &'static str {
        "exists"
    }

    fn parameters(&self) -> &'static [Parameter] {
        &[Parameter {
            name: "path",
            kind: Kind::ANY,
            required: true,
        }]
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
                    got: Kind::UNDEFINED, // TODO: fix this
                    span,
                });
            }
        };

        Ok(FunctionCall {
            function: Box::new(ExistsFunc { query }),
            span: cx.span,
        })
    }
}

#[derive(Clone)]
struct ExistsFunc {
    query: Spanned<Query>,
}

impl Expression for ExistsFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let exists = match self.query.deref() {
            Query::External(path) => match cx.target.get(path) {
                Ok(got) => got.is_some(),
                Err(err) => match err {
                    ContextError::NotFound => false,
                    _ => {
                        return Err(ExpressionError::Error {
                            message: err.to_string(),
                            span: self.query.span,
                        });
                    }
                },
            },
            Query::Internal(index, path) => cx.get(*index).get(path).is_some(),
        };

        Ok(exists.into())
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
    use value::parse_target_path;

    #[test]
    fn exists() {
        compile_and_run(
            vec![parse_target_path(".key").unwrap().into()],
            Exists,
            TypeDef::boolean(),
            Ok(true.into()),
        )
    }

    #[test]
    fn not_exist() {
        compile_and_run(
            vec![parse_target_path(".foo").unwrap().into()],
            Exists,
            TypeDef::boolean(),
            Ok(false.into()),
        )
    }
}
