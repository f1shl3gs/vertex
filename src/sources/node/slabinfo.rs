use configurable::Configurable;
use event::{Metric, tags};
use framework::config::{serde_regex, serde_regex_option};
use serde::{Deserialize, Serialize};

use super::{Error, Paths, read_file_no_stat};

fn default_include() -> regex::Regex {
    regex::Regex::new(".*").unwrap()
}

#[derive(Clone, Configurable, Debug, Deserialize, Serialize)]
pub struct Config {
    /// Regex of items to include in collector
    #[serde(default = "default_include", with = "serde_regex")]
    include: regex::Regex,

    /// Regex of items to exclude in collector
    #[serde(default, with = "serde_regex_option")]
    exclude: Option<regex::Regex>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            include: default_include(),
            exclude: None,
        }
    }
}

impl Config {
    fn ignore(&self, name: &str) -> bool {
        if let Some(exclude) = &self.exclude
            && !exclude.as_str().is_empty()
            && exclude.is_match(name)
        {
            return true;
        }

        !self.include.as_str().is_empty() && !self.include.is_match(name)
    }
}

pub async fn collect(config: Config, paths: Paths) -> Result<Vec<Metric>, Error> {
    let content = read_file_no_stat(paths.proc().join("slabinfo"))?;

    let slabs = parse_slab_info(&content)?;
    let mut metrics = Vec::with_capacity(slabs.len() * 5);
    for slab in slabs {
        if config.ignore(slab.name) {
            continue;
        }

        let tags = tags!("slab" => slab.name);
        metrics.extend([
            Metric::gauge_with_tags(
                "node_slabinfo_active_objects",
                "The number of objects that are currently active (i.e., in use).",
                slab.obj_active,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "node_slabinfo_objects",
                "The total number of allocated objects (i.e., objects that are both in use and not in use).",
                slab.obj_num,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "node_slabinfo_object_size_bytes",
                "The size of objects in this slab, in bytes.",
                slab.obj_size,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "node_slabinfo_objects_per_slab",
                "The number of objects stored in each slab.",
                slab.obj_per_slab,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "node_slabinfo_pages_per_slab",
                "The number of pages allocated for each slab.",
                slab.pages_per_slab,
                tags,
            )
        ]);
    }

    Ok(metrics)
}

struct Slab<'a> {
    name: &'a str,
    obj_active: i64,
    obj_num: i64,
    obj_size: i64,
    obj_per_slab: i64,
    pages_per_slab: i64,
    // tunables
    limit: i64,
    batch: i64,
    shared_factor: i64,
    slab_active: i64,
    slab_num: i64,
    shared_avail: i64,
}

fn parse_slab_info(content: &str) -> Result<Vec<Slab<'_>>, Error> {
    let mut slabs = Vec::new();

    // skip version and header line
    for line in content.lines().skip(2) {
        let fields = line.split_ascii_whitespace().collect::<Vec<_>>();
        if fields.len() != 16 {
            return Err(Error::Malformed("slab line"));
        }

        let name = fields[0];
        let obj_active = fields[1].parse()?;
        let obj_num = fields[2].parse()?;
        let obj_size = fields[3].parse()?;
        let obj_per_slab = fields[4].parse()?;
        let pages_per_slab = fields[5].parse()?;
        let limit = fields[8].parse()?;
        let batch = fields[9].parse()?;
        let shared_factor = fields[10].parse()?;
        let slab_active = fields[13].parse()?;
        let slab_num = fields[14].parse()?;
        let shared_avail = fields[15].parse()?;

        slabs.push(Slab {
            name,
            obj_active,
            obj_num,
            obj_size,
            obj_per_slab,
            pages_per_slab,
            // tunables
            limit,
            batch,
            shared_factor,
            slab_active,
            slab_num,
            shared_avail,
        })
    }

    Ok(slabs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use regex::Regex;

    #[test]
    fn exclude() {
        for (exclude, include, name, want) in [
            ("", "", "eth0", false),
            ("", "^💩0$", "💩0", false),
            ("", "^💩0$", "💩1", true),
            ("", "^💩0$", "veth0", true),
            ("^💩", "", "💩3", true),
            ("^💩", "", "veth0", false),
        ] {
            let maybe_exclude = if exclude.is_empty() {
                None
            } else {
                Some(Regex::new(exclude).unwrap())
            };

            let config = Config {
                include: Regex::new(include).unwrap(),
                exclude: maybe_exclude,
            };

            assert_eq!(
                want,
                config.ignore(name),
                "include: {include:?} exclude: {exclude:?}, name: {name}, result: {want}",
            );
        }
    }

    #[test]
    fn parse() {
        let content = std::fs::read_to_string("tests/node/fixtures/proc/slabinfo").unwrap();
        let slabs = parse_slab_info(&content).unwrap();

        assert_eq!(slabs.len(), 300);

        assert_eq!(slabs[0].name, "pid_3");
        assert_eq!(slabs[0].obj_active, 375);
        assert_eq!(slabs[0].obj_num, 532);
        assert_eq!(slabs[0].obj_size, 576);
        assert_eq!(slabs[0].obj_per_slab, 28);
        assert_eq!(slabs[0].pages_per_slab, 4);
        assert_eq!(slabs[0].limit, 0);
        assert_eq!(slabs[0].batch, 0);
        assert_eq!(slabs[0].shared_factor, 0);
        assert_eq!(slabs[0].slab_active, 19);
        assert_eq!(slabs[0].slab_num, 19);
        assert_eq!(slabs[0].shared_avail, 0);
    }
}
