use std::collections::BTreeMap;

use crate::context::TargetValue;
use value::{map_value, Value};

use crate::tests::run;

#[test]
fn success() {
    for (input, want) in [
        (
            r#"
        .x = []
        .x[5] = "foo"

        "#,
            map_value!(
                "x" => Value::Array(vec![
                    Value::Null,
                    Value::Null,
                    Value::Null,
                    Value::Null,
                    Value::Null,
                    Value::Bytes("foo".into())
                ])
            ),
        ),
        (
            r#"
            .res = { foo: 2 }
            "#,
            map_value!(
                "res" => map_value!(
                    "foo" => 2
                )
            ),
        ),
    ] {
        let mut target = TargetValue {
            metadata: Value::Object(BTreeMap::default()),
            value: Value::Object(BTreeMap::default()),
        };
        let _ = run(input, &mut target).expect(input);

        assert_eq!(want, target.value, "{}", input)
    }
}
