use crate::event::Metric;
use crate::sources::node::errors::Error;

use std::io;

pub async fn gather(proc_path: &str) -> Result<Vec<Metric>, ()> {
    todo!()
}

struct Stat {
    name: String,

}

async fn get_cpu_freq_stat(sys_path: &str) -> Result<Vec<Stat>, Error> {
    let cpus = glob::glob(&format!("{}/devices/system/cpu/cpu[0-9]*", sys_path))
        .map_err(|err| {
            let inner = io::Error::new(io::ErrorKind::InvalidInput, err);

            Error::from(inner).with_message("No cpu files were found")
        })?;

    let stats = Vec::new();

    for entry in cpus {
        match entry {
            Ok(path) => {
                println!("find {:?}", path)
            },

            Err(err) => {
                println!("err {}", err)
            }
        }
    }

    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_cpu_freq_stat() {
        let sys_path = "testdata/sys";
        get_cpu_freq_stat(sys_path).await.unwrap();
    }
}