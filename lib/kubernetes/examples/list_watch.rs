use futures::StreamExt;
use kubernetes::{Client, ListParams, ObjectMeta, Resource, WatchEvent, WatchParams};
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

    let mut params = ListParams {
        label_selector: None,
        field_selector: None,
        timeout: None,
        limit: Some(2),
        continue_token: None,
        version_match: None,
        resource_version: None,
    };

    loop {
        let list = client.list::<Pod>(&params).await.unwrap();
        println!(
            "{:?} {:?}",
            list.metadata.r#continue, list.metadata.resource_version
        );

        for pod in list.items {
            println!("{}/{}", pod.metadata.namespace, pod.metadata.name);
        }

        if let Some(resource_version) = list.metadata.resource_version {
            params.resource_version = Some(resource_version);
        }

        match list.metadata.r#continue {
            Some(continuation_token) => {
                params.continue_token = Some(continuation_token);
            }
            None => break,
        }
    }

    println!("list done");

    // watch
    let mut resource_version = params.resource_version.unwrap();
    let params = WatchParams {
        label_selector: params.label_selector,
        field_selector: params.field_selector,
        timeout: None,
        bookmarks: true,
        send_initial_events: false,
    };

    loop {
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
                        resource_version = bookmark.metadata.resource_version;
                        println!("bookmark: {resource_version}");
                    }
                    WatchEvent::Error(err) => {
                        println!("error event: {:?}", err);
                    }
                },
                Err(err) => {
                    println!("poll next {:?}", err);
                }
            }
        }

        println!("watch timeout, re-watching pods...")
    }
}
