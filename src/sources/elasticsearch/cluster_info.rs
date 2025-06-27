use event::{Metric, tags};
use serde::Deserialize;

use super::Elasticsearch;

/// `Version` is the version info retrievable from the / endpoint
#[allow(dead_code)]
#[derive(Deserialize)]
struct Version {
    number: String,
    build_flavor: String,
    build_type: String,
    build_hash: String,
    build_date: String, // it should be DateTime
    build_snapshot: bool,
    lucene_version: String,
    minimum_wire_compatibility_version: String,
    minimum_index_compatibility_version: String,
}

/// `ClusterInfo` is the cluster info retrievable from the / endpoint
#[allow(dead_code)]
#[derive(Deserialize)]
struct ClusterInfo {
    name: String,
    cluster_name: String,
    cluster_uuid: String,
    version: Version,
    tagline: String,
}

impl Elasticsearch {
    pub async fn cluster_info(&self) -> Vec<Metric> {
        match self.fetch::<ClusterInfo>("/").await {
            Ok(info) => {
                vec![Metric::gauge_with_tags(
                    "elasticsearch_version",
                    "Elasticsearch version information",
                    1,
                    tags!(
                        "cluster" => info.cluster_name,
                        "cluster_uuid" => info.cluster_uuid,
                        "build_date" => info.version.build_date,
                        "build_hash" => info.version.build_hash,
                        "version" => info.version.number,
                        "lucene_version" => info.version.lucene_version
                    ),
                )]
            }
            Err(err) => {
                error!(message = "Fetch elasticsearch cluster info failed", %err);
                vec![]
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode() {
        let input = r#"{
  "name" : "58a07c8bfb2c",
  "cluster_name" : "docker-cluster",
  "cluster_uuid" : "mgv5kDdxR8aEN0kFuse3kA",
  "version" : {
    "number" : "7.17.5",
    "build_flavor" : "default",
    "build_type" : "docker",
    "build_hash" : "8d61b4f7ddf931f219e3745f295ed2bbc50c8e84",
    "build_date" : "2022-06-23T21:57:28.736740635Z",
    "build_snapshot" : false,
    "lucene_version" : "8.11.1",
    "minimum_wire_compatibility_version" : "6.8.0",
    "minimum_index_compatibility_version" : "6.0.0-beta1"
  },
  "tagline" : "You Know, for Search"
}
"#;
        let xd = &mut serde_json::Deserializer::from_str(input);
        let result: Result<ClusterInfo, _> = serde_path_to_error::deserialize(xd);
        if let Err(err) = result {
            let inner = err.inner();
            let path = err.path();
            panic!("{path} {inner:?}")
        }
    }
}
