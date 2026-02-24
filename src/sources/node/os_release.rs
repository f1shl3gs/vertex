use std::collections::BTreeMap;
use std::path::PathBuf;

use chrono::NaiveDate;
use event::{Metric, tags};

use super::Error;

const ETC_OS_RELEASE: &str = "etc/os-release";
const USR_LIB_OS_RELEASE: &str = "usr/lib/os-release";

pub async fn gather(root_path: PathBuf) -> Result<Vec<Metric>, Error> {
    let content = std::fs::read_to_string(root_path.join(ETC_OS_RELEASE))
        .or_else(|_err| std::fs::read_to_string(root_path.join(USR_LIB_OS_RELEASE)))
        .map_err(|_err| Error::NoData)?;

    let infos = parse_os_release(&content);
    let mut metrics = vec![Metric::gauge_with_tags(
        "node_os_info",
        "A metric with a constant '1' value labeled by build_id, id, id_like, image_id, image_version, name, pretty_name, variant, variant_id, version, version_codename, version_id.",
        1,
        tags!(
            "name" => infos.get("NAME").copied().unwrap_or(""),
            "id" => infos.get("ID").copied().unwrap_or(""),
            "id_like" => infos.get("ID_LIKE").copied().unwrap_or(""),
            "pretty_name" => infos.get("PRETTY_NAME").copied().unwrap_or(""),
            "variant" => infos.get("VARIANT").copied().unwrap_or(""),
            "variant_id" => infos.get("VARIANT_ID").copied().unwrap_or(""),
            "version" => infos.get("VERSION").copied().unwrap_or(""),
            "version_id" => infos.get("VERSION_ID").copied().unwrap_or(""),
            "version_codename" => infos.get("VERSION_CODENAME").copied().unwrap_or(""),
            "build_id" => infos.get("BUILD_ID").copied().unwrap_or(""),
            "image_id" => infos.get("IMAGE_ID").copied().unwrap_or(""),
            "image_version" => infos.get("IMAGE_VERSION").copied().unwrap_or("")
        ),
    )];

    if let Some(version) = infos.get("VERSION_ID") {
        let version = version.parse::<f64>().unwrap_or_default();
        metrics.push(Metric::gauge_with_tags(
            "node_os_version",
            "Metric containing the major.minor part of the OS version.",
            version,
            tags!(
                "id" => infos.get("ID").copied().unwrap_or(""),
                "id_like" => infos.get("ID_LIKE").copied().unwrap_or(""),
                "name" => infos.get("NAME").copied().unwrap_or("")
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

fn parse_os_release(content: &str) -> BTreeMap<&str, &str> {
    let mut map = BTreeMap::new();
    for line in content.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };

        map.insert(key, value.trim_matches('"'));
    }

    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse() {
        let content = include_str!("../../../tests/node/usr/lib/os-release");
        let info = parse_os_release(content);

        assert_eq!(*info.get("NAME").unwrap(), "Ubuntu");
        assert_eq!(*info.get("ID").unwrap(), "ubuntu");
        assert_eq!(*info.get("ID_LIKE").unwrap(), "debian");
        assert_eq!(*info.get("PRETTY_NAME").unwrap(), "Ubuntu 20.04.2 LTS");
        assert_eq!(*info.get("VERSION").unwrap(), "20.04.2 LTS (Focal Fossa)");
        assert_eq!(*info.get("VERSION_ID").unwrap(), "20.04");
        assert_eq!(*info.get("VERSION_CODENAME").unwrap(), "focal");
    }
}
