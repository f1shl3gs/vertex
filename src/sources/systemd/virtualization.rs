use event::{Metric, tags};

use super::dbus::{Client, Error};

pub async fn collect(client: &mut Client) -> Result<Metric, Error> {
    let typ = client
        .call::<String>(
            "/org/freedesktop/systemd1",
            "Get",
            "org.freedesktop.systemd1",
            "org.freedesktop.DBus.Properties",
            &["org.freedesktop.systemd1.Manager", "Virtualization"],
        )
        .await?;

    let typ = if typ.is_empty() {
        // if no virtualization type is returned, assume it's bare metal
        "none"
    } else {
        typ.trim_matches('\"')
    };

    Ok(Metric::gauge_with_tags(
        "systemd_virtualization_info",
        "Detected virtualization technology",
        1,
        tags!(
            "virtualization_type" => typ,
        ),
    ))
}
