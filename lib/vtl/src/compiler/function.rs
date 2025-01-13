mod abs;
mod append;
mod assert_eq;
mod ceil;
mod compact;
mod contains;
mod del;
mod ends_with;
mod exists;
mod find;
mod flatten;
mod floor;
mod format;
mod from_unix_timestamp;
mod get;
mod get_env;
mod get_hostname;
mod includes;
mod is_array;
mod is_bool;
mod is_empty;
mod is_float;
mod is_integer;
mod is_ipv4;
mod is_ipv6;
mod is_object;
mod is_string;
mod is_timestamp;
mod keys;
mod length;
mod log;
mod lowercase;
mod r#match;
mod merge;
mod now;
mod parse_json;
mod parse_query;
mod parse_timestamp;
mod parse_url;
mod parse_user_agent;
mod push;
mod replace;
mod round;
mod set;
mod slice;
mod split;
mod starts_with;
mod to_bool;
mod to_float;
mod to_integer;
mod to_string;
mod to_unix_timestamp;
mod trim;
mod r#typeof;
mod unique;
mod uppercase;
mod values;
mod xxhash;

use super::expr::Expr;
use super::function_call::FunctionCall;
use super::parser::SyntaxError;
use super::state::TypeState;
use super::Expression;
use super::{Kind, Span, Spanned};

pub struct ArgumentList {
    name: &'static str,
    parameters: &'static [Parameter],

    arguments: Vec<Spanned<Expr>>,
}

impl ArgumentList {
    pub fn new(name: &'static str, parameters: &'static [Parameter]) -> Self {
        Self {
            name,
            parameters,
            arguments: Vec::with_capacity(parameters.len()),
        }
    }

    pub fn inner(self) -> Vec<Spanned<Expr>> {
        self.arguments
    }

    pub fn push(&mut self, expr: Spanned<Expr>, state: &TypeState) -> Result<(), SyntaxError> {
        let index = self.arguments.len();

        if let Some(parameter) = self.parameters.get(index) {
            if !parameter.kind.intersects(expr.type_def(state).kind) {
                return Err(SyntaxError::InvalidFunctionArgumentType {
                    function: self.name,
                    argument: parameter.name,
                    want: parameter.kind,
                    got: expr.type_def(state).kind,
                    span: expr.span,
                });
            }
        }

        self.arguments.push(expr);

        Ok(())
    }

    pub fn len(&self) -> usize {
        self.arguments.len()
    }

    pub fn get(&mut self) -> Spanned<Expr> {
        assert!(!self.arguments.is_empty());
        self.arguments.remove(0)
    }

    pub fn get_opt(&mut self) -> Option<Spanned<Expr>> {
        if self.arguments.is_empty() {
            return None;
        }

        Some(self.arguments.remove(0))
    }

    pub fn get_bool_opt(&mut self) -> Result<Option<bool>, SyntaxError> {
        match self.get_opt() {
            Some(expr) => match expr.node {
                Expr::Boolean(b) => Ok(Some(b)),
                _ => Err(SyntaxError::UnexpectedToken {
                    got: expr.to_string(),
                    want: Some("const value true or false".to_string()),
                    span: expr.span,
                }),
            },
            None => Ok(None),
        }
    }

    pub fn get_string_opt(&mut self) -> Result<Option<Spanned<String>>, SyntaxError> {
        match self.get_opt() {
            Some(expr) => match expr.node {
                Expr::String(s) => {
                    let s = String::from_utf8_lossy(&s);
                    Ok(Some(Spanned::new(s.to_string(), expr.span)))
                }
                _ => Err(SyntaxError::UnexpectedToken {
                    got: expr.to_string(),
                    want: Some("string literal".to_string()),
                    span: expr.span,
                }),
            },
            None => Ok(None),
        }
    }

    pub fn get_string(&mut self) -> Result<Spanned<String>, SyntaxError> {
        let expr = self.get();
        match expr.node {
            Expr::String(s) => {
                let s = String::from_utf8_lossy(&s);
                Ok(Spanned::new(s.to_string(), expr.span))
            }
            got => Err(SyntaxError::UnexpectedToken {
                got: got.to_string(),
                want: Some("string literal".into()),
                span: expr.span,
            }),
        }
    }
}

pub struct FunctionCompileContext {
    // span of the Token::FunctionCall
    pub span: Span,
}

pub struct Parameter {
    /// The name of the parameter
    pub name: &'static str,

    /// The type kind this parameter expects to receive.
    pub kind: Kind,

    /// Whether or not this is a required parameter
    pub required: bool,
}

pub trait Function: Send + Sync {
    /// The identifier by which the function can be called.
    fn identifier(&self) -> &'static str;

    /// An optional list of parameters the function accepts.
    ///
    /// This list is used at compile-time to check function arity.
    /// and argument type definition.
    fn parameters(&self) -> &'static [Parameter] {
        &[]
    }

    /// Compile a [`Function`] into a type that can be resolved to an
    /// [`Expr`].
    ///
    /// This function is called at compile-time for any `Function` used in the
    /// program.
    ///
    /// At runtime, the `Expression` returned by this function is executed and
    /// resolved to its final [`Value`].
    fn compile(
        &self,
        cx: FunctionCompileContext,
        arguments: ArgumentList,
    ) -> Result<FunctionCall, SyntaxError>;
}

pub fn builtin_functions() -> Vec<Box<dyn Function>> {
    vec![
        Box::new(abs::Abs),
        Box::new(append::Append),
        Box::new(assert_eq::AssertEq),
        Box::new(ceil::Ceil),
        Box::new(compact::Compact),
        Box::new(contains::Contains),
        Box::new(del::Del),
        Box::new(ends_with::EndsWith),
        Box::new(exists::Exists),
        Box::new(find::Find),
        Box::new(flatten::Flatten),
        Box::new(floor::Floor),
        Box::new(format::Format),
        Box::new(from_unix_timestamp::FromUnixTimestamp),
        Box::new(get::Get),
        Box::new(get_env::GetEnv),
        Box::new(get_hostname::GetHostname),
        Box::new(includes::Includes),
        Box::new(is_array::IsArray),
        Box::new(is_bool::IsBool),
        Box::new(is_empty::IsEmpty),
        Box::new(is_float::IsFloat),
        Box::new(is_integer::IsInteger),
        Box::new(is_ipv4::IsIpv4),
        Box::new(is_ipv6::IsIpv6),
        Box::new(is_object::IsObject),
        Box::new(is_string::IsString),
        Box::new(is_timestamp::IsTimestamp),
        Box::new(keys::Keys),
        Box::new(length::Length),
        Box::new(log::Log),
        Box::new(lowercase::Lowercase),
        Box::new(r#match::Match),
        Box::new(merge::Merge),
        Box::new(now::Now),
        Box::new(parse_json::ParseJson),
        Box::new(parse_query::ParseQuery),
        Box::new(parse_timestamp::ParseTimestamp),
        Box::new(parse_url::ParseUrl),
        Box::new(parse_user_agent::ParseUserAgent),
        Box::new(push::Push),
        Box::new(replace::Replace),
        Box::new(set::Set),
        Box::new(slice::Slice),
        Box::new(split::Split),
        Box::new(starts_with::StartsWith),
        Box::new(to_bool::ToBool),
        Box::new(to_float::ToFloat),
        Box::new(to_integer::ToInteger),
        Box::new(to_string::ToString),
        Box::new(to_unix_timestamp::ToUnixTimestamp),
        Box::new(trim::Trim),
        Box::new(r#typeof::TypeOf),
        Box::new(unique::Unique),
        Box::new(uppercase::Uppercase),
        Box::new(values::Values),
        Box::new(xxhash::XXHash),
    ]
}

#[cfg(test)]
pub fn compile_and_run<F: Function>(
    arguments: Vec<Expr>,
    func: F,
    td: crate::compiler::TypeDef,
    want: Result<value::Value, crate::compiler::ExpressionError>,
) {
    use chrono::{TimeZone, Utc};
    use value::{value, Value};

    use crate::context::Context;
    use crate::TargetValue;

    let state = TypeState::default();
    let func = Box::new(func);

    let mut arguments_list = ArgumentList::new(func.identifier(), func.parameters());
    for argument in arguments {
        if let Err(err) = arguments_list.push(Spanned::new(argument, Span::empty()), &state) {
            panic!("{}", err)
        }
    }

    let call = func
        .compile(
            FunctionCompileContext {
                span: Span::empty(),
            },
            arguments_list,
        )
        .unwrap();

    assert_eq!(
        call.type_def(&state),
        td,
        "want: {}, got: {}",
        td.kind,
        call.type_def(&state).kind
    );

    let ts = Utc.with_ymd_and_hms(2021, 1, 1, 0, 0, 0).unwrap();
    let mut variables = Vec::with_capacity(8);
    variables.reserve(8);

    let mut cx = Context {
        target: &mut TargetValue {
            metadata: Value::Object(Default::default()),
            value: value!({
                "key": "value",
                "int": 1,
                "float": 1.2,
                "array": [1, 2, 3],
                "null": null,
                "timestamp": ts,
                "map": {
                    "k1": "v1"
                }
            }),
        },
        variables: &mut variables,
    };

    let got = call.resolve(&mut cx);
    assert_eq!(got, want);
}
