use std::time::Duration;

use futures::StreamExt;
use kubernetes::{Client, ObjectMeta, Resource};
use kubernetes::{Event, InitialListStrategy, WatchConfig, watcher};
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
    tracing_subscriber::fmt::init();

    let client = Client::new(None).unwrap();

    let version = client.version().await.unwrap();
    println!("api server version: {}.{}", version.major, version.minor);

    let version = format!("{}.{}", version.major, version.minor)
        .parse::<f64>()
        .unwrap();
    let config = WatchConfig {
        label_selector: None,
        field_selector: None,
        timeout: Some(10),
        initial_list_strategy: {
            if version >= 1.32 {
                // send_initial_events is added in 1.27, but it is not enabled by default,
                // it is enabled by default in 1.32
                InitialListStrategy::StreamingList
            } else {
                InitialListStrategy::ListWatch
            }
        },
        bookmark: true,
    };

    println!("initial list strategy: {:?}", config.initial_list_strategy);

    let stream = watcher::<Pod>(client, config);
    tokio::pin!(stream);

    while let Some(result) = stream.next().await {
        match result {
            Ok(event) => match event {
                Event::Apply(pod) => {
                    println!("apply pod {}/{}", pod.metadata.namespace, pod.metadata.name);
                }
                Event::Deleted(pod) => {
                    println!(
                        "deleted pod {}/{}",
                        pod.metadata.namespace, pod.metadata.name
                    );
                }
                Event::Init => {
                    println!("init start");
                }
                Event::InitApply(pod) => {
                    println!(
                        "init apply pod {}/{}",
                        pod.metadata.namespace, pod.metadata.name
                    );
                }
                Event::InitDone => {
                    println!("init done");
                }
            },
            Err(err) => {
                println!("poll next {err:?}");

                // backoff
                tokio::time::sleep(Duration::from_secs(5)).await;

                break;
            }
        }
    }
}
