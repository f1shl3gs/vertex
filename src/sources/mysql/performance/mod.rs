mod events_statements;
mod events_statements_sum;
mod events_waits;
mod file_events;
mod file_instances;
mod index_io_waits;
mod memory_events;
mod replication_applier_status_by_worker;
mod replication_group_member_stats;
mod replication_group_members;
mod table_io_waits;
mod table_lock_waits;

use configurable::Configurable;
use event::Metric;
use serde::{Deserialize, Serialize};

use super::connection::{Connection, Error, Flavor};

#[derive(Configurable, Clone, Default, Serialize, Deserialize, Debug)]
pub struct Config {
    /// Collect metrics from performance_schema.events_waits_summary_global_by_event_name
    #[serde(default)]
    events_waits: bool,

    #[serde(default)]
    events_statements: Option<events_statements::Config>,

    /// Since 5.7, Collect metrics of grand sums from performance_schema.events_statements_summary_by_digest
    #[serde(default)]
    events_statements_sum: bool,

    /// Since 5.6, Collect metrics from performance_schema.file_summary_by_event_name
    #[serde(default)]
    file_events: bool,

    /// Collect metrics from performance_schema.file_summary_by_instance
    #[serde(default)]
    file_instances: Option<file_instances::Config>,

    /// Collect metrics from performance_schema.memory_summary_global_by_event_name
    #[serde(default)]
    memory_events: Option<memory_events::Config>,

    /// Collect metrics from performance_schema.replication_group_members
    #[serde(default)]
    replication_group_members: bool,

    /// Collect metrics from performance_schema.replication_group_member_stats
    #[serde(default)]
    replication_group_member_stats: bool,

    /// Collect metrics from performance_schema.replication_applier_status_by_worker
    #[serde(default)]
    replication_applier_status_by_worker: bool,

    /// Collect metrics from performance_schema.table_io_waits_summary_by_index_usage
    #[serde(default)]
    index_io_waits: bool,

    /// Since 5.6, Collect metrics from performance_schema.table_io_waits_summary_by_table
    #[serde(default)]
    table_io_waits: bool,

    /// Collect metrics from performance_schema.table_io_waits_summary_by_table
    #[serde(default)]
    table_lock_waits: bool,
}

pub async fn collect(conn: &mut Connection, conf: &Config) -> Result<Vec<Metric>, Error> {
    let version = conn.version();

    let mut metrics = Vec::new();

    if let Some(conf) = &conf.events_statements
        && version >= 5.6
    {
        metrics.extend(events_statements::collect(conn, conf).await?);
    }

    if conf.events_statements_sum && version >= 5.7 {
        metrics.extend(events_statements_sum::collect(conn).await?);
    }

    if conf.events_waits && version >= 5.5 {
        metrics.extend(events_waits::collect(conn).await?);
    }

    if conf.file_events && version >= 5.6 {
        metrics.extend(file_events::collect(conn).await?);
    }

    if let Some(conf) = &conf.file_instances
        && version >= 5.5
    {
        metrics.extend(file_instances::collect(conn, conf).await?);
    }

    if conf.index_io_waits && version >= 5.6 {
        metrics.extend(index_io_waits::collect(conn).await?);
    }

    if let Some(conf) = &conf.memory_events
        && version >= 5.7
    {
        metrics.extend(memory_events::collect(conn, conf).await?);
    }

    if conf.replication_applier_status_by_worker && version >= 8.0 {
        metrics.extend(replication_applier_status_by_worker::collect(conn).await?);
    }

    if conf.replication_group_member_stats && version >= 5.7 {
        metrics.extend(replication_group_member_stats::collect(conn).await?);
    }

    if conf.replication_group_members && version >= 5.7 {
        metrics.extend(replication_group_members::collect(conn).await?);
    }

    if conf.table_io_waits && version >= 5.6 {
        metrics.extend(table_io_waits::collect(conn).await?);
    }

    if conf.table_lock_waits && version >= 5.6 {
        metrics.extend(table_lock_waits::collect(conn).await?);
    }

    Ok(metrics)
}
