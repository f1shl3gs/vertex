use futures::StreamExt;
use kubernetes::resource::pod::Pod;
use kubernetes::{Client, Config, WatchEvent, WatchParams};

#[tokio::main]
async fn main() {
    let config = Config::load().unwrap();

    let client = Client::new(config, None);

    let version = client.version().await.unwrap();
    println!("api server version: {}.{}", version.major, version.minor);

    // NB: This example is Streaming List which is implement in Kubernetes 1.27,
    // earlier version only support ListWatch with pagination
    let mut resource_version = "0".to_string();
    loop {
        let param = WatchParams {
            label_selector: None,
            field_selector: None,
            timeout: None,
            bookmarks: true,
            send_initial_events: false,
        };
        let stream = client
            .watch::<Pod>(param, resource_version.clone())
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

        println!("watch timeout, re-watching pods...");
    }
}
