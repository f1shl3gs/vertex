use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::Deserialize;

use super::{Client, Counter, Error, Gauge};

// ServerPath is the HTTP path of the JSON v1 server resource.
const SERVER_PATH: &str = "/json/v1/server";
// TasksPath is the HTTP path of the JSON v1 tasks resource.
const TASKS_PATH: &str = "/json/v1/tasks";
// ZonesPath is the HTTP path of the JSON v1 zones resource.
const ZONES_PATH: &str = "/json/v1/zones";

#[derive(Deserialize)]
struct Resolver {
    #[serde(default)]
    cache: HashMap<String, u64>,
    #[serde(default)]
    qtypes: HashMap<String, u64>,
    #[serde(default)]
    stats: HashMap<String, u64>,
}

#[derive(Deserialize)]
struct View {
    resolver: Resolver,
}

#[derive(Deserialize)]
struct Statistics {
    #[serde(rename = "boot-time")]
    boot_time: DateTime<Utc>,
    #[serde(rename = "config-time")]
    config_time: DateTime<Utc>,
    #[serde(default)]
    opcodes: HashMap<String, u64>,
    #[serde(default)]
    qtypes: HashMap<String, u64>,
    #[serde(default)]
    nsstats: HashMap<String, u64>,
    #[serde(default)]
    rcodes: HashMap<String, u64>,
    #[serde(default)]
    zonestats: HashMap<String, u64>,
    #[serde(default)]
    views: HashMap<String, View>,
}

#[derive(Deserialize)]
struct Zone {
    name: String,
    class: String,
    serial: u32,
}

#[derive(Deserialize)]
struct ZoneView {
    zones: Vec<Zone>,
}

#[derive(Deserialize)]
struct ZoneStatistics {
    views: HashMap<String, ZoneView>,
}

#[derive(Deserialize)]
struct TaskManager {
    #[serde(rename = "tasks-running")]
    tasks_running: u64,
    #[serde(rename = "worker-threads")]
    worker_threads: u64,
}

#[derive(Deserialize)]
struct TaskStatistics {
    taskmgr: TaskManager,
}

impl Client {
    pub(super) async fn json_v1(&self) -> Result<super::Statistics, Error> {
        let mut output = super::Statistics::default();

        let server = self.fetch::<Statistics>(SERVER_PATH).await?;
        output.server.boot_time = server.boot_time;
        output.server.config_time = server.config_time;

        for (name, value) in server.opcodes {
            output
                .server
                .incoming_requests
                .push(Counter { name, value });
        }
        for (name, value) in server.qtypes {
            output.server.incoming_queries.push(Counter { name, value });
        }
        for (name, value) in server.nsstats {
            output
                .server
                .name_server_stats
                .push(Counter { name, value });
        }
        for (name, value) in server.rcodes {
            output.server.server_rcodes.push(Counter { name, value });
        }
        for (name, value) in server.zonestats {
            output.server.zone_statistics.push(Counter { name, value });
        }

        for (name, view) in server.views {
            let cache = view
                .resolver
                .cache
                .into_iter()
                .map(|(name, value)| Gauge { name, value })
                .collect::<Vec<_>>();
            let resolver_queries = view
                .resolver
                .qtypes
                .into_iter()
                .map(|(name, value)| Counter { name, value })
                .collect::<Vec<_>>();
            let resolver_stats = view
                .resolver
                .stats
                .into_iter()
                .map(|(name, value)| Counter { name, value })
                .collect::<Vec<_>>();

            output.views.push(super::View {
                name,
                cache,
                resolver_stats,
                resolver_queries,
            })
        }

        let zone = self.fetch::<ZoneStatistics>(ZONES_PATH).await?;
        for (name, view) in zone.views {
            let zone_data = view
                .zones
                .into_iter()
                .filter(|zone| zone.class == "IN")
                .map(|zone| super::ZoneCounter {
                    name: zone.name,
                    serial: zone.serial,
                })
                .collect::<Vec<_>>();

            output.zone_views.push(super::ZoneView { name, zone_data });
        }

        let task = self.fetch::<TaskStatistics>(TASKS_PATH).await?;
        output.task_manager.thread_model.tasks_running = task.taskmgr.tasks_running;
        output.task_manager.thread_model.worker_threads = task.taskmgr.worker_threads;

        Ok(output)
    }
}
