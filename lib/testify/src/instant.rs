use std::cell::RefCell;
use std::time::Duration;

thread_local! {
    pub static TIME: RefCell<Duration> = RefCell::new(Duration::default());
}

pub struct MockClock;

impl MockClock {
    /// Advance the internal Instant clock by this 'Duration'
    pub fn advance(time: Duration) {
        TIME.with(|t| {
            let t = &mut *t.borrow_mut();
            *t += time;
        })
    }

    fn get_time() -> Duration {
        TIME.with(|t| *t.borrow())
    }
}

#[derive(Debug)]
pub struct Instant(Duration);

impl Instant {
    pub fn now() -> Self {
        Self(TIME.with(|t| *t.borrow()))
    }

    pub fn elapsed(&self) -> Duration {
        MockClock::get_time() - self.0
    }
}
