use std::collections::VecDeque;

use futures::stream::BoxStream;
use futures::{Stream, StreamExt};
use tracing::{debug, error, warn};

use super::Resource;
use super::client::{Client, Error, VersionMatch};
use super::client::{ListParams, WatchEvent, WatchParams};

pub enum Event<T> {
    /// An object was added or modified
    Apply(T),

    /// An object was deleted
    ///
    /// NOTE: This should not be used for managing persistent state elsewhere, since
    /// events may be lost if the watcher is unavailable. Use Finalizers instead.
    Deleted(T),

    /// The watch stream was restarted.
    ///
    /// A series of `InitApply` events are expected to follow until all matching objects
    /// have been listed. This event can be used to prepare a buffer for `InitApply` events.
    Init,

    /// Received an object during `Init`
    ///
    /// Objects returned here are either from the initial stream using the `StreamingList`
    /// strategy, or from pages using the `ListWatch` strategy.
    ///
    /// These events can be passed up if having a complete set of objects is not a concern.
    /// If you need to wait for a complete set, please buffer these events until an `InitDone`
    InitApply(T),

    /// The initialisation is complete
    ///
    /// this can be used as a signal to replace buffered store contents atomically. No more
    /// `InitApply` events will happen until the next `Init` event.
    ///
    /// Any objects that were previously applied but are not listed in any of the `InitApply` events
    /// should be assumed to have been Deleted
    InitDone,
}

#[derive(Copy, Clone, Debug, Default)]
pub enum InitialListStrategy {
    #[default]
    ListWatch,
    StreamingList,
}

enum State<R: Resource> {
    Start {
        initial_list_strategy: InitialListStrategy,
        resource_version: Option<String>,
    },
    Listing {
        continue_token: Option<String>,
        objects: VecDeque<R>,
        last_bookmark: Option<String>,
    },
    InitialWatch {
        params: WatchParams,
        resource_version: Option<String>,
        initial_list_done: bool,
    },
    Watching {
        resource_version: String,
        stream: BoxStream<'static, Result<WatchEvent<R>, Error>>,
        initial_list_done: bool,
    },
}

impl<R: Resource> Default for State<R> {
    fn default() -> Self {
        State::Start {
            initial_list_strategy: InitialListStrategy::default(),
            resource_version: None,
        }
    }
}

#[derive(Default)]
pub struct Config {
    pub label_selector: Option<String>,
    pub field_selector: Option<String>,
    pub timeout: Option<u32>,
    // pub list_semantic: ListSemantic,
    pub initial_list_strategy: InitialListStrategy,
    pub bookmark: bool,
}

impl Config {
    fn list_params(&self) -> ListParams {
        ListParams {
            label_selector: self.label_selector.clone(),
            field_selector: self.field_selector.clone(),
            timeout: self.timeout,
            limit: Some(500),
            continue_token: None,
            version_match: Some(VersionMatch::NotOlderThan),
            resource_version: Some("0".to_string()),
        }
    }

    fn watch_params(&self) -> WatchParams {
        WatchParams {
            label_selector: self.label_selector.clone(),
            field_selector: self.field_selector.clone(),
            timeout: self.timeout,
            bookmarks: self.bookmark,
            send_initial_events: match self.initial_list_strategy {
                InitialListStrategy::StreamingList => true,
                InitialListStrategy::ListWatch => false,
            },
        }
    }
}

/// Watches a Kubernetes Resource for changes continuously, this automatically tries to
/// recover the stream upon errors.
pub fn watcher<R: Resource + 'static>(
    client: Client,
    config: Config,
) -> impl Stream<Item = Result<Event<R>, Error>> {
    let initial_state = State::Start {
        initial_list_strategy: config.initial_list_strategy,
        resource_version: None,
    };

    futures::stream::unfold(
        (client, config, initial_state),
        |(client, config, mut state)| async {
            loop {
                let (result, new_state) = step(&client, &config, state).await;
                state = new_state;

                if let Some(result) = result {
                    return Some((result, (client, config, state)));
                }
            }
        },
    )
}

async fn step<R: Resource + 'static>(
    client: &Client,
    config: &Config,
    state: State<R>,
) -> (Option<Result<Event<R>, Error>>, State<R>) {
    match state {
        State::Start {
            initial_list_strategy,
            resource_version,
        } => {
            let new_state = match initial_list_strategy {
                InitialListStrategy::ListWatch => State::Listing {
                    continue_token: None,
                    objects: VecDeque::new(),
                    last_bookmark: None,
                },
                InitialListStrategy::StreamingList => {
                    let params = config.watch_params();

                    State::InitialWatch {
                        params,
                        resource_version,
                        initial_list_done: false,
                    }
                }
            };

            (Some(Ok(Event::Init)), new_state)
        }
        State::Listing {
            continue_token,
            mut objects,
            last_bookmark,
        } => {
            if let Some(obj) = objects.pop_front() {
                return (
                    Some(Ok(Event::InitApply(obj))),
                    State::Listing {
                        continue_token,
                        objects,
                        last_bookmark,
                    },
                );
            }

            // check if we need to perform more pages
            if continue_token.is_none() {
                if let Some(resource_version) = last_bookmark {
                    // we have drained the last page - move on to next state
                    debug!(message = "list done, start watching");

                    let mut params = config.watch_params();
                    params.send_initial_events = false;

                    return (
                        Some(Ok(Event::InitDone)),
                        State::InitialWatch {
                            params,
                            resource_version: Some(resource_version),
                            initial_list_done: true,
                        },
                    );
                }
            }

            let params = config.list_params();
            match client.list(&params).await {
                Ok(list) => (
                    None,
                    State::Listing {
                        continue_token: list.metadata.r#continue,
                        objects: VecDeque::from(list.items),
                        last_bookmark: list.metadata.resource_version,
                    },
                ),
                Err(err) => (Some(Err(err)), State::default()),
            }
        }
        State::InitialWatch {
            params,
            resource_version,
            initial_list_done,
        } => {
            let result = match resource_version.as_ref() {
                Some(resource_version) => client.watch::<R>(&params, resource_version).await,
                None => client.watch::<R>(&params, "0").await,
            };

            match result {
                Ok(stream) => (
                    None,
                    State::Watching {
                        resource_version: resource_version.unwrap_or_else(|| "0".to_string()),
                        stream,
                        initial_list_done,
                    },
                ),
                Err(err) => {
                    match &err {
                        Error::Api(err) => {
                            warn!(message = "watch initial list error with 403", ?err);
                        }
                        err => {
                            debug!(message = "watch initial list error", ?err);
                        }
                    }

                    (
                        Some(Err(err)),
                        State::Start {
                            initial_list_strategy: config.initial_list_strategy,
                            resource_version,
                        },
                    )
                }
            }
        }
        State::Watching {
            mut stream,
            resource_version,
            initial_list_done,
        } => {
            match stream.next().await {
                Some(result) => {
                    match result {
                        Ok(event) => {
                            let result = match event {
                                WatchEvent::Added(obj) | WatchEvent::Modified(obj) => {
                                    if initial_list_done {
                                        Ok(Event::Apply(obj))
                                    } else {
                                        Ok(Event::InitApply(obj))
                                    }
                                }
                                WatchEvent::Deleted(obj) => {
                                    // Kubernetes claims these events are impossible
                                    // https://kubernetes.io/docs/reference/using-api/api-concepts/#streaming-lists
                                    error!("got deleted event during initial watch. this is a bug");

                                    Ok(Event::Deleted(obj))
                                }
                                WatchEvent::Bookmark(bookmark) => {
                                    // Added in 1.27, enabled in 1.32
                                    let initial_list_done = bookmark
                                        .metadata
                                        .annotations
                                        .contains_key("k8s.io/initial-events-end");

                                    let event = if initial_list_done {
                                        debug!(message = "initial list done");
                                        Some(Ok(Event::InitDone))
                                    } else {
                                        None
                                    };

                                    return (
                                        event,
                                        State::Watching {
                                            resource_version: bookmark.metadata.resource_version,
                                            stream,
                                            initial_list_done,
                                        },
                                    );
                                }
                                WatchEvent::Error(err) => {
                                    // HTTP GONE, means we have desynced and need to start over and re-list :(
                                    let new_state = if err.code == 410 {
                                        State::Start {
                                            initial_list_strategy: config.initial_list_strategy,
                                            resource_version: Some(resource_version),
                                        }
                                    } else {
                                        State::Watching {
                                            resource_version,
                                            stream,
                                            initial_list_done,
                                        }
                                    };

                                    if err.code == 403 {
                                        warn!(message = "watch event error", ?err);
                                    } else {
                                        debug!(message = "watch event error", ?err);
                                    }

                                    return (Some(Err(Error::Api(err))), new_state);
                                }
                            };

                            (
                                Some(result),
                                State::Watching {
                                    resource_version,
                                    stream,
                                    initial_list_done,
                                },
                            )
                        }
                        Err(err) => (Some(Err(err)), State::default()),
                    }
                }
                None => {
                    debug!(message = "watch stream timeout", timeout = config.timeout);

                    let mut params = config.watch_params();
                    params.send_initial_events = false;

                    // watch timeout
                    (
                        None,
                        State::InitialWatch {
                            params,
                            resource_version: Some(resource_version),
                            initial_list_done,
                        },
                    )
                }
            }
        }
    }
}
