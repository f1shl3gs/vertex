use std::collections::BTreeMap;
use std::time::Instant;

use super::gcra::{GcraState, Quota};

pub struct RateLimiter {
    quota: Quota,
    state: GcraState,
}

impl RateLimiter {
    pub fn new(quota: Quota) -> Self {
        Self {
            quota,
            state: Default::default(),
        }
    }

    pub fn check(&mut self) -> bool {
        self.state.check_and_modify(&self.quota, 1).is_ok()
    }
}

/// KeyedRateLimiter is not thread-safe, but in our case, that is what we
/// want.
pub struct KeyedRateLimiter {
    quota: Quota,
    states: BTreeMap<String, GcraState>,
}

impl KeyedRateLimiter {
    pub const fn new(quota: Quota) -> Self {
        Self {
            quota,
            states: BTreeMap::new(),
        }
    }
}

impl KeyedRateLimiter {
    pub fn check(&mut self, key: &str) -> bool {
        let state = match self.states.get_mut(key) {
            Some(state) => state,
            None => {
                self.states.entry(key.to_string())
                    .or_insert_with(GcraState::default)
            }
        };

        state.check_and_modify(&self.quota, 1).is_ok()
    }

    pub fn retain_recent(&mut self) {
        let now = Instant::now();

        self.states.retain(|_key, state| match state.tat {
            Some(tat) => tat > now,
            None => false,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::thread::sleep;
    use std::time::Duration;

    use super::*;

    const LIMIT: u32 = 3u32;
    const WINDOW: Duration = Duration::from_secs(1);

    #[test]
    fn rl() {
        let quota = Quota::new(LIMIT, WINDOW);
        let mut limiter = RateLimiter::new(quota);
        for _i in 0..LIMIT {
            assert!(limiter.check());
        }
        assert!(!limiter.check());
        sleep(WINDOW);
        assert!(limiter.check())
    }

    #[test]
    fn keyed() {
        let key_1 = "foo";
        let key_2 = "bar";
        let quota = Quota::new(LIMIT, WINDOW);

        let mut rl = KeyedRateLimiter::new(quota);
        for _i in 0..LIMIT {
            assert!(rl.check(key_1));
            assert!(rl.check(key_2));
        }

        assert!(!rl.check(key_1));
        assert!(!rl.check(key_2));

        sleep(2 * WINDOW);
        assert!(rl.check(key_1));

        rl.retain_recent();
        assert!(rl.states.contains_key(key_1));
        assert!(!rl.states.contains_key(key_2));
    }
}
