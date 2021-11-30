

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum Concurrency {
    None,
    Adaptive,
    Fixed(usize),
}