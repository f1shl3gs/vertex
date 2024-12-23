use std::collections::BTreeMap;

use event::{tags, Metric};

use super::{read_to_string, Error};

const ETC_OS_RELEASE: &str = "/etc/os-release";
const USR_LIB_OS_RELEASE: &str = "/usr/lib/os-release";

pub async fn gather() -> Result<Vec<Metric>, Error> {
    let mut infos = release_infos()?;

    let mut metrics = vec![
        Metric::gauge_with_tags(
            "node_os_info",
            "A metric with a constant '1' value labeled by build_id, id, id_like, image_id, image_version, name, pretty_name, variant, variant_id, version, version_codename, version_id.",
            1,
            tags!(
                "name" => infos.remove("NAME").unwrap_or_default(),
                "id" => infos.remove("ID").unwrap_or_default(),
                "id_like" => infos.remove("ID_LIKE").unwrap_or_default(),
                "pretty_name" => infos.remove("PRETTY_NAME").unwrap_or_default(),
                "variant" => infos.remove("VARIANT").unwrap_or_default(),
                "variant_id" => infos.remove("VARIANT_ID").unwrap_or_default(),
                "version" => infos.remove("VERSION").unwrap_or_default(),
                "version_id" => infos.remove("VERSION_ID").unwrap_or_default(),
                "version_codename" => infos.remove("VERSION_CODENAME").unwrap_or_default(),
                "build_id" => infos.remove("BUILD_ID").unwrap_or_default(),
                "image_id" => infos.remove("IMAGE_ID").unwrap_or_default(),
                "image_version" => infos.remove("IMAGE_VERSION").unwrap_or_default()
            ),
        ),
    ];

    if let Some(version) = infos.remove("VERSION") {
        let version: f64 = version.parse().unwrap_or_default();
        metrics.push(Metric::gauge_with_tags(
            "node_os_version",
            "Metric containing the major.minor part of the OS version.",
            version,
            tags!(
                "id" => infos.remove("ID").unwrap_or_default(),
                "id_link" => infos.remove("ID_LIKE").unwrap_or_default(),
                "name" => infos.remove("NAME").unwrap_or_default()
            ),
        ));
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
    let content = read_to_string(path)?;
    let mut envs = BTreeMap::new();

    for line in content.lines() {
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
}
