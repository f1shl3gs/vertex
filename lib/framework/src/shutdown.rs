use std::{
    collections::HashMap,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use futures::{future, ready, Future, FutureExt};
use tokio::time::{timeout_at, Instant};
use tripwire::{Trigger, Tripwire};

use crate::config::ComponentKey;

#[derive(Default)]
pub struct ShutdownCoordinator {
    begun_triggers: HashMap<ComponentKey, Trigger>,
    force_triggers: HashMap<ComponentKey, Trigger>,
    complete_tripwires: HashMap<ComponentKey, Tripwire>,
}

impl ShutdownCoordinator {
    /// Create the necessary Triggers and tripwires for coordinating shutdown
    /// of this Source and stores them as needed. Return the ShutdownSignal for
    /// this Source as well as a Tripwire that will be notified if the Source
    /// should be forcibly shutdown
    pub fn register_source(&mut self, name: &ComponentKey) -> (ShutdownSignal, Tripwire) {
        let (begun_trigger, begun_tripwire) = Tripwire::new();
        let (force_trigger, force_tripwire) = Tripwire::new();
        let (complete_trigger, complete_tripwire) = Tripwire::new();

        self.begun_triggers.insert(name.clone(), begun_trigger);
        self.force_triggers.insert(name.clone(), force_trigger);
        self.complete_tripwires
            .insert(name.clone(), complete_tripwire);

        let shutdown_signal = ShutdownSignal::new(begun_tripwire, complete_trigger);

        (shutdown_signal, force_tripwire)
    }

    pub fn register_extension(&mut self, name: &ComponentKey) -> (ShutdownSignal, Tripwire) {
        let (begun_trigger, begun_tripwire) = Tripwire::new();
        let (force_trigger, force_tripwire) = Tripwire::new();
        let (complete_trigger, complete_tripwire) = Tripwire::new();

        self.begun_triggers.insert(name.clone(), begun_trigger);
        self.force_triggers.insert(name.clone(), force_trigger);
        self.complete_tripwires
            .insert(name.clone(), complete_tripwire);

        let shutdown_signal = ShutdownSignal::new(begun_tripwire, complete_trigger);

        (shutdown_signal, force_tripwire)
    }

    /// Takes ownership of all internal state for the given source from another ShutdownCoordinator.
    ///
    /// # Panics
    ///
    /// Panics if the other coordinator already had its triggers removed.
    pub fn takeover_source(&mut self, name: &ComponentKey, other: &mut Self) {
        let existing = self.begun_triggers.insert(
            name.clone(),
            other.begun_triggers.remove(name).unwrap_or_else(|| {
                panic!(
                    "other ShutdownCoordinator didn't have a begun trigger for {}",
                    name
                )
            }),
        );

        if existing.is_some() {
            panic!(
                "ShutdownCoordinator already has a begun trigger for source {}",
                name
            )
        }

        let existing = self.force_triggers.insert(
            name.clone(),
            other.force_triggers.remove(name).unwrap_or_else(|| {
                panic!(
                    "other ShutdownCoordinator didn't have a force trigger for {}",
                    name
                )
            }),
        );
        if existing.is_some() {
            panic!(
                "ShutdownCoordinator already has a force trigger for source {}",
                name
            );
        }

        let existing = self.complete_tripwires.insert(
            name.clone(),
            other.complete_tripwires.remove(name).unwrap_or_else(|| {
                panic!(
                    "Other ShutdownCoordinator didn't have a complete tripwire for {}",
                    name
                );
            }),
        );
        if existing.is_some() {
            panic!(
                "ShutdownCoordinator already has a complete tripwire for source {}",
                name
            );
        }
    }

    /// Sends a signal to begin shutting down to all sources, and returns a future
    /// that resolves once all sources have either shut down completely, or have
    /// been sent the force shutdown signal. The force shutdown signal will be sent
    /// to any sources that don't cleanly shut down before the give `deadline`.
    pub fn shutdown_all(self, deadline: Instant) -> impl Future<Output = ()> {
        let mut complete_futures = Vec::new();
        let begun_triggers = self.begun_triggers;
        let mut complete_tripwires = self.complete_tripwires;
        let mut force_triggers = self.force_triggers;

        for (name, trigger) in begun_triggers {
            trigger.cancel();

            let complete_tripwire = complete_tripwires.remove(&name).unwrap_or_else(|| {
                panic!(
                    "complete tripwire for source '{}' not found in the ShutdownCoordinator",
                    name
                )
            });

            let force_trigger = force_triggers.remove(&name).unwrap_or_else(|| {
                panic!(
                    "force_trigger for source '{}' not found in the ShutdownCoordinator",
                    name,
                )
            });

            complete_futures.push(ShutdownCoordinator::shutdown_source_complete(
                complete_tripwire,
                force_trigger,
                name,
                deadline,
            ));
        }

        futures::future::join_all(complete_futures).map(|_| ())
    }

    /// Sends the signal to the given source to begin shutting down. Return
    /// a future that resolves when the source has finished shutting down
    /// cleanly or been sent the force shutdown signal. The returned future
    /// resolves to a bool that indicates if the source shut down cleanly
    /// before the given `deadline`. If the result is false then that means
    /// the source failed to shut down before `deadline` and had to be force-shutdown.
    pub fn shutdown_source(
        &mut self,
        name: &ComponentKey,
        deadline: Instant,
    ) -> impl Future<Output = bool> {
        let begin_trigger = self.begun_triggers.remove(name).unwrap_or_else(|| {
            panic!(
                "begun_trigger for source '{}' not found in the ShutdownCoordinator",
                name
            )
        });

        // This is what actually triggers the source to begin shutting down
        begin_trigger.cancel();

        let complete_tripwire = self.complete_tripwires.remove(name).unwrap_or_else(|| {
            panic!(
                "complete_tripwire for source '{}' not found in the ShutdownCoordinator",
                name
            )
        });

        let force_trigger = self.force_triggers.remove(name).unwrap_or_else(|| {
            panic!(
                "force_trigger for source '{}' not found in the ShutdownCoordinator",
                name
            )
        });

        ShutdownCoordinator::shutdown_source_complete(
            complete_tripwire,
            force_trigger,
            name.to_owned(),
            deadline,
        )
    }

    /// Returned future will finish once all sources have finished
    pub fn shutdown_tripwire(&self) -> future::BoxFuture<'static, ()> {
        let futs = self
            .complete_tripwires
            .values()
            .cloned()
            .map(|tripwire| tripwire.boxed());

        future::join_all(futs)
            .map(|_| info!("All sources have finished"))
            .boxed()
    }

    fn shutdown_source_complete(
        complete_tripwire: Tripwire,
        force_trigger: Trigger,
        name: ComponentKey,
        deadline: Instant,
    ) -> impl Future<Output = bool> {
        async move {
            if timeout_at(deadline, complete_tripwire).await.is_ok() {
                force_trigger.disable();
                true
            } else {
                error!(
                    "Some '{}' failed to shutdown before deadline. Forcing shutdown",
                    name
                );

                force_trigger.cancel();
                false
            }
        }
        .boxed()
    }
}

/// When this struct goes out of scope and its internal refcount goes
/// to 0 it is a signal that its corresponding Source has completed
/// executing and may be cleaned up. It is the responsibility
/// of each Source to ensure that at least one copy of this handle
/// remains alive for the entire lifetime of the Source.
#[derive(Clone)]
pub struct ShutdownSignalToken {
    _complete: Arc<Trigger>,
}

impl ShutdownSignalToken {
    fn new(trigger: Trigger) -> Self {
        Self {
            _complete: Arc::new(trigger),
        }
    }
}

/// Passed to each Source to coordinate the global shutdown process.
#[pin_project::pin_project]
#[derive(Clone)]
pub struct ShutdownSignal {
    /// This will be triggered when global shutdown has begun, and is a
    /// sign to the Source to begin its shutdown process.
    #[pin]
    begin: Option<Tripwire>,

    /// When a Source allows this to go out of scope it informs the global
    /// shutdown coordinator that this Source's local shutdown process is
    /// complete.
    /// Optional only so that `poll()` can move the handle out and return it.
    completed: Option<ShutdownSignalToken>,
}

impl Future for ShutdownSignal {
    type Output = ShutdownSignalToken;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.as_mut().project().begin.as_pin_mut() {
            Some(fut) => {
                ready!(fut.poll(cx));

                info!("shutdown signal ready");

                let mut pinned = self.project();
                pinned.begin.set(None);

                Poll::Ready(pinned.completed.take().unwrap())
            }
            // TODO: This should almost certainly be a panic to avoid deadlocking in
            // the case of a poll-after-ready situation.
            None => Poll::Pending,
        }
    }
}

impl ShutdownSignal {
    pub fn new(tripwire: Tripwire, trigger: Trigger) -> Self {
        Self {
            begin: Some(tripwire),
            completed: Some(ShutdownSignalToken::new(trigger)),
        }
    }

    #[cfg(any(test, feature = "test-util"))]
    pub fn noop() -> Self {
        let (trigger, tripwire) = Tripwire::new();
        Self {
            begin: Some(tripwire),
            completed: Some(ShutdownSignalToken::new(trigger)),
        }
    }

    #[cfg(any(test, feature = "test-util"))]
    pub fn new_wired() -> (Trigger, ShutdownSignal, Tripwire) {
        let (trigger_shutdown, tripwire) = Tripwire::new();
        let (trigger, shutdown_done) = Tripwire::new();
        let shutdown = ShutdownSignal::new(tripwire, trigger);

        (trigger_shutdown, shutdown, shutdown_done)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{Duration, Instant};

    #[tokio::test]
    async fn shutdown_coordinator_shutdown_source_clean() {
        let mut shutdown = ShutdownCoordinator::default();
        let key = ComponentKey::from("test");

        let (shutdown_signal, _) = shutdown.register_source(&key);

        let deadline = Instant::now() + Duration::from_secs(1);
        let shutdown_complete = shutdown.shutdown_source(&key, deadline);

        drop(shutdown_signal);

        let success = shutdown_complete.await;
        assert!(success);
    }

    #[tokio::test]
    async fn shutdown_coordinator_shutdown_source_force() {
        let mut shutdown = ShutdownCoordinator::default();
        let key = ComponentKey::from("test");

        let (_shutdown_signal, force_shutdown_tripwire) = shutdown.register_source(&key);

        let deadline = Instant::now() + Duration::from_secs(1);
        let shutdown_complete = shutdown.shutdown_source(&key, deadline);

        // Since we never drop the ShutdownSignal the ShutdownCoordinator assumes the Source is
        // still running and must force shutdown.
        let success = shutdown_complete.await;
        assert!(!success);

        let finished = futures::poll!(force_shutdown_tripwire.boxed());
        assert_eq!(finished, Poll::Ready(()));
    }
}
