# Kubernetes

This Kubernetes library intentionally does not aspire to include every feature in `kube-rs`
and specifically targets the use case and feature set 
needed by us.

- Ultra lightweight
- Resources agnostic

## StreamingList
StreamingList is added in 1.27, but it is not enabled by default.
Then, it is enabled by default in 1.32.x

https://kubernetes.io/docs/reference/command-line-tools-reference/feature-gates/

## Resource or CRD
This Library does not provide any Resources or CRD.

If you want to add something you need, just implement
`Resource`
```rust
use serde::Deserialize;
use kubernetes::resource::{Metadata, Resource};

#[derive(Deserialize)]
pub struct JobSpec {
    
}

/// Add a link to the Resource is very necessary, case there might be a lot
/// of `Job` with difference GROUP and VERSION
///
/// https://kubernetes.io/docs/reference/generated/kubernetes-api/v1.31/#job-v1-batch
#[derive(Deserialize)]
pub struct Job {
    metadata: Metadata,
    spec: JobSpec
}

impl Resource for Job {
    /// The group of the resource, or the empty string if the resource doesn't have a
    /// group.
    const GROUP: &'static str = "batch";

    /// The version of the resource.
    const VERSION: &'static str = "v1";

    /// The plural of this resource, which is used to construct URLS
    const PLURAL: &'static str = "jobs";
    
    // you might not need to implement this   
    fn url_path(namespace: Option<&str>) -> String {
        todo!()
    }
}

#[tokio::main]
async fn main() {
    // do something with Job
}
```
