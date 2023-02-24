use std::borrow::Cow;
use std::fmt::Formatter;
use std::path::PathBuf;

use bytes::Bytes;
use chrono::format::{Item, StrftimeItems};
use chrono::Utc;
use configurable::schema::{generate_string_schema, SchemaGenerator, SchemaObject};
use configurable::{Configurable, ConfigurableString, GenerateError};
use event::{log::Value, EventRef, Metric};
use log_schema::log_schema;
use once_cell::sync::Lazy;
use regex::{Captures, Regex};
use serde::de::{Error, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\{\{(?P<key>[^\}]+)\}\}").unwrap());

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq, Ord, PartialOrd)]
pub struct Template {
    src: String,
    has_ts: bool,
    has_fields: bool,
}

impl Configurable for Template {
    fn generate_schema(_gen: &mut SchemaGenerator) -> Result<SchemaObject, GenerateError> {
        Ok(generate_string_schema())
    }
}

impl ConfigurableString for Template {}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Error)]
pub enum TemplateParseError {
    #[error("Invalid strftime item")]
    StrftimeError,
}

#[derive(Clone, Debug, Eq, PartialEq, Error)]
pub enum TemplateRenderingError {
    #[error("Missing fields on event: {0:?}")]
    MissingKeys(Vec<String>),
}

impl TryFrom<&str> for Template {
    type Error = TemplateParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Template::try_from(Cow::Borrowed(value))
    }
}

impl TryFrom<String> for Template {
    type Error = TemplateParseError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Template::try_from(Cow::Owned(value))
    }
}

impl TryFrom<PathBuf> for Template {
    type Error = TemplateParseError;

    fn try_from(value: PathBuf) -> Result<Self, Self::Error> {
        Template::try_from(value.to_string_lossy().into_owned())
    }
}

impl TryFrom<Cow<'_, str>> for Template {
    type Error = TemplateParseError;

    fn try_from(src: Cow<'_, str>) -> Result<Self, Self::Error> {
        let (has_err, is_dynamic) = StrftimeItems::new(&src)
            .fold((false, false), |(err, dynamic), item| {
                (err || is_error(&item), dynamic || is_dynamic(&item))
            });

        if has_err {
            Err(TemplateParseError::StrftimeError)
        } else {
            Ok(Template {
                has_fields: RE.is_match(&src),
                src: src.into_owned(),
                has_ts: is_dynamic,
            })
        }
    }
}

const fn is_error(item: &Item) -> bool {
    matches!(item, Item::Error)
}

const fn is_dynamic(item: &Item) -> bool {
    match item {
        Item::Fixed(_) => true,
        Item::Numeric(_, _) => true,
        Item::Error => false,
        Item::Space(_) | Item::OwnedSpace(_) => false,
        Item::Literal(_) | Item::OwnedLiteral(_) => false,
    }
}

impl Template {
    pub fn render<'a>(
        &self,
        event: impl Into<EventRef<'a>>,
    ) -> Result<Bytes, TemplateRenderingError> {
        self.render_string(event.into()).map(Into::into)
    }

    pub fn render_string<'a>(
        &self,
        event: impl Into<EventRef<'a>>,
    ) -> Result<String, TemplateRenderingError> {
        let event = event.into();

        match (self.has_fields, self.has_ts) {
            (false, false) => Ok(self.src.clone()),
            (true, false) => render_fields(&self.src, event),
            (false, true) => Ok(render_timestamp(&self.src, event)),
            (true, true) => {
                let tmp = render_fields(&self.src, event)?;
                Ok(render_timestamp(&tmp, event))
            }
        }
    }

    pub fn get_fields(&self) -> Option<Vec<String>> {
        if self.has_fields {
            RE.captures_iter(&self.src)
                .map(|c| {
                    c.get(1)
                        .map(|s| s.as_str().trim().to_string())
                        .expect("src should match regex")
                })
                .collect::<Vec<_>>()
                .into()
        } else {
            None
        }
    }

    pub const fn is_dynamic(&self) -> bool {
        self.has_fields || self.has_ts
    }

    pub fn get_ref(&self) -> &str {
        &self.src
    }
}

fn render_fields(src: &str, event: EventRef) -> Result<String, TemplateRenderingError> {
    let mut missing_keys = Vec::new();
    let out = RE
        .replace_all(src, |caps: &Captures<'_>| {
            let key = caps
                .get(1)
                .map(|m| m.as_str().trim())
                .expect("src should match regex");

            match event {
                EventRef::Log(log) => log.get_field(key).map(|val| val.to_string_lossy()),
                EventRef::Metric(metric) => render_metric_field(key, metric),
                EventRef::Trace(_span) => todo!(),
            }
            .unwrap_or_else(|| {
                missing_keys.push(key.to_owned());
                String::new()
            })
        })
        .into_owned();

    if missing_keys.is_empty() {
        Ok(out)
    } else {
        Err(TemplateRenderingError::MissingKeys(missing_keys))
    }
}

fn render_metric_field(key: &str, metric: &Metric) -> Option<String> {
    match key {
        "name" => Some(metric.name().into()),
        // "namespace" => Some()
        _ if key.starts_with("tags.") => metric.tag_value(&key[5..]).map(|v| v.to_string()),
        _ => None,
    }
}

fn render_timestamp(src: &str, event: EventRef<'_>) -> String {
    let timestamp = match event {
        EventRef::Log(log) => log
            .get_field(log_schema().timestamp_key())
            .and_then(Value::as_timestamp)
            .copied(),
        EventRef::Metric(metric) => metric.timestamp(),
        _ => todo!(),
    };

    if let Some(ts) = timestamp {
        ts.format(src).to_string()
    } else {
        Utc::now().format(src).to_string()
    }
}

impl<'de> Deserialize<'de> for Template {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(TemplateVisitor)
    }
}

struct TemplateVisitor;

impl<'de> Visitor<'de> for TemplateVisitor {
    type Value = Template;

    fn expecting(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "a string")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Template::try_from(v).map_err(serde::de::Error::custom)
    }
}

impl Serialize for Template {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // TODO: determine if we should serialize this as a struct or just the str
        serializer.serialize_str(&self.src)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use event::{fields, tags, Event};

    #[test]
    fn get_fields() {
        let tests = vec![
            ("{{ foo }}", Some(vec!["foo".to_string()])),
            (
                "{{ foo }}-{{ bar }}",
                Some(vec!["foo".to_string(), "bar".to_string()]),
            ),
            ("nofield", None),
            ("%F", None),
        ];

        for (input, want) in tests {
            let fields = Template::try_from(input).unwrap().get_fields();

            assert_eq!(fields, want)
        }
    }

    #[test]
    fn is_dynamic() {
        let tests = vec![
            ("/kube-demo/%F", true),
            ("/kube-demo/echo", false),
            ("/kube-demo/{{ foo }}", true),
            ("/kube-demo/{{ foo }}/%F", true),
        ];

        for (input, want) in tests {
            assert_eq!(
                Template::try_from(input).unwrap().is_dynamic(),
                want,
                "input: {}",
                input,
            );
        }
    }

    #[test]
    fn render_log_static() {
        let log = Event::from("hello world");
        let template = Template::try_from("foo").unwrap();

        assert_eq!(Ok(Bytes::from("foo")), template.render(&log))
    }

    #[test]
    fn render_log_dynamic() {
        let mut event = Event::from("hello world");
        event.as_mut_log().insert_field("log_stream", "stream");

        let template = Template::try_from("{{ log_stream }}").unwrap();
        assert_eq!(Ok(Bytes::from("stream")), template.render(&event))
    }

    #[test]
    fn render_log_dyanmic_with_prefix() {
        let mut event = Event::from("hello world");
        event.as_mut_log().insert_field("log_stream", "stream");
        let template = Template::try_from("abcd-{{log_stream}}").unwrap();

        assert_eq!(Ok(Bytes::from("abcd-stream")), template.render(&event));
    }

    #[test]
    fn render_log_dynamic_with_suffix() {
        let mut event = Event::from("hello world");
        event.as_mut_log().insert_field("log_stream", "stream");
        let template = Template::try_from("{{ log_stream }}-suffix").unwrap();
        assert_eq!(Ok(Bytes::from("stream-suffix")), template.render(&event));
    }

    #[test]
    fn render_log_dynamic_missing_key() {
        let event = Event::from("hello world");
        let template = Template::try_from("{{log_stream}}-{{foo}}").unwrap();

        assert_eq!(
            Err(TemplateRenderingError::MissingKeys(vec![
                "log_stream".to_string(),
                "foo".to_string()
            ])),
            template.render(&event)
        )
    }

    #[test]
    fn render_log_dynamic_multiple_keys() {
        let mut event = Event::from("hello world");
        event.as_mut_log().insert_field("foo", "bar");
        event.as_mut_log().insert_field("baz", "quux");
        let template = Template::try_from("stream-{{foo}}-{{baz}}.log").unwrap();

        assert_eq!(
            Ok(Bytes::from("stream-bar-quux.log")),
            template.render(&event)
        );
    }

    #[test]
    fn render_log_dynamic_weird_junk() {
        let mut event = Event::from("hello world");
        event.as_mut_log().insert_field("foo", "bar");
        event.as_mut_log().insert_field("baz", "quux");
        let template = Template::try_from(r"{stream}{\{{}}}-{{foo}}-{{baz}}.log").unwrap();

        assert_eq!(
            Ok(Bytes::from(r"{stream}{\{{}}}-bar-quux.log")),
            template.render(&event)
        )
    }

    #[test]
    fn render_log_timestamp_strftime_style() {
        let ts = Utc.with_ymd_and_hms(2001, 2, 3, 4, 5, 6).unwrap();
        let mut event = Event::from("hello world");
        event.as_mut_log().insert_field("timestamp", ts);
        let template = Template::try_from("abcd-%F").unwrap();

        assert_eq!(Ok(Bytes::from("abcd-2001-02-03")), template.render(&event));
    }

    #[test]
    fn render_log_timestamp_multiple_strftime_style() {
        let ts = Utc.with_ymd_and_hms(2001, 2, 3, 4, 5, 6).unwrap();
        let mut event = Event::from("hello world");
        event.as_mut_log().insert_field("timestamp", ts);
        let template = Template::try_from("abcd-%F_%T").unwrap();

        assert_eq!(
            Ok(Bytes::from("abcd-2001-02-03_04:05:06")),
            template.render(&event)
        )
    }

    #[test]
    fn render_log_dynamic_with_strftime() {
        let ts = Utc.with_ymd_and_hms(2001, 2, 3, 4, 5, 6).unwrap();
        let mut event = Event::from("hello world");
        event.as_mut_log().insert_field("foo", "butts");
        event.as_mut_log().insert_field("timestamp", ts);

        let template = Template::try_from("{{ foo }}-%F_%T").unwrap();

        assert_eq!(
            Ok(Bytes::from("butts-2001-02-03_04:05:06")),
            template.render(&event)
        );
    }

    #[test]
    fn render_log_dynamic_with_nested_strftime() {
        let ts = Utc.with_ymd_and_hms(2001, 2, 3, 4, 5, 6).unwrap();
        let mut event = Event::from("hello world");
        event.as_mut_log().insert_field("format", "%F");
        event.as_mut_log().insert_field("timestamp", ts);

        let template = Template::try_from("nested {{ format }} %T").unwrap();

        assert_eq!(
            Ok(Bytes::from("nested 2001-02-03 04:05:06")),
            template.render(&event)
        )
    }

    #[test]
    fn render_log_dynamic_with_reverse_nested_strftime() {
        let ts = Utc.with_ymd_and_hms(2001, 2, 3, 4, 5, 6).unwrap();
        let event = Event::from(fields!(
            log_schema().message_key() => "hello world",
            "%F" => "foo",
            log_schema().timestamp_key() => ts
        ));

        let template = Template::try_from("nested {{ \"%F\" }} %T").unwrap();

        assert_eq!(
            Ok(Bytes::from("nested foo 04:05:06")),
            template.render(&event)
        )
    }

    fn sample_metric() -> Metric {
        Metric::sum("a-counter", "", 1)
            .with_timestamp(Some(Utc.with_ymd_and_hms(2002, 3, 4, 5, 6, 7).unwrap()))
    }

    #[test]
    fn render_metric_timestamp() {
        let template = Template::try_from("timestamp %F %T").unwrap();

        assert_eq!(
            Ok(Bytes::from("timestamp 2002-03-04 05:06:07")),
            template.render(&sample_metric())
        )
    }

    #[test]
    fn render_metric_with_tags() {
        let template = Template::try_from("name={{name}} component={{tags.component}}").unwrap();
        let metric = sample_metric().with_tags(tags!(
            "test" => "true",
            "component" => "template"
        ));

        assert_eq!(
            Ok(Bytes::from("name=a-counter component=template")),
            template.render(&metric)
        );
    }

    #[test]
    fn render_metric_missing_key() {
        let template = Template::try_from("name={{name}} component={{tags.component}}").unwrap();

        assert_eq!(
            Err(TemplateRenderingError::MissingKeys(vec![
                "tags.component".to_string()
            ])),
            template.render(&sample_metric())
        );
    }

    #[test]
    fn strftime_error() {
        assert_eq!(
            Template::try_from("%E").unwrap_err(),
            TemplateParseError::StrftimeError
        )
    }
}
