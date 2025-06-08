use value::{Kind, Value};

use crate::SyntaxError;
use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Function, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::state::TypeState;
use crate::compiler::{Expression, ExpressionError, Spanned, TypeDef};
use crate::context::Context;

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

    fn compile(&self, mut arguments: ArgumentList) -> Result<FunctionCall, SyntaxError> {
        let value = arguments.get();

        Ok(FunctionCall {
            function: Box::new(KebabCaseFunc { value }),
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

/// Converts a string to kebab case.
///
/// This function takes a string slice as its argument, then returns a `String`
/// of which the case style is kebab case.
///
/// This function targets the upper and lower cases of ASCII alphabets for
/// capitalization, and all characters except ASCII alphabets and ASCII numbers
/// are replaced to hyphens as word separators.
fn kebabcase(input: &str) -> String {
    let mut result = String::with_capacity(input.len() + input.len() / 2);
    // .len returns byte count but ok in this case!

    enum ChIs {
        FirstOfStr,
        NextOfUpper,
        NextOfContdUpper,
        NextOfSepMark,
        NextOfKeepedMark,
        Others,
    }
    let mut flag = ChIs::FirstOfStr;

    for ch in input.chars() {
        if ch.is_ascii_uppercase() {
            match flag {
                ChIs::FirstOfStr => {
                    result.push(ch.to_ascii_lowercase());
                    flag = ChIs::NextOfUpper;
                }
                ChIs::NextOfUpper | ChIs::NextOfContdUpper => {
                    result.push(ch.to_ascii_lowercase());
                    flag = ChIs::NextOfContdUpper;
                }
                _ => {
                    result.push('-');
                    result.push(ch.to_ascii_lowercase());
                    flag = ChIs::NextOfUpper;
                }
            }
        } else if ch.is_ascii_lowercase() {
            match flag {
                ChIs::NextOfContdUpper => {
                    if let Some(prev) = result.pop() {
                        result.push('-');
                        result.push(prev);
                    }
                }
                ChIs::NextOfSepMark | ChIs::NextOfKeepedMark => {
                    result.push('-');
                }
                _ => (),
            }
            result.push(ch);
            flag = ChIs::Others;
        } else if ch.is_ascii_digit() {
            if let ChIs::NextOfSepMark = flag {
                result.push('-')
            }
            result.push(ch);
            flag = ChIs::NextOfKeepedMark;
        } else {
            match flag {
                ChIs::FirstOfStr => (),
                _ => flag = ChIs::NextOfSepMark,
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
    fn it_should_convert_camel_case() {
        let result = kebabcase("abcDefGHIjk");
        assert_eq!(result, "abc-def-gh-ijk");
    }

    #[test]
    fn it_should_convert_pascal_case() {
        let result = kebabcase("AbcDefGHIjk");
        assert_eq!(result, "abc-def-gh-ijk");
    }

    #[test]
    fn it_should_convert_snake_case() {
        let result = kebabcase("abc_def_ghi");
        assert_eq!(result, "abc-def-ghi");
    }

    #[test]
    fn it_should_convert_kebab_case() {
        let result = kebabcase("abc-def-ghi");
        assert_eq!(result, "abc-def-ghi");
    }

    #[test]
    fn it_should_convert_train_case() {
        let result = kebabcase("Abc-Def-Ghi");
        assert_eq!(result, "abc-def-ghi");
    }

    #[test]
    fn it_should_convert_macro_case() {
        let result = kebabcase("ABC_DEF_GHI");
        assert_eq!(result, "abc-def-ghi");
    }

    #[test]
    fn it_should_convert_cobol_case() {
        let result = kebabcase("ABC-DEF-GHI");
        assert_eq!(result, "abc-def-ghi");
    }

    #[test]
    fn it_should_keep_digits() {
        let result = kebabcase("abc123-456defG789HIJklMN12");
        assert_eq!(result, "abc123-456-def-g789-hi-jkl-mn12");
    }

    #[test]
    fn it_should_convert_when_starting_with_digit() {
        let result = kebabcase("123abc456def");
        assert_eq!(result, "123-abc456-def");

        let result = kebabcase("123ABC456DEF");
        assert_eq!(result, "123-abc456-def");
    }

    #[test]
    fn it_should_treat_marks_as_separators() {
        let result = kebabcase(":.abc~!@def#$ghi%&jk(lm)no/?");
        assert_eq!(result, "abc-def-ghi-jk-lm-no");
    }

    #[test]
    fn it_should_convert_empty() {
        let result = kebabcase("");
        assert_eq!(result, "");
    }

    #[test]
    fn it_should_treat_number_sequence_by_default() {
        let result = kebabcase("abc123Def456#Ghi789");
        assert_eq!(result, "abc123-def456-ghi789");

        let result = kebabcase("ABC123-DEF456#GHI789");
        assert_eq!(result, "abc123-def456-ghi789");

        let result = kebabcase("abc123-def456#ghi789");
        assert_eq!(result, "abc123-def456-ghi789");

        let result = kebabcase("ABC123_DEF456#GHI789");
        assert_eq!(result, "abc123-def456-ghi789");

        let result = kebabcase("Abc123Def456#Ghi789");
        assert_eq!(result, "abc123-def456-ghi789");

        let result = kebabcase("abc123_def456#ghi789");
        assert_eq!(result, "abc123-def456-ghi789");

        let result = kebabcase("Abc123-Def456#-Ghi789");
        assert_eq!(result, "abc123-def456-ghi789");

        let result = kebabcase("000-abc123_def456#ghi789");
        assert_eq!(result, "000-abc123-def456-ghi789");
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
