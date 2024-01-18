use std::collections::BTreeMap;

use value::{value, Value};

use crate::tests::run;
use crate::TargetValue;

#[test]
fn success() {
    for (input, want) in [
        (
            r#"
        .x = []
        .x[5] = "foo"

        "#,
            value!({
                "x": [null, null, null, null, null, "foo"],
            }),
        ),
        (
            r#"
            .res = { "foo": 2 }
            "#,
            value!({
                "res": {"foo": 2}
            }),
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
