use value::{Kind, OwnedSegment, OwnedValuePath, Value};

use crate::SyntaxError;
use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Function, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::state::TypeState;
use crate::compiler::{Expression, ExpressionError, Spanned, TypeDef};
use crate::context::Context;

pub struct Get;

impl Function for Get {
    fn identifier(&self) -> &'static str {
        "get"
    }

    fn parameters(&self) -> &'static [Parameter] {
        &[
            Parameter {
                name: "value",
                kind: Kind::ARRAY_OR_OBJECT,
                required: true,
            },
            Parameter {
                name: "path",
                kind: Kind::ARRAY,
                required: false,
            },
        ]
    }

    fn compile(&self, mut arguments: ArgumentList) -> Result<FunctionCall, SyntaxError> {
        let value = arguments.get();
        let path = arguments.get();

        Ok(FunctionCall {
            function: Box::new(GetFunc { value, path }),
        })
    }
}

#[derive(Clone)]
struct GetFunc {
    value: Spanned<Expr>,
    path: Spanned<Expr>,
}

impl Expression for GetFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let value = self.value.resolve(cx)?;

        let path = match self.path.resolve(cx)? {
            Value::Array(segments) => {
                if segments.is_empty() {
                    return Ok(value);
                }

                let mut insert = OwnedValuePath::root();

                for segment in segments {
                    let segment = match segment {
                        Value::Bytes(path) => {
                            OwnedSegment::Field(String::from_utf8_lossy(&path).into())
                        }
                        Value::Integer(index) => OwnedSegment::Index(index as isize),
                        value => {
                            return Err(ExpressionError::UnexpectedType {
                                want: Kind::BYTES_OR_INTEGER,
                                got: value.kind(),
                                span: self.path.span, // it should be segment path
                            });
                        }
                    };

                    insert.push_segment(segment);
                }

                insert
            }
            value => {
                return Err(ExpressionError::UnexpectedType {
                    want: Kind::ARRAY,
                    got: value.kind(),
                    span: self.path.span,
                });
            }
        };

        match value.get(&path) {
            Some(value) => Ok(value.clone()),
            None => Ok(Value::Null),
        }
    }

    fn type_def(&self, _state: &TypeState) -> TypeDef {
        TypeDef {
            fallible: false,
            kind: Kind::ANY,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::Span;
    use crate::compiler::function::compile_and_run;
    use value::parse_target_path;

    #[test]
    fn get() {
        let path = Expr::Array(vec![
            Spanned::new(Expr::String("array".into()), Span::empty()),
            Spanned::new(Expr::Integer(2), Span::empty()),
        ]);

        compile_and_run(
            vec![parse_target_path(".").unwrap().into(), path],
            Get,
            TypeDef::any(),
            Ok(Value::Integer(3)),
        )
    }

    #[test]
    fn get_not_exists() {
        let path = Expr::Array(vec![
            Spanned::new(Expr::String("array".into()), Span::empty()),
            Spanned::new(Expr::Integer(3), Span::empty()),
        ]);

        compile_and_run(
            vec![parse_target_path(".").unwrap().into(), path],
            Get,
            TypeDef::any(),
            Ok(Value::Null),
        )
    }
}
