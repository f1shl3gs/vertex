mod key;
mod value;

// re-export
pub use key::Key;
pub use value::{Array, Value};

use std::alloc::{Layout, alloc, dealloc};
use std::cmp::Ordering::{Greater, Less};
use std::collections::BTreeMap;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::ptr::{NonNull, drop_in_place, slice_from_raw_parts_mut};

use serde::de::{MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use typesize::TypeSize;

const GROWTH_ALIGNMENT: isize = 4;

#[allow(clippy::cast_sign_loss)]
#[inline]
fn aligned(amount: usize) -> usize {
    ((amount as isize + GROWTH_ALIGNMENT - 1) & -GROWTH_ALIGNMENT) as usize
}

#[derive(Clone, Debug, Hash, PartialEq)]
pub struct Entry {
    key: Key,
    value: Value,
}

/// Tags is a vec ordered by key.
pub struct Tags {
    len: usize,
    cap: usize,

    // Since we never allocate on heap unless our capacity is bigger than
    // capacity, and heap capacity cannot be less than 1.
    // Therefore, pointer cannot be null too.
    data: NonNull<Entry>,
}

unsafe impl Send for Tags {}
unsafe impl Sync for Tags {}

impl TypeSize for Tags {
    fn allocated_bytes(&self) -> usize {
        self.iter()
            .map(|(key, value)| {
                let val_len = match value {
                    Value::String(s) => s.len(),
                    Value::Array(Array::String(ss)) => ss.iter().map(|s| s.len()).sum(),
                    _ => 0,
                };

                key.len() + val_len
            })
            .sum()
    }
}

impl Clone for Tags {
    fn clone(&self) -> Self {
        let layout = Layout::array::<Entry>(self.cap).unwrap();
        let data: NonNull<Entry> = unsafe {
            let data: NonNull<Entry> = NonNull::new_unchecked(alloc(layout)).cast();
            for i in 0..self.len {
                let from = &(*self.data.as_ptr().add(i));
                data.as_ptr().add(i).write(from.clone());
            }

            data
        };

        Self {
            data,
            len: self.len,
            cap: self.cap,
        }
    }
}

impl Default for Tags {
    #[inline]
    fn default() -> Self {
        Self::with_capacity(4)
    }
}

impl Debug for Tags {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut first = true;

        self.iter().try_for_each(|(key, value)| {
            if first {
                first = false;
                f.write_fmt(format_args!("{}={}", key, value))
            } else {
                f.write_fmt(format_args!(",{}={}", key, value))
            }
        })
    }
}

impl Drop for Tags {
    fn drop(&mut self) {
        unsafe {
            drop_in_place(slice_from_raw_parts_mut(self.data.as_ptr(), self.len));

            let layout = Layout::array::<Entry>(self.cap).expect("build layout");
            dealloc(self.data.as_ptr().cast(), layout);
        }
    }
}

impl Eq for Tags {}

impl Hash for Tags {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.len.hash(state);
        for elt in self {
            elt.hash(state);
        }
    }
}

impl FromIterator<(Key, Value)> for Tags {
    fn from_iter<T: IntoIterator<Item = (Key, Value)>>(iter: T) -> Self {
        let mut tags = Self::default();
        iter.into_iter().for_each(|(k, v)| tags.insert(k, v));

        tags
    }
}

pub struct Iter<'a> {
    pos: usize,
    len: usize,
    data: NonNull<Entry>,
    _marker: PhantomData<&'a usize>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = (&'a Key, &'a Value);

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos == self.len {
            return None;
        }

        unsafe {
            let entry = &(*self.data.as_ptr().add(self.pos));
            self.pos += 1;
            Some((&entry.key, &entry.value))
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len - self.pos;
        (len, Some(len))
    }

    #[inline]
    fn count(self) -> usize
    where
        Self: Sized,
    {
        self.len
    }
}

pub struct TagIntoIter {
    data: NonNull<Entry>,
    len: usize,
    cap: usize,

    pos: usize,
}

impl Drop for TagIntoIter {
    fn drop(&mut self) {
        unsafe {
            if self.pos < self.len {
                let slice =
                    slice_from_raw_parts_mut(self.data.as_ptr().add(self.pos), self.len - self.pos);
                drop_in_place(slice);
            }

            let layout = Layout::array::<Entry>(self.cap).expect("build layout");
            dealloc(self.data.as_ptr().cast(), layout);
        }
    }
}

impl Iterator for TagIntoIter {
    type Item = (Key, Value);

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos == self.len {
            return None;
        }

        let entry = unsafe {
            let ptr = self.data.as_ptr().add(self.pos);
            std::ptr::read(ptr)
        };

        self.pos += 1;

        Some((entry.key, entry.value))
    }
}

impl IntoIterator for Tags {
    type Item = (Key, Value);
    type IntoIter = TagIntoIter;

    fn into_iter(self) -> Self::IntoIter {
        let this = ManuallyDrop::new(self);

        TagIntoIter {
            data: this.data,
            len: this.len,
            cap: this.cap,
            pos: 0,
        }
    }
}

impl<'a> IntoIterator for &'a Tags {
    type Item = (&'a Key, &'a Value);
    type IntoIter = Iter<'a>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl PartialEq for Tags {
    fn eq(&self, other: &Self) -> bool {
        if !self.len.eq(&other.len) {
            return false;
        }

        unsafe {
            for i in 0..self.len {
                let a = self.data.as_ptr().add(i);
                let b = other.data.as_ptr().add(i);
                if !(*a).eq(&(*b)) {
                    return false;
                }
            }

            true
        }
    }
}

impl Serialize for Tags {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.len))?;
        for (key, value) in self {
            map.serialize_entry(key, value)?;
        }

        map.end()
    }
}

impl<'de> Deserialize<'de> for Tags {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct TagsVisitor;

        impl<'de> Visitor<'de> for TagsVisitor {
            type Value = Tags;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("object")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut tags = Tags::default();
                while let Some((key, value)) = map.next_entry::<Key, Value>()? {
                    tags.insert(key, value);
                }

                Ok(tags)
            }
        }

        deserializer.deserialize_map(TagsVisitor)
    }
}

impl From<BTreeMap<String, String>> for Tags {
    fn from(value: BTreeMap<String, String>) -> Self {
        let mut tags = Tags::with_capacity(value.len());
        for (k, v) in value {
            tags.insert(k, v);
        }

        tags
    }
}

impl Tags {
    const MAX_SIZE: usize = 128;

    /// Creates an empty `Tags` with at least the specified capacity.
    ///
    /// # Panics
    ///
    /// Allocate failed
    pub fn with_capacity(cap: usize) -> Self {
        let layout = Layout::array::<Entry>(cap).expect("build layout");
        let ptr = unsafe { NonNull::new_unchecked(alloc(layout)).cast() };

        Self {
            len: 0,
            cap,
            data: ptr,
        }
    }

    /// Returns the length of the Tags.
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns the number of elements the Tags can hold without reallocating.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.cap
    }

    /// Returns whether or not the `MiniVec` has a length greater than 0.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline]
    pub fn iter(&self) -> Iter<'_> {
        Iter {
            pos: 0,
            len: self.len,
            data: self.data,
            #[allow(clippy::default_constructed_unit_structs)]
            _marker: PhantomData::default(),
        }
    }

    /// Inserts a key-value pair into the tags
    pub fn insert(&mut self, key: impl Into<Key> + AsRef<str>, value: impl Into<Value>) {
        match self.binary_search(key.as_ref()) {
            Ok(pos) => unsafe {
                let elt = self.data.as_ptr().add(pos);
                (*elt).value = value.into();
            },
            Err(pos) => {
                if self.len == self.cap {
                    self.grow(aligned(self.cap + 1));
                }

                unsafe {
                    let ptr = self.data.as_ptr().add(pos);
                    if pos < self.len {
                        std::ptr::copy(ptr, ptr.add(1), self.len - pos);
                    }

                    ptr.write(Entry {
                        key: key.into(),
                        value: value.into(),
                    });
                    self.len += 1;
                }
            }
        }
    }

    /// Removes and returns the element by key, shifting all elements after
    /// it to the left.
    pub fn remove(&mut self, key: impl AsRef<str>) -> Option<Value> {
        match self.binary_search(key.as_ref()) {
            Ok(pos) => unsafe {
                let ptr = self.data.as_ptr().add(pos);
                let entry = std::ptr::read(ptr);
                if pos < self.len - 1 {
                    std::ptr::copy(ptr.add(1), ptr, self.len - pos - 1);
                }

                self.len -= 1;
                Some(entry.value)
            },
            // not found
            Err(_pos) => None,
        }
    }

    /// Returns true if the map contains a value for the specified key.
    #[inline]
    pub fn contains(&self, key: &str) -> bool {
        self.binary_search(key).is_ok()
    }

    /// Returns a reference to the value corresponding to the key.
    pub fn get(&self, key: impl AsRef<str>) -> Option<&Value> {
        if let Ok(pos) = self.binary_search(key.as_ref()) {
            unsafe {
                let ptr = self.data.as_ptr().add(pos);
                Some(&(*ptr).value)
            }
        } else {
            None
        }
    }

    /// Returns a mutable reference to the value corresponding to the key.
    pub fn get_mut(&mut self, key: impl AsRef<str>) -> Option<&mut Value> {
        if let Ok(pos) = self.binary_search(key.as_ref()) {
            unsafe {
                let entry = self.data.as_ptr().add(pos);
                Some(&mut (*entry).value)
            }
        } else {
            None
        }
    }

    /// Binary searches this slice.
    ///
    /// If the slice is not sorted or if the compare does not implement
    /// an order consistent with the sort order of the underlying slice,
    /// the returned result is unspecified and meaningless.
    ///
    /// If the value is found then [`Result::Ok`] is returned, containing
    /// the index of the matching element. If there are multiple matches,
    /// then any one of the matches could be returned. The index is chosen
    /// deterministically, but is subject to change in future versions of
    /// Rust. If the value is not found then [`Result::Err`] is returned,
    /// containing the index where a matching element could be inserted
    /// while maintaining sorted order.
    #[inline]
    fn binary_search(&self, key: &str) -> Result<usize, usize> {
        let mut size = self.len;
        let mut left = 0;
        let mut right = size;
        while left < right {
            let mid = left + size / 2;

            let sk = unsafe {
                let ptr = self.data.as_ptr().add(mid);
                &(*ptr).key
            };

            let cmp = key.cmp(sk);

            // The reason why we use if/else control flow rather than match
            // is because match reorders comparison operations, which is perf
            // sensitive.
            // This is x86 asm for u8: https://rust.godbolt.org/z/8Y8Pra.
            if cmp == Greater {
                left = mid + 1;
            } else if cmp == Less {
                right = mid;
            } else {
                // unsafe { core::intrinsics::assume(mid < self.len()) };
                return Ok(mid);
            }

            size = right - left;
        }

        // unsafe { core::intrinsics::assume(left <= self.len()) };
        Err(left)
    }

    /// Re-allocate to set the capacity to `new_cap`.
    ///
    /// Panics if `new_cap` is less than the vector's length.
    fn grow(&mut self, new_cap: usize) {
        assert!(new_cap <= Tags::MAX_SIZE, "overflow");

        unsafe {
            let new_layout = Layout::array::<Entry>(new_cap).unwrap();
            let old_layout = Layout::array::<Entry>(self.cap).unwrap();
            let new_ptr =
                std::alloc::realloc(self.data.as_ptr().cast(), old_layout, new_layout.size());

            self.data = NonNull::new_unchecked(new_ptr.cast());
            self.cap = new_cap;
        }
    }

    #[allow(unsafe_op_in_unsafe_fn)]
    unsafe fn remove_by_index(&mut self, index: usize) -> Entry {
        let ptr = self.data.as_ptr().add(index);
        let entry = std::ptr::read(ptr);
        if index < self.len - 1 {
            std::ptr::copy(ptr.add(1), ptr, self.len - index - 1);
        }

        self.len -= 1;
        entry
    }

    /// Retains only the elements specified by the predicate.
    ///
    /// In other words, remove all elements Key/Value pairs for which
    /// `f(&Key, &Value)` returns `false`.
    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&Key, &Value) -> bool,
    {
        let mut pos = 0;

        unsafe {
            while pos < self.len {
                let ptr = self.data.as_ptr().add(pos);
                let entry = &(*ptr);
                if !f(&entry.key, &entry.value) {
                    let _ = self.remove_by_index(pos);
                    continue;
                }

                pos += 1;
            }
        }
    }
}

#[macro_export]
macro_rules! tags {
    // count helper: transform any expression into 1
    (@one $x:expr) => (1usize);

    // Done without trailing comma
    ( $($x:expr => $y:expr),* ) => ({
        let count = 0usize $(+ tags!(@one $x))*;
        let mut _tags = $crate::tags::Tags::with_capacity(count);
        $(
            _tags.insert($x, $y);
        )*
        _tags
    });
    // Done with trailing comma
    ( $($x:expr => $y:expr,)* ) => (
        tags!{$($x => $y),*}
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::Utc;
    use rand::SeedableRng;
    use rand::rngs::StdRng;
    use rand::seq::SliceRandom;

    #[test]
    fn align() {
        for (input, want) in [(1, 4), (2, 4), (3, 4), (4, 4), (5, 8)] {
            assert_eq!(aligned(input), want);
        }
    }

    #[test]
    fn fuzz() {
        #[allow(clippy::cast_sign_loss)]
        let mut rng = {
            let now = Utc::now().timestamp_nanos_opt().unwrap();
            StdRng::seed_from_u64(now as u64)
        };

        for _i in 0..1000 {
            let mut keys = vec!["a", "b", "c"];
            keys.shuffle(&mut rng);

            let mut tags = Tags::with_capacity(4);
            for key in keys {
                tags.insert(key, 1);
            }

            assert_eq!(tags.len, 3);
            assert_eq!(tags.cap, 4);

            let sorted = tags
                .iter()
                .map(|(key, _value)| key.as_str().to_string())
                .collect::<Vec<_>>();
            assert_eq!(sorted, ["a", "b", "c"]);

            let mut keys = vec!["a", "b", "c"];
            keys.shuffle(&mut rng);
            for key in keys {
                tags.remove(key);
            }

            assert_eq!(tags.len, 0);
            assert_eq!(tags.cap, 4);
        }
    }

    #[test]
    fn clone() {
        let t1 = tags!(
            "foo" => "bar"
        );

        let mut t2 = t1.clone();
        assert_eq!(t1, t2);

        t2.remove("foo");
        assert_eq!(t1.len(), 1);
        assert_eq!(t2.len(), 0);
    }

    #[test]
    fn get() {
        let mut t1 = Tags::with_capacity(1);
        assert_eq!(t1.len, 0);
        assert_eq!(t1.cap, 1);

        t1.insert("foo", "bar");
        assert_eq!(t1.len, 1);
        assert_eq!(t1.cap, 1);

        let value = t1.get("foo").unwrap();
        assert_eq!(t1.len, 1);
        assert_eq!(t1.cap, 1);
        assert_eq!(value, &Value::from("bar"));
    }

    #[test]
    fn get_mut() {
        let mut tags = tags!(
            "foo" => "bar"
        );

        let value = tags.get_mut("foo").unwrap();
        *value = Value::I64(1);

        assert_eq!(tags, tags!("foo" => 1));
    }

    #[test]
    fn grow() {
        let mut t1 = Tags::with_capacity(1);
        t1.insert("foo", "bar");
        assert_eq!(t1.len, 1);
        assert_eq!(t1.cap, 1);

        t1.insert("bar", "foo");
        assert_eq!(t1.len, 2);
        assert_eq!(t1.cap, 4);
    }

    #[test]
    fn remove() {
        let keys = ["a", "b", "c"];
        for key in keys {
            let mut tags = Tags::with_capacity(4);
            for key in keys {
                tags.insert(key, 1);
            }

            assert_eq!(tags.len, 3);
            assert_eq!(tags.cap, 4);

            let value = tags.remove(key).unwrap();
            assert_eq!(tags.len, 2);
            assert_eq!(tags.cap, 4);
            assert_eq!(value, Value::from(1));
        }
    }

    #[test]
    fn remove_to_empty() {
        let mut tags = tags!(
            "foo" => "bar"
        );

        assert_eq!(tags.len(), 1);
        assert_eq!(tags.capacity(), 1);
        assert_eq!(tags.remove("foo"), Some(Value::String("bar".into())));
        assert!(tags.is_empty());
        assert_eq!(tags.len(), 0);
        assert_eq!(tags.capacity(), 1);
    }

    #[test]
    fn iter() {
        let tags = tags!(
            "a" => 1,
            "ab" => 3,
            "aa" => 2,
        );

        let keys = tags
            .into_iter()
            .map(|(key, _value)| key.to_string())
            .collect::<Vec<_>>();

        assert_eq!(keys, ["a", "aa", "ab"]);
    }

    #[test]
    fn retain_first() {
        let mut tags = tags!(
            "a" => "a",
            "b" => "b",
            "c" => "c"
        );

        tags.retain(|key, _value| key.as_str() != "a");

        assert_eq!(
            tags,
            tags!(
                "b" => "b",
                "c" => "c"
            )
        );
        assert_eq!(tags.len(), 2);
        assert_eq!(tags.capacity(), 3);
    }

    #[test]
    fn retain_middle() {
        let mut tags = tags!(
            "a" => "a",
            "b" => "b",
            "c" => "c"
        );

        tags.retain(|key, _value| key.as_str() != "b");

        assert_eq!(
            tags,
            tags!(
                "a" => "a",
                "c" => "c"
            )
        );
        assert_eq!(tags.len(), 2);
        assert_eq!(tags.capacity(), 3);
    }

    #[test]
    fn retain_last() {
        let mut tags = tags!(
            "a" => "a",
            "b" => "b",
            "c" => "c"
        );

        tags.retain(|key, _value| key.as_str() != "c");

        assert_eq!(
            tags,
            tags!(
                "a" => "a",
                "b" => "b"
            )
        );
        assert_eq!(tags.len(), 2);
        assert_eq!(tags.capacity(), 3);
    }

    #[test]
    fn into_iter() {
        let tags = tags!();
        let iter = tags.into_iter();
        assert_eq!(iter.pos, 0);
        drop(iter);

        let tags = tags!(
            "a" => "a",
            "b" => "b",
        );
        let mut iter = tags.into_iter();
        assert_eq!(iter.pos, 0);
        assert_eq!(iter.len, 2);

        let (key, value) = iter.next().unwrap();
        assert_eq!(iter.pos, 1);
        assert_eq!(iter.len, 2);
        assert_eq!(key.as_str(), "a");
        assert_eq!(value.to_string(), "a");
        drop(iter);
    }
}
