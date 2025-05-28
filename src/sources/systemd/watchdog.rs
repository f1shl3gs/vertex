use event::{Metric, tags};

use super::dbus::{Client, Error};

pub async fn collect(client: &mut Client) -> Result<Vec<Metric>, Error> {
    let mut metrics = Vec::new();

    let resp = client
        .call::<String>(
            "/org/freedesktop/systemd1",
            "Get",
            "org.freedesktop.systemd1",
            "org.freedesktop.DBus.Properties",
            &["org.freedesktop.systemd1.Manager", "WatchdogDevice"],
        )
        .await?;

    metrics.push(Metric::gauge(
        "systemd_watchdog_enabled",
        "systemd watchdog enabled",
        !resp.is_empty(),
    ));

    if resp.is_empty() {
        return Ok(metrics);
    }

    metrics.extend([
        Metric::gauge_with_tags(
            "systemd_watchdog_last_ping_monotonic_seconds",
            "systemd watchdog last ping monotonic seconds",
            1,
            tags!(
                "device" => "type",
            ),
        ),
        Metric::gauge_with_tags(
            "systemd_watchdog_last_ping_time_seconds",
            "systemd watchdog last ping time seconds",
            1,
            tags!(
                "device" => "type",
            ),
        ),
        Metric::gauge_with_tags(
            "systemd_watchdog_runtime_seconds",
            "systemd watchdog runtime seconds",
            1,
            tags!(
                "device" => "type",
            ),
        ),
    ]);

    Ok(metrics)
}
