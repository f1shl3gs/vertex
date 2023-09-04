use std::collections::HashMap;

use framework::config;
use framework::config::{ConfigDiff, Format};
use framework::topology;

async fn load(config: &str, format: config::Format) -> Result<Vec<String>, Vec<String>> {
    let c = config::load_from_str(config, format)?;

    let diff = ConfigDiff::initial(&c);
    let c2 = config::load_from_str(config, format).unwrap();

    match (
        config::warnings(&c2),
        topology::build_pieces(&c, &diff, HashMap::new()).await,
    ) {
        (warnings, Ok(_pieces)) => Ok(warnings),
        (_, Err(errs)) => Err(errs),
    }
}

#[tokio::test]
#[ignore]
async fn bad_type() {
    let errs = load(
        r#"
sources:
    in:
        type: generator
sinks:
    out:
        type: abcdefg
        inputs:
        - in
"#,
        Format::YAML,
    )
    .await
    .unwrap_err();

    assert_eq!(errs.len(), 1);
    assert!(errs[0].contains("unknown variant `abcdefg`, expected one of"));
}

#[tokio::test]
#[ignore]
async fn bad_input() {
    let errs = load(
        r#"
sources:
    in:
        type: generator
sinks:
    out:
        type: stdout
        inputs:
        - abc
"#,
        Format::YAML,
    )
    .await
    .unwrap_err();

    assert_eq!(errs.len(), 1);
    assert!(errs[0].contains(r#""abc" for sink "out" doesn't match any components"#))
}

#[tokio::test]
#[ignore]
async fn warnings() {
    let warnings = load(
        r#"
sources:
    in1:
        type: generator
    in2:
        type: generator

transforms:
    add1:
        type: rewrite
        inputs:
            - in1
        operations:
            - type: set
              key: foo
              value: bar
    add2:
        type: rewrite
        inputs:
            - in1
        operations:
            - type: set
              key: foo
              value: bar
sinks:
    out1:
        type: stdout
        inputs:
        - add1
"#,
        Format::YAML,
    )
    .await
    .unwrap();

    assert_eq!(
        warnings,
        vec![
            "Transform \"add2\" has no consumers",
            "Source \"in2\" has no consumers"
        ]
    )
}
