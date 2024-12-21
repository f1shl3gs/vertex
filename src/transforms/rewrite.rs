use std::path::PathBuf;

use configurable::{configurable_component, Configurable};
use event::Events;
use framework::config::{DataType, Output, TransformConfig, TransformContext};
use framework::{SyncTransform, Transform, TransformOutputsBuf};
use serde::{Deserialize, Serialize};
use vtl::{Diagnostic, Program};

use crate::common::vtl::{precompute_metric_value, LogTarget, MetricTarget};

#[derive(Configurable, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum Source {
    /// Absolutely path of the VTL file
    File(PathBuf),

    /// VTL content
    Script(String),
}

impl Source {
    fn load(&self) -> Result<String, std::io::Error> {
        match self {
            Source::File(path) => std::fs::read_to_string(path),
            Source::Script(content) => Ok(content.to_string()),
        }
    }
}

#[configurable_component(transform, name = "rewrite")]
struct Config {
    #[serde(flatten)]
    source: Source,
}

#[async_trait::async_trait]
#[typetag::serde(name = "rewrite")]
impl TransformConfig for Config {
    async fn build(&self, _cx: &TransformContext) -> framework::Result<Transform> {
        let script = self.source.load()?;

        match vtl::compile(&script) {
            Ok(program) => {
                let rewrite = Rewrite {
                    // error_mode: self.error_mode.clone(),
                    program,
                };

                Ok(Transform::synchronous(rewrite))
            }
            Err(err) => {
                let diagnostic = Diagnostic::new(script);
                Err(diagnostic.snippets(err).into())
            }
        }
    }

    fn input_type(&self) -> DataType {
        DataType::Log | DataType::Metric
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::new(DataType::Log | DataType::Metric)]
    }

    fn enable_concurrency(&self) -> bool {
        true
    }
}

const DROPPED: &str = "dropped";

#[derive(Clone)]
struct Rewrite {
    // error_mode: ErrorMode,
    program: Program,
}

impl SyncTransform for Rewrite {
    fn transform(&mut self, events: Events, output: &mut TransformOutputsBuf) {
        match events {
            Events::Logs(logs) => {
                let mut success = Vec::with_capacity(logs.len());
                let mut dropped = Vec::new();

                for log in logs {
                    let mut target = LogTarget { log };
                    match self.program.run(&mut target) {
                        Ok(_value) => {
                            success.push(target.log);
                        }
                        Err(err) => {
                            warn!(message = "run VTL script failed", %err, internal_log_rate_limit = true);
                            dropped.push(target.log);
                        }
                    }
                }

                output.push(success.into());
                if !dropped.is_empty() {
                    output.push_named(DROPPED, dropped.into());
                }
            }
            Events::Metrics(metrics) => {
                let mut success = Vec::with_capacity(metrics.len());
                let mut dropped = Vec::new();

                for metric in metrics {
                    let value = precompute_metric_value(&metric, self.program.target_queries());
                    let mut target = MetricTarget { metric, value };

                    match self.program.run(&mut target) {
                        Ok(_value) => {
                            success.push(target.metric);
                        }
                        Err(err) => {
                            warn!(
                                message = "run VTL script failed",
                                %err,
                                internal_log_rate_limit = true
                            );

                            dropped.push(target.metric);
                        }
                    }
                }

                output.push(success.into());
                if !dropped.is_empty() {
                    output.push_named(DROPPED, dropped.into());
                }
            }
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize() {
        let src = Source::Script("foo".to_string());
        let data = serde_json::to_string(&src).unwrap();
        assert_eq!(data, r#"{"script":"foo"}"#);
    }

    #[ignore]
    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<Config>()
    }
}
