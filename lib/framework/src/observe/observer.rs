use std::collections::{BTreeMap, VecDeque};
use std::pin::Pin;
use std::sync::{LazyLock, Mutex};
use std::task::{Context, Poll};

use futures::Stream;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::broadcast::{Receiver, Sender};
use tokio_util::sync::ReusableBoxFuture;

use super::endpoint::Endpoint;

#[derive(Debug)]
pub enum Error {
    Closed,
    NotFound,
}

static OBSERVERS: LazyLock<Mutex<BTreeMap<String, (Vec<Endpoint>, Sender<Vec<Change>>)>>> =
    LazyLock::new(|| Mutex::new(BTreeMap::new()));

#[inline]
pub fn available_observers() -> Vec<String> {
    OBSERVERS.lock().unwrap().keys().cloned().collect()
}

// return all observer's current endpoints
pub fn current_endpoints() -> BTreeMap<String, Vec<Endpoint>> {
    let observers = OBSERVERS.lock().unwrap();
    let mut endpoints = BTreeMap::new();

    for (name, (current, _sender)) in observers.iter() {
        endpoints.insert(name.clone(), current.clone());
    }

    endpoints
}

/// Return the receiver count of the sender
pub fn receiver_count(name: &str) -> Option<usize> {
    let observers = OBSERVERS.lock().unwrap();
    let (_, sender) = observers.get(name)?;
    Some(sender.receiver_count())
}

pub struct Observer {
    name: String,
}

impl Observer {
    /// register must be call when build extension, so that sources can subscribe it
    /// when build Source.
    pub fn register(name: impl Into<String>) -> Observer {
        let name = name.into();

        OBSERVERS
            .lock()
            .unwrap()
            .insert(name.clone(), (Vec::new(), Sender::new(16)));

        Observer { name }
    }

    #[inline]
    pub fn publish(&self, endpoints: Vec<Endpoint>) -> Result<(), Error> {
        let mut observers = OBSERVERS.lock().unwrap();

        match observers.get_mut(&self.name) {
            Some((current, sender)) => {
                let changes = build_changes(current, &endpoints);
                if changes.is_empty() {
                    return Ok(());
                }

                // update cached endpoints for later subscribe
                *current = endpoints;

                sender.send(changes).map_err(|_| Error::Closed)?;
                Ok(())
            }
            None => Err(Error::NotFound),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Change {
    Add(Vec<Endpoint>),
    Remove(Vec<Endpoint>),
    Update(Vec<Endpoint>),
}

pub struct Notifier {
    pending: VecDeque<Change>,

    receiver: ReusableBoxFuture<'static, (Result<Vec<Change>, RecvError>, Receiver<Vec<Change>>)>,
}

impl Stream for Notifier {
    type Item = Change;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if let Some(change) = self.pending.pop_front() {
            return Poll::Ready(Some(change));
        }

        let result = match self.receiver.poll(cx) {
            Poll::Ready((result, receiver)) => {
                self.receiver.set(make_future(receiver));
                result
            }
            Poll::Pending => {
                return Poll::Pending;
            }
        };

        match result {
            Ok(changes) => {
                self.pending.extend(changes);
                Poll::Ready(self.pending.pop_front())
            }
            Err(err) => match err {
                RecvError::Closed => {
                    println!("observer closed");
                    Poll::Ready(None)
                }
                RecvError::Lagged(_) => Poll::Pending,
            },
        }
    }
}

impl Notifier {
    /// `subscribe` must be called when build source, cause the topology can valid the
    /// config.
    pub fn subscribe(name: &str) -> Option<Notifier> {
        let observers = OBSERVERS.lock().unwrap();
        let (current, sender) = observers.get(name)?;
        let mut pending = VecDeque::new();
        if !current.is_empty() {
            pending.push_back(Change::Add(current.clone()));
        }

        let receiver = sender.subscribe();

        Some(Notifier {
            pending,
            receiver: ReusableBoxFuture::new(make_future(receiver)),
        })
    }
}

async fn make_future(
    mut receiver: Receiver<Vec<Change>>,
) -> (Result<Vec<Change>, RecvError>, Receiver<Vec<Change>>) {
    let result = receiver.recv().await;
    (result, receiver)
}

fn build_changes(existing: &[Endpoint], new_endpoints: &[Endpoint]) -> Vec<Change> {
    let mut to_add = Vec::new();
    let mut to_remove = Vec::new();
    let mut to_update = Vec::new();

    for new in new_endpoints {
        match existing.iter().find(|existing| existing.id == new.id) {
            Some(existing) => {
                if !existing.eq(new) {
                    to_update.push(new.clone());
                }
            }
            None => {
                to_add.push(new.clone());
            }
        }
    }

    for existing in existing {
        if !new_endpoints.iter().any(|new| new.id == existing.id) {
            to_remove.push(existing.clone());
        }
    }

    let mut changes = Vec::with_capacity(3);
    if !to_remove.is_empty() {
        changes.push(Change::Remove(to_remove));
    }

    if !to_update.is_empty() {
        changes.push(Change::Update(to_update));
    }

    if !to_add.is_empty() {
        changes.push(Change::Add(to_add));
    }

    changes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::collect_ready;

    use value::Value;

    fn mock_endpoint(id: i32, target: i32) -> Endpoint {
        Endpoint {
            id: id.to_string(),
            typ: "test".into(),
            target: target.to_string(),
            details: Value::Null,
        }
    }

    #[tokio::test]
    async fn subscribe_in_the_middle() {
        let name = "foo";
        let observer = Observer::register(name.to_string());
        let mut notifier = Notifier::subscribe(name).unwrap();

        // first state empty
        let changes = collect_ready(&mut notifier);
        assert!(changes.is_empty());

        // pub empty, so it is empty too
        observer.publish(vec![]).unwrap();
        let changes = collect_ready(&mut notifier);
        assert!(changes.is_empty());

        observer.publish(vec![mock_endpoint(1, 2)]).unwrap();
        let changes = collect_ready(&mut notifier);
        assert_eq!(changes, vec![Change::Add(vec![mock_endpoint(1, 2)])]);

        let mut notifier2 = Notifier::subscribe(name).unwrap();
        let changes = collect_ready(&mut notifier2);
        assert_eq!(changes, vec![Change::Add(vec![mock_endpoint(1, 2)])]);

        observer
            .publish(vec![mock_endpoint(1, 2), mock_endpoint(2, 3)])
            .unwrap();
        let changes1 = collect_ready(&mut notifier);
        let changes2 = collect_ready(&mut notifier2);
        assert_eq!(changes1, vec![Change::Add(vec![mock_endpoint(2, 3)])]);
        assert_eq!(changes1, changes2);
    }

    #[tokio::test]
    async fn pubsub() {
        let name = "pubsub";
        let observer = Observer::register(name.to_string());
        let mut notifier = Notifier::subscribe(name).unwrap();

        for (input, changes) in [
            (vec![], vec![]),
            (vec![(1, 2)], vec![Change::Add(vec![mock_endpoint(1, 2)])]),
            (vec![(1, 2)], vec![]),
            (vec![], vec![Change::Remove(vec![mock_endpoint(1, 2)])]),
            (vec![], vec![]),
            (
                vec![(1, 1), (2, 2), (3, 3)],
                vec![Change::Add(vec![
                    mock_endpoint(1, 1),
                    mock_endpoint(2, 2),
                    mock_endpoint(3, 3),
                ])],
            ),
            (vec![(1, 1), (2, 2), (3, 3)], vec![]),
            (
                vec![(1, 2), (2, 2), (4, 4)],
                vec![
                    Change::Remove(vec![mock_endpoint(3, 3)]),
                    Change::Update(vec![mock_endpoint(1, 2)]),
                    Change::Add(vec![mock_endpoint(4, 4)]),
                ],
            ),
        ] {
            let endpoints = input
                .into_iter()
                .map(|(id, target)| mock_endpoint(id, target))
                .collect::<Vec<_>>();

            observer.publish(endpoints).unwrap();

            let got = collect_ready(&mut notifier);
            assert_eq!(got, changes);
        }
    }
}
