use crate::sink::util::TimestampFormat;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
struct TransformerInner {
    #[serde(default, skip_serializing_if = "skip_serializing_if_default")]
    only_fields: Option<Vec<OwnedPath>>,
    #[serde(default, skip_serializing_if = "skip_serializing_if_default")]
    expect_fields: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "skip_serializing_if_default")]
    timestamp_format: Option<TimestampFormat>,
}
