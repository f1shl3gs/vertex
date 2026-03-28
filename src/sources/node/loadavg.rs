use event::Metric;

use super::{Error, Paths, read_file_no_stat};

pub async fn collect(paths: Paths) -> Result<Vec<Metric>, Error> {
    let content = read_file_no_stat(paths.proc().join("loadavg"))?;
    let loads = parse_loadavg(&content)?;

    Ok(vec![
        Metric::gauge("node_load1", "1m load average", loads[0]),
        Metric::gauge("node_load5", "5m load average", loads[1]),
        Metric::gauge("node_load15", "15m load average", loads[2]),
    ])
}

fn parse_loadavg(content: &str) -> Result<Vec<f64>, Error> {
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

    #[tokio::test]
    async fn smoke() {
        let paths = Paths::test();
        let metrics = collect(paths).await.unwrap();
        assert_eq!(metrics.len(), 3);
    }

    #[test]
    fn parse() {
        let content = std::fs::read_to_string("tests/node/fixtures/proc/loadavg").unwrap();
        let loads = parse_loadavg(&content).unwrap();

        assert_eq!(loads[0], 0.02);
        assert_eq!(loads[1], 0.04);
        assert_eq!(loads[2], 0.05)
    }
}
