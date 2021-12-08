use rand::{Rng, thread_rng};
use rand::distributions::Alphanumeric;

pub fn random_string(len: usize) -> String {
    thread_rng().sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect::<String>()
}

pub fn random_lines(len: usize) -> impl Iterator<Item=String> {
    std::iter::repeat(())
        .map(move |_| random_string(len))
}