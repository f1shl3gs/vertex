use super::assert_ok;

#[test]
fn success() {
    for (input, want) in [
        (
            r#"
            if true {
                "foo"
            }
            "#,
            "foo",
        ),
        (
            r#"
            if false {
                "foo"
            } else {
                "bar"
            }
            "#,
            "bar",
        ),
    ] {
        assert_ok(input, want.into())
    }
}
