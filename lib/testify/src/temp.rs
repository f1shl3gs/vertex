use std::path::PathBuf;

pub fn temp_file() -> PathBuf {
    let path = std::env::temp_dir();
    let file_name = super::random::random_string(16);
    path.join(file_name)
}