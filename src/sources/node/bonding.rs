use std::io::ErrorKind;
use std::path::Path;

use event::{Metric, tags};

use super::{Error, Paths, read_file_no_stat};

pub async fn collect(paths: Paths) -> Result<Vec<Metric>, Error> {
    let root = paths.sys().join("class/net");
    let content = read_file_no_stat(root.join("bonding_masters"))?;

    let mut metrics = Vec::new();
    for master in content.split_ascii_whitespace() {
        let (slaves, active) = read_bonding_stats(&root, master)?;

        let tags = tags!("master" => master);
        metrics.extend([
            Metric::gauge_with_tags(
                "node_bonding_slaves",
                "Number of configured slaves per bonding interface.",
                slaves,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "node_bonding_active",
                "Number of active slaves per bonding interface.",
                active,
                tags,
            ),
        ]);
    }

    Ok(metrics)
}

fn read_bonding_stats(root: &Path, master: &str) -> Result<(u32, u32), Error> {
    let path = root.join(format!("{master}/bonding/slaves"));
    let content = read_file_no_stat(path)?;

    let mut slaves = 0;
    let mut active = 0;
    for slave in content.split_ascii_whitespace() {
        let state = match read_file_no_stat(
            root.join(format!("{master}/lower_{slave}/bonding_slave/mii_status")),
        ) {
            Ok(state) => state,
            Err(err) => {
                // some older? kernels use slave_ prefix
                if err.kind() != ErrorKind::NotFound {
                    return Err(err.into());
                }

                read_file_no_stat(
                    root.join(format!("{master}/slave_{slave}/bonding_slave/mii_status")),
                )?
            }
        };

        slaves += 1;
        active += if state.trim() == "up" { 1 } else { 0 };
    }

    Ok((slaves, active))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn smoke() {
        let paths = Paths::test();
        let metrics = collect(paths).await.unwrap();
        assert_ne!(metrics.len(), 0);
    }

    #[test]
    fn bonding_stats() {
        let path = Path::new("tests/bonding/sys/class/net");
        let (slaves, active) = read_bonding_stats(path, "bond0").unwrap();
        assert_eq!(slaves, 0);
        assert_eq!(active, 0);

        let (slaves, active) = read_bonding_stats(path, "int").unwrap();
        assert_eq!(slaves, 2);
        assert_eq!(active, 1);

        let (slaves, active) = read_bonding_stats(path, "dmz").unwrap();
        assert_eq!(slaves, 2);
        assert_eq!(active, 2);
    }
}
