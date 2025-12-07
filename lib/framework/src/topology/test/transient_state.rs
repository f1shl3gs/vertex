use std::sync::Arc;

use configurable::configurable_component;
use futures::{FutureExt, future};
use tokio::sync::Mutex;
use tripwire::{Trigger, Tripwire};

use crate::topology::test::{NoopTransformConfig, NullSinkConfig, start_topology};
use crate::{
    Error, Source,
    config::{Config, OutputType, SourceConfig, SourceContext},
};

#[configurable_component(source, name = "mock")]
struct MockSourceConfig {
    #[configurable(skip)]
    #[serde(skip)]
    tripwire: Arc<Mutex<Option<Tripwire>>>,
}

impl MockSourceConfig {
    pub fn new() -> (Trigger, Self) {
        let (trigger, tripwire) = Tripwire::new();
        (
            trigger,
            Self {
                tripwire: Arc::new(Mutex::new(Some(tripwire))),
            },
        )
    }
}

#[async_trait::async_trait]
#[typetag::serde(name = "mock")]
impl SourceConfig for MockSourceConfig {
    async fn build(&self, cx: SourceContext) -> Result<Source, Error> {
        let tripwire = self.tripwire.lock().await;
        let output = cx.output;

        Ok(Box::pin(
            future::select(
                cx.shutdown.map(|_| ()).boxed(),
                tripwire.clone().unwrap().boxed(),
            )
            .map(|_| drop(output))
            .unit_error(),
        ))
    }

    fn outputs(&self) -> Vec<OutputType> {
        vec![OutputType::log()]
    }
}

#[configurable_component(source, name = "noop")]
struct NoopSourceConfig;

#[async_trait::async_trait]
#[typetag::serde(name = "noop")]
impl SourceConfig for NoopSourceConfig {
    async fn build(&self, cx: SourceContext) -> Result<Source, Error> {
        let shutdown = cx.shutdown;

        Ok(Box::pin(async move {
            shutdown.await;
            Ok(())
        }))
    }

    fn outputs(&self) -> Vec<OutputType> {
        vec![OutputType::log()]
    }
}

#[tokio::test]
async fn closed_source() {
    let mut old_config = Config::builder();
    let (trigger_old, source) = MockSourceConfig::new();
    old_config.add_source("in", source);
    old_config.add_transform("trans", &["in"], NoopTransformConfig {});
    old_config.add_sink("out1", &["trans"], NullSinkConfig {});
    old_config.add_sink("out2", &["trans"], NullSinkConfig {});

    let mut new_config = Config::builder();
    let (_trigger_new, source) = MockSourceConfig::new();
    new_config.add_source("in", source);
    new_config.add_transform("trans", &["in"], NoopTransformConfig);
    new_config.add_sink("out1", &["trans"], NullSinkConfig {});

    let (mut topology, _crash) = start_topology(old_config.compile().unwrap(), false).await;

    trigger_old.cancel();

    topology.sources_finished().await;

    assert!(
        topology
            .reload_config_and_respawn(new_config.compile().unwrap())
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn remove_sink() {
    crate::trace::init(false, false, "debug", 10);

    let mut old_config = Config::builder();
    old_config.add_source("in", NoopSourceConfig);
    old_config.add_transform("trans", &["in"], NoopTransformConfig);
    old_config.add_sink("out1", &["trans"], NullSinkConfig);
    old_config.add_sink("out2", &["trans"], NullSinkConfig);

    let mut new_config = Config::builder();
    new_config.add_source("in", NoopSourceConfig);
    new_config.add_transform("trans", &["in"], NoopTransformConfig);
    new_config.add_sink("out1", &["trans"], NullSinkConfig);

    let (mut topology, _crash) = start_topology(old_config.compile().unwrap(), false).await;
    assert!(
        topology
            .reload_config_and_respawn(new_config.compile().unwrap())
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn remove_transform() {
    crate::trace::init(false, false, "debug", 10);

    let mut old_config = Config::builder();
    old_config.add_source("in", NoopSourceConfig);
    old_config.add_transform("trans1", &["in"], NoopTransformConfig);
    old_config.add_transform("trans2", &["trans1"], NoopTransformConfig);
    old_config.add_sink("out1", &["trans1"], NullSinkConfig);
    old_config.add_sink("out2", &["trans2"], NullSinkConfig);

    let mut new_config = Config::builder();
    new_config.add_source("in", NoopSourceConfig);
    new_config.add_transform("trans1", &["in"], NoopTransformConfig);
    new_config.add_sink("out1", &["trans1"], NullSinkConfig);

    let (mut topology, _crash) = start_topology(old_config.compile().unwrap(), false).await;
    assert!(
        topology
            .reload_config_and_respawn(new_config.compile().unwrap())
            .await
            .unwrap()
    );
}
