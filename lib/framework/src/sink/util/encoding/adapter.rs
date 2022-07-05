use event::Event;
use lookup::OwnedPath;
use serde::{Deserialize, Serialize};

use crate::config::skip_serializing_if_default;
use crate::sink::util::encoding::EncodingConfiguration;
use crate::sink::util::{validate_fields, TimestampFormat};

#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
struct TransformerInner {
    #[serde(default, skip_serializing_if = "skip_serializing_if_default")]
    only_fields: Option<Vec<OwnedPath>>,
    #[serde(default, skip_serializing_if = "skip_serializing_if_default")]
    expect_fields: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "skip_serializing_if_default")]
    timestamp_format: Option<TimestampFormat>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct Transformer(TransformerInner);

impl Transformer {
    pub fn new(
        only_fields: Option<Vec<OwnedPath>>,
        expect_fields: Option<Vec<String>>,
        timestamp_format: Option<TimestampFormat>,
    ) -> Result<Self, crate::Error> {
        let inner = TransformerInner {
            only_fields,
            expect_fields,
            timestamp_format,
        };

        validate_fields(inner.only_fields.as_deref(), inner.expect_fields.as_deref())?;

        Ok(Self(inner))
    }

    pub fn transform(&self, event: &mut Event) {
        self.apply_rules(event);
    }
}

impl EncodingConfiguration for Transformer {
    type Codec = ();

    fn codec(&self) -> &Self::Codec {
        &()
    }

    fn schema(&self) -> &Option<String> {
        &None
    }

    fn only_fields(&self) -> &Option<Vec<OwnedPath>> {
        &self.0.only_fields
    }

    fn except_fields(&self) -> &Option<Vec<String>> {
        &self.0.expect_fields
    }

    fn timestamp_format(&self) -> &Option<TimestampFormat> {
        &self.0.timestamp_format
    }
}
