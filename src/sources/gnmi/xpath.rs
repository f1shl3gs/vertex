use super::proto::{Path, PathElem};

pub fn parse(input: &str) -> Result<Path, &str> {
    if input.is_empty() {
        return Ok(Path::default());
    }

    let chars = input.char_indices();
    let mut elem = Vec::new();

    let mut inside_brackets = false;
    let mut start = 0usize;
    let mut key_start = 0;
    let mut key = input;
    let mut value_start = 0;
    let mut current = PathElem::default();
    let mut escaping = false;

    for (index, ch) in chars {
        match ch {
            '/' => {
                if inside_brackets {
                    continue;
                }

                if start == index {
                    start = index + 1;
                    continue;
                }

                if current.name.is_empty() {
                    current.name = input[start..index].to_string();
                }

                elem.push(current);
                current = PathElem::default();

                start = index + 1;
            }
            '[' => {
                if escaping {
                    escaping = false;
                    continue;
                }

                inside_brackets = true;
                key_start = index + 1;

                if current.name.is_empty() {
                    current.name = input[start..index].to_string();
                }
            }
            '=' => {
                if escaping {
                    escaping = false;
                    continue;
                }

                if !inside_brackets {
                    return Err(&input[index..]);
                }

                key = &input[key_start..index];
                value_start = index + 1;
            }
            ']' => {
                if escaping {
                    escaping = false;
                    continue;
                }

                if !inside_brackets {
                    return Err(&input[index..]);
                }

                inside_brackets = false;

                current
                    .key
                    .insert(key.to_string(), escape(&input[value_start..index]));
            }
            '\\' => {
                escaping = true;
            }
            _ => {}
        }
    }

    elem.push(PathElem {
        name: input[start..].to_string(),
        ..Default::default()
    });

    Ok(Path {
        elem,
        ..Default::default()
    })
}

fn escape(input: &str) -> String {
    input.replace("\\[", "[").replace("\\]", "]")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn parse_element() {
        for (_name, input, expected) in [
            ("empty path", "", Some(vec![])),
            (
                "no first slash",
                "a/b/c",
                Some(vec!["a".into(), "b".into(), "c".into()]),
            ),
            (
                "path without list element",
                "/a/b/c",
                Some(vec!["a".into(), "b".into(), "c".into()]),
            ),
            (
                "path containing a single-key list",
                "/a/b[k1=10]/c",
                Some(vec![
                    "a".into(),
                    PathElem {
                        name: "b".to_string(),
                        key: {
                            let mut map = BTreeMap::new();
                            map.insert("k1".into(), "10".into());
                            map
                        },
                    },
                    "c".into(),
                ]),
            ),
            (
                "path containning a single-key list, but invalid list name",
                r#"/a/b\[k1=10]/c"#,
                None,
            ),
            (
                "path containing a single-key list with / in key leaf value",
                "/a/b[k1=10.10.10.10/24]/c",
                Some(vec![
                    "a".into(),
                    PathElem {
                        name: "b".to_string(),
                        key: {
                            let mut map = BTreeMap::new();
                            map.insert("k1".into(), "10.10.10.10/24".into());
                            map
                        },
                    },
                    "c".into(),
                ]),
            ),
            (
                "path containing a single-key List with [ in key leaf value",
                r#"/a/b[k1=10.10.10.10\[24]/c"#,
                Some(vec![
                    "a".into(),
                    PathElem {
                        name: "b".to_string(),
                        key: {
                            let mut map = BTreeMap::new();
                            map.insert("k1".into(), "10.10.10.10[24".into());
                            map
                        },
                    },
                    "c".into(),
                ]),
            ),
            (
                "path containing a single-key List with ] in key leaf value",
                r#"/a/b[k1=10.10.10.10\]24]/c"#,
                Some(vec![
                    "a".into(),
                    PathElem {
                        name: "b".to_string(),
                        key: {
                            let mut map = BTreeMap::new();
                            map.insert("k1".into(), "10.10.10.10]24".into());
                            map
                        },
                    },
                    "c".into(),
                ]),
            ),
            (
                "path containing multiple Lists",
                "/a/b[k1=v1]/c/d[k2=v2]/e",
                Some(vec![
                    "a".into(),
                    PathElem {
                        name: "b".to_string(),
                        key: {
                            let mut map = BTreeMap::new();
                            map.insert("k1".into(), "v1".into());
                            map
                        },
                    },
                    "c".into(),
                    PathElem {
                        name: "d".to_string(),
                        key: {
                            let mut map = BTreeMap::new();
                            map.insert("k2".into(), "v2".into());
                            map
                        },
                    },
                    "e".into(),
                ]),
            ),
            (
                "path containing a multi-key List",
                r#"/a/b[k1=exact][k2=10.10.10.10/24]/c"#,
                Some(vec![
                    "a".into(),
                    PathElem {
                        name: "b".to_string(),
                        key: {
                            let mut map = BTreeMap::new();
                            map.insert("k1".into(), "exact".into());
                            map.insert("k2".into(), "10.10.10.10/24".into());
                            map
                        },
                    },
                    "c".into(),
                ]),
            ),
            (
                r#"path containing a multi-key List with \][ in key leaf value"#,
                r#"/a/b[k1=10\][][k2=abc]/c"#,
                Some(vec![
                    "a".into(),
                    PathElem {
                        name: "b".to_string(),
                        key: {
                            let mut map = BTreeMap::new();
                            map.insert("k1".to_string(), "10][".to_string());
                            map.insert("k2".to_string(), "abc".to_string());
                            map
                        },
                    },
                    "c".into(),
                ]),
            ),
            (
                "path containing a multi-key List but missing ] in second key-value string",
                r#"/a/b[k1=10][k2=abc/c"#,
                None,
            ),
            (
                "path containing a multi-key List with unescaped [ in second key leaf name",
                r#"/a/b[k1=10][[k2=abc]/c"#,
                None,
            ),
            (
                "path containing a multi-key List, second key-value pair without [ and ]",
                r#"/a/b[k1=10]k2=abc/c"#,
                None,
            ),
        ] {
            match parse(input) {
                Ok(got) => {
                    if let Some(expected) = expected {
                        assert_eq!(got.elem, expected, "input: \"{}\"", input);
                    }
                }
                Err(err) => {
                    if expected.is_none() {
                        continue;
                    }

                    panic!("input: \"{}\"\n{}", input, err)
                }
            }
        }
    }
}
