use event::log::path_iter::PathComponent;
use event::Event;
use serde::{Deserialize, Serialize};

use crate::config::skip_serializing_if_default;
use crate::sink::util::encoding::EncodingConfiguration;
use crate::sink::util::TimestampFormat;

#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
struct TransformerInner {
    #[serde(default, skip_serializing_if = "skip_serializing_if_default")]
    only_fields: Option<Vec<PathComponent<'static>>>,
    #[serde(default, skip_serializing_if = "skip_serializing_if_default")]
    expect_fields: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "skip_serializing_if_default")]
    timestamp_format: Option<TimestampFormat>,
}

pub struct Transformer(TransformerInner);

impl Transformer {
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

    fn only_fields(&self) -> &Option<Vec<PathComponent>> {
        &self.0.only_fields
    }

    fn except_fields(&self) -> &Option<Vec<String>> {
        &self.0.expect_fields
    }

    fn timestamp_format(&self) -> &Option<TimestampFormat> {
        &self.0.timestamp_format
    }
}
