use std::collections::BTreeMap;

use event::{tags, Metric};
use nom::branch::alt;
use nom::bytes::complete::{escaped, tag, take_while};
use nom::character::complete::none_of;
use nom::sequence::{delimited, tuple};
use nom::IResult;

use super::{read_to_string, Error};

const ETC_OS_RELEASE: &str = "/etc/os-release";
const USR_LIB_OS_RELEASE: &str = "/usr/lib/os-release";

pub async fn gather() -> Result<Vec<Metric>, Error> {
    let infos = release_infos().await?;
    let dv = &"".to_string();

    let mut metrics = vec![
        Metric::gauge_with_tags(
            "node_os_info",
            "A metric with a constant '1' value labeled by build_id, id, id_like, image_id, image_version, name, pretty_name, variant, variant_id, version, version_codename, version_id.",
            1,
            tags!(
                "name" => infos.get("NAME").unwrap_or(dv),
                "id" => infos.get("ID").unwrap_or(dv),
                "id_like" => infos.get("ID_LIKE").unwrap_or(dv),
                "pretty_name" => infos.get("PRETTY_NAME").unwrap_or(dv),
                "variant" => infos.get("VARIANT").unwrap_or(dv),
                "variant_id" => infos.get("VARIANT_ID").unwrap_or(dv),
                "version" => infos.get("VERSION").unwrap_or(dv),
                "version_id" => infos.get("VERSION_ID").unwrap_or(dv),
                "version_codename" => infos.get("VERSION_CODENAME").unwrap_or(dv),
                "build_id" => infos.get("BUILD_ID").unwrap_or(dv),
                "image_id" => infos.get("IMAGE_ID").unwrap_or(dv),
                "image_version" => infos.get("IMAGE_VERSION").unwrap_or(dv)
            ),
        ),
    ];

    if let Some(version) = infos.get("VERSION") {
        let version = version.parse().unwrap_or(0.0);
        metrics.push(Metric::gauge_with_tags(
            "node_os_version",
            "Metric containing the major.minor part of the OS version.",
            version,
            tags!(
                "id" => infos.get("ID").unwrap_or(dv),
                "id_link" => infos.get("ID_LIKE").unwrap_or(dv),
                "name" => infos.get("NAME").unwrap_or(dv)
            ),
        ));
    }

    Ok(metrics)
}

fn parse_quoted(input: &str) -> IResult<&str, &str> {
    let esc = escaped(none_of("\\\""), '\\', tag("\""));
    let esc_or_empty = alt((esc, tag("")));
    let res = delimited(tag("\""), esc_or_empty, tag("\""))(input)?;

    Ok(res)
}

fn parse_none_quoted(input: &str) -> IResult<&str, &str> {
    // dummy but works
    let n = take_while(|_c| true)(input)?;
    Ok(n)
}

async fn release_infos() -> Result<BTreeMap<String, String>, Error> {
    for path in [ETC_OS_RELEASE, USR_LIB_OS_RELEASE] {
        match parse_os_release(path).await {
            Ok(infos) => return Ok(infos),
            Err(_err) => continue,
        }
    }

    Err(Error::from("No invalid os release file"))
}

async fn parse_os_release(path: &str) -> Result<BTreeMap<String, String>, Error> {
    let content = read_to_string(path).await?;
    let mut envs = BTreeMap::new();

    for line in content.lines() {
        if let Ok((_, (key, _, value))) = tuple((
            take_while(|c: char| c.is_uppercase() || c == '_'),
            tag("="),
            alt((parse_quoted, parse_none_quoted)),
        ))(line)
        {
            envs.insert(key.to_string(), value.to_string());
        }
    }

    Ok(envs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_parse_os_release() {
        let path = format!("tests/fixtures{}", USR_LIB_OS_RELEASE);
        let m = parse_os_release(&path).await.unwrap();

        assert_eq!(m.get("NAME").unwrap(), "Ubuntu");
        assert_eq!(m.get("ID").unwrap(), "ubuntu");
        assert_eq!(m.get("ID_LIKE").unwrap(), "debian");
        assert_eq!(m.get("PRETTY_NAME").unwrap(), "Ubuntu 20.04.2 LTS");
        assert_eq!(m.get("VERSION").unwrap(), "20.04.2 LTS (Focal Fossa)");
        assert_eq!(m.get("VERSION_ID").unwrap(), "20.04");
        assert_eq!(m.get("VERSION_CODENAME").unwrap(), "focal");
    }
}
