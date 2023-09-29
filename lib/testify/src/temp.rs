use std::path::PathBuf;

use super::random::random_string;

pub fn temp_file() -> PathBuf {
    let path = std::env::temp_dir();
    let file_name = super::random::random_string(16);
    path.join(file_name)
}

#[inline]
pub fn temp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(random_string(16));

    std::fs::create_dir(&dir).unwrap();

    dir
}
