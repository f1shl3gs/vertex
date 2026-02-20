use std::collections::HashSet;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::sync::Arc;

use configurable::{Configurable, configurable_component};
use dashmap::DashMap;
use event::Events;
use event::tags::{Key, Value as TagValue};
use framework::config::{InputType, OutputType, TransformConfig, TransformContext};
use framework::{FunctionTransform, OutputBuffer, Transform};
use serde::de::{Error, Visitor};
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Configurable, Copy, Clone, Debug, Serialize, Default)]
#[serde(rename_all = "snake_case")]
enum LimitExceededAction {
    #[default]
    Drop,

    DropTag,
}

impl<'de> Deserialize<'de> for LimitExceededAction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Action;

        impl Visitor<'_> for Action {
            type Value = LimitExceededAction;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str(r#""drop" or "drop_tag""#)
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                match v {
                    "drop" => Ok(LimitExceededAction::Drop),
                    "drop_tag" => Ok(LimitExceededAction::DropTag),
                    _ => Err(Error::unknown_variant(v, &["drop", "drop_tag"])),
                }
            }
        }

        deserializer.deserialize_any(Action)
    }
}

const fn default_cache_size() -> usize {
    4 * 1024 * 1024 // 4KB
}

/// Controls the approach token for tracking tag cardinality.
#[derive(Configurable, Copy, Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
enum Mode {
    /// Tracks cardinality probabilistically.
    ///
    /// This mode has lower memory requirements than `exact`, but may occasionally
    /// allow metric events to pass through the transform even when they contain
    /// new tags that exceed the configured limit. The rate at which this happens
    /// can be controlled by changing the value of `cache_size_per_tag`
    Probabilistic {
        /// The size of the cache for detecting duplicate tags, in bytes,
        ///
        /// The larger the cache size, the less likely it is to have a false
        /// positive, or a case where we allow a new value for tag even after
        /// we have reached the configured limits.
        #[serde(default = "default_cache_size", with = "humanize::bytes::serde")]
        cache_size_per_tag: usize,
    },

    /// Tracks cardinality exactly.
    ///
    /// This mode has higher memory requirements than `probabilistic`, but
    /// never falsely outputs metrics with new tags after the limit has
    /// been hit.
    Exact,
}

/// Rate limits one or more metric streams to limit load on downstream services, or
/// to enforce usage quotas on users.
#[configurable_component(transform, name = "cardinality")]
#[serde(deny_unknown_fields)]
struct Config {
    /// How many distinct values for any given key.
    limit: usize,

    /// The behavior of limit exceeded action.
    #[serde(default)]
    action: LimitExceededAction,

    #[serde(flatten)]
    mode: Mode,
}

#[async_trait::async_trait]
#[typetag::serde(name = "cardinality")]
impl TransformConfig for Config {
    async fn build(&self, _cx: &TransformContext) -> crate::Result<Transform> {
        Ok(Transform::function(Cardinality::new(
            self.limit,
            self.action,
            self.mode,
        )))
    }

    fn input(&self) -> InputType {
        InputType::metric()
    }

    fn outputs(&self) -> Vec<OutputType> {
        vec![OutputType::metric()]
    }

    fn enable_concurrency(&self) -> bool {
        true
    }
}

struct BloomFilter<T: ?Sized> {
    filter: sbbf::Filter,
    count: usize,
    _pd: PhantomData<T>,
}

unsafe impl<T> Send for BloomFilter<T> {}
unsafe impl<T> Sync for BloomFilter<T> {}

impl<T: Hash + ?Sized> BloomFilter<T> {
    fn new(size: usize) -> Self {
        let filter = sbbf::Filter::new(32, size);

        Self {
            filter,
            count: 0,
            _pd: PhantomData,
        }
    }

    fn insert(&mut self, value: &T) {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        value.hash(&mut hasher);
        let hash = hasher.finish();

        if !self.filter.insert(hash) {
            self.count += 1;
        }
    }

    fn contains(&self, value: &T) -> bool {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        value.hash(&mut hasher);
        let hash = hasher.finish();

        self.filter.contains(hash)
    }
}

/// Container for storing the set of accepted values for a given tag key.
enum AcceptedTagValueSet {
    Set(HashSet<TagValue>),
    Bloom(BloomFilter<TagValue>),
}

impl AcceptedTagValueSet {
    fn new(limit: usize, mode: Mode) -> Self {
        match mode {
            Mode::Exact => Self::Set(HashSet::with_capacity(limit)),
            Mode::Probabilistic { cache_size_per_tag } => {
                let num_bits = cache_size_per_tag / 8; // Convert bytes to bits
                let bloom = BloomFilter::new(num_bits);
                Self::Bloom(bloom)
            }
        }
    }

    fn contains(&self, value: &TagValue) -> bool {
        match self {
            AcceptedTagValueSet::Set(set) => set.contains(value),
            AcceptedTagValueSet::Bloom(bloom) => bloom.contains(value),
        }
    }

    fn len(&self) -> usize {
        match self {
            AcceptedTagValueSet::Set(set) => set.len(),
            AcceptedTagValueSet::Bloom(bloom) => bloom.count,
        }
    }

    fn insert(&mut self, value: &TagValue) {
        match self {
            AcceptedTagValueSet::Set(set) => {
                set.insert(value.clone());
            }
            AcceptedTagValueSet::Bloom(bloom) => bloom.insert(value),
        }
    }
}

#[derive(Clone)]
struct Cardinality {
    limit: usize,
    action: LimitExceededAction,
    mode: Mode,
    accepted_tags: Arc<DashMap<Key, AcceptedTagValueSet>>,
}

impl Cardinality {
    pub fn new(limit: usize, action: LimitExceededAction, mode: Mode) -> Self {
        Self {
            limit,
            action,
            mode,
            accepted_tags: Arc::new(DashMap::new()),
        }
    }

    /// Takes in key and a value corresponding to a tag on an incoming Metric
    /// event. If that value is already part of set of accepted values for that
    /// key, then simply returns true. If that value is not yet part of the
    /// accepted values for that key, checks whether we have hit the limit
    /// for that key yet and if not adds the value to the set of accepted values
    /// for the key and returns true, otherwise returns false. A false return
    /// value indicates to the caller that the value is not accepted for this
    /// key, and the configured limit_exceed_action should be taken.
    fn try_accept_tag(&mut self, key: &Key, value: &TagValue) -> bool {
        if !self.accepted_tags.contains_key(key) {
            self.accepted_tags
                .insert(key.clone(), AcceptedTagValueSet::new(self.limit, self.mode));
        }

        let mut set = self.accepted_tags.get_mut(key).expect("should exist");
        if set.contains(value) {
            // Tag value has already been accepted, nothing more to do
            return true;
        }

        // Tag value not yet part of the accepted set
        if set.len() < self.limit {
            // accept the new value
            set.insert(value);
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
                            LimitExceededAction::Drop => continue 'outer,
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
    use event::{Metric, MetricValue, tags};
    use framework::config::TransformContext;

    // TODO: fix this
    //
    // #[test]
    // fn generate_config() {
    //     crate::testing::test_generate_config::<Config>()
    // }

    #[test]
    fn bloom_filter() {
        let mut filter: BloomFilter<str> = BloomFilter::new(10);
        assert_eq!(filter.count, 0);

        filter.insert("foo");
        assert!(filter.contains("foo"));
        assert_eq!(filter.count, 1);

        filter.insert("foo");
        assert!(filter.contains("foo"));
        assert_eq!(filter.count, 1);

        filter.insert("bar");
        assert!(filter.contains("bar"));
        assert_eq!(filter.count, 2);
    }

    #[test]
    fn hashset_accepted_tag_set() {
        let foo = TagValue::from("foo");
        let bar = TagValue::from("bar");

        let mut set = AcceptedTagValueSet::new(100, Mode::Exact);
        assert_eq!(set.len(), 0);

        set.insert(&foo);
        assert!(set.contains(&foo));
        assert_eq!(set.len(), 1);

        set.insert(&bar);
        assert!(set.contains(&bar));
        assert!(set.contains(&foo));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn bloom_accepted_tag_set() {
        let foo = TagValue::from("foo");
        let bar = TagValue::from("bar");

        let mut set = AcceptedTagValueSet::new(
            100,
            Mode::Probabilistic {
                cache_size_per_tag: default_cache_size(),
            },
        );
        assert_eq!(set.len(), 0);

        set.insert(&foo);
        assert!(set.contains(&foo));
        assert_eq!(set.len(), 1);

        set.insert(&bar);
        assert!(set.contains(&bar));
        assert!(set.contains(&foo));
        assert_eq!(set.len(), 2);
    }

    async fn run(config: Config, input: Vec<Metric>) -> OutputBuffer {
        let mut cardinality = config.build(&TransformContext::default()).await.unwrap();
        let cardinality = cardinality.as_function();

        let mut buf = OutputBuffer::with_capacity(1);
        cardinality.transform(&mut buf, input.into());
        buf
    }

    #[tokio::test]
    async fn transform_drop() {
        let config = Config {
            limit: 0,
            action: LimitExceededAction::Drop,
            mode: Mode::Exact,
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
        let config = Config {
            limit: 0,
            action: LimitExceededAction::DropTag,
            mode: Mode::Exact,
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
        assert!(metric.tags.is_empty())
    }
}
