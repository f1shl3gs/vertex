use std::path::PathBuf;
use rand::{Rng, thread_rng};
use rand::distributions::Alphanumeric;

fn random_string(len: usize) -> String {
    thread_rng().sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect::<String>()
}

pub fn temp_file() -> PathBuf {
    let path = std::env::temp_dir();
    let file_name = random_string(16);
    path.join(file_name)
}