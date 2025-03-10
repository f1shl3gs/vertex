use std::hash::{Hash, Hasher};
use std::time::Duration;

fn calculate_hash<T: Hash>(t: &T) -> u64 {
    // Default hasher is not the fastest, but it's totally fine here, cause
    // this func is not in the hot path.
    let mut s = std::collections::hash_map::DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

// offset returns the time until the next scrape cycle for the target.
pub fn offset<H: Hash>(h: &H, interval: Duration, jitter_seed: u64, now: i64) -> Duration {
    let hv = calculate_hash(h);
    let base = interval.as_nanos() as i64 - now % interval.as_nanos() as i64;
    let offset = (hv ^ jitter_seed) % interval.as_nanos() as u64;

    let mut next = base + offset as i64;
    if next > interval.as_nanos() as i64 {
        next -= interval.as_nanos() as i64
    }

    Duration::from_nanos(next as u64)
}

#[cfg(test)]
mod tests {
    use framework::config::default_interval;
    use testify::random::random_string;

    use super::*;

    #[test]
    fn spread_offset() {
        let n = 1000;
        let interval = default_interval();

        for _i in 0..n {
            let s = random_string(20);
            let o = offset(&s, interval, 0, 100);
            assert!(o < interval);
        }
    }

    #[test]
    fn equal_offset() {
        let t1 = String::from("boo");
        let t2 = String::from("boo");
        let t3 = String::from("far");

        let interval = default_interval();

        let now = 100;
        let o1 = offset(&t1, interval, 0, now);
        let o2 = offset(&t2, interval, 0, now);
        let o3 = offset(&t3, interval, 0, now);
        assert!(o1 < interval);
        assert!(o2 < interval);
        assert!(o3 < interval);
        assert_eq!(o1, o2);
        assert_ne!(o2, o3);
    }
}
