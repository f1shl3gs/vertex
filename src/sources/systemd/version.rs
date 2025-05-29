use event::{Metric, tags};

use super::dbus::{Client, Error};

pub async fn collect(client: &mut Client) -> Result<(f64, Metric), Error> {
    let value = client
        .call::<String>(
            "/org/freedesktop/systemd1",
            "Get",
            "org.freedesktop.systemd1",
            "org.freedesktop.DBus.Properties",
            &["org.freedesktop.systemd1.Manager", "Version"],
        )
        .await?;

    let features = client
        .call::<String>(
            "/org/freedesktop/systemd1",
            "Get",
            "org.freedesktop.systemd1",
            "org.freedesktop.DBus.Properties",
            &["org.freedesktop.systemd1.Manager", "Features"],
        )
        .await?;

    let version = match parse_partial(&value) {
        Ok(value) => value,
        Err(_err) => {
            warn!(message = "parse systemd version failed", version = value);
            0.0
        }
    };

    Ok((
        version,
        Metric::gauge_with_tags(
            "systemd_version",
            "A metric with a constant '1' value labeled by version and features",
            1,
            tags!(
                "features" => features,
                "version" => value,
            ),
        ),
    ))
}

fn parse_partial(input: &str) -> Result<f64, std::num::ParseFloatError> {
    let mut pos = 0;
    let mut dot = false;
    let data = input.as_bytes();
    while pos < data.len() {
        let ch = data[pos];
        if ch.is_ascii_digit() {
            pos += 1;
            continue;
        }

        if ch == b'.' {
            if dot {
                break;
            }

            pos += 1;
            dot = true;

            continue;
        }

        break;
    }

    input[..pos].parse::<f64>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse() {
        assert_eq!(parse_partial("257.5-6.fc42"), Ok(257.5));
        assert_eq!(parse_partial("257"), Ok(257.0));
        assert_eq!(parse_partial("257.5"), Ok(257.5));
    }
}
