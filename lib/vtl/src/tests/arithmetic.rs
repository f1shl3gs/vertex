use super::assert_ok;

#[test]
fn success() {
    for (input, want) in [
        // addition
        ("1 + 2", 3.into()),
        ("1.1 + 1.2", 2.3.into()),
        ("1 + 2.0", 3.0.into()),
        ("1.0 + 2", 3.0.into()),
        ("null + \"bar\"", "bar".into()),
        (r#""foo" + null"#, "foo".into()),
        (r#""foo" + "bar""#, "foobar".into()),
        // division
        ("3.5 / 1.6", 2.1875.into()),
        ("2.5 / 2", 1.25.into()),
        ("5 / 2", 2.5.into()),
        ("6 / 2.5", 2.4.into()),
        // multiplication
        ("3.0 * 5.0", 15.0.into()),
        ("3.0 * 5", 15.0.into()),
        ("3 * 5.0", 15.0.into()),
        ("3 * 5", 15.into()),
        // subtraction
        ("2.0 - 0.5", 1.5.into()),
        ("2.5 - 1", 1.5.into()),
        ("1 - 1", 0.into()),
        ("2 - 0.5", 1.5.into()),
        // exponent
        // ("2 ^ 2", 0.into()),
    ] {
        assert_ok(input, want)
    }
}
