use std::collections::HashMap;
use vertex::config;
use vertex::config::{ConfigDiff, Format};
use vertex::topology;

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
#[cfg(all(
    feature = "sources-generator",
    feature = "transforms-add_tags",
    feature = "sinks-stdout"
))]
async fn happy_path() {
    load(
        r#"
sources:
    in:
        type: generator

transforms:
    add_tags:
        type: add_tags
        inputs:
            - in
        tags:
            foo: bar

sinks:
    stdout:
        type: stdout
        inputs:
            - add_tags
        "#,
        Format::YAML,
    )
    .await
    .unwrap();

    load(
        r#"{
  "sources": {
    "in": {
      "type": "generator"
    }
  },
  "transforms": {
    "add_tags": {
      "type": "add_tags",
      "inputs": [
        "in"
      ],
      "tags": {
        "foo": "bar"
      }
    }
  },
  "sinks": {
    "stdout": {
      "type": "stdout",
      "inputs": [
        "add_tags"
      ]
    }
  }
}"#,
        Format::JSON,
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn early_eof() {
    let errs = load("sinks:\nfo", Format::YAML).await.unwrap_err();

    assert_eq!(errs.len(), 1);
    assert_eq!(
        errs[0],
        "sinks: invalid type: string \"fo\", expected a map at line 2 column 1"
    );
}

#[tokio::test]
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
        type: add_tags
        inputs:
            - in1
        tags:
            foo: bar
    add2:
        type: add_tags
        inputs:
            - in1
        tags:
            foo: bar
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

// TODO: check cycle
