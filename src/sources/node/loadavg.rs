use std::path::PathBuf;

use event::Metric;

use super::{Error, read_string};

pub async fn gather(proc_path: PathBuf) -> Result<Vec<Metric>, Error> {
    let loads = get_load(proc_path)?;

    Ok(vec![
        Metric::gauge("node_load1", "1m load average", loads[0]),
        Metric::gauge("node_load5", "5m load average", loads[1]),
        Metric::gauge("node_load15", "15m load average", loads[2]),
    ])
}

fn get_load(path: PathBuf) -> Result<Vec<f64>, Error> {
    let content = read_string(path.join("loadavg"))?;

    let loads = content
        .split_ascii_whitespace()
        .take(3)
        .map(|s| s.parse::<f64>())
        .collect::<Result<Vec<_>, _>>()?;

    if loads.len() != 3 {
        return Err(Error::NoData);
    }

    Ok(loads)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse() {
        let root = PathBuf::from("tests/node/proc");
        let loads = get_load(root).unwrap();

        assert_eq!(loads[0], 0.02);
        assert_eq!(loads[1], 0.04);
        assert_eq!(loads[2], 0.05)
    }
}
