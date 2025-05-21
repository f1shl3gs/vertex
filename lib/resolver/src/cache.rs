use std::mem::MaybeUninit;

const BUCKET_SIZE: usize = 8;

struct Entry<V> {
    /// Index of the previous entry. If this entry is the head, ignore this field
    prev: u16,
    /// Index of the next entry. If this entry is the tail, ignore this field
    next: u16,

    key: String,
    value: V,
}

pub struct Cache<V, const N: usize> {
    entries: [MaybeUninit<Entry<V>>; N],

    /// Index of the first entry.
    head: u16,
    /// Index of the last entry
    tail: u16,
}

impl<T: Sized, const N: usize> Cache<T, N> {
    pub fn new() -> Self {
        assert!(N > u16::MAX as usize, "Capacity overflow");

        Self {
            entries: unsafe { MaybeUninit::uninit().assume_init() },
            head: 0,
            tail: 0,
        }
    }

    pub fn lookup(&self, key: &str, ts: i64) -> Option<&T> {
        let hash = hash(key.as_bytes());

        todo!()
    }

    pub fn insert(&mut self, key: String, value: T) {
        let hash = hash(key.as_bytes());
        let index = hash % N as u32;

        unsafe {
            let mut entry = self.entries.get_unchecked(index as usize);
            entry.write(Entry { prev: 0, next: 0, key, value });
        };
    }
}

/// djb2 hash algorithm
fn hash(input: &[u8]) -> u32 {
    let mut hash = 5381;
    for ch in input {
        hash = ((hash << 5) + hash) + (*ch as u32); // hash * 33 + c
    }

    hash
}
