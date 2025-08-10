mod go;
mod java;
mod noident;
mod python;
mod regex;

use std::time::Duration;

use bytes::{Bytes, BytesMut};
use configurable::Configurable;
use serde::{Deserialize, Serialize};
use tail::multiline::Logic;

pub mod serde_regex_bytes {
    use std::borrow::Cow;

    use regex::bytes::Regex;
    use serde::{Deserializer, Serializer};

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Regex, D::Error> {
        let s: Cow<str> = serde::Deserialize::deserialize(deserializer)?;
        Regex::new(&s).map_err(serde::de::Error::custom)
    }

    pub fn serialize<S: Serializer>(re: &Regex, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(re.as_str())
    }
}

const fn default_timeout() -> Duration {
    Duration::from_millis(200)
}

#[derive(Configurable, Debug, Deserialize, Serialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum Mode {
    Go,
    Java,
    NoIdent,
    Python,
    Regex {
        #[serde(with = "serde_regex_bytes")]
        start: ::regex::bytes::Regex,
    },
}

impl Mode {
    pub fn build(&self) -> crate::Result<MergeLogic> {
        let logic = match self {
            Mode::Go => MergeLogic::Go(go::Golang::default()),
            Mode::Java => MergeLogic::Java(java::Java::default()),
            Mode::NoIdent => MergeLogic::NoIdent(noident::NoIndent),
            Mode::Python => MergeLogic::Python(python::Python::default()),
            Mode::Regex { start } => MergeLogic::Regex(regex::Regex {
                regex: start.clone(),
            }),
        };

        Ok(logic)
    }
}

#[derive(Clone)]
pub enum MergeLogic {
    None,
    Go(go::Golang),
    Java(java::Java),
    NoIdent(noident::NoIndent),
    Python(python::Python),
    Regex(regex::Regex),
}

impl Logic for MergeLogic {
    fn is_start(&mut self, line: &[u8]) -> bool {
        match self {
            MergeLogic::None => true,
            MergeLogic::Go(go) => go.is_start(line),
            MergeLogic::Java(java) => java.is_start(line),
            MergeLogic::NoIdent(noident) => noident.is_start(line),
            MergeLogic::Python(python) => python.is_start(line),
            MergeLogic::Regex(regex) => regex.is_start(line),
        }
    }

    fn merge(&self, stashed: &mut BytesMut, data: Bytes) {
        match self {
            MergeLogic::None => unreachable!(),
            MergeLogic::Go(go) => go.merge(stashed, data),
            MergeLogic::Java(java) => java.merge(stashed, data),
            MergeLogic::NoIdent(noident) => noident.merge(stashed, data),
            MergeLogic::Python(python) => python.merge(stashed, data),
            MergeLogic::Regex(regex) => regex.merge(stashed, data),
        }
    }
}

#[derive(Configurable, Debug, Deserialize, Serialize)]
pub struct Config {
    /// The maximum amount of time to wait for the next additional line.
    ///
    /// Once this timeout is reached, the buffered message is guaranteed to be flushed,
    /// even if incomplete.
    #[serde(default = "default_timeout", with = "humanize::duration::serde")]
    pub timeout: Duration,

    #[serde(flatten)]
    pub mode: Mode,
}

#[cfg(test)]
fn assert_logic<L: Logic>(mut logic: L, input: &[&str], want: &[&str]) {
    use bytes::{BufMut, Bytes, BytesMut};

    let mut got = vec![];
    let mut stashed = BytesMut::new();
    for line in input {
        if logic.is_start(line.as_bytes()) {
            if stashed.is_empty() {
                stashed.put_slice(line.as_bytes());
                continue;
            }

            got.push(stashed.clone().freeze());
            stashed.clear();
            continue;
        }

        logic.merge(&mut stashed, Bytes::copy_from_slice(line.as_bytes()));
    }

    for (index, (got, want)) in got.into_iter().zip(want).enumerate() {
        let got = String::from_utf8_lossy(&got);
        assert_eq!(got.as_ref(), *want, "line {index}");
    }
}
