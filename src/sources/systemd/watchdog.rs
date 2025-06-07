use event::{Metric, tags};

use super::dbus::{Client, Error};

pub async fn collect(client: &mut Client) -> Result<Vec<Metric>, Error> {
    let device = client
        .call::<String>(
            "/org/freedesktop/systemd1",
            "Get",
            "org.freedesktop.systemd1",
            "org.freedesktop.DBus.Properties",
            &["org.freedesktop.systemd1.Manager", "WatchdogDevice"],
        )
        .await?;

    if device.is_empty() {
        return Ok(vec![Metric::gauge(
            "systemd_watchdog_enabled",
            "systemd watchdog enabled",
            0,
        )]);
    }

    let last_ping_timestamp_monotonic = client
        .call::<u64>(
            "/org/freedesktop/systemd1",
            "Get",
            "org.freedesktop.systemd1",
            "org.freedesktop.DBus.Properties",
            &[
                "org.freedesktop.systemd1.Manager",
                "WatchdogLastPingTimestampMonotonic",
            ],
        )
        .await?;
    let last_ping_timestamp = client
        .call::<u64>(
            "/org/freedesktop/systemd1",
            "Get",
            "org.freedesktop.systemd1",
            "org.freedesktop.DBus.Properties",
            &[
                "org.freedesktop.systemd1.Manager",
                "WatchdogLastPingTimestamp",
            ],
        )
        .await?;
    let runtime_usec = client
        .call::<u64>(
            "/org/freedesktop/systemd1",
            "Get",
            "org.freedesktop.systemd1",
            "org.freedesktop.DBus.Properties",
            &["org.freedesktop.systemd1.Manager", "RuntimeWatchdogUSec"],
        )
        .await?;

    Ok(vec![
        Metric::gauge("systemd_watchdog_enabled", "systemd watchdog enabled", 1),
        Metric::gauge_with_tags(
            "systemd_watchdog_last_ping_monotonic_seconds",
            "systemd watchdog last ping monotonic seconds",
            last_ping_timestamp_monotonic / 1_000_000,
            tags!(
                "device" => &device,
            ),
        ),
        Metric::gauge_with_tags(
            "systemd_watchdog_last_ping_time_seconds",
            "systemd watchdog last ping time seconds",
            last_ping_timestamp / 1_000_000,
            tags!(
                "device" => &device,
            ),
        ),
        Metric::gauge_with_tags(
            "systemd_watchdog_runtime_seconds",
            "systemd watchdog runtime seconds",
            runtime_usec / 1_000_000,
            tags!(
                "device" => device,
            ),
        ),
    ])
}
