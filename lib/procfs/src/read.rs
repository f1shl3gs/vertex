use std::io::Read;

/// `read_to_string` should be a async function, but the implement do sync calls from
/// std, which will not call spawn_blocking and create extra threads for IO reading. It
/// actually reduce cpu usage an memory. The `tokio-uring` should be introduce once it's
/// ready.
///
/// The files this function will(should) be reading is under `/sys` and `/proc` which is
/// relative small and the filesystem is kind of `tmpfs`, so the performance should never
/// be a problem.
pub async fn read_to_string<P: AsRef<Path>>(path: P) -> Result<String, std::io::Error> {
    let mut file = std::fs::File::open(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    Ok(content.trim_end().to_string())
}