use std::collections::BTreeMap;

use chrono::NaiveDate;
use event::{tags, Metric};

use super::Error;

const ETC_OS_RELEASE: &str = "/etc/os-release";
const USR_LIB_OS_RELEASE: &str = "/usr/lib/os-release";

pub async fn gather() -> Result<Vec<Metric>, Error> {
    let infos = release_infos()?;

    let mut metrics = vec![
        Metric::gauge_with_tags(
            "node_os_info",
            "A metric with a constant '1' value labeled by build_id, id, id_like, image_id, image_version, name, pretty_name, variant, variant_id, version, version_codename, version_id.",
            1,
            tags!(
                "name" => infos.get("NAME").cloned().unwrap_or_default(),
                "id" => infos.get("ID").cloned().unwrap_or_default(),
                "id_like" => infos.get("ID_LIKE").cloned().unwrap_or_default(),
                "pretty_name" => infos.get("PRETTY_NAME").cloned().unwrap_or_default(),
                "variant" => infos.get("VARIANT").cloned().unwrap_or_default(),
                "variant_id" => infos.get("VARIANT_ID").cloned().unwrap_or_default(),
                "version" => infos.get("VERSION").cloned().unwrap_or_default(),
                "version_id" => infos.get("VERSION_ID").cloned().unwrap_or_default(),
                "version_codename" => infos.get("VERSION_CODENAME").cloned().unwrap_or_default(),
                "build_id" => infos.get("BUILD_ID").cloned().unwrap_or_default(),
                "image_id" => infos.get("IMAGE_ID").cloned().unwrap_or_default(),
                "image_version" => infos.get("IMAGE_VERSION").cloned().unwrap_or_default()
            ),
        ),
    ];

    if let Some(version) = infos.get("VERSION") {
        let version: f64 = version.parse().unwrap_or_default();
        metrics.push(Metric::gauge_with_tags(
            "node_os_version",
            "Metric containing the major.minor part of the OS version.",
            version,
            tags!(
                "id" => infos.get("ID").cloned().unwrap_or_default(),
                "id_link" => infos.get("ID_LIKE").cloned().unwrap_or_default(),
                "name" => infos.get("NAME").cloned().unwrap_or_default()
            ),
        ));
    }

    if let Some(support_end) = infos.get("SUPPORT_END") {
        let date = NaiveDate::parse_from_str(support_end, "%Y-%m-%d").unwrap();
        let timestamp = date.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp();

        metrics.push(Metric::gauge(
            "node_os_support_end_timestamp_seconds",
            "Metric containing the end-of-life date timestamp of the OS",
            timestamp,
        ))
    }

    Ok(metrics)
}

fn release_infos() -> Result<BTreeMap<String, String>, Error> {
    for path in [ETC_OS_RELEASE, USR_LIB_OS_RELEASE] {
        match parse_os_release(path) {
            Ok(infos) => return Ok(infos),
            Err(_err) => continue,
        }
    }

    Err(Error::from("No invalid os release file"))
}

fn parse_os_release(path: &str) -> Result<BTreeMap<String, String>, Error> {
    let data = std::fs::read_to_string(path)?;

    let mut envs = BTreeMap::new();
    for line in data.lines() {
        if let Some((key, value)) = line.split_once('=') {
            let value = value.trim_matches('"').to_string();
            envs.insert(key.to_string(), value);
        }
    }

    Ok(envs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_parse_os_release() {
        let path = format!("tests/node/{}", USR_LIB_OS_RELEASE);
        let m = parse_os_release(&path).unwrap();

        assert_eq!(m.get("NAME").unwrap(), "Ubuntu");
        assert_eq!(m.get("ID").unwrap(), "ubuntu");
        assert_eq!(m.get("ID_LIKE").unwrap(), "debian");
        assert_eq!(m.get("PRETTY_NAME").unwrap(), "Ubuntu 20.04.2 LTS");
        assert_eq!(m.get("VERSION").unwrap(), "20.04.2 LTS (Focal Fossa)");
        assert_eq!(m.get("VERSION_ID").unwrap(), "20.04");
        assert_eq!(m.get("VERSION_CODENAME").unwrap(), "focal");
    }

    #[test]
    fn parse() {
        let support_end = "2025-12-15";
        let date = NaiveDate::parse_from_str(support_end, "%Y-%m-%d").unwrap();
        let timestamp = date.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp();
        println!("{:?}", timestamp);
    }
}
