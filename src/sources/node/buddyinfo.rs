use event::{Metric, tags};

use super::{Error, Paths, read_string};

pub async fn collect(paths: Paths) -> Result<Vec<Metric>, Error> {
    let content = read_string(paths.proc().join("buddyinfo"))?;

    let mut metrics = Vec::new();
    for info in parse_buddy_info(&content)? {
        for (index, size) in info.sizes.into_iter().enumerate() {
            metrics.push(Metric::gauge_with_tags(
                "node_buddyinfo_blocks",
                "Count of free blocks according to size.",
                size,
                tags!(
                    "node" => info.node,
                    "zone" => info.zone,
                    "size" => index
                ),
            ))
        }
    }

    Ok(metrics)
}

// BuddyInfo is the details parsed from /proc/buddyinfo.
// The data is comprised of an array of free fragments of each size.
// The sizes are 2^n*PAGE_SIZE, where n is the array index.
struct BuddyInfo<'a> {
    node: &'a str,
    zone: &'a str,
    sizes: Vec<f64>,
}

fn parse_buddy_info(content: &str) -> Result<Vec<BuddyInfo<'_>>, Error> {
    let mut infos = Vec::new();

    let mut buckets = None;
    for line in content.lines() {
        let fields = line.split_ascii_whitespace().collect::<Vec<_>>();
        if fields.len() < 4 {
            return Err(Error::Malformed("buddyinfo"));
        }

        let node = fields[1].trim_end_matches(',');
        let zone = fields[3].trim_end_matches(',');

        match buckets {
            Some(buckets) => {
                if buckets != fields.len() - 4 {
                    return Err(Error::Malformed("mismatched buckets in buddyinfo"));
                }
            }
            None => {
                buckets = Some(fields.len() - 4);
            }
        }

        let sizes = fields[4..]
            .iter()
            .map(|field| field.parse::<f64>())
            .collect::<Result<Vec<_>, _>>()?;

        infos.push(BuddyInfo { node, zone, sizes })
    }

    Ok(infos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ok() {
        let content = std::fs::read_to_string("tests/node/fixtures/proc/buddyinfo").unwrap();
        let infos = parse_buddy_info(&content).unwrap();

        assert_eq!(infos.len(), 3);

        assert_eq!(infos[0].zone, "DMA");
        assert_eq!(infos[2].zone, "Normal");
        assert_eq!(infos[2].sizes[0], 4381.0);
        assert_eq!(infos[1].sizes[1], 572.0);
    }

    #[test]
    fn short() {
        let input = r#"Node 0, zone
Node 0, zone
Node 0, zone
"#;

        let result = parse_buddy_info(input);
        assert!(result.is_err());
    }

    #[test]
    fn mismatch() {
        let input = r#"Node 0, zone      DMA      1      0      1      0      2      1      1      0      1      1      3
Node 0, zone    DMA32    759    572    791    475    194     45     12      0      0      0      0      0
Node 0, zone   Normal   4381   1093    185   1530    567    102      4      0      0      0
"#;

        let result = parse_buddy_info(input);
        assert!(result.is_err());
    }
}
