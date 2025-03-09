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

static OBSERVERS: LazyLock<Mutex<BTreeMap<String, Sender<Vec<Endpoint>>>>> =
    LazyLock::new(|| Mutex::new(BTreeMap::new()));

// implement `Drop` for the Observer seems to be a good idea,
// but observable extension might be respawned when config changed, at that time,
// notifier stream will stop and terminate
/*
impl Drop for Observer {
    fn drop(&mut self) {
        OBSERVERS.lock().unwrap().remove(self.name());
    }
}
*/

/// register must be call when build extension, so that sources can subscribe it
/// when build Source.
pub fn register(name: String) -> Observer {
    OBSERVERS
        .lock()
        .unwrap()
        .insert(name.clone(), Sender::new(16));

    Observer { name }
}

/// `subscribe` must be called when build source, cause the topology can valid the
/// config.
pub fn subscribe(name: &str) -> Option<Notifier> {
    let observers = OBSERVERS.lock().unwrap();
    let receiver = observers.get(name)?.subscribe();

    Some(Notifier {
        endpoints: vec![],
        pendings: VecDeque::with_capacity(32),
        receiver: ReusableBoxFuture::new(make_future(receiver)),
    })
}

#[inline]
pub fn available_observers() -> Vec<String> {
    OBSERVERS.lock().unwrap().keys().cloned().collect()
}

pub struct Observer {
    name: String,
}

impl Observer {
    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[inline]
    pub fn publish(&mut self, endpoints: Vec<Endpoint>) -> Result<(), Error> {
        let mut observers = OBSERVERS.lock().unwrap();

        match observers.get_mut(self.name()) {
            Some(sender) => {
                sender.send(endpoints).map_err(|_| Error::Closed)?;
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
    // endpoints looks like not necessary but we need it,
    // if new subscriber create when vertex reload
    endpoints: Vec<Endpoint>,
    pendings: VecDeque<Change>,

    receiver:
        ReusableBoxFuture<'static, (Result<Vec<Endpoint>, RecvError>, Receiver<Vec<Endpoint>>)>,
}

impl Stream for Notifier {
    type Item = Change;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if let Some(change) = self.pendings.pop_front() {
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
            Ok(endpoints) => {
                let changes = changes(&self.endpoints, &endpoints);
                if changes.is_empty() {
                    return Poll::Pending;
                }

                self.pendings.extend(changes);
                self.endpoints = endpoints;

                Poll::Ready(self.pendings.pop_front())
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

async fn make_future(
    mut receiver: Receiver<Vec<Endpoint>>,
) -> (Result<Vec<Endpoint>, RecvError>, Receiver<Vec<Endpoint>>) {
    let result = receiver.recv().await;
    (result, receiver)
}

fn changes(existing: &[Endpoint], new_endpoints: &[Endpoint]) -> Vec<Change> {
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

    #[tokio::test]
    async fn pubsub() {
        let name = "pubsub";
        let mut observer = register(name.to_string());
        let mut notifier = subscribe(name).unwrap();

        fn mock_endpoint(id: i32, target: i32) -> Endpoint {
            Endpoint {
                id: id.to_string(),
                target: target.to_string(),
                details: Value::Null,
            }
        }

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
