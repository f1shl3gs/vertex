mod builder;
mod fanout;
mod running;
mod task;

#[cfg(test)]
mod test;

// re-exprot
pub use builder::{build_pieces, Pieces};
pub use fanout::{ControlChannel, ControlMessage, Fanout};
pub use running::RunningTopology;

use buffers::channel::{BufferReceiver, BufferSender};
use event::Events;
use futures::{Future, FutureExt};
use std::panic::AssertUnwindSafe;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use task::{Task, TaskOutput};
use tokio::sync::mpsc;

use crate::config::{ComponentKey, Config, ConfigDiff, OutputId};

type BuiltBuffer = (
    BufferSender<Events>,
    Arc<Mutex<Option<BufferReceiver<Events>>>>,
    buffers::Acker,
);

type Outputs = HashMap<OutputId, fanout::ControlChannel>;

type TaskHandle = tokio::task::JoinHandle<Result<TaskOutput, ()>>;

pub async fn start_validate(
    config: Config,
    diff: ConfigDiff,
    mut pieces: Pieces,
) -> Option<(RunningTopology, mpsc::UnboundedReceiver<()>)> {
    let (abort_tx, abort_rx) = mpsc::unbounded_channel();

    let mut running_topology = RunningTopology::new(config, abort_tx);

    if !running_topology
        .run_healthchecks(&diff, &mut pieces, running_topology.config.health_checks)
        .await
    {
        return None;
    }

    running_topology.connect_diff(&diff, &mut pieces).await;
    running_topology.spawn_diff(&diff, pieces);

    Some((running_topology, abort_rx))
}

/// If the closure returns false, then the element is removed
fn retain<T>(vec: &mut Vec<T>, mut retain_filter: impl FnMut(&mut T) -> bool) {
    let mut i = 0;
    while let Some(data) = vec.get_mut(i) {
        if retain_filter(data) {
            i += 1;
        } else {
            let _ = vec.remove(i);
        }
    }
}

pub fn take_healthchecks(diff: &ConfigDiff, pieces: &mut Pieces) -> Vec<(ComponentKey, Task)> {
    (&diff.sinks.to_change | &diff.sinks.to_add)
        .into_iter()
        .filter_map(|id| pieces.health_checks.remove(&id).map(move |task| (id, task)))
        .collect()
}

async fn handle_errors(
    task: impl Future<Output = Result<TaskOutput, ()>>,
    abort_tx: mpsc::UnboundedSender<()>,
) -> Result<TaskOutput, ()> {
    AssertUnwindSafe(task)
        .catch_unwind()
        .await
        .map_err(|_| ())
        .and_then(|res| res)
        .map_err(|_| {
            error!("An error occurred that vector couldn't handle.");
            let _ = abort_tx.send(());
        })
}

pub async fn build_or_log_errors(
    config: &Config,
    diff: &ConfigDiff,
    buffers: HashMap<ComponentKey, BuiltBuffer>,
) -> Option<Pieces> {
    match builder::build_pieces(config, diff, buffers).await {
        Err(errors) => {
            for err in errors {
                error!(
                    message = "Configuration error.",
                    %err
                );
            }
            None
        }
        Ok(new_pieces) => Some(new_pieces),
    }
}
