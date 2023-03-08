use chrono::{DateTime, Utc};
use serde::Deserialize;

use super::{Client, Error, Gauge, TaskManager};

#[derive(Deserialize)]
pub struct Counter {
    pub name: String,
    pub counter: u64,
}

impl From<Counter> for super::Counter {
    fn from(c: Counter) -> Self {
        super::Counter {
            name: c.name,
            counter: c.counter,
        }
    }
}

#[derive(Deserialize)]
struct QueriesIn {
    #[serde(default, rename = "rdtype")]
    counters: Vec<Counter>,
}

#[derive(Deserialize)]
struct Requests {
    #[serde(default, rename = "opcode")]
    counters: Vec<Counter>,
}

#[derive(Deserialize)]
struct Server {
    #[serde(rename = "boot-time")]
    boot_time: DateTime<Utc>,
    #[serde(rename = "nsstat")]
    nsstats: Vec<Counter>,
    #[serde(rename = "queries-in")]
    queries_in: QueriesIn,
    requests: Requests,
    #[serde(default, rename = "zonestat")]
    zonestats: Vec<Counter>,
}

#[derive(Deserialize)]
struct Zone {
    name: String,
    rdataclass: String,
    serial: String,
}

#[derive(Deserialize)]
struct Zones {
    #[serde(default, rename = "zone")]
    zones: Vec<Zone>,
}

#[derive(Deserialize)]
struct Cache {
    #[serde(default, rename = "rrset")]
    counters: Vec<Gauge>,
}

#[derive(Deserialize)]
struct View {
    name: String,
    cache: Cache,
    #[serde(default)]
    rdtype: Vec<Counter>,
    #[serde(default)]
    resstat: Vec<Counter>,
    zones: Zones,
}

#[derive(Deserialize)]
struct Views {
    #[serde(default, rename = "view")]
    views: Vec<View>,
}

#[derive(Deserialize)]
struct Statistics {
    server: Server,
    taskmgr: TaskManager,
    views: Views,
}

#[derive(Deserialize)]
struct Bind {
    statistics: Statistics,
}

#[derive(Deserialize)]
struct Isc {
    bind: Bind,
}

impl Client {
    pub(super) async fn v2(&self) -> Result<super::Statistics, Error> {
        let mut s = super::Statistics::default();
        let root = self.fetch::<Isc>("/").await?;

        let stats = root.bind.statistics;

        s.server.boot_time = stats.server.boot_time;
        for c in stats.server.queries_in.counters {
            s.server.incoming_queries.push(c.into());
        }
        for c in stats.server.requests.counters {
            s.server.incoming_requests.push(c.into());
        }
        for c in stats.server.nsstats {
            s.server.name_server_stats.push(c.into());
        }
        for c in stats.server.zonestats {
            s.server.zone_statistics.push(c.into());
        }
        for view in stats.views.views {
            let mut v = super::View {
                name: view.name.clone(),
                cache: view.cache.counters,
                resolver_stats: vec![],
                resolver_queries: vec![],
            };
            let mut zv = super::ZoneView {
                name: view.name,
                zone_data: vec![],
            };
            for c in view.rdtype {
                v.resolver_queries.push(c.into());
            }
            for c in view.resstat {
                v.resolver_stats.push(c.into());
            }
            for zone in view.zones.zones {
                if zone.rdataclass != "IN" {
                    continue;
                }

                zv.zone_data.push(super::ZoneCounter {
                    name: zone.name,
                    serial: zone.serial,
                });
            }
            s.zone_views.push(zv);
            s.views.push(v);
        }

        s.task_manager = stats.taskmgr;

        Ok(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Buf;

    #[test]
    fn decode() {
        let data = std::fs::read("tests/fixtures/bind/v2.xml").unwrap();
        let xd = &mut serde_xml_rs::Deserializer::new_from_reader(data.reader());
        let result: Result<Isc, _> = serde_path_to_error::deserialize(xd);
        if let Err(err) = result {
            let inner = err.inner();
            let path = err.path();
            panic!("{} {:?}", path, inner)
        }
    }
}
