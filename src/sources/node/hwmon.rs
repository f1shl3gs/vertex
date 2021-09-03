use crate::{
    tags,
    gauge_metric,
    event::{Metric, MetricValue},
};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use crate::sources::node::errors::Error;
use crate::sources::node::{read_to_string, read_into};
use std::collections::BTreeMap;

pub async fn gather(sys_path: &str) -> Result<Vec<Metric>, ()> {
    let path = format!("{}/class/hwmon", sys_path);
    let mut dirs = tokio::fs::read_dir(path).await
        .map_err(|err| {
            warn!("read hwmon dir failed"; "err" => err);
        })?;

    let mut metrics = Vec::new();
    while let Some(entry) = dirs.next_entry().await.map_err(|err| {
        warn!("read next entry of hwmon dirs failed"; "err" => err);
    })? {
        let meta = entry.metadata().await.map_err(|err| {
            warn!("read metadata failed"; "err" => err);
        })?;

        let file_type = meta.file_type();
        if file_type.is_symlink() {
            match tokio::fs::read_link(entry.path()).await {
                Ok(path) => {
                    let name = human_readable_chip_name(entry.path()).await
                        .map_err(|err| {
                            println!("err {}", err);
                            err
                        })
                        .unwrap_or("".to_string());

                    metrics.push(gauge_metric!(
                        "node_hwmon_chip_names",
                        "Annotation metric for human-readable chip names",
                        1.0,
                        "chip" => "a",
                        "chip_name" => name
                    ))
                }

                Err(err) => {
                    println!("read link failed {}", err);
                }
            }

            continue;
        }


        println!("handle {:?}", entry);
    }

    Err(())
}

async fn hwmon_stats(dir: &str) {}

async fn collect_sensor_data(dir: &str) -> Result<BTreeMap<String, BTreeMap<String, f64>>, Error> {
    let mut dirs = tokio::fs::read_dir(dir).await.map_err(Error::from)?;

    let mut stats = BTreeMap::<String, BTreeMap<String, f64>>::new();
    while let Some(entry) = dirs.next_entry().await.map_err(Error::from)? {
        let path = entry.path().clone();
        let filename = path.file_name().unwrap().to_str().unwrap();

        if let Ok((sensor, num, property)) = explode_sensor_filename(filename) {
            if !is_hwmon_sensor(sensor) {
                continue;
            }

            match read_into(entry.path()).await {
                Ok(v) => {
                    let sensor = format!("{}{}", sensor, num);
                    if !stats.contains_key(&sensor) {
                        stats.insert(sensor.clone(), BTreeMap::new());
                    }

                    let props = stats.get_mut(&sensor).unwrap();
                    props.insert(property.to_string(), v);
                }
                Err(err) => {
                    continue;
                }
            }
        }
    }

    Ok(stats)
}

// explode_sensor_filename splits a sensor name into <type><num>_<property>
fn explode_sensor_filename(name: &str) -> Result<(&str, &str, &str), ()> {
    let s = name.as_bytes();

    let mut typ_end = 0;
    let mut num_end = 0;

    // consume type
    for i in 0..s.len() {
        let c = s[i];
        if c >= b'0' && c <= b'9' {
            typ_end = i;
            break;
        }
    }

    // we never meet an number
    if typ_end == 0 {
        return Err(());
    }

    // consume num until we meet '_'
    for i in typ_end..s.len() {
        let c = s[i];
        if c == b'_' {
            num_end = i;
            break;
        }
    }

    // we never meet the property separator '_'
    if num_end == typ_end {
        return Err(());
    }

    Ok((&name[0..typ_end], &name[typ_end..num_end], &name[num_end + 1..]))
}

// human_readable_name is similar to the methods in
async fn human_readable_chip_name(dir: PathBuf) -> Result<String, Error> {
    let mut path = dir.to_str().unwrap().to_string();
    path.push_str("/name");

    read_to_string(path).await.map_err(Error::from)
}

fn hwmon_name(path: &str) -> Result<String, ()> {
    // generate a name for a sensor path

    // sensor numbering depends on the order of linux module loading and
    // is thus unstable.
    // However the path of the device has to be stable:
    // - /sys/devices/<bus>/<device>
    // Some hardware monitors have a "name" file that exports a human readable
    // name that can be used.

    // human readable names would be bat0 or coretemp, while a path string
    // could be platform_applesmc.768

    // preference 1: construct a name based on device name, always unique

    todo!()
}

fn is_hwmon_sensor(s: &str) -> bool {
    ["vrm", "beep_enable", "update_interval", "in", "cpu", "fan",
        "pwm", "temp", "curr", "power", "energy", "humidity",
        "intrusion", ].contains(&s)
}

fn sanitized(s: &str) -> String {
    let s = s.trim().to_lowercase();
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::option::Option::Some;

    #[tokio::test]
    async fn test_gather() {
        let path = "testdata/sys";

        gather(path).await;
    }

    #[test]
    fn test_explode_sensor_filename() {
        let input = "fan1_input";

        let (typ, id, property) = explode_sensor_filename(input).unwrap();
        assert_eq!(typ, "fan");
        assert_eq!(id, "1");
        assert_eq!(property, "input");

        let input = "fan_i";
        assert!(explode_sensor_filename(input).is_err())
    }

    #[test]
    fn test_is_hwmon_sensor() {
        assert!(is_hwmon_sensor("fan"));
        assert!(is_hwmon_sensor("foo"));
    }

    #[tokio::test]
    async fn test_collect_sensor_data() {
        let path = "testdata/sys/class/hwmon/hwmon3";
        let kvs = collect_sensor_data(path).await.unwrap();

        println!("{:?}", kvs);
    }
}
