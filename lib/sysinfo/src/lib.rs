#[cfg(unix)]
mod unix;

pub fn os() -> Option<String> {
    #[cfg(unix)]
    return unix::os_version();

    #[cfg(not(unix))]
    None
}

pub fn machine_id() -> std::io::Result<String> {
    #[cfg(unix)]
    return unix::machine_id();

    #[cfg(not(unix))]
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "unsupported OS",
    ))
}

#[cfg(unix)]
pub use unix::kernel;
