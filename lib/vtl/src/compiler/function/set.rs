use value::{Kind, OwnedSegment, OwnedValuePath, Value};

use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Function, FunctionCompileContext, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::state::TypeState;
use crate::compiler::{Expression, ExpressionError, Spanned, TypeDef};
use crate::context::Context;
use crate::SyntaxError;

pub struct Set;

impl Function for Set {
    fn identifier(&self) -> &'static str {
        "set"
    }

    fn parameters(&self) -> &'static [Parameter] {
        &[
            Parameter {
                name: "value",
                kind: Kind::OBJECT,
                required: true,
            },
            Parameter {
                name: "path",
                kind: Kind::ARRAY,
                required: false,
            },
            Parameter {
                name: "data",
                kind: Kind::ANY,
                required: false,
            },
        ]
    }

    fn compile(
        &self,
        cx: FunctionCompileContext,
        mut arguments: ArgumentList,
    ) -> Result<FunctionCall, SyntaxError> {
        let value = arguments.get();
        let path = arguments.get();
        let data = arguments.get_opt();

        Ok(FunctionCall {
            function: Box::new(SetFunc { value, path, data }),
            span: cx.span,
        })
    }
}

#[derive(Clone)]
struct SetFunc {
    value: Spanned<Expr>,
    path: Spanned<Expr>,
    data: Option<Spanned<Expr>>,
}

impl Expression for SetFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let mut value = self.value.resolve(cx)?;
        let path = self.path.resolve(cx)?;

        let path = match path {
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

        match &self.data {
            Some(data) => {
                let data = data.resolve(cx)?;
                value.insert(&path, data);
            }
            None => {
                value.insert(&path, Value::Null);
            }
        }

        Ok(Value::Null)
    }

    fn type_def(&self, _state: &TypeState) -> TypeDef {
        TypeDef {
            fallible: false,
            kind: Kind::UNDEFINED,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::function::compile_and_run;
    use crate::compiler::Span;
    use value::parse_target_path;

    #[test]
    fn set() {
        let path = Expr::Array(vec![
            Spanned::new(Expr::String("array".into()), Span::empty()),
            Spanned::new(Expr::Integer(2), Span::empty()),
        ]);

        compile_and_run(
            vec![
                parse_target_path(".").unwrap().into(),
                path,
                Expr::String("bar".into()),
            ],
            Set,
            TypeDef::undefined(),
            Ok(Value::Null),
        )
    }

    #[test]
    fn set_none() {
        let path = Expr::Array(vec![
            Spanned::new(Expr::String("array".into()), Span::empty()),
            Spanned::new(Expr::Integer(2), Span::empty()),
        ]);

        compile_and_run(
            vec![parse_target_path(".").unwrap().into(), path],
            Set,
            TypeDef::undefined(),
            Ok(Value::Null),
        )
    }
}
