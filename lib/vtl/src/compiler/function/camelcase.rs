use value::{Kind, Value};

use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Function, FunctionCompileContext, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::state::TypeState;
use crate::compiler::{Expression, ExpressionError, Spanned, TypeDef};
use crate::context::Context;
use crate::SyntaxError;

pub struct CamelCase;

impl Function for CamelCase {
    fn identifier(&self) -> &'static str {
        "camelcase"
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
            function: Box::new(CamelCaseFunc { value }),
            span: cx.span,
        })
    }
}

#[derive(Clone)]
struct CamelCaseFunc {
    value: Spanned<Expr>,
}

impl Expression for CamelCaseFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        match self.value.resolve(cx)? {
            Value::Bytes(value) => {
                let value = String::from_utf8_lossy(&value);

                Ok(camelcase(value.as_ref()).into())
            }
            value => Err(ExpressionError::UnexpectedType {
                want: Kind::BYTES,
                got: value.kind(),
                span: self.value.span,
            }),
        }
    }

    fn type_def(&self, _state: &TypeState) -> TypeDef {
        TypeDef::bytes()
    }
}

/// Converts a string to camel case.
///
/// This function takes a string slice as its argument, then returns a `String`
/// of which the case style is camel case.
///
/// This function targets only the upper and lower cases of ASCII alphabets for
/// capitalization, and all characters except ASCII alphabets and ASCII numbers
/// are eliminated as word separators.
fn camelcase(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    // .len returns byte count but ok in this case!

    enum ChIs {
        FirstOfStr,
        InFirstWord,
        NextOfUpper,
        NextOfMark,
        Others,
    }
    let mut flag = ChIs::FirstOfStr;

    for ch in input.chars() {
        if ch.is_ascii_uppercase() {
            match flag {
                ChIs::FirstOfStr | ChIs::InFirstWord => {
                    result.push(ch.to_ascii_lowercase());
                    flag = ChIs::InFirstWord;
                }
                ChIs::NextOfUpper => {
                    result.push(ch.to_ascii_lowercase());
                    //flag = ChIs::NextOfUpper;
                }
                _ => {
                    result.push(ch);
                    flag = ChIs::NextOfUpper
                }
            }
        } else if ch.is_ascii_lowercase() {
            match flag {
                ChIs::NextOfUpper => {
                    if let Some(prev) = result.pop() {
                        result.push(prev.to_ascii_uppercase());
                        result.push(ch);
                        flag = ChIs::Others;
                    }
                }

                ChIs::NextOfMark => {
                    result.push(ch.to_ascii_uppercase());
                    flag = ChIs::NextOfUpper;
                }
                _ => {
                    result.push(ch);
                    flag = ChIs::Others;
                }
            }
        } else if ch.is_ascii_digit() {
            result.push(ch);
            flag = ChIs::NextOfMark;
        } else {
            match flag {
                ChIs::FirstOfStr => (),
                _ => flag = ChIs::NextOfMark,
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::function::compile_and_run;

    #[test]
    fn convert() {
        for (input, want) in [
            ("abcDefGHIjk", "abcDefGhIjk"),
            ("AbcDefGHIjk", "abcDefGhIjk"),
            ("abc_def_ghi", "abcDefGhi"),
            ("abc-def-ghi", "abcDefGhi"),
            ("Abc-Def-Ghi", "abcDefGhi"),
            ("ABC_DEF_GHI", "abcDefGhi"),
            ("ABC-DEF-GHI", "abcDefGhi"),
            ("abc123-456defG789HIJklMN12", "abc123456DefG789HiJklMn12"),
            ("123abc456def", "123Abc456Def"),
            ("123ABC456DEF", "123Abc456Def"),
            (":.abc~!@def#$ghi%&jk(lm)no/?", "abcDefGhiJkLmNo"),
            ("", ""),
            ("abc123Def456#Ghi789", "abc123Def456Ghi789"),
            ("ABC123-DEF456#GHI789", "abc123Def456Ghi789"),
            ("abc123-def456#ghi789", "abc123Def456Ghi789"),
            ("ABC123_DEF456#GHI789", "abc123Def456Ghi789"),
            ("Abc123Def456#Ghi789", "abc123Def456Ghi789"),
            ("abc123_def456#ghi789", "abc123Def456Ghi789"),
            ("Abc123-Def456#-Ghi789", "abc123Def456Ghi789"),
            ("000-abc123_def456#ghi789", "000Abc123Def456Ghi789"),
            ("abc123Def456#Ghi789", "abc123Def456Ghi789"),
            ("ABC-123-DEF-456#GHI-789", "abc123Def456Ghi789"),
            ("abc-123-def-456#ghi-789", "abc123Def456Ghi789"),
            ("ABC_123_DEF_456#GHI_789", "abc123Def456Ghi789"),
            ("Abc123Def456#Ghi789", "abc123Def456Ghi789"),
            ("abc_123_def_456#ghi_789", "abc123Def456Ghi789"),
            ("Abc-123-Def-456#Ghi-789", "abc123Def456Ghi789"),
            ("000_abc_123_def_456#ghi_789", "000Abc123Def456Ghi789"),
        ] {
            compile_and_run(
                vec![input.into()],
                CamelCase,
                TypeDef::bytes(),
                Ok(Value::Bytes(want.into())),
            )
        }
    }

    #[test]
    fn simple() {
        compile_and_run(
            vec!["into_camel".into()],
            CamelCase,
            TypeDef::bytes(),
            Ok(Value::Bytes("intoCamel".into())),
        )
    }

    #[test]
    fn no_case() {
        compile_and_run(
            vec!["into_camel".into()],
            CamelCase,
            TypeDef::bytes(),
            Ok(Value::Bytes("intoCamel".into())),
        )
    }
}
