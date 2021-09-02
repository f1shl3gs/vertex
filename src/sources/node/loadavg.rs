use crate::{
    gauge_metric,
    event::{Metric, MetricValue},
    sources::node::read_to_string,
};
use std::{
    path::PathBuf,
};

pub async fn gather(proc_path: &str) -> Result<Vec<Metric>, ()> {
    let root = PathBuf::from(proc_path);

    match get_load(root).await {
        Ok(loads) => {
            Ok(vec![
                gauge_metric!(
                    "node_load1",
                    "1m load average",
                    loads[0]
                ),
                gauge_metric!(
                    "node_load5",
                    "5m load average",
                    loads[1]
                ),
                gauge_metric!(
                    "node_load15",
                    "15m load average",
                    loads[2]
                ),
            ])
        }

        Err(err) => {
            warn!("read loadavg failed {}", err);
            Err(())
        }
    }
}

async fn get_load(mut path: PathBuf) -> Result<Vec<f64>, std::io::Error> {
    path.push("loadavg");

    let content = read_to_string(path).await?;
    let loads = content.split_ascii_whitespace()
        .map(|part| part.parse::<f64>().unwrap_or(0.0))
        .collect::<Vec<f64>>();

    Ok(loads)
}

// todo: handle it
fn getloadavg() -> (f64, f64, f64) {
    let mut data: [libc::c_double; 3] = [0.0, 0.0, 0.0];
    let result = unsafe { libc::getloadavg(data.as_mut_ptr(), 3) };

    if result == 3 {
        (data[0], data[1], data[2])
    } else {
        (0.0, 0.0, 0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_load() {
        let root = PathBuf::from("testdata/proc");
        let loads = get_load(root).await.unwrap();

        assert_eq!(loads[0], 0.02);
        assert_eq!(loads[1], 0.04);
        assert_eq!(loads[2], 0.05)
    }
}