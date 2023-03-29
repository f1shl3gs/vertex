use std::fmt::Formatter;

use chrono::{DateTime, Utc};
use serde::de::MapAccess;
use serde::{Deserialize, Deserializer};

use super::{Client, Error, Gauge, TaskManager};

pub const STATUS_PATH: &str = "/xml/v3/status";

const SERVER_PATH: &str = "/xml/v3/server";
const TASKS_PATH: &str = "/xml/v3/tasks";
const ZONES_PATH: &str = "/xml/v3/zones";

#[derive(Deserialize)]
struct Counters {
    #[serde(rename = "@type")]
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
struct Cache {
    #[serde(default, rename = "rrset")]
    counters: Vec<Gauge>,
}

struct View {
    name: String,
    cache: Cache,
    counters: Vec<Counters>,
}

// serde_xml_rs cannot handle array if the elements is unordered, e.g.
// <store>
//   <foo/>
//   <bar/>
//   <foo/>
// </store>
//
// https://github.com/RReverser/serde-xml-rs/issues/5
impl<'de> Deserialize<'de> for View {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        const FIELDS: &[&str] = &["secs", "nanos"];

        enum Field {
            Name,
            Cache,
            Counters,
        }

        impl<'de> Deserialize<'de> for Field {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct FieldVisitor;

                impl<'de> serde::de::Visitor<'de> for FieldVisitor {
                    type Value = Field;

                    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                        formatter.write_str("`name`, `cache` or `counters`")
                    }

                    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                    where
                        E: serde::de::Error,
                    {
                        match v {
                            "@name" => Ok(Field::Name),
                            "cache" => Ok(Field::Cache),
                            "counters" => Ok(Field::Counters),
                            _ => Err(serde::de::Error::unknown_field(v, FIELDS)),
                        }
                    }
                }

                deserializer.deserialize_identifier(FieldVisitor)
            }
        }

        struct ViewVisitor;

        impl<'de> serde::de::Visitor<'de> for ViewVisitor {
            type Value = View;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("struct View")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut name = None;
                let mut cache = None;
                let mut counters: Option<Vec<Counters>> = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Name => {
                            if name.is_some() {
                                return Err(serde::de::Error::duplicate_field("name"));
                            }

                            name = Some(map.next_value::<String>()?)
                        }
                        Field::Cache => {
                            if cache.is_some() {
                                return Err(serde::de::Error::duplicate_field("cache"));
                            }

                            cache = Some(map.next_value::<Cache>()?);
                        }
                        Field::Counters => {
                            let value = map.next_value::<Vec<Counters>>()?;
                            let mut cs = counters.unwrap_or_default();
                            cs.extend(value);
                            counters = Some(cs);
                        }
                    }
                }

                let name = name.unwrap_or_default();
                let cache = cache.ok_or_else(|| serde::de::Error::missing_field("cache"))?;
                let counters = counters.unwrap_or_default();

                Ok(View {
                    name,
                    cache,
                    counters,
                })
            }
        }

        deserializer.deserialize_struct("View", FIELDS, ViewVisitor)
    }
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

        for view in stats.views.views {
            let mut v = super::View {
                name: view.name,
                cache: view.cache.counters,
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
        let xd = &mut quick_xml::de::Deserializer::from_reader(data.reader());
        let result: Result<Statistics, _> = serde_path_to_error::deserialize(xd);

        let stats = match result {
            Ok(s) => s,
            Err(err) => {
                let inner = err.inner();
                let path = err.path();
                panic!("{} {:?}", path, inner)
            }
        };

        assert_eq!(stats.server.boot_time, DateTime::parse_from_rfc3339("2021-07-15T05:11:08.926Z").unwrap());
        assert_eq!(stats.taskmgr.thread_model.worker_threads, 5);
        assert_eq!(stats.views.views.len(), 2)
    }

    #[test]
    fn decode_zones() {
        let data = std::fs::read("tests/fixtures/bind/v3/zones").unwrap();
        let xd = &mut quick_xml::de::Deserializer::from_reader(data.reader());
        let result: Result<ZoneStatistics, _> = serde_path_to_error::deserialize(xd);
        if let Err(err) = result {
            let inner = err.inner();
            let path = err.path();
            panic!("{} {:?}", path, inner)
        }
    }
}
