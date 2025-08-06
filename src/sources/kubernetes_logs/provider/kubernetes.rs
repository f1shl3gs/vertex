use std::collections::HashMap;

use futures::StreamExt;
use futures::stream::BoxStream;
use kubernetes::{Client, Error, Event, WatchConfig, watcher};

use super::pod::Pod;

pub struct KubernetesProvider {
    stream: BoxStream<'static, Result<Event<Pod>, Error>>,

    pods: HashMap<String, Pod>,

    // wait a while to ensure this component can tail all data
    deleted: HashMap<String, Pod>,
}

impl KubernetesProvider {
    pub fn new(
        label_selector: Option<String>,
        field_selector: Option<String>,
    ) -> Result<Self, Error> {
        let client = Client::new(None)?;

        let config = WatchConfig {
            label_selector,
            field_selector,
            bookmark: true,
            ..Default::default()
        };

        Ok(KubernetesProvider {
            stream: watcher::<Pod>(client, config).boxed(),
            pods: Default::default(),
            deleted: Default::default(),
        })
    }

    pub async fn list(&mut self) -> std::io::Result<Vec<Pod>> {
        while let Some(result) = self.stream.next().await {
            match result {
                Ok(event) => match event {
                    Event::Apply(pod) => {
                        self.pods.insert(pod.metadata.uid.clone(), pod);
                        break;
                    }
                    Event::Deleted(pod) => {
                        match self.pods.remove(&pod.metadata.uid) {
                            None => {}
                            Some(_pod) => {
                                self.deleted.insert(pod.metadata.uid.clone(), pod);
                            }
                        }
                        break;
                    }
                    Event::InitDone => break,
                    Event::Init => {}
                    Event::InitApply(pod) => {
                        self.pods.insert(pod.metadata.uid.clone(), pod);
                    }
                },
                Err(err) => {
                    error!(
                        message = "wait next event failed",
                        %err
                    );
                }
            }
        }

        let pods = self.pods.values().cloned().collect::<Vec<_>>();

        Ok(pods)
    }
}
