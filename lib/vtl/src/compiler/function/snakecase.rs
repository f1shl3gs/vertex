use value::{Kind, Value};

use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Function, FunctionCompileContext, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::state::TypeState;
use crate::compiler::{Expression, ExpressionError, Spanned, TypeDef};
use crate::context::Context;
use crate::SyntaxError;

pub struct SnakeCase;

impl Function for SnakeCase {
    fn identifier(&self) -> &'static str {
        "snakecase"
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
            function: Box::new(SnakeCaseFunc { value }),
            span: cx.span,
        })
    }
}

#[derive(Clone)]
struct SnakeCaseFunc {
    value: Spanned<Expr>,
}

impl Expression for SnakeCaseFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let value = self.value.resolve(cx)?;
        let Value::Bytes(value) = value else {
            return Err(ExpressionError::UnexpectedType {
                want: Kind::BYTES,
                got: value.kind(),
                span: self.value.span,
            });
        };

        let value = String::from_utf8_lossy(&value);

        Ok(snake_case(value.as_ref()).into())
    }

    fn type_def(&self, _state: &TypeState) -> TypeDef {
        TypeDef {
            fallible: true,
            kind: Kind::BYTES,
        }
    }
}

/// Converts a string to snake case.
///
/// This function takes a string slice as its argument, then returns a `String`
/// of which the case style is snake case.
///
/// This function targets the upper and lower cases of ASCII alphabets for
/// capitalization, and all characters except ASCII alphabets and ASCII numbers
/// are replaced to underscores as word separators.
fn snake_case(input: &str) -> String {
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
                    result.push('_');
                    result.push(ch.to_ascii_lowercase());
                    flag = ChIs::NextOfUpper;
                }
            }
        } else if ch.is_ascii_lowercase() {
            match flag {
                ChIs::NextOfContdUpper => {
                    if let Some(prev) = result.pop() {
                        result.push('_');
                        result.push(prev);
                    }
                }
                ChIs::NextOfSepMark | ChIs::NextOfKeepedMark => {
                    result.push('_');
                }
                _ => (),
            }
            result.push(ch);
            flag = ChIs::Others;
        } else if ch.is_ascii_digit() {
            if let ChIs::NextOfSepMark = flag {
                result.push('_')
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
    fn simple() {
        compile_and_run(
            vec!["camelCase".into()],
            SnakeCase,
            TypeDef::bytes().fallible(),
            Ok(Value::Bytes("camel_case".into())),
        )
    }

    #[test]
    fn no_case() {
        compile_and_run(
            vec!["camel_case".into()],
            SnakeCase,
            TypeDef::bytes().fallible(),
            Ok(Value::Bytes("camel_case".into())),
        )
    }

    #[test]
    fn it_should_convert_camel_case() {
        let result = snake_case("abcDefGHIjk");
        assert_eq!(result, "abc_def_gh_ijk");
    }

    #[test]
    fn it_should_convert_pascal_case() {
        let result = snake_case("AbcDefGHIjk");
        assert_eq!(result, "abc_def_gh_ijk");
    }

    #[test]
    fn it_should_convert_snake_case() {
        let result = snake_case("abc_def_ghi");
        assert_eq!(result, "abc_def_ghi");
    }

    #[test]
    fn it_should_convert_kebab_case() {
        let result = snake_case("abc-def-ghi");
        assert_eq!(result, "abc_def_ghi");
    }

    #[test]
    fn it_should_convert_train_case() {
        let result = snake_case("Abc-Def-Ghi");
        assert_eq!(result, "abc_def_ghi");
    }

    #[test]
    fn it_should_convert_macro_case() {
        let result = snake_case("ABC_DEF_GHI");
        assert_eq!(result, "abc_def_ghi");
    }

    #[test]
    fn it_should_convert_cobol_case() {
        let result = snake_case("ABC-DEF-GHI");
        assert_eq!(result, "abc_def_ghi");
    }

    #[test]
    fn it_should_keep_digits() {
        let result = snake_case("abc123-456defG789HIJklMN12");
        assert_eq!(result, "abc123_456_def_g789_hi_jkl_mn12");
    }

    #[test]
    fn it_should_convert_when_starting_with_digit() {
        let result = snake_case("123abc456def");
        assert_eq!(result, "123_abc456_def");

        let result = snake_case("123ABC456DEF");
        assert_eq!(result, "123_abc456_def");
    }

    #[test]
    fn it_should_treat_marks_as_separators() {
        let result = snake_case(":.abc~!@def#$ghi%&jk(lm)no/?");
        assert_eq!(result, "abc_def_ghi_jk_lm_no");
    }

    #[test]
    fn it_should_convert_empty() {
        let result = snake_case("");
        assert_eq!(result, "");
    }
}
