use std::time::Duration;

use futures::StreamExt;
use kubernetes::{Client, ObjectMeta, Resource, WatchEvent, WatchParams};
use serde::Deserialize;

/// Pod is a collection of containers that can run on a host. This resource
/// is created by clients and scheduled onto hosts.
#[derive(Debug, Deserialize)]
pub struct Pod {
    /// Standard object's metadata.
    ///
    /// More info: https://git.k8s.io/community/contributors/devel/sig-architecture/api-conventions.md#metadata
    pub metadata: ObjectMeta,
}

impl Resource for Pod {
    const GROUP: &'static str = "";
    const VERSION: &'static str = "v1";
    const KIND: &'static str = "Pod";
    const PLURAL: &'static str = "pods";
}

#[tokio::main]
async fn main() {
    let client = Client::new(None).unwrap();

    let version = client.version().await.unwrap();
    println!("api server version: {}.{}", version.major, version.minor);

    // NB: This example is Streaming List which is implement in Kubernetes 1.27,
    // earlier version only support ListWatch with pagination
    let mut resource_version = "0".to_string();
    let mut send_initial_events = true;
    loop {
        let params = WatchParams {
            label_selector: None,
            field_selector: None,
            timeout: None,
            bookmarks: true,
            send_initial_events,
        };

        let stream = client
            .watch::<Pod>(&params, resource_version.clone())
            .await
            .unwrap();
        tokio::pin!(stream);

        while let Some(result) = stream.next().await {
            match result {
                Ok(watch_event) => match watch_event {
                    WatchEvent::Added(pod) => {
                        println!("add pod: {:?}", pod.metadata.uid);
                    }
                    WatchEvent::Modified(pod) => {
                        println!("modify pod: {:?}", pod.metadata.uid);
                    }
                    WatchEvent::Deleted(pod) => {
                        println!("delete pod: {:?}", pod.metadata.uid);
                    }
                    WatchEvent::Bookmark(bookmark) => {
                        let marks_initial_end = bookmark
                            .metadata
                            .annotations
                            .contains_key("k8s.io/initial-events-end");
                        if marks_initial_end {
                            send_initial_events = false;
                            println!("initial done");
                        }
                        resource_version = bookmark.metadata.resource_version;
                        println!("bookmark: {resource_version}");
                    }
                    WatchEvent::Error(err) => {
                        println!("error event: {:?}", err);
                    }
                },
                Err(err) => {
                    println!("poll next {:?}", err);

                    // backoff
                    tokio::time::sleep(Duration::from_secs(5)).await;

                    break;
                }
            }
        }

        println!("watch timeout, re-watching pods...");
    }
}
