mod arithmetic;
mod assignment;
mod for_statement;
mod if_statement;

use std::collections::BTreeMap;

use value::Value;

use crate::compiler::{Compiler, ExpressionError};
use crate::context::TargetValue;

pub fn assert_ok(input: &str, want: Value) {
    let got = run(
        input,
        &mut TargetValue {
            metadata: Value::Object(BTreeMap::default()),
            value: Value::Object(BTreeMap::default()),
        },
    )
    .unwrap();

    assert_eq!(got, want, "{}", input)
}

pub fn run(input: &str, target: &mut TargetValue) -> Result<Value, ExpressionError> {
    let mut program = Compiler::compile(input).unwrap();
    program.run(target)
}
