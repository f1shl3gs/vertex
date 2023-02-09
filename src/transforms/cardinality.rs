use std::collections::HashMap;
use std::fmt::{Debug, Formatter};

use async_trait::async_trait;
use bloom::{BloomFilter, ASMS};
use configurable::{configurable_component, Configurable};
use event::tags::{Key, Value};
use event::{EventContainer, Events};
use framework::config::{DataType, Output, TransformConfig, TransformContext};
use framework::{FunctionTransform, OutputBuffer, Transform};
use serde::de::{Error, Visitor};
use serde::{Deserialize, Deserializer, Serialize};

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Configurable, Copy, Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum LimitExceededAction {
    DropEvent,
    DropTag,
}

impl<'de> Deserialize<'de> for LimitExceededAction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Action;

        impl<'de> Visitor<'de> for Action {
            type Value = LimitExceededAction;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str(r##""drop" or "drop_tag""##)
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                match v {
                    "drop" => Ok(LimitExceededAction::DropEvent),
                    "drop_tag" => Ok(LimitExceededAction::DropTag),
                    _ => Err(serde::de::Error::unknown_variant(v, &["drop", "drop_tag"])),
                }
            }
        }

        deserializer.deserialize_any(Action)
    }
}

impl Default for LimitExceededAction {
    fn default() -> Self {
        Self::DropEvent
    }
}

#[configurable_component(transform, name = "cardinality")]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
#[serde(deny_unknown_fields)]
struct CardinalityConfig {
    /// How many distict values for any given key.
    #[configurable(required)]
    pub limit: usize,

    /// The behavior of limit exceeded action.
    #[serde(default)]
    pub action: LimitExceededAction,
}

#[async_trait]
#[typetag::serde(name = "cardinality")]
impl TransformConfig for CardinalityConfig {
    async fn build(&self, _cx: &TransformContext) -> crate::Result<Transform> {
        Ok(Transform::function(Cardinality::new(
            self.limit,
            self.action,
        )))
    }

    fn input_type(&self) -> DataType {
        DataType::Metric
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Metric)]
    }
}

struct TagValueSet {
    elements: usize,
    filter: BloomFilter,
}

impl Debug for TagValueSet {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Bloom")
    }
}

impl TagValueSet {
    pub fn new(limit: usize) -> Self {
        let bits = (5000 * 1024) / 8;
        let hashes = bloom::optimal_num_hashes(bits, limit as u32);

        Self {
            elements: 0,
            filter: BloomFilter::with_size(bits, hashes),
        }
    }

    #[inline]
    fn contains(&self, value: &str) -> bool {
        self.filter.contains(&value)
    }

    #[inline]
    const fn len(&self) -> usize {
        self.elements
    }

    fn insert(&mut self, val: &str) -> bool {
        let inserted = self.filter.insert(&val);

        if inserted {
            self.elements += 1;
        }

        inserted
    }
}

struct Cardinality {
    limit: usize,
    action: LimitExceededAction,
    accepted_tags: HashMap<Key, TagValueSet>,
}

impl Clone for Cardinality {
    fn clone(&self) -> Self {
        Self {
            limit: self.limit,
            action: self.action,
            accepted_tags: HashMap::new(),
        }
    }
}

impl Cardinality {
    pub fn new(limit: usize, action: LimitExceededAction) -> Self {
        Self {
            limit,
            action,
            accepted_tags: HashMap::new(),
        }
    }

    /// Takes in key and a value corresponding to a tag on an incoming Metric
    /// event. If that value is already part of set of accepted values for that
    /// key, then simply retruns true. If that value is not yet part of the
    /// accepted values for that key, checks whether we have hit the limit
    /// for that key yet and if not adds the value to the set of accepted values
    /// for the key and returns true, otherwise returns false. A false return
    /// value indicates to the caller that the value is not accepted for this
    /// key, and the configured limit_exceed_action should be taken.
    fn try_accept_tag(&mut self, key: &Key, value: &Value) -> bool {
        if !self.accepted_tags.contains_key(key) {
            self.accepted_tags
                .insert(key.clone(), TagValueSet::new(self.limit));
        }

        let set = self.accepted_tags.get_mut(key).unwrap();
        if set.contains(&value.as_str()) {
            // Tag value has already been accepted, nothing more to do
            return true;
        }

        // Tag value not yet part of the accepted set
        if set.len() < self.limit {
            // accept the new value
            set.insert(&value.as_str());
            if set.len() == self.limit {
                // emit limit reached event
            }

            true
        } else {
            // New tag value is rejected
            false
        }
    }
}

impl FunctionTransform for Cardinality {
    fn transform(&mut self, output: &mut OutputBuffer, events: Events) {
        let mut new_metrics = Vec::with_capacity(events.len());

        if let Events::Metrics(metrics) = events {
            'outer: for mut metric in metrics {
                let mut to_delete = vec![];

                for (k, v) in metric.tags() {
                    if !self.try_accept_tag(k, v) {
                        // reject
                        match self.action {
                            LimitExceededAction::DropEvent => continue 'outer,
                            LimitExceededAction::DropTag => to_delete.push(k.clone()),
                        }
                    }
                }

                for k in &to_delete {
                    metric.remote_tag(k);
                }

                new_metrics.push(metric);
            }
        }

        if !new_metrics.is_empty() {
            output.push(new_metrics);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use event::{tags, Metric, MetricValue};
    use framework::config::TransformContext;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<CardinalityConfig>()
    }

    #[test]
    fn test_tag_value_set() {
        let mut set = TagValueSet::new(10);
        assert_eq!(set.len(), 0);
        assert!(!set.contains("foo"));

        assert!(set.insert("foo"));
        assert!(set.contains("foo"));
        assert_eq!(set.len(), 1);

        assert!(set.insert("bar"));
        assert!(set.contains("bar"));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_tag_value_set_limit() {
        let total = 50;
        let mut set = TagValueSet::new(total);
        for i in 0..total {
            let val = format!("{}", i);
            assert!(set.insert(&val));
        }

        assert_eq!(set.len(), total);

        for i in 0..total {
            let val = format!("{}", i);
            assert!(!set.insert(&val))
        }
    }

    async fn run(config: CardinalityConfig, input: Vec<Metric>) -> OutputBuffer {
        let mut cardinality = config.build(&TransformContext::default()).await.unwrap();
        let cardinality = cardinality.as_function();

        let mut buf = OutputBuffer::with_capacity(1);
        cardinality.transform(&mut buf, input.into());
        buf
    }

    #[tokio::test]
    async fn transform_drop() {
        let config = CardinalityConfig {
            limit: 0,
            action: LimitExceededAction::DropEvent,
        };

        let metric = Metric::gauge_with_tags(
            "foo",
            "",
            1,
            tags!(
                "key" => "value"
            ),
        );

        let output = run(config, vec![metric]).await;

        assert!(output.is_empty())
    }

    #[tokio::test]
    async fn drop_tag() {
        let config = CardinalityConfig {
            limit: 0,
            action: LimitExceededAction::DropTag,
        };

        let metric = Metric::gauge_with_tags(
            "foo",
            "",
            1,
            tags!(
                "key" => "value"
            ),
        );

        let output = run(config, vec![metric]).await;
        assert_eq!(output.len(), 1);
        let metric = output.first().unwrap().into_metric();
        assert_eq!(metric.name(), "foo");
        assert_eq!(metric.value, MetricValue::Gauge(1.0));
        assert!(metric.series.tags.is_empty())
    }
}
