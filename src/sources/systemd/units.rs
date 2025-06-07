use event::{Metric, tags};
use regex::Regex;

use super::dbus::{Client, Error, Variant, padding};

#[derive(Debug)]
struct Unit {
    /// The primary unit name as string
    name: String,
    // The human readable description string
    // description: String,
    // The load state (i.e. whether the unit file has been loaded successfully)
    // load_state: string,
    /// The active state (i.e. whether the unit is currently started or not)
    active_state: String,
    // The sub state (a more fine-grained version of the active state that is specific to the unit type, which the active state is not)
    // sub_state: String,
    // A unit that is being followed in its state by this unit, if there is any, otherwise the empty string.
    // followed: String,
    /// The unit object path
    path: String,
    // job_id: u32,         // If there is a job queued for the job unit the numeric job id, 0 otherwise
    // job_type: String     // The job type as string
    // job_path: String     // The job object path
}

impl Variant for Vec<Unit> {
    fn decode(input: &[u8]) -> Result<Self, Error> {
        fn read_string(input: &[u8], pos: &mut usize) -> String {
            let len = u32::from_le_bytes((&input[*pos..*pos + 4]).try_into().unwrap()) as usize;
            *pos += 4;

            let s = String::from_utf8_lossy(&input[*pos..*pos + len]).to_string();
            *pos += len + 1 + padding(len + 1, 4);

            s
        }

        fn skip_string(input: &[u8], pos: &mut usize) {
            let len = u32::from_le_bytes((&input[*pos..*pos + 4]).try_into().unwrap()) as usize;

            *pos += 4;
            *pos += len + 1 + padding(len + 1, 4);
        }

        if input.len() < 9 {
            return Err(Error::BodyTooSmall);
        }

        let mut units = Vec::new();
        let mut pos = 8;

        while pos < input.len() {
            let name = read_string(input, &mut pos);
            // skip `description`
            skip_string(input, &mut pos);
            let load_state = read_string(input, &mut pos);
            let active_state = read_string(input, &mut pos);
            // skip sub_state
            skip_string(input, &mut pos);
            // skip followed
            skip_string(input, &mut pos);
            let path = read_string(input, &mut pos);
            // skip job_id
            pos += 4;
            // skip `job_type`
            skip_string(input, &mut pos);
            // skip `job_path`
            skip_string(input, &mut pos);

            pos += padding(pos, 8);

            if load_state != "loaded" {
                continue;
            }

            units.push(Unit {
                name,
                active_state,
                path,
            })
        }

        Ok(units)
    }
}

pub async fn collect(
    client: &mut Client,
    include: &Regex,
    exclude: &Regex,
    version: f64,
) -> Result<Vec<Metric>, Error> {
    let units = client
        .call::<Vec<Unit>>(
            "/org/freedesktop/systemd1",
            "ListUnits",
            "org.freedesktop.systemd1",
            "org.freedesktop.systemd1.Manager",
            &[],
        )
        .await?;

    // 20 is just a guess, and it should be fine
    let mut metrics = Vec::with_capacity(units.len() * 20);
    for unit in units {
        if !include.is_match(unit.name.as_str()) || exclude.is_match(unit.name.as_str()) {
            continue;
        }

        let partial = collect_unit(client, unit, version).await?;
        metrics.extend(partial);
    }

    Ok(metrics)
}

const STATE_NAMES: [&str; 5] = ["active", "activating", "deactivating", "inactive", "failed"];

async fn collect_unit(client: &mut Client, unit: Unit, version: f64) -> Result<Vec<Metric>, Error> {
    let mut metrics = Vec::with_capacity(9);

    let typ = if let Some(pos) = unit.name.rfind('.') {
        &unit.name[pos + 1..]
    } else {
        "Unknown"
    };

    for state in STATE_NAMES {
        metrics.push(Metric::gauge_with_tags(
            "systemd_unit_state",
            "Systemd unit",
            state == unit.active_state,
            tags!(
                "name" => unit.name.as_str(),
                "type" => typ,
                "state" => state,
            ),
        ))
    }

    let active_enter_timestamp = client
        .call::<u64>(
            unit.path.as_str(),
            "Get",
            "org.freedesktop.systemd1",
            "org.freedesktop.DBus.Properties",
            &["org.freedesktop.systemd1.Unit", "ActiveEnterTimestamp"],
        )
        .await?;
    let active_exit_timestamp = client
        .call::<u64>(
            unit.path.as_str(),
            "Get",
            "org.freedesktop.systemd1",
            "org.freedesktop.DBus.Properties",
            &["org.freedesktop.systemd1.Unit", "ActiveExitTimestamp"],
        )
        .await?;
    let inactive_enter_timestamp = client
        .call::<u64>(
            unit.path.as_str(),
            "Get",
            "org.freedesktop.systemd1",
            "org.freedesktop.DBus.Properties",
            &["org.freedesktop.systemd1.Unit", "InactiveEnterTimestamp"],
        )
        .await?;
    let inactive_exit_timestamp = client
        .call::<u64>(
            unit.path.as_str(),
            "Get",
            "org.freedesktop.systemd1",
            "org.freedesktop.DBus.Properties",
            &["org.freedesktop.systemd1.Unit", "InactiveExitTimestamp"],
        )
        .await?;

    metrics.extend([
        Metric::gauge_with_tags(
            "systemd_unit_active_enter_time_seconds",
            "Last time the unit transitioned into the active state",
            active_enter_timestamp as f64 / 1_000_000.0,
            tags!(
                "name" => unit.name.as_str(),
                "type" => typ,
            ),
        ),
        Metric::gauge_with_tags(
            "systemd_unit_active_exit_time_seconds",
            "Last time the unit transitioned out of the active state",
            active_exit_timestamp as f64 / 1_000_000.0,
            tags!(
                "name" => unit.name.as_str(),
                "type" => typ,
            ),
        ),
        Metric::gauge_with_tags(
            "systemd_unit_inactive_enter_time_seconds",
            "Last time the unit transitioned into the inactive state",
            inactive_enter_timestamp as f64 / 1_000_000.0,
            tags!(
                "name" => unit.name.as_str(),
                "type" => typ,
            ),
        ),
        Metric::gauge_with_tags(
            "systemd_unit_inactive_exit_time_seconds",
            "Last time the unit transitioned out of the inactive state",
            inactive_exit_timestamp as f64 / 1_000_000.0,
            tags!(
                "name" => unit.name.as_str(),
                "type" => typ,
            ),
        ),
    ]);

    match typ {
        "service" => {
            match collect_service_info(client, &unit, typ).await {
                Ok(partial) => metrics.extend(partial),
                Err(err) => {
                    warn!(
                        message = "failed to collect service info",
                        %err
                    );
                }
            }

            if version >= 235.0 {
                match collect_service_restart(client, &unit).await {
                    Ok(partial) => metrics.extend(partial),
                    Err(err) => {
                        warn!(
                            message = "failed to collect service restart",
                            %err
                        )
                    }
                }

                match collect_ip_accounting(client, &unit).await {
                    Ok(partial) => metrics.extend(partial),
                    Err(err) => {
                        warn!(
                            message = "failed to collect ip accounting",
                            %err
                        )
                    }
                }
            }
        }
        "mount" => match collect_mount_info(client, &unit).await {
            Ok(partial) => metrics.extend(partial),
            Err(err) => {
                warn!(
                    message = "failed to collect mount info",
                    %err
                );
            }
        },
        "timer" => match collect_timer(client, &unit).await {
            Ok(partial) => metrics.extend(partial),
            Err(err) => {
                warn!(
                    message = "failed to collect timer metrics",
                    %err
                );
            }
        },
        "socket" => match collect_socket(client, &unit).await {
            Ok(partial) => metrics.extend(partial),
            Err(err) => {
                warn!(
                    message = "failed to collect socket metrics",
                    %err
                );
            }
        },
        _ => {}
    }

    Ok(metrics)
}

async fn collect_service_info(
    client: &mut Client,
    unit: &Unit,
    typ: &str,
) -> Result<Vec<Metric>, Error> {
    let service_type = client
        .call::<String>(
            unit.path.as_str(),
            "Get",
            "org.freedesktop.systemd1",
            "org.freedesktop.DBus.Properties",
            &["org.freedesktop.systemd1.Service", "Type"],
        )
        .await?;

    let start = if unit.active_state == "active" {
        client
            .call::<u64>(
                unit.path.as_str(),
                "Get",
                "org.freedesktop.systemd1",
                "org.freedesktop.DBus.Properties",
                &["org.freedesktop.systemd1.Unit", "ActiveEnterTimestamp"],
            )
            .await?
    } else {
        0
    };

    let mut metrics = vec![
        Metric::gauge_with_tags(
            "systemd_unit_info",
            "Mostly-static metadata for all unit types",
            1,
            tags!(
                "name" => unit.name.as_str(),
                "type" => typ,
                "service_type" => service_type
            ),
        ),
        Metric::gauge_with_tags(
            "systemd_unit_start_time_seconds",
            "Start time of the unit since unix epoch in seconds.",
            start as f64 / 1_000_000.0,
            tags!(
                "name" => unit.name.as_str(),
                "type" => typ,
            ),
        ),
    ];

    match collect_service_tasks(client, unit, typ).await {
        Ok(partial) => metrics.extend(partial),
        Err(err) => {
            warn!(
                message = "Failed to collect service tasks metrics",
                %err
            );

            return Err(err);
        }
    }

    Ok(metrics)
}

async fn collect_service_tasks(
    client: &mut Client,
    unit: &Unit,
    typ: &str,
) -> Result<Vec<Metric>, Error> {
    let value = client
        .call::<u64>(
            unit.path.as_str(),
            "Get",
            "org.freedesktop.systemd1",
            "org.freedesktop.DBus.Properties",
            &["org.freedesktop.systemd1.Service", "TasksCurrent"],
        )
        .await?;

    let mut metrics = Vec::with_capacity(2);
    // don't set tasks_current if dbus reports max uint64
    if value != u64::MAX {
        metrics.push(Metric::gauge_with_tags(
            "systemd_unit_tasks_current",
            "Current number of tasks per Systemd unit",
            value,
            tags!(
                "name" => unit.name.as_str(),
            ),
        ));
    }

    let value = client
        .call::<u64>(
            unit.path.as_str(),
            "Get",
            "org.freedesktop.systemd1",
            "org.freedesktop.DBus.Properties",
            &["org.freedesktop.systemd1.Service", "TasksMax"],
        )
        .await?;
    if value != u64::MAX {
        metrics.push(Metric::gauge_with_tags(
            "systemd_unit_tasks_max",
            "Maximum number of tasks per Systemd unit",
            value,
            tags!(
                "name" => unit.name.as_str(),
                "type" => typ
            ),
        ));
    }

    Ok(metrics)
}

async fn collect_mount_info(client: &mut Client, unit: &Unit) -> Result<Vec<Metric>, Error> {
    let value = client
        .call::<String>(
            unit.path.as_str(),
            "Get",
            "org.freedesktop.systemd1",
            "org.freedesktop.DBus.Properties",
            &["org.freedesktop.systemd1.Mount", "Type"],
        )
        .await?;

    Ok(vec![Metric::gauge_with_tags(
        "systemd_mount_info",
        "Mostly-static metadata for all unit types",
        1,
        tags!(
            "name" => unit.name.as_str(),
            "type" => value,
        ),
    )])
}

async fn collect_timer(client: &mut Client, unit: &Unit) -> Result<Vec<Metric>, Error> {
    let value = client
        .call::<u64>(
            unit.path.as_str(),
            "Get",
            "org.freedesktop.systemd1",
            "org.freedesktop.DBus.Properties",
            &["org.freedesktop.systemd1.Timer", "LastTriggerUSec"],
        )
        .await?;

    Ok(vec![Metric::gauge_with_tags(
        "systemd_timer_last_trigger_seconds",
        "Seconds since epoch of last trigger.",
        value as f64 / 1_000_000.0,
        tags!(
            "name" => unit.name.as_str(),
        ),
    )])
}

async fn collect_socket(client: &mut Client, unit: &Unit) -> Result<Vec<Metric>, Error> {
    let accepted = client
        .call::<u32>(
            unit.path.as_str(),
            "Get",
            "org.freedesktop.systemd1",
            "org.freedesktop.DBus.Properties",
            &["org.freedesktop.systemd1.Socket", "NAccepted"],
        )
        .await?;

    let current_connection = client
        .call::<u32>(
            unit.path.as_str(),
            "Get",
            "org.freedesktop.systemd1",
            "org.freedesktop.DBus.Properties",
            &["org.freedesktop.systemd1.Socket", "NConnections"],
        )
        .await?;

    let refused = client
        .call::<u32>(
            unit.path.as_str(),
            "Get",
            "org.freedesktop.systemd1",
            "org.freedesktop.DBus.Properties",
            &["org.freedesktop.systemd1.Socket", "NRefused"],
        )
        .await?;

    Ok(vec![
        Metric::sum_with_tags(
            "systemd_socket_accepted_connections_total",
            "Total number of accepted socket connections",
            accepted,
            tags!(
                "name" => unit.name.as_str(),
            ),
        ),
        Metric::gauge_with_tags(
            "systemd_socket_current_connections",
            "Current number of socket connections",
            current_connection,
            tags!(
                "name" => unit.name.as_str(),
            ),
        ),
        Metric::gauge_with_tags(
            "systemd_socket_refused_connections_total",
            "Total number of refused socket connections",
            refused,
            tags!(
                "name" => unit.name.as_str(),
            ),
        ),
    ])
}

async fn collect_service_restart(client: &mut Client, unit: &Unit) -> Result<Vec<Metric>, Error> {
    let restarts = client
        .call::<u32>(
            unit.path.as_str(),
            "Get",
            "org.freedesktop.systemd1",
            "org.freedesktop.DBus.Properties",
            &["org.freedesktop.systemd1.Service", "NRestarts"],
        )
        .await?;

    Ok(vec![Metric::sum_with_tags(
        "systemd_service_restart_total",
        "Service unit count of Restart triggers",
        restarts,
        tags!(
            "name" => unit.name.as_str(),
        ),
    )])
}

async fn collect_ip_accounting(client: &mut Client, unit: &Unit) -> Result<Vec<Metric>, Error> {
    let mut metrics = Vec::with_capacity(4);

    for (property, name, description) in [
        (
            "IPIngressBytes",
            "systemd_service_ip_ingress_bytes",
            "Service unit ingress IP accounting in bytes.",
        ),
        (
            "IPEgressBytes",
            "systemd_service_ip_egress_bytes",
            "Service unit egress IP accounting in bytes.",
        ),
        (
            "IPIngressPackets",
            "systemd_service_ip_ingress_packets_total",
            "Service unit ingress IP accounting in packets.",
        ),
        (
            "IPEgressPackets",
            "systemd_service_ip_egress_packets_total",
            "Service unit egress IP accounting in packets.",
        ),
    ] {
        let value = client
            .call::<u64>(
                unit.path.as_str(),
                "Get",
                "org.freedesktop.systemd1",
                "org.freedesktop.DBus.Properties",
                &["org.freedesktop.systemd1.Service", property],
            )
            .await?;

        metrics.push(Metric::sum_with_tags(
            name,
            description,
            value,
            tags!(
                "name" => unit.name.as_str(),
            ),
        ))
    }

    Ok(metrics)
}
