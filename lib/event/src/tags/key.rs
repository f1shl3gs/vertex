use std::alloc::{Layout, alloc};
use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::mem::transmute;
use std::ops::Deref;
use std::ptr::{NonNull, copy_nonoverlapping};
use std::slice::from_raw_parts;

use typesize::TypeSize;

const MAX_SIZE: usize = size_of::<String>(); // 24
const MAX_LENGTH: usize = 128;
const INLINE_CAP: usize = 23;

const HEAP_MASK: u8 = 216;
const STATIC_MASK: u8 = 217;
const LENGTH_MASK: u8 = 0b1100_0000;

/// 1. String memory layout
///    pointer(8b) + capacity(8b) + len(8b)
///
/// 2. Static string
///    pointer(8b) + len(8b)
#[repr(C)]
pub struct Key {
    cap: usize,
    data: NonNull<u8>,
    len: u32,
    padding1: u16,
    padding2: u8,

    // Heap: the last u8 is always 0, cause the key's length is
    // limited to 128.
    //
    // Static str: last | TYPE_MASK == true
    //
    // Inline: length of inline
    last: u8,
}

impl Key {
    /// Create a `Key` from &'static str.
    #[inline]
    pub const fn from_static(s: &'static str) -> Key {
        Key {
            data: unsafe { NonNull::new_unchecked(s.as_ptr().cast_mut()) },
            cap: 0,
            len: s.len() as u32,
            padding1: 0,
            padding2: 0,
            last: STATIC_MASK,
        }
    }

    #[inline]
    pub fn from_string(s: String) -> Key {
        let mut key = unsafe { transmute::<String, Key>(s) };
        key.last = HEAP_MASK;

        key

        // if s.len() <= INLINE_CAP {
        //     Key::inline(&s)
        // } else {
        //     unsafe { transmute::<String, Key>(s) }
        // }
    }

    fn inline(text: &str) -> Key {
        let len = text.len();
        let key = Key {
            cap: 0,
            data: NonNull::dangling(),
            len: 0,
            padding1: 0,
            padding2: 0,
            last: len as u8 | LENGTH_MASK,
        };

        unsafe {
            copy_nonoverlapping(text.as_ptr(), key.as_ptr().cast_mut(), len);
        }

        key
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        let slice = self.as_bytes();

        unsafe { std::str::from_utf8_unchecked(slice) }
    }

    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        let (ptr, len) = if self.last >= HEAP_MASK {
            // String or static str
            (self.data.as_ptr().cast_const(), self.len as usize)
        } else {
            // inline
            (
                (self as *const Self).cast::<u8>(),
                (self.last - LENGTH_MASK) as usize,
            )
        };

        unsafe { from_raw_parts(ptr, len) }
    }
}

impl Clone for Key {
    fn clone(&self) -> Self {
        unsafe {
            // There are only two cases we need to care about: If the string is
            // allocated on the heap or not. If it is, then the data must be cloned
            // properly, otherwise we can simply copy the `Key`.
            if self.last == HEAP_MASK {
                if self.len <= INLINE_CAP as u32 {
                    let mut key = [0u8; MAX_SIZE];
                    key[MAX_SIZE - 1] = self.len as u8 | LENGTH_MASK;
                    copy_nonoverlapping(self.data.as_ref(), key.as_mut_ptr(), self.len as usize);

                    return transmute::<[u8; MAX_SIZE], Key>(key);
                }

                let layout = Layout::array::<u8>(self.len as usize).expect("valid layout");
                let data = alloc(layout);
                copy_nonoverlapping(self.data.as_ptr(), data, self.len as usize);

                Key {
                    data: NonNull::new_unchecked(data),
                    cap: self.cap,
                    len: self.len,
                    padding1: 0,
                    padding2: 0,
                    last: HEAP_MASK,
                }
            } else {
                // SAFETY: We just checked that `self` can be copied because it is an
                // inline string or a reference to a `&'static str`.
                std::ptr::read(self)
            }
        }
    }
}

impl Drop for Key {
    fn drop(&mut self) {
        if self.last == HEAP_MASK && self.cap != 0 {
            unsafe {
                let layout = Layout::array::<u8>(self.cap).expect("valid capacity");
                std::alloc::dealloc(self.data.as_ptr(), layout);
            }
        }
    }
}

impl fmt::Debug for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_str().fmt(f)
    }
}

impl AsRef<str> for Key {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Deref for Key {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl Eq for Key {}

impl From<&str> for Key {
    fn from(v: &str) -> Self {
        let len = v.len();
        if len <= INLINE_CAP {
            let mut data = [0u8; MAX_SIZE];

            return unsafe {
                copy_nonoverlapping(v.as_ptr(), data.as_mut_ptr(), len);
                data[MAX_SIZE - 1] = len as u8 | LENGTH_MASK;
                transmute::<[u8; 24], Key>(data)
            };
        }

        Key::from_string(v.to_string())
    }
}

impl From<&String> for Key {
    #[inline]
    fn from(v: &String) -> Self {
        if v.len() <= INLINE_CAP {
            Key::inline(v)
        } else {
            Key::from_string(v.to_string())
        }
    }
}

impl From<String> for Key {
    fn from(value: String) -> Self {
        Key::from_string(value)
    }
}

impl From<Key> for String {
    fn from(key: Key) -> Self {
        if key.last == HEAP_MASK {
            return unsafe { transmute::<Key, String>(key) };
        }

        key.as_str().to_string()
    }
}

impl Hash for Key {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.deref().hash(state);
    }
}

impl Ord for Key {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl PartialEq for Key {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.as_str().eq(other.as_str())
    }
}

impl PartialOrd for Key {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

unsafe impl Send for Key {}
unsafe impl Sync for Key {}

impl TypeSize for Key {
    #[inline]
    fn allocated_bytes(&self) -> usize {
        if self.last == 0 { self.cap } else { 0 }
    }
}

mod serde {
    use std::fmt::Formatter;
    use std::mem::transmute;

    use serde::de::{Error, Visitor};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use super::{INLINE_CAP, Key, MAX_LENGTH};

    impl<'de> Deserialize<'de> for Key {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            struct KeyVisitor;

            impl Visitor<'_> for KeyVisitor {
                type Value = Key;

                fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                    formatter.write_str("a string")
                }

                fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                where
                    E: Error,
                {
                    let len = v.len();
                    if len == 0 || len > MAX_LENGTH {
                        return Err(Error::custom(
                            "key length should be large than 0 and less than 128",
                        ));
                    }

                    let key = if len <= INLINE_CAP {
                        Key::inline(v)
                    } else {
                        unsafe { transmute::<String, Key>(v.to_string()) }
                    };

                    Ok(key)
                }
            }

            deserializer.deserialize_str(KeyVisitor)
        }
    }

    impl Serialize for Key {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            serializer.serialize_str(self.as_str())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::mem::size_of;

    use super::*;

    #[test]
    fn size() {
        assert_eq!(size_of::<String>(), size_of::<Key>());
        assert_eq!(size_of::<String>(), size_of::<Option<Key>>());
    }

    #[test]
    fn static_string() {
        for input in [
            "",
            "foo",
            "abcdefghijklmnopqrstuvw",
            "abcdefghijklmnopqrstuvwxyz",
        ] {
            let key = Key::from_static(input);
            let k1 = key.clone();
            assert_eq!(key.as_str(), input);
            assert_eq!(k1.as_str(), input);
        }
    }

    #[test]
    fn heap_string() {
        for input in [
            "",
            "foo",
            "abcdefghijklmnopqrstuvw",
            "abcdefghijklmnopqrstuvwxyz",
        ] {
            let key = Key::from_string(input.to_string());
            let k1 = key.clone();
            assert_eq!(key.as_str(), input);
            assert_eq!(k1.as_str(), input);
            drop(key);
            drop(k1);
        }
    }

    #[test]
    fn inline_string() {
        for input in ["", "foo", "abcdefghijklmnopqrstuvw"] {
            let key = Key::inline(input);
            let key1 = key.clone();
            assert_eq!(key.as_str(), input);
            assert_eq!(key1.as_str(), input);
            drop(key1);
            drop(key);
        }
    }

    #[test]
    fn deserialize() {
        #[derive(::serde::Deserialize)]
        struct Foo {
            key: Key,
        }

        let want = Key::from_static("foo");
        let got: Foo = serde_json::from_str(r#"{"key": "foo"}"#).unwrap();
        assert_eq!(want, got.key);
    }
}
