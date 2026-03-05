use std::path::Path;

use event::Metric;

use super::{Error, Paths, read_string};

pub async fn collect(paths: Paths) -> Result<Vec<Metric>, Error> {
    let (allocated, maximum) = read_file_nr(paths.proc())?;

    Ok(vec![
        Metric::gauge(
            "node_filefd_allocated",
            "File descriptor statistics: allocated",
            allocated,
        ),
        Metric::gauge(
            "node_filefd_maximum",
            "File descriptor statistics: maximum",
            maximum,
        ),
    ])
}

fn read_file_nr(proc_path: &Path) -> Result<(u64, u64), Error> {
    let content = read_string(proc_path.join("sys/fs/file-nr"))?;

    // the file-nr proc is only 1 line with 3 values
    let parts = content.split_ascii_whitespace().collect::<Vec<_>>();

    let allocated = parts[0].parse()?;
    let maximum = parts[2].parse()?;

    Ok((allocated, maximum))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_nr() {
        let path = Path::new("tests/node/fixtures/proc");
        let (allocated, maximum) = read_file_nr(path).unwrap();

        assert_eq!(allocated, 1024);
        assert_eq!(maximum, 1631329);
    }
}
