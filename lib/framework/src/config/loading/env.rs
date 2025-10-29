use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::LazyLock;

use regex::{Captures, Regex};

static ENVIRONMENT_VARIABLE_INTERPOLATION_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?x)
        \$\$|
        \$([[:word:].]+)|
        \$\{([[:word:].]+)(?:(:?-|:?\?)([^}]*))?\}",
    )
    .unwrap()
});

pub fn loading() -> HashMap<String, String> {
    let mut vars = std::env::vars_os()
        .filter_map(|(k, v)| match (k.into_string(), v.into_string()) {
            (Ok(k), Ok(v)) => Some((k, v)),
            _ => None,
        })
        .collect::<HashMap<_, _>>();

    if !vars.contains_key("HOSTNAME")
        && let Ok(hostname) = hostname::get()
    {
        vars.insert("HOSTNAME".into(), hostname);
    }

    vars
}

pub fn interpolate<'a>(
    input: &'a str,
    vars: &HashMap<String, String>,
) -> Result<Cow<'a, str>, Vec<String>> {
    let mut errs = Vec::new();

    // https://www.gnu.org/savannah-checkouts/gnu/bash/manual/bash.html#Shell-Parameter-Expansion-1
    let interpolated = ENVIRONMENT_VARIABLE_INTERPOLATION_REGEX
        .replace_all(input, |caps: &Captures<'_>| {
            let flags = caps.get(3).map(|m| m.as_str()).unwrap_or_default();
            let def_or_err = caps.get(4).map(|m| m.as_str()).unwrap_or_default();

            caps.get(1)
                .or_else(|| caps.get(2))
                .map(|m| m.as_str())
                .map(|name| {
                    let val = vars.get(name).map(|v| v.as_str());
                    match flags {
                        // ${parameter:-word}
                        // ${parameter-word}
                        //
                        // If parameter is unset or null, the expansion of word is substituted.
                        // Otherwise, the value of parameter is substituted.
                        ":-" => match val {
                            Some(v) if !v.is_empty() => v,
                            _ => def_or_err,
                        },
                        "-" => val.unwrap_or(def_or_err),

                        // ${parameter:?word}
                        // ${parameter?word}
                        //
                        // if parameter is null or unset, an error is occurred
                        ":?" => match val {
                            Some(v) if !v.is_empty() => v,
                            _ => {
                                errs.push(format!(
                                    "Non-empty environment variable required in config. name = {name:?}, error = {def_or_err:?}",
                                ));

                                ""
                            }
                        },
                        "?" => val.unwrap_or_else(|| {
                            errs.push(format!(
                                "Missing environment variable required in config. name = {name:?}, error = {def_or_err:?}",
                            ));

                            ""
                        }),
                        _ => val.unwrap_or_else(|| {
                            errs.push(format!(
                                "Missing environment variable in config. name = {name:?}",
                            ));

                            ""
                        })
                    }
                })
                .unwrap_or("$")
                .to_string()
        });

    if errs.is_empty() {
        Ok(interpolated)
    } else {
        Err(errs)
    }
}

#[cfg(test)]
mod test {
    use super::interpolate;
    #[test]
    fn interpolation() {
        let vars = vec![
            ("FOO".into(), "dogs".into()),
            ("FOOBAR".into(), "cats".into()),
            // Java commonly uses .s in env var names
            ("FOO.BAR".into(), "turtles".into()),
            ("EMPTY".into(), "".into()),
        ]
        .into_iter()
        .collect();

        assert_eq!("dogs", interpolate("$FOO", &vars).unwrap());
        assert_eq!("dogs", interpolate("${FOO}", &vars).unwrap());
        assert_eq!("cats", interpolate("${FOOBAR}", &vars).unwrap());
        assert_eq!("xcatsy", interpolate("x${FOOBAR}y", &vars).unwrap());
        assert!(interpolate("x$FOOBARy", &vars).is_err());
        assert_eq!("$ x", interpolate("$ x", &vars).unwrap());
        assert_eq!("$FOO", interpolate("$$FOO", &vars).unwrap());
        assert_eq!("dogs=bar", interpolate("$FOO=bar", &vars).unwrap());
        assert!(interpolate("$NOT_FOO", &vars).is_err());
        assert!(interpolate("$NOT-FOO", &vars).is_err());
        assert_eq!("turtles", interpolate("$FOO.BAR", &vars).unwrap());
        assert_eq!("${FOO x", interpolate("${FOO x", &vars).unwrap());
        assert_eq!("${}", interpolate("${}", &vars).unwrap());
        assert_eq!("dogs", interpolate("${FOO:-cats}", &vars).unwrap());
        assert_eq!("dogcats", interpolate("${NOT:-dogcats}", &vars).unwrap());
        assert_eq!(
            "dogs and cats",
            interpolate("${NOT:-dogs and cats}", &vars).unwrap()
        );
        assert_eq!("${:-cats}", interpolate("${:-cats}", &vars).unwrap());
        assert_eq!("", interpolate("${NOT:-}", &vars).unwrap());
        assert_eq!("cats", interpolate("${NOT-cats}", &vars).unwrap());
        assert_eq!("", interpolate("${EMPTY-cats}", &vars).unwrap());
        assert_eq!("dogs", interpolate("${FOO:?error cats}", &vars).unwrap());
        assert_eq!("dogs", interpolate("${FOO?error cats}", &vars).unwrap());
        assert_eq!("", interpolate("${EMPTY?error cats}", &vars).unwrap());
        assert!(interpolate("${NOT:?error cats}", &vars).is_err());
        assert!(interpolate("${NOT?error cats}", &vars).is_err());
        assert!(interpolate("${EMPTY:?error cats}", &vars).is_err());
    }
}
