use event::{Metric, tags};
use regex::Regex;

use super::dbus::{Client, Error, Variant, padding};

#[derive(Debug)]
struct Unit {
    name: String,
    active_state: String,
    path: String,
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
            // let _description = self.read_str();
            skip_string(input, &mut pos);
            let load_state = read_string(input, &mut pos);
            let active_state = read_string(input, &mut pos);
            // let _sub_state = self.read_str();
            skip_string(input, &mut pos);
            // let _followed = self.read_str();
            skip_string(input, &mut pos);
            let path = read_string(input, &mut pos);
            // let _job_id = self.read_u32_le();
            pos += 4;
            // let _job_type = self.read_str();
            skip_string(input, &mut pos);
            // let _job_path = self.read_str();
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
) -> Result<Vec<Metric>, Error> {
    let mut metrics = Vec::new();

    let units = client
        .call::<Vec<Unit>>(
            "/org/freedesktop/systemd1",
            "ListUnits",
            "org.freedesktop.systemd1",
            "org.freedesktop.systemd1.Manager",
            &[],
        )
        .await?;

    for unit in units {
        if !include.is_match(unit.name.as_str()) || exclude.is_match(unit.name.as_str()) {
            continue;
        }

        let partial = collect_unit(client, unit).await?;
        metrics.extend(partial);
    }

    Ok(metrics)
}

const STATE_NAMES: [&str; 5] = ["active", "activating", "deactivating", "inactive", "failed"];

async fn collect_unit(client: &mut Client, unit: Unit) -> Result<Vec<Metric>, Error> {
    let mut metrics = Vec::new();

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

    let value = client
        .call::<u64>(
            unit.path.as_str(),
            "Get",
            "org.freedesktop.systemd1",
            "org.freedesktop.DBus.Properties",
            &["org.freedesktop.systemd1.Unit", "ActiveEnterTimestamp"],
        )
        .await?;
    metrics.push(Metric::gauge_with_tags(
        "systemd_unit_active_enter_time_seconds",
        "Last time the unit transitioned into the active state",
        value as f64 / 1_000_000.0,
        tags!(
            "name" => unit.name.as_str(),
            "type" => typ,
        ),
    ));

    let value = client
        .call::<u64>(
            unit.path.as_str(),
            "Get",
            "org.freedesktop.systemd1",
            "org.freedesktop.DBus.Properties",
            &["org.freedesktop.systemd1.Unit", "ActiveExitTimestamp"],
        )
        .await?;
    metrics.push(Metric::gauge_with_tags(
        "systemd_unit_active_exit_time_seconds",
        "Last time the unit transitioned out of the active state",
        value as f64 / 1_000_000.0,
        tags!(
            "name" => unit.name.as_str(),
            "type" => typ,
        ),
    ));

    let value = client
        .call::<u64>(
            unit.path.as_str(),
            "Get",
            "org.freedesktop.systemd1",
            "org.freedesktop.DBus.Properties",
            &["org.freedesktop.systemd1.Unit", "InactiveEnterTimestamp"],
        )
        .await?;
    metrics.push(Metric::gauge_with_tags(
        "systemd_unit_inactive_enter_time_seconds",
        "Last time the unit transitioned into the inactive state",
        value as f64 / 1_000_000.0,
        tags!(
            "name" => unit.name.as_str(),
            "type" => typ,
        ),
    ));

    let value = client
        .call::<u64>(
            unit.path.as_str(),
            "Get",
            "org.freedesktop.systemd1",
            "org.freedesktop.DBus.Properties",
            &["org.freedesktop.systemd1.Unit", "InactiveExitTimestamp"],
        )
        .await?;
    metrics.push(Metric::gauge_with_tags(
        "systemd_unit_inactive_exit_time_seconds",
        "Last time the unit transitioned out of the inactive state",
        value as f64 / 1_000_000.0,
        tags!(
            "name" => unit.name.as_str(),
            "type" => typ,
        ),
    ));

    match typ {
        "service" => {
            // service info
            let service_type = client
                .call::<String>(
                    unit.path.as_str(),
                    "Get",
                    "org.freedesktop.systemd1",
                    "org.freedesktop.DBus.Properties",
                    &["org.freedesktop.systemd1.Service", "Type"],
                )
                .await?;

            metrics.push(Metric::gauge_with_tags(
                "systemd_unit_info",
                "Mostly-static metadata for all unit types",
                1,
                tags!(
                    "name" => unit.name.as_str(),
                    "type" => typ,
                    "mount_type" => "",
                    "service_type" => service_type
                ),
            ));

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

            metrics.push(Metric::gauge_with_tags(
                "systemd_unit_start_time_seconds",
                "Start time of the unit since unix epoch in seconds.",
                start as f64 / 1_000_000.0,
                tags!(
                    "name" => unit.name.as_str(),
                    "type" => typ,
                ),
            ));

            match collect_service_tasks(client, &unit, typ).await {
                Ok(partial) => metrics.extend(partial),
                Err(err) => {
                    warn!(
                        message = "Failed to collect service tasks metrics",
                        %err
                    );

                    return Err(err);
                }
            }
        }
        "mount" => match collect_mount_info(client, &unit, typ).await {
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
        "socket" => match collect_socket_conn(client, &unit).await {
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

async fn collect_mount_info(
    client: &mut Client,
    unit: &Unit,
    typ: &str,
) -> Result<Vec<Metric>, Error> {
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
            "service_type" => typ
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

async fn collect_socket_conn(client: &mut Client, unit: &Unit) -> Result<Vec<Metric>, Error> {
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

/*
#[derive(Debug)]
struct UnitItem {
    name: String,
    description: String,
    load_state: String,
    active_state: String,
    sub_state: String,
    followed: String,
    path: String,
    job_id: u32,
    job_type: String,
    job_path: String,
}

struct Units {
    data: Vec<u8>,
    pos: usize,
}

impl Units {
    fn new(data: Vec<u8>) -> Self {
        Self { data, pos: 8 }
    }

    fn read_u32_le(&mut self) -> u32 {
        let value = u32::from_le_bytes((&self.data[self.pos..self.pos + 4]).try_into().unwrap());

        self.pos += 4;

        value
    }

    fn read_str(&mut self) -> String {
        let len = self.read_u32_le() as usize;

        let s = String::from_utf8_lossy(&self.data[self.pos..self.pos + len]).to_string();

        self.pos += len + 1 + padding(len + 1, 4);

        s
    }
}

impl Iterator for Units {
    type Item = UnitItem;

    fn next(&'_ mut self) -> Option<Self::Item> {
        if self.pos >= self.data.len() {
            return None;
        }

        let name = self.read_str();
        let description = self.read_str();
        let load_state = self.read_str();
        let active_state = self.read_str();
        let sub_state = self.read_str();
        let followed = self.read_str();
        let path = self.read_str();
        let job_id = self.read_u32_le();
        let job_type = self.read_str();
        let job_path = self.read_str();

        self.pos += padding(self.pos, 8);

        Some(UnitItem {
            name,
            description,
            load_state,
            active_state,
            sub_state,
            followed,
            path,
            job_id,
            job_type,
            job_path,
        })
    }
}
*/
