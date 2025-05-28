use event::{Metric, tags};

use super::dbus::{Client, Error};

const STAGES: [&str; 18] = [
    "Finish",
    "Firmware",
    "Loader",
    "Kernel",
    "InitRD",
    "InitRDGeneratorsStart",
    "InitRDGeneratorsFinish",
    "InitRDSecurityStart",
    "InitRDSecurityFinish",
    "InitRDUnitsLoadStart",
    "InitRDUnitsLoadFinish",
    "GeneratorsStart",
    "GeneratorsFinish",
    "SecurityStart",
    "SecurityFinish",
    "Userspace",
    "UnitsLoadStart",
    "UnitsLoadFinish",
];

pub async fn collect(client: &mut Client) -> Result<Vec<Metric>, Error> {
    let metrics = boot_stage_timestamps(client).await?;

    Ok(metrics)
}

async fn boot_stage_timestamps(client: &mut Client) -> Result<Vec<Metric>, Error> {
    let mut metrics = Vec::new();

    for stage in STAGES {
        let value = client
            .call::<u64>(
                "/org/freedesktop/systemd1",
                "Get",
                "org.freedesktop.systemd1",
                "org.freedesktop.DBus.Properties",
                &[
                    "org.freedesktop.systemd1.Manager",
                    format!("{}TimestampMonotonic", stage).as_str(),
                ],
            )
            .await?;

        metrics.push(Metric::gauge_with_tags(
            "systemd_boot_monotonic_seconds",
            "Systemd boot stage monotonic timestamps",
            value as f64 / 1_000_000.0,
            tags!(
                "stage" => stage,
            ),
        ));

        match client
            .call::<u64>(
                "/org/freedesktop/systemd1",
                "Get",
                "org.freedesktop.systemd1",
                "org.freedesktop.DBus.Properties",
                &[
                    "org.freedesktop.systemd1.Manager",
                    format!("{}Timestamp", stage).as_str(),
                ],
            )
            .await
        {
            Ok(value) => metrics.push(Metric::gauge_with_tags(
                "systemd_boot_time_seconds",
                "Systemd boot stage timestamps",
                value / 1000000,
                tags!(
                    "stage" => stage,
                ),
            )),
            Err(err) => {
                warn!(
                    message = "Failed to get systemd boot stage timestamps",
                    ?err
                );
            }
        };
    }

    Ok(metrics)
}
