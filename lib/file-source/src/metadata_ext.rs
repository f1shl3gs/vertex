use std::fs::File;
use std::os::unix::fs::MetadataExt;

#[cfg(not(windows))]
pub trait PortableFileExt {
    fn portable_dev(&self) -> std::io::Result<u64>;
    fn portable_ino(&self) -> std::io::Result<u64>;
}

#[cfg(unix)]
impl PortableFileExt for File {
    fn portable_dev(&self) -> std::io::Result<u64> {
        Ok(self.metadata()?.dev())
    }

    fn portable_ino(&self) -> std::io::Result<u64> {
        Ok(self.metadata()?.ino())
    }
}
