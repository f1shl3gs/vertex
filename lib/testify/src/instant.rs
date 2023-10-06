use std::cell::RefCell;
use std::time::Duration;

thread_local! {
    static TIME: RefCell<Duration> = RefCell::new(Duration::default());
}

pub struct MockClock;

impl MockClock {
    /// Advance the internal Instant clock by this 'Duration'
    pub fn advance(time: Duration) {
        TIME.with_borrow_mut(|t| *t += time)
    }

    fn get_time() -> Duration {
        TIME.with_borrow(|t| *t)
    }
}

#[derive(Debug)]
pub struct Instant(Duration);

impl Instant {
    pub fn now() -> Self {
        Self(MockClock::get_time())
    }

    pub fn elapsed(&self) -> Duration {
        MockClock::get_time() - self.0
    }
}
