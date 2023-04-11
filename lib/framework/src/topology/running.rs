use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use buffers::channel::BufferSender;
use event::Events;
use futures::{future, Future, FutureExt};
use tokio::sync::{mpsc, watch};
use tokio::time::{interval, sleep_until};
use tokio::time::{Duration, Instant};
use tracing::Instrument;

use crate::config::{ComponentKey, Config, ConfigDiff, HealthcheckOptions, OutputId, Resource};
use crate::shutdown::ShutdownCoordinator;
use crate::topology::builder::Pieces;
use crate::topology::task::TaskOutput;
use crate::topology::{
    build_or_log_errors, handle_errors, retain, take_healthchecks, BuiltBuffer, ControlChannel,
    ControlMessage, Outputs, TaskHandle,
};
use crate::trigger::DisabledTrigger;

// Watcher types for topology changes. These are currently specific to receiving
// `Outputs`. This could be expanded in the future to send an enum of types if,
// for example, this included a new 'Inputs' type.
type WatchTx = watch::Sender<Outputs>;
pub type WatchRx = watch::Receiver<Outputs>;

pub struct RunningTopology {
    inputs: HashMap<ComponentKey, BufferSender<Events>>,
    outputs: HashMap<OutputId, ControlChannel>,
    source_tasks: HashMap<ComponentKey, TaskHandle>,
    tasks: HashMap<ComponentKey, TaskHandle>,
    shutdown_coordinator: ShutdownCoordinator,
    detach_triggers: HashMap<ComponentKey, DisabledTrigger>,
    pub(crate) config: Config,
    abort_tx: mpsc::UnboundedSender<()>,
    watch: (WatchTx, WatchRx),
}

impl RunningTopology {
    pub fn new(config: Config, abort_tx: mpsc::UnboundedSender<()>) -> Self {
        Self {
            inputs: HashMap::new(),
            outputs: HashMap::new(),
            config,
            shutdown_coordinator: ShutdownCoordinator::default(),
            detach_triggers: HashMap::new(),
            source_tasks: HashMap::new(),
            tasks: HashMap::new(),
            abort_tx,
            watch: watch::channel(HashMap::new()),
        }
    }

    /// Signal that all sources in this topology are ended
    ///
    /// The future returned by this function will finish once all the sources in
    /// this topology have finished. This allows the caller to wait for or
    /// detect that the sources in the topology are no longer
    /// producing. [`Application`][crate::app::Application], as an example, uses this as a
    /// shutdown signal.
    pub fn sources_finished(&self) -> future::BoxFuture<'static, ()> {
        self.shutdown_coordinator.shutdown_tripwire()
    }

    /// Shut down all topology components
    ///
    /// This function sends the shutdown signal to all sources in this topology
    /// and returns a future that resolves once all components (sources,
    /// transforms, and sinks) have finished shutting down. Transforms and sinks
    /// will shut down automatically once their input tasks finish.
    ///
    /// This function takes ownership of `self`, so once it returns everything
    /// in the [`RunningTopology`] instance has been dropped except for the
    /// `tasks` map. This map gets moved into the returned future and is used to
    /// poll for when the tasks have completed. Once the returned future is
    /// dropped then everything from this RunningTopology instance is fully
    /// dropped.
    pub fn stop(self) -> impl Future<Output = ()> {
        // Create handy handles collections of all tasks for the subsequent
        // operations.
        let mut wait_handles = Vec::new();
        // We need a Vec here since source components have two tasks. One for
        // pump in self.tasks, and the other for source in self.source_tasks.
        let mut check_handles = HashMap::<ComponentKey, Vec<_>>::new();

        // We need to give some time to the sources to gracefully shutdown, so
        // we will merge them with other tasks.
        for (key, task) in self.tasks.into_iter().chain(self.source_tasks.into_iter()) {
            let task = task.map(|_result| ()).shared();

            wait_handles.push(task.clone());
            check_handles.entry(key).or_default().push(task);
        }

        // If we reach this, we will forcefully shutdown the sources.
        let deadline = Instant::now() + Duration::from_secs(60);

        // If we reach the deadline, this future will print out which components
        // won't gracefully shutdown since we will start to forcefully shutdown
        // the sources.
        let mut check_handles2 = check_handles.clone();
        let timeout = async move {
            sleep_until(deadline).await;
            // Remove all tasks that have shutdown.
            check_handles2.retain(|_key, handles| {
                retain(handles, |handle| handle.peek().is_none());
                !handles.is_empty()
            });
            let remaining_components = check_handles2
                .keys()
                .map(|item| item.to_string())
                .collect::<Vec<_>>()
                .join(", ");

            error!(
              message = "Failed to gracefully shut down in time. Killing components.",
                components = ?remaining_components
            );
        };

        // Reports in intervals which components are still running.
        let mut interval = interval(Duration::from_secs(5));
        let reporter = async move {
            loop {
                interval.tick().await;
                // Remove all tasks that have shutdown.
                check_handles.retain(|_key, handles| {
                    retain(handles, |handle| handle.peek().is_none());
                    !handles.is_empty()
                });
                let remaining_components = check_handles
                    .keys()
                    .map(|item| item.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");

                let time_remaining = match deadline.checked_duration_since(Instant::now()) {
                    Some(remaining) => format!("{} seconds left", remaining.as_secs()),
                    None => "overdue".to_string(),
                };

                info!(
                    message = "Shutting down... Waiting on running components.",
                    remaining_components = ?remaining_components,
                    time_remaining = ?time_remaining
                );
            }
        };

        // Finishes once all tasks have shutdown.
        let success = futures::future::join_all(wait_handles).map(|_| ());

        // Aggregate future that ends once anything detects that all tasks have
        // shutdown.
        let shutdown_complete_future = future::select_all(vec![
            Box::pin(timeout) as future::BoxFuture<'static, ()>,
            Box::pin(reporter) as future::BoxFuture<'static, ()>,
            Box::pin(success) as future::BoxFuture<'static, ()>,
        ]);

        // Now kick off the shutdown process by shutting down the sources.
        let source_shutdown_complete = self.shutdown_coordinator.shutdown_all(deadline);

        futures::future::join(source_shutdown_complete, shutdown_complete_future).map(|_| ())
    }

    /// On Error, topology is in invalid state.
    /// May change components even if reload fails.
    pub async fn reload_config_and_respawn(&mut self, new_config: Config) -> Result<bool, ()> {
        if self.config.global != new_config.global {
            error!(
                message =
                "Global options can't be changed while reloading config file; reload aborted. Please restart vector to reload the configuration file."
            );
            return Ok(false);
        }

        let diff = ConfigDiff::new(&self.config, &new_config);

        // Checks passed so let's shutdown the difference.
        let buffers = self.shutdown_diff(&diff, &new_config).await;

        // Gives windows some time to make available any port
        // released by shutdown components.
        // Issue: https://github.com/timberio/vector/issues/3035
        if cfg!(windows) {
            // This value is guess work.
            tokio::time::sleep(Duration::from_millis(200)).await;
        }

        // Now let's actually build the new pieces.
        if let Some(mut new_pieces) = build_or_log_errors(&new_config, &diff, buffers.clone()).await
        {
            if self
                .run_healthchecks(&diff, &mut new_pieces, new_config.healthchecks)
                .await
            {
                self.connect_diff(&diff, &mut new_pieces).await;
                self.spawn_diff(&diff, new_pieces);
                self.config = new_config;
                // We have successfully changed to new config.
                return Ok(true);
            }
        }

        // We need to rebuild the removed.
        info!("Rebuilding old configuration.");
        let diff = diff.flip();
        if let Some(mut new_pieces) = build_or_log_errors(&self.config, &diff, buffers).await {
            if self
                .run_healthchecks(&diff, &mut new_pieces, self.config.healthchecks)
                .await
            {
                self.connect_diff(&diff, &mut new_pieces).await;
                self.spawn_diff(&diff, new_pieces);
                // We have successfully returned to old config.
                return Ok(false);
            }
        }

        // We failed in rebuilding the old state.
        error!("Failed in rebuilding the old configuration.");

        Err(())
    }

    pub(crate) async fn run_healthchecks(
        &mut self,
        diff: &ConfigDiff,
        pieces: &mut Pieces,
        options: HealthcheckOptions,
    ) -> bool {
        if options.enabled {
            let healthchecks = take_healthchecks(diff, pieces)
                .into_iter()
                .map(|(_, task)| task);
            let healthchecks = future::try_join_all(healthchecks);

            info!("Running healthchecks.");
            if options.require_healthy {
                let success = healthchecks.await;

                if success.is_ok() {
                    info!("All healthchecks passed.");
                    true
                } else {
                    error!("Sinks unhealthy.");
                    false
                }
            } else {
                tokio::spawn(healthchecks);
                true
            }
        } else {
            true
        }
    }

    /// Shuts down any changed/removed component in the given configuration diff.
    ///
    /// If buffers for any of the changed/removed components can be recovered,
    /// they'll be returned.
    async fn shutdown_diff(
        &mut self,
        diff: &ConfigDiff,
        new_config: &Config,
    ) -> HashMap<ComponentKey, BuiltBuffer> {
        // First, we shutdown any changed/removed sources. This ensures that we can
        // allow downstream components to terminate naturally by virtue of the flow
        // of events stopping.
        if diff.sources.any_changed_or_removed() {
            let timeout = Duration::from_secs(30);
            let mut source_shutdown_handles = Vec::new();

            let deadline = Instant::now() + timeout;
            for key in &diff.sources.to_remove {
                debug!(message = "Removing source", component = %key);

                let previous = self.tasks.remove(key).unwrap();
                drop(previous); // detach and forget

                self.remove_outputs(key);
                source_shutdown_handles
                    .push(self.shutdown_coordinator.shutdown_source(key, deadline));
            }

            for key in &diff.sources.to_change {
                debug!(message = "Changing source", component = %key);

                self.remove_outputs(key);
                source_shutdown_handles
                    .push(self.shutdown_coordinator.shutdown_source(key, deadline));
            }

            debug!(
                "Waiting for up to {} seconds for source(s) to finish shutting down",
                timeout.as_secs()
            );
            futures::future::join_all(source_shutdown_handles).await;

            // Final cleanup pass now that all changed/removed sources have signalled as
            // having shutdown.
            for key in diff.sources.removed_and_changed() {
                if let Some(task) = self.source_tasks.remove(key) {
                    task.await.unwrap().unwrap();
                }
            }
        }

        // Next, we shutdown any changed/removed transforms. Same as before: we
        // want allow downstream componets to terminate naturally by virtue of the
        // flow of events stopping.
        //
        // Since transforms are entirely driven by the flow of events into them
        // from upstream components, the shutdown of sources they depend on, or
        // the shutdown of transforms they depend on, and thus the closing of
        // their buffer, will naturally cause them to shutdown, which is why we
        // don't do any manual triggering of shutdown here.
        for key in &diff.transforms.to_remove {
            debug!(message = "Removing transform", compoent = %key);

            let previous = self.tasks.remove(key).unwrap();
            drop(previous); // detach and forget

            self.remove_inputs(key, diff).await;
            self.remove_outputs(key);
        }

        for key in &diff.transforms.to_change {
            debug!(message = "Changing transform", component = %key);

            self.remove_inputs(key, diff).await;
            self.remove_outputs(key);
        }

        // Now, we'll process any changed/removed sinks.
        //
        // At this point both the old and new config don't have conflicts
        // in their resource usage. So if we combine their resources, all
        // found conflicts are between to be removed and to be added components.
        let remove_sink = diff
            .sinks
            .removed_and_changed()
            .map(|key| (key, self.config.sinks[key].resources(key)));
        let add_source = diff
            .sources
            .changed_and_added()
            .map(|key| (key, new_config.sources[key].inner.resources()));
        let add_sink = diff
            .sinks
            .changed_and_added()
            .map(|key| (key, new_config.sinks[key].resources(key)));
        let conflicts = Resource::conflicts(
            remove_sink.map(|(key, value)| ((true, key), value)).chain(
                add_sink
                    .chain(add_source)
                    .map(|(key, value)| ((false, key), value)),
            ),
        )
        .into_iter()
        .flat_map(|(_, components)| components)
        .collect::<HashSet<_>>();
        // Existing conflicting sinks
        let conflicting_sinks = conflicts
            .into_iter()
            .filter(|&(existing_sink, _)| existing_sink)
            .map(|(_, key)| key.clone());

        // For any sink whose buffer configuration didn't change, we can reuse their
        // buffer.
        let reuse_buffers = diff
            .sinks
            .to_change
            .iter()
            .filter(|&key| self.config.sinks[key].buffer == new_config.sinks[key].buffer)
            .cloned()
            .collect::<HashSet<_>>();

        // For any existing sink that has a conflicting resource dependency with a
        // changed/added sink, or for any sink that we want to reuse their buffer,
        // we need to explicit wait for them to finish processing so we can reclaim
        // ownership of those resources/buffers.
        let wait_for_sinks = conflicting_sinks
            .chain(reuse_buffers.iter().cloned())
            .collect::<HashSet<_>>();

        // First, we remove any inputs to removed sinks so they can naturally shutdown.
        for key in &diff.sinks.to_remove {
            debug!(message = "Removing sink", component = %key);
            self.remove_inputs(key, diff).await;
        }

        // After that, for any changed sinks, we temporarily detach their inputs
        // (not remove) so they can naturally shutdown and allow us to recover their
        // buffers if possible
        let mut buffer_tx = HashMap::new();

        for key in &diff.sinks.to_change {
            debug!(message = "Changing sink", component = %key);

            if reuse_buffers.contains(key) {
                self.detach_triggers
                    .remove(key)
                    .unwrap()
                    .into_inner()
                    .cancel();

                // We explicitly clone the input side of the buffer and store it
                // so we don't lose it when we remove the inputs below.
                //
                // We clone instead of removeing here because otherwise the input
                // will be missing for the rest of the reload process, which violates
                // the assumption that all previous inputs for components not being removed are
                // still available. It's simpler to allow the "old" input to stick around and be
                // replaced (even though that's basically a no-op since we're reusing the same
                // buffer) than it is to pass around info about which sinks are having their buffers
                // reused and treat them differently at other stages.
                buffer_tx.insert(key.clone(), self.inputs.get(key).unwrap().clone());
            }

            self.remove_inputs(key, diff).await;
        }

        // Now that we've disconnected or temporarily detached the inputs to all changed/removed
        // sinks, we can actually wait for them to shutdown before collecting any buffers that are
        // marked for reuse.
        //
        // If a sink we're removing isn't tying up any resource that a changed/added sink depends
        // on, we don't bother waiting for it to shutdown.
        for key in &diff.sinks.to_remove {
            let previous = self.tasks.remove(key).unwrap();
            if wait_for_sinks.contains(key) {
                debug!(message = "Waiting for sink to shutdown", %key);
                previous.await.unwrap().unwrap();
            } else {
                drop(previous); // detach and forget
            }
        }

        let mut buffers = HashMap::<ComponentKey, BuiltBuffer>::new();
        for key in &diff.sinks.to_change {
            if wait_for_sinks.contains(key) {
                debug!(message = "Waiting for sink to shutdown", %key);

                let previous = self.tasks.remove(key).unwrap();
                let buffer = previous.await.unwrap().unwrap();

                if reuse_buffers.contains(key) {
                    // We clone instead of removing here because otherwise the input will be
                    // missing for the rest of the reload process, which violates the assumption
                    // that all previous inputs for components not being removed are still
                    // available. It's simpler to allow the "old" input to stick around and be
                    // replaced (even though that's basically a no-op since we're reusing the same
                    // buffer) than it is  to pass around info about which sinks are having their
                    // buffers reused and treat them differently at other stages.
                    let tx = buffer_tx.remove(key).unwrap();
                    let rx = match buffer {
                        TaskOutput::Sink(rx) => rx.into_inner(),
                        _ => unreachable!(),
                    };

                    buffers.insert(key.clone(), (tx, Arc::new(Mutex::new(Some(rx)))));
                }
            }
        }

        buffers
    }

    /// Rewires topology
    pub(crate) async fn connect_diff(&mut self, diff: &ConfigDiff, new_pieces: &mut Pieces) {
        // Sources
        for key in diff.sources.changed_and_added() {
            self.setup_outputs(key, new_pieces).await;
        }

        // Transforms
        // Make sure all transform outputs are set up before another transform
        // might try use it as an input
        for key in diff.transforms.changed_and_added() {
            self.setup_outputs(key, new_pieces).await;
        }

        for key in &diff.transforms.to_change {
            self.replace_inputs(key, new_pieces, diff).await;
        }

        for key in &diff.transforms.to_add {
            self.setup_inputs(key, new_pieces).await;
        }

        // Sinks
        for key in &diff.sinks.to_change {
            self.replace_inputs(key, new_pieces, diff).await;
        }

        for key in &diff.sinks.to_add {
            self.setup_inputs(key, new_pieces).await;
        }

        // Broadcast changes to subscribers.
        if !self.watch.0.is_closed() {
            self.watch
                .0
                .send(
                    self.outputs
                        .iter()
                        .map(|item| (item.0.clone(), item.1.clone()))
                        .collect::<HashMap<_, _>>(),
                )
                .expect("Couldn't broadcast config changes.");
        }
    }

    /// Starts new and changed pieces of topology.
    pub(crate) fn spawn_diff(&mut self, diff: &ConfigDiff, mut new_pieces: Pieces) {
        // Sources
        for key in &diff.sources.to_change {
            info!(message = "Rebuilding source.", key = %key);
            self.spawn_source(key, &mut new_pieces);
        }

        for key in &diff.sources.to_add {
            info!(message = "Starting source.", key = %key);
            self.spawn_source(key, &mut new_pieces);
        }

        // Transforms
        for key in &diff.transforms.to_change {
            info!(message = "Rebuilding transform.", key = %key);
            self.spawn_transform(key, &mut new_pieces);
        }

        for key in &diff.transforms.to_add {
            info!(message = "Starting transform.", key = %key);
            self.spawn_transform(key, &mut new_pieces);
        }

        // Sinks
        for key in &diff.sinks.to_change {
            info!(message = "Rebuilding sink.", key = %key);
            self.spawn_sink(key, &mut new_pieces);
        }

        for key in &diff.sinks.to_add {
            info!(message = "Starting sink.", key = %key);
            self.spawn_sink(key, &mut new_pieces);
        }

        // Extensions
        for key in &diff.extensions.to_change {
            info!(message = "Rebuilding extension", key = %key);
            self.spawn_extension(key, &mut new_pieces);
        }

        for key in &diff.extensions.to_add {
            info!(message = "Starting extension", key = %key);
            self.spawn_extension(key, &mut new_pieces);
        }
    }

    fn spawn_sink(&mut self, key: &ComponentKey, new_pieces: &mut Pieces) {
        let task = new_pieces.tasks.remove(key).unwrap();
        let task = handle_errors(task, self.abort_tx.clone());
        let spawned = tokio::spawn(task);
        if let Some(previous) = self.tasks.insert(key.clone(), spawned) {
            drop(previous); // detach and forget
        }
    }

    fn spawn_extension(&mut self, key: &ComponentKey, new_pieces: &mut Pieces) {
        let task = new_pieces.tasks.remove(key).unwrap();
        let span = error_span!(
            "extension",
            component_kind = "extension",
            component_key = %task.key(),
            component_type = %task.typetag(),
        );

        let task = handle_errors(task, self.abort_tx.clone()).instrument(span);
        let spawned = tokio::spawn(task);
        if let Some(previous) = self.tasks.insert(key.clone(), spawned) {
            drop(previous); // detach and forget
        }

        self.shutdown_coordinator
            .takeover_source(key, &mut new_pieces.shutdown_coordinator);
    }

    fn spawn_transform(&mut self, key: &ComponentKey, new_pieces: &mut Pieces) {
        let task = new_pieces.tasks.remove(key).unwrap();
        let span = error_span!(
            "transform",
            component_kind = "transform",
            component_key = %task.key(),
            component_type = %task.typetag(),
        );
        let task = handle_errors(task, self.abort_tx.clone()).instrument(span);
        let spawned = tokio::spawn(task);
        if let Some(previous) = self.tasks.insert(key.clone(), spawned) {
            drop(previous); // detach and forget
        }
    }

    fn spawn_source(&mut self, key: &ComponentKey, new_pieces: &mut Pieces) {
        let task = new_pieces.tasks.remove(key).unwrap();
        let span = error_span!(
            "source",
            component_kind = "source",
            component_key = %task.key(),
            component_type = %task.typetag(),
        );
        let task = handle_errors(task, self.abort_tx.clone()).instrument(span.clone());
        let spawned = tokio::spawn(task);
        if let Some(previous) = self.tasks.insert(key.clone(), spawned) {
            drop(previous); // detach and forget
        }

        self.shutdown_coordinator
            .takeover_source(key, &mut new_pieces.shutdown_coordinator);

        let source_task = new_pieces.source_tasks.remove(key).unwrap();
        let source_task = handle_errors(source_task, self.abort_tx.clone()).instrument(span);
        self.source_tasks
            .insert(key.clone(), tokio::spawn(source_task));
    }

    fn remove_outputs(&mut self, key: &ComponentKey) {
        self.outputs.retain(|id, _output| &id.component != key);
    }

    async fn remove_inputs(&mut self, key: &ComponentKey, diff: &ConfigDiff) {
        self.inputs.remove(key);
        self.detach_triggers.remove(key);

        let sink_inputs = self.config.sinks.get(key).map(|s| &s.inputs);
        let trans_inputs = self.config.transforms.get(key).map(|t| &t.inputs);
        let old_inputs = sink_inputs.or(trans_inputs).unwrap();

        for input in old_inputs {
            if let Some(output) = self.outputs.get_mut(input) {
                if diff.contains(&input.component) || diff.is_removed(key) {
                    // If the input we're removing ourselves from is changing, that means its
                    // outputs will be recreated, so instead of pausing the sink, we just delete
                    // it outright to ensure things are clean. Additionally, if this component
                    // itself is being removed, then pausing makes no sense because it isn't
                    // coming back.
                    debug!(
                        message = "Removing component input from fanout",
                        component = %key,
                        fanout = %input
                    );

                    let _ = output.send(ControlMessage::Remove(key.clone()));
                } else {
                    // We know that if this component is connected to a given input, and it isn't
                    // being changed, then it will exist when we reconnect inputs, so we should
                    // pause it now to pause further sends through that component until we
                    // reconnect.
                    debug!(
                        message = "Pausing componnet input in fanout",
                        component = %key,
                        fanout = %input
                    );

                    let _ = output.send(ControlMessage::Replace(key.clone(), None));
                }
            }
        }
    }

    async fn setup_outputs(&mut self, key: &ComponentKey, new_pieces: &mut Pieces) {
        let outputs = new_pieces.outputs.remove(key).unwrap();
        for (port, output) in outputs {
            let id = OutputId {
                component: key.clone(),
                port,
            };
            for (sink_key, sink) in &self.config.sinks {
                if sink.inputs.iter().any(|i| i == &id) {
                    // Sink may have been removed with the new config so it may not
                    // be present.
                    if let Some(input) = self.inputs.get(sink_key) {
                        let _ = output.send(ControlMessage::Add(sink_key.clone(), input.clone()));
                    }
                }
            }
            for (transform_key, transform) in &self.config.transforms {
                if transform.inputs.iter().any(|i| i == &id) {
                    // Transform may have been removed with the new config so it may
                    // not be present.
                    if let Some(input) = self.inputs.get(transform_key) {
                        let _ =
                            output.send(ControlMessage::Add(transform_key.clone(), input.clone()));
                    }
                }
            }

            self.outputs.insert(id.clone(), output);
        }
    }

    async fn setup_inputs(&mut self, key: &ComponentKey, new_pieces: &mut Pieces) {
        let (tx, inputs) = new_pieces.inputs.remove(key).unwrap();

        for input in inputs {
            // This can only fail if we are disconnected, which is a valid situation.
            let _ = self
                .outputs
                .get_mut(&input)
                .expect("unknown output")
                .send(ControlMessage::Add(key.clone(), tx.clone()));
        }

        self.inputs.insert(key.clone(), tx);
        new_pieces
            .detach_triggers
            .remove(key)
            .map(|trigger| self.detach_triggers.insert(key.clone(), trigger.into()));
    }

    async fn replace_inputs(
        &mut self,
        key: &ComponentKey,
        new_pieces: &mut Pieces,
        diff: &ConfigDiff,
    ) {
        let (tx, inputs) = new_pieces.inputs.remove(key).unwrap();

        let sink_inputs = self.config.sinks.get(key).map(|s| &s.inputs);
        let trans_inputs = self.config.transforms.get(key).map(|t| &t.inputs);
        let old_inputs = sink_inputs
            .or(trans_inputs)
            .unwrap()
            .iter()
            .collect::<HashSet<_>>();

        let new_inputs = inputs.iter().collect::<HashSet<_>>();

        let inputs_to_remove = &old_inputs - &new_inputs;
        let mut inputs_to_add = &new_inputs - &old_inputs;
        let replace_candidates = old_inputs.intersection(&new_inputs);
        let mut inputs_to_replace = HashSet::new();

        // If the source component of an input was also rebuilt, we need to send an add message
        // instead of a replace message.
        for input in replace_candidates {
            if diff
                .sources
                .changed_and_added()
                .chain(diff.transforms.changed_and_added())
                .any(|key| key == &input.component)
            {
                inputs_to_add.insert(input);
            } else {
                inputs_to_replace.insert(input);
            }
        }

        for input in inputs_to_remove {
            if let Some(output) = self.outputs.get_mut(input) {
                // This can only fail if we are disconnected, which is a valid situation.
                let _ = output.send(ControlMessage::Remove(key.clone()));
            }
        }

        for input in inputs_to_add {
            // This can only fail if we are disconnected, which is a valid situation.
            let _ = self
                .outputs
                .get_mut(input)
                .unwrap()
                .send(ControlMessage::Add(key.clone(), tx.clone()));
        }

        for &input in inputs_to_replace {
            // This can only fail if we are disconnected, which is a valid situation.
            let _ = self
                .outputs
                .get_mut(input)
                .unwrap()
                .send(ControlMessage::Replace(key.clone(), Some(tx.clone())));
        }

        self.inputs.insert(key.clone(), tx);
        new_pieces
            .detach_triggers
            .remove(key)
            .map(|trigger| self.detach_triggers.insert(key.clone(), trigger.into()));
    }

    /// Borrows the Config
    pub const fn config(&self) -> &Config {
        &self.config
    }

    /// Subscribe to topology changes. This will receive an `Outputs` currently, but may be
    /// expanded in the future to accommodate `Inputs`. This is used by the 'tap' API to observe
    /// config changes, and re-wire tap sinks.
    pub fn watch(&self) -> watch::Receiver<Outputs> {
        self.watch.1.clone()
    }
}
