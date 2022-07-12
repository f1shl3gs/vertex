use crate::sources::bind::client::{Client, Error, Gauge, TaskManager};
use chrono::{DateTime, Utc};
use serde::Deserialize;

pub const STATUS_PATH: &str = "/xml/v3/status";

const SERVER_PATH: &str = "/xml/v3/server";
const TASKS_PATH: &str = "/xml/v3/tasks";
const ZONES_PATH: &str = "/xml/v3/zones";

#[derive(Deserialize)]
struct Counters {
    #[serde(rename = "type")]
    typ: String,
    #[serde(default, rename = "counter")]
    counters: Vec<super::Counter>,
}

#[derive(Deserialize)]
struct Server {
    #[serde(rename = "boot-time")]
    boot_time: DateTime<Utc>,
    #[serde(rename = "config-time")]
    config_time: DateTime<Utc>,
    #[serde(default)]
    counters: Vec<Counters>,
}

#[derive(Deserialize)]
struct View {
    name: String,
    #[serde(default)]
    cache: Vec<Gauge>,
    #[serde(default)]
    counters: Vec<Counters>,
}

#[derive(Deserialize)]
struct Statistics {
    server: Server,
    taskmgr: TaskManager,
    #[serde(default, rename = "view")]
    views: Vec<View>,
}

#[derive(Deserialize)]
struct ZoneCounter {
    name: String,
    rdataclass: String,
    serial: u64,
}

#[derive(Deserialize)]
struct ZoneView {
    name: String,
    zones: Vec<ZoneCounter>,
}

#[derive(Deserialize, Default)]
struct ZoneStatistics {
    #[serde(default, rename = "view")]
    views: Vec<ZoneView>,
}

impl Client {
    pub(super) async fn v3(&self) -> Result<super::Statistics, Error> {
        let mut s = super::Statistics::default();

        let stats = self.fetch::<Statistics>(SERVER_PATH).await?;
        s.server.boot_time = stats.server.boot_time;
        s.server.config_time = stats.server.config_time;
        for cs in stats.server.counters {
            match cs.typ.as_str() {
                "opcode" => s.server.incoming_requests.extend(cs.counters),
                "qtype" => s.server.incoming_queries.extend(cs.counters),
                "nsstat" => s.server.name_server_stats.extend(cs.counters),
                "zonestat" => s.server.zone_statistics.extend(cs.counters),
                "rcode" => s.server.server_rcodes.extend(cs.counters),
                _ => {} // this shall not happen
            }
        }

        for view in stats.views {
            let mut v = super::View {
                name: view.name,
                cache: view.cache,
                resolver_stats: vec![],
                resolver_queries: vec![],
            };
            for cs in view.counters {
                match cs.typ.as_str() {
                    "resqtype" => v.resolver_queries.extend(cs.counters),
                    "resstats" => v.resolver_stats.extend(cs.counters),
                    _ => {} // this shall not happen
                }
            }

            s.views.push(v);
        }

        let zonestats = self.fetch::<ZoneStatistics>(ZONES_PATH).await?;
        for view in zonestats.views {
            let mut v = super::ZoneView {
                name: view.name,
                zone_data: vec![],
            };
            for zone in view.zones {
                if zone.rdataclass != "IN" {
                    continue;
                }

                v.zone_data.push(super::ZoneCounter {
                    name: zone.name,
                    // TODO: does this `to_string` really necessary!?
                    serial: zone.serial.to_string(),
                });
            }
            s.zone_views.push(v);
        }

        let ts = self.fetch::<Statistics>(TASKS_PATH).await?;
        s.task_manager = ts.taskmgr;

        Ok(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Buf;

    #[test]
    fn decode_server() {
        let data = std::fs::read("tests/fixtures/bind/v3/server").unwrap();
        let xd = &mut serde_xml_rs::Deserializer::new_from_reader(data.reader());
        let result: Result<Statistics, _> = serde_path_to_error::deserialize(xd);
        if let Err(err) = result {
            panic!("{} {:?}", err.path().to_string(), err.into_inner())
        }
    }

    #[test]
    fn decode_zones() {
        let data = std::fs::read("tests/fixtures/bind/v3/zones").unwrap();
        let xd = &mut serde_xml_rs::Deserializer::new_from_reader(data.reader());
        let result: Result<ZoneStatistics, _> = serde_path_to_error::deserialize(xd);
        if let Err(err) = result {
            panic!("{} {:?}", err.path().to_string(), err.into_inner())
        }
    }
}
