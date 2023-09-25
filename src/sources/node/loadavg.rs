use std::path::PathBuf;

use event::Metric;

use super::{read_to_string, Error};

pub async fn gather(proc_path: &str) -> Result<Vec<Metric>, Error> {
    let root = PathBuf::from(proc_path);

    match get_load(root).await {
        Ok(loads) => Ok(vec![
            Metric::gauge("node_load1", "1m load average", loads[0]),
            Metric::gauge("node_load5", "5m load average", loads[1]),
            Metric::gauge("node_load15", "15m load average", loads[2]),
        ]),

        Err(err) => Err(Error::from(err)),
    }
}

async fn get_load(mut path: PathBuf) -> Result<Vec<f64>, std::io::Error> {
    path.push("loadavg");

    let content = read_to_string(path).await?;
    let loads = content
        .split_ascii_whitespace()
        .map(|part| part.parse::<f64>().unwrap_or(0.0))
        .collect::<Vec<f64>>();

    Ok(loads)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_load() {
        let root = PathBuf::from("tests/fixtures/proc");
        let loads = get_load(root).await.unwrap();

        assert_eq!(loads[0], 0.02);
        assert_eq!(loads[1], 0.04);
        assert_eq!(loads[2], 0.05)
    }
}
