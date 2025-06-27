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
    let mut metrics = Vec::with_capacity(2 * STAGES.len());

    for stage in STAGES {
        let timestamp_monotonic = client
            .call::<u64>(
                "/org/freedesktop/systemd1",
                "Get",
                "org.freedesktop.systemd1",
                "org.freedesktop.DBus.Properties",
                &[
                    "org.freedesktop.systemd1.Manager",
                    format!("{stage}TimestampMonotonic").as_str(),
                ],
            )
            .await?;
        let timestamp = client
            .call::<u64>(
                "/org/freedesktop/systemd1",
                "Get",
                "org.freedesktop.systemd1",
                "org.freedesktop.DBus.Properties",
                &[
                    "org.freedesktop.systemd1.Manager",
                    format!("{stage}Timestamp").as_str(),
                ],
            )
            .await?;

        metrics.extend([
            Metric::gauge_with_tags(
                "systemd_boot_monotonic_seconds",
                "Systemd boot stage monotonic timestamps",
                timestamp_monotonic as f64 / 1_000_000.0,
                tags!(
                    "stage" => stage,
                ),
            ),
            Metric::gauge_with_tags(
                "systemd_boot_time_seconds",
                "Systemd boot stage timestamps",
                timestamp / 1_000_000,
                tags!(
                    "stage" => stage,
                ),
            ),
        ]);
    }

    Ok(metrics)
}
