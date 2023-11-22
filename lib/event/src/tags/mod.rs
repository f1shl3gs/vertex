mod key;
mod value;

// re-export
pub use key::Key;
pub use value::{Array, Value};

use std::alloc::{alloc, dealloc, Layout};
use std::borrow::Cow;
use std::cmp::Ordering::{Greater, Less};
use std::collections::BTreeMap;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::ptr::{drop_in_place, slice_from_raw_parts_mut, NonNull};

use measurable::ByteSizeOf;
use serde::de::{MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

const GROWTH_ALIGNMENT: isize = 4;

#[allow(clippy::cast_sign_loss)]
#[inline]
fn aligned(amount: usize) -> usize {
    ((amount as isize + GROWTH_ALIGNMENT - 1) & -GROWTH_ALIGNMENT) as usize
}

#[derive(Clone, Debug, Hash, PartialEq)]
struct Entry {
    key: Key,
    value: Value,
}

struct Inner {
    len: usize,
    cap: usize,
    // Since we never allocate on heap unless our capacity is bigger than
    // capacity, and heap capacity cannot be less than 1.
    // Therefore, pointer cannot be null too.
    data: NonNull<Entry>,
}

impl Inner {
    #[inline]
    fn insert(&mut self, pos: usize, entry: Entry) {
        if self.len == self.cap {
            self.grow(aligned(self.cap + 1));
        }

        unsafe {
            let ptr = self.data.as_ptr().add(pos);
            if pos < self.len {
                std::ptr::copy(ptr, ptr.add(1), self.len - pos);
            }

            ptr.write(entry);
            self.len += 1;
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

            let cmp = key.cmp(sk.as_str());

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
}

impl Clone for Inner {
    fn clone(&self) -> Self {
        let layout = Layout::array::<Entry>(self.cap).unwrap();
        let data: NonNull<Entry> = unsafe {
            let data: NonNull<Entry> = NonNull::new_unchecked(alloc(layout)).cast();
            for i in 0..self.len {
                let from = (*self.data.as_ptr().add(i)).clone();
                data.as_ptr().add(i).write(from);
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

impl Drop for Inner {
    fn drop(&mut self) {
        unsafe {
            drop_in_place(slice_from_raw_parts_mut(self.data.as_ptr(), self.len));

            let layout = Layout::array::<Entry>(self.cap).expect("build layout");
            dealloc(self.data.as_ptr().cast(), layout);
        }
    }
}

impl PartialEq for Inner {
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

#[derive(Clone, PartialEq)]
#[repr(transparent)]
pub struct Tags(Cow<'static, Inner>);

unsafe impl Send for Tags {}
unsafe impl Sync for Tags {}

impl ByteSizeOf for Tags {
    fn allocated_bytes(&self) -> usize {
        self.len() * std::mem::size_of::<Entry>()
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

        self.into_iter().try_for_each(|(key, value)| {
            if first {
                first = false;
                f.write_fmt(format_args!("{}={}", key, value))
            } else {
                f.write_fmt(format_args!(",{}={}", key, value))
            }
        })
    }
}

impl Eq for Tags {}

impl Hash for Tags {
    fn hash<H: Hasher>(&self, state: &mut H) {
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
    inner: &'a Inner,
}

impl<'a> Iterator for Iter<'a> {
    type Item = (&'a Key, &'a Value);

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos == self.inner.len {
            return None;
        }

        unsafe {
            let entry = &(*self.inner.data.as_ptr().add(self.pos));
            self.pos += 1;
            Some((&entry.key, &entry.value))
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.inner.len - self.pos;
        (len, Some(len))
    }

    #[inline]
    fn count(self) -> usize
    where
        Self: Sized,
    {
        self.inner.len
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

impl Serialize for Tags {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.len()))?;
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
                while let Some((key, value)) = map.next_entry::<String, Value>()? {
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
        let data: NonNull<Entry> = unsafe { NonNull::new_unchecked(alloc(layout)).cast() };

        Self(Cow::Owned(Inner { len: 0, cap, data }))
    }

    /// Returns the length of the Tags.
    #[inline]
    pub fn len(&self) -> usize {
        self.0.len
    }

    /// Returns the number of elements the Tags can hold without reallocating.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.0.cap
    }

    /// Returns whether or not the `MiniVec` has a length greater than 0.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.len == 0
    }

    #[inline]
    pub fn iter(&self) -> Iter {
        Iter {
            pos: 0,
            inner: self.0.as_ref(),
        }
    }

    /// Inserts a key-value pair into the tags
    pub fn insert(&mut self, key: impl Into<Key> + AsRef<str>, value: impl Into<Value>) {
        match self.0.binary_search(key.as_ref()) {
            Ok(pos) => unsafe {
                let elt = self.0.data.as_ptr().add(pos);
                (*elt).value = value.into();
            },
            Err(pos) => self.0.to_mut().insert(
                pos,
                Entry {
                    key: key.into(),
                    value: value.into(),
                },
            ),
        }
    }

    pub fn try_insert(&mut self, key: impl Into<Key> + AsRef<str>, value: impl Into<Value>) {
        if let Err(pos) = self.0.binary_search(key.as_ref()) {
            self.0.to_mut().insert(
                pos,
                Entry {
                    key: key.into(),
                    value: value.into(),
                },
            );
        }
    }

    /// Removes and returns the element by key, shifting all elements after
    /// it to the left.
    pub fn remove(&mut self, key: impl AsRef<str>) -> Option<Value> {
        match self.0.binary_search(key.as_ref()) {
            Ok(pos) => unsafe {
                let inner = self.0.to_mut();
                let ptr = inner.data.as_ptr().add(pos);
                let entry = std::ptr::read(ptr);
                if pos < inner.len - 1 {
                    std::ptr::copy(ptr.add(1), ptr, inner.len - pos - 1);
                }

                inner.len -= 1;
                Some(entry.value)
            },
            // not found
            Err(_pos) => None,
        }
    }

    /// Returns a reference to the value corresponding to the key.
    pub fn get(&self, key: impl AsRef<str>) -> Option<&Value> {
        if let Ok(pos) = self.0.binary_search(key.as_ref()) {
            unsafe {
                let ptr = self.0.data.as_ptr().add(pos);
                Some(&(*ptr).value)
            }
        } else {
            None
        }
    }

    /// Returns a mutable reference to the value corresponding to the key.
    pub fn get_mut(&mut self, key: impl AsRef<str>) -> Option<&mut Value> {
        if let Ok(pos) = self.0.binary_search(key.as_ref()) {
            unsafe {
                let entry = self.0.data.as_ptr().add(pos);
                Some(&mut (*entry).value)
            }
        } else {
            None
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
    use rand::rngs::StdRng;
    use rand::seq::SliceRandom;
    use rand::SeedableRng;

    #[test]
    fn align() {
        for (input, want) in [
            (1, 4),
            (2, 4),
            (3, 4),
            (4, 4),
            (5, 8),
            (6, 8),
            (7, 8),
            (8, 8),
            (9, 12),
            (127, 128),
        ] {
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

            assert_eq!(tags.0.len, 3);
            assert_eq!(tags.0.cap, 4);

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

            assert_eq!(tags.0.len, 0);
            assert_eq!(tags.0.cap, 4);
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
        assert_eq!(t1.0.len, 0);
        assert_eq!(t1.0.cap, 1);

        t1.insert("foo", "bar");
        assert_eq!(t1.0.len, 1);
        assert_eq!(t1.0.cap, 1);

        let value = t1.get("foo").unwrap();
        assert_eq!(t1.0.len, 1);
        assert_eq!(t1.0.cap, 1);
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
        assert_eq!(t1.0.len, 1);
        assert_eq!(t1.0.cap, 1);

        t1.insert("bar", "foo");
        assert_eq!(t1.0.len, 2);
        assert_eq!(t1.0.cap, 4);
    }

    #[test]
    fn remove() {
        let keys = ["a", "b", "c"];
        for key in keys {
            let mut tags = Tags::with_capacity(4);
            for key in keys {
                tags.insert(key, 1);
            }

            assert_eq!(tags.0.len, 3);
            assert_eq!(tags.0.cap, 4);

            let value = tags.remove(key).unwrap();
            assert_eq!(tags.0.len, 2);
            assert_eq!(tags.0.cap, 4);
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
}
