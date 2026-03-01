mod interpolate;

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::time::Duration;

use bytes::Bytes;
use configurable::{Configurable, configurable_component};
use framework::config::{
    ComponentKey, DataType, GlobalOptions, OutputType, ProxyConfig, Resource, SourceConfig,
    SourceContext,
};
use framework::observe::{Change, Endpoint, Notifier, available_observers};
use framework::{Pipeline, ShutdownSignal, Source};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tripwire::{Trigger, Tripwire};
use value::Value;
use vtl::{Diagnostic, Program, TargetValue};

#[derive(Configurable, Debug, Deserialize, Serialize)]
struct TemplateConfig {
    /// VTL script to filter endpoints
    rule: Option<String>,

    /// Source config to interpolate
    config: Value,
}

#[configurable_component(source, name = "multiplier")]
struct Config {
    /// The name of the observer extension to use
    observer: String,

    /// The template of source config to interpolate
    templates: Vec<TemplateConfig>,
}

#[async_trait::async_trait]
#[typetag::serde(name = "multiplier")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        let Some(notifier) = Notifier::subscribe(&self.observer) else {
            return Err(format!(
                "observer `{}` is not configured, available: {:?}",
                self.observer,
                available_observers()
            )
            .into());
        };

        let mut templates = vec![];
        for template in &self.templates {
            match template.config.get("type") {
                // multiplier in multiplier is not allowed
                Some(typ) => {
                    if typ == &Value::Bytes(Bytes::from_static(b"multiplier")) {
                        return Err("multiplier in multiplier is not allowed".into());
                    }
                }
                None => {
                    return Err("`type` field is required by source template".into());
                }
            }

            let program = match &template.rule {
                Some(rule) => {
                    let program = vtl::compile_with(rule, &["id", "type", "target", "details"])
                        .map_err(|err| Diagnostic::new(rule).snippets(err))?;

                    Some(program)
                }
                None => None,
            };

            templates.push(SourceTemplate {
                program,
                config: template.config.clone(),
            });
        }

        Ok(Box::pin(run(templates, cx, notifier)))
    }

    fn outputs(&self) -> Vec<OutputType> {
        vec![OutputType::new(DataType::All)]
    }

    fn resources(&self) -> Vec<Resource> {
        vec![]
    }
}

struct ShutdownCoordinator {
    begun_triggers: BTreeMap<String, Trigger>,
    force_triggers: BTreeMap<String, Trigger>,
    complete_tripwires: BTreeMap<String, Tripwire>,
}

impl ShutdownCoordinator {
    fn register(&mut self, key: String) -> (ShutdownSignal, Tripwire) {
        let (begun_trigger, begun_tripwire) = Tripwire::new();
        let (force_trigger, force_tripwire) = Tripwire::new();
        let (complete_trigger, complete_tripwire) = Tripwire::new();

        self.begun_triggers.insert(key.clone(), begun_trigger);
        self.force_triggers.insert(key.clone(), force_trigger);
        self.complete_tripwires.insert(key, complete_tripwire);

        let shutdown_signal = ShutdownSignal::new(begun_tripwire, complete_trigger);

        (shutdown_signal, force_tripwire)
    }

    async fn shutdown(&mut self, name: &str, timeout: Duration) {
        let Some(begin_trigger) = self.begun_triggers.remove(name) else {
            // not added, that totally normal
            return;
        };

        // This is what actually triggers the source to begin shutting down
        begin_trigger.cancel();

        let complete_tripwire = self.complete_tripwires.remove(name).unwrap_or_else(|| {
            panic!(
                "complete_tripwire for sub source '{name}' not found in the ShutdownCoordinator",
            )
        });

        let force_trigger = self.force_triggers.remove(name).unwrap_or_else(|| {
            panic!("force_trigger for sub source '{name}' not found in the ShutdownCoordinator",)
        });

        shutdown_source_complete(name.to_string(), complete_tripwire, force_trigger, timeout).await
    }

    async fn shutdown_all(mut self, timeout: Duration) {
        let mut complete_futures = vec![];

        for (name, trigger) in self.begun_triggers {
            trigger.cancel();

            let complete_tripwire = self.complete_tripwires.remove(&name).unwrap_or_else(|| {
                panic!(
                    "complete tripwire for sub source '{name}' not found in the ShutdownCoordinator",
                )
            });

            let force_trigger = self.force_triggers.remove(&name).unwrap_or_else(|| {
                panic!("force_trigger for source '{name}' not found in the ShutdownCoordinator",)
            });

            complete_futures.push(shutdown_source_complete(
                name,
                complete_tripwire,
                force_trigger,
                timeout,
            ));
        }

        futures::future::join_all(complete_futures).await;
    }
}

async fn shutdown_source_complete(
    key: String,
    complete_tripwire: Tripwire,
    force_trigger: Trigger,
    timeout: Duration,
) {
    if tokio::time::timeout(timeout, complete_tripwire)
        .await
        .is_ok()
    {
        force_trigger.disable();
    } else {
        error!(
            message = "Failed to shutdown before deadline, forcing shutdown",
            key
        );

        force_trigger.cancel();
    }
}

async fn run(
    templates: Vec<SourceTemplate>,
    mut cx: SourceContext,
    mut notifier: Notifier,
) -> Result<(), ()> {
    let mut shutdown_coordinator = ShutdownCoordinator {
        begun_triggers: Default::default(),
        force_triggers: Default::default(),
        complete_tripwires: Default::default(),
    };

    loop {
        let change = tokio::select! {
            _ = &mut cx.shutdown => break,
            received = notifier.next() => match received {
                Some(change) => change,
                // notifier shutdown
                None => break,
            },
        };

        match change {
            Change::Add(endpoints) => {
                for endpoint in endpoints {
                    for (index, template) in templates.iter().enumerate() {
                        let key = format!("{}/{}/{}", cx.key.id(), index, &endpoint.id);

                        match template.filter(&endpoint) {
                            Ok(true) => {}
                            Ok(false) => continue,
                            Err(_err) => {
                                error!(message = "Failed to filter endpoint", ?endpoint);
                                continue;
                            }
                        }

                        debug!(message = "start sub source", ?key);

                        let (shutdown, force_shutdown_tripwire) =
                            shutdown_coordinator.register(key.clone());

                        if let Err(err) = template
                            .start(
                                &endpoint,
                                key,
                                shutdown,
                                force_shutdown_tripwire,
                                cx.output.clone(),
                                cx.globals.clone(),
                                cx.proxy.clone(),
                            )
                            .await
                        {
                            error!(message = "Failed to start sub source", ?endpoint, %err);
                        }
                    }
                }
            }
            Change::Remove(endpoints) => {
                for endpoint in endpoints {
                    for (index, _template) in templates.iter().enumerate() {
                        let key = format!("{}/{}/{}", cx.key.id(), index, &endpoint.id);

                        debug!(message = "remove sub source", ?key);

                        shutdown_coordinator
                            .shutdown(&key, Duration::from_secs(5))
                            .await;
                    }
                }
            }
            Change::Update(endpoints) => {
                for endpoint in endpoints {
                    for (index, template) in templates.iter().enumerate() {
                        let key = format!("{}/{}/{}", cx.key.id(), index, &endpoint.id);

                        debug!(message = "remove sub source while updating", ?key);
                        // shutdown anyway
                        shutdown_coordinator
                            .shutdown(&key, Duration::from_secs(5))
                            .await;

                        match template.filter(&endpoint) {
                            Ok(true) => {}
                            Ok(false) => continue,
                            Err(_err) => {
                                error!(message = "Failed to filter endpoint", ?endpoint);
                                continue;
                            }
                        }

                        debug!(message = "start sub source while updating", ?key);

                        let (shutdown, force_shutdown_tripwire) =
                            shutdown_coordinator.register(key.clone());
                        if let Err(err) = template
                            .start(
                                &endpoint,
                                key,
                                shutdown,
                                force_shutdown_tripwire,
                                cx.output.clone(),
                                cx.globals.clone(),
                                cx.proxy.clone(),
                            )
                            .await
                        {
                            error!(message = "Failed to re-start sub source", ?endpoint, %err);
                        }
                    }
                }
            }
        }
    }

    shutdown_coordinator
        .shutdown_all(Duration::from_secs(15))
        .await;

    debug!("Shutdown coordinator complete");

    Ok(())
}

type TaskHandle = tokio::task::JoinHandle<()>;

struct SourceTemplate {
    program: Option<Program>,
    config: Value,
}

impl SourceTemplate {
    fn filter(&self, endpoint: &Endpoint) -> Result<bool, ()> {
        let Some(program) = self.program.as_ref() else {
            return Ok(true);
        };

        let mut variables = vec![Value::Null; program.type_state().variables.len()];
        variables[0] = Value::Bytes(Bytes::from(endpoint.id.clone()));
        variables[1] = {
            let b = match &endpoint.typ {
                Cow::Borrowed(b) => Bytes::from_static(b.as_bytes()),
                Cow::Owned(s) => Bytes::from(s.clone()),
            };

            Value::Bytes(b)
        };
        variables[2] = Value::Bytes(Bytes::from(endpoint.target.clone()));
        variables[3] = endpoint.details.clone();

        let mut cx = vtl::Context {
            target: &mut TargetValue {
                metadata: Value::Null,
                value: Value::Null,
            },
            variables: &mut variables,
        };

        match program.resolve(&mut cx) {
            Ok(Value::Boolean(b)) => Ok(b),
            Ok(value) => {
                error!(
                    message = "source template resolved to non-boolean value",
                    ?value
                );

                Err(())
            }
            Err(err) => {
                error!(
                    message = "Unable to run the source template match",
                    %err,
                );

                Err(())
            }
        }
    }

    async fn start(
        &self,
        endpoint: &Endpoint,
        key: String,
        shutdown: ShutdownSignal,
        force_shutdown_tripwire: Tripwire,
        output: Pipeline,
        globals: GlobalOptions,
        proxy: ProxyConfig,
    ) -> crate::Result<TaskHandle> {
        let cx = SourceContext {
            key: ComponentKey::from(key.clone()),
            output,
            shutdown,
            globals,
            proxy,
            acknowledgements: false,
        };

        let config = interpolate::interpolate(&self.config, endpoint)?;
        let source = serde_json::from_value::<Box<dyn SourceConfig>>(config)
            .map_err(interpolate::Error::Deserialize)?
            .build(cx)
            .await?;

        let task = tokio::spawn(async move {
            let result = tokio::select! {
                biased;

                _ = force_shutdown_tripwire => Ok(()),
                result = source => result,
            };

            match result {
                Ok(()) => {
                    debug!(message = "sub source finished", ?key);
                }
                Err(()) => {
                    debug!(message = "sub source returned an error", ?key);
                }
            }
        });

        Ok(task)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>();
    }
}
