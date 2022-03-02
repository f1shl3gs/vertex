use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FieldsSpec {
    pub labels: String,
}

impl Default for FieldsSpec {
    fn default() -> Self {
        Self {
            labels: "kubernetes.namespace_labels".to_string(),
        }
    }
}
