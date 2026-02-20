use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};

/// A trait to fetch an accurate estimate of the total memory usage of a value.
pub trait TypeSize: Sized {
    /// Return the in-memory size of this type.
    ///
    /// This function returns the total number of bytes that [`std::mem::size_of`]
    /// does in addition to any interior allocated bytes. Its default implementation
    /// is `std::mem::size_of` + `TypeSize::allocated_bytes`
    fn size_of(&self) -> usize {
        size_of::<Self>() + self.allocated_bytes()
    }

    /// Returns the allocated bytes of this type.
    ///
    /// This function returns the total number of bytes that have been allocated
    /// interior to this type instance. It does not include any bytes that are
    /// captured by [`std::mem::size_of`] except for any bytes that are interior
    /// to this type, For instance, `BTreeMap<String, Vec<u8>>` would count all
    /// bytes for `String` and `Vec<u8>` instances but not the exterior bytes
    /// for `BTreeMap`.
    fn allocated_bytes(&self) -> usize;
}

macro_rules! type_size {
    ($($typ:ty),+) => {
        $(
            impl TypeSize for $typ {
                #[inline]
                fn allocated_bytes(&self) -> usize {
                    0
                }
            }
        )*
    };
}

type_size!(u64, i64);

impl TypeSize for String {
    fn allocated_bytes(&self) -> usize {
        self.len()
    }
}

impl TypeSize for Cow<'static, str> {
    fn allocated_bytes(&self) -> usize {
        match self {
            Cow::Borrowed(_) => 0,
            Cow::Owned(s) => s.len(),
        }
    }
}

impl<T: TypeSize> TypeSize for Option<T> {
    fn allocated_bytes(&self) -> usize {
        match self {
            None => 0,
            Some(v) => v.size_of(),
        }
    }
}

impl<K, V> TypeSize for BTreeMap<K, V>
where
    K: TypeSize,
    V: TypeSize,
{
    fn allocated_bytes(&self) -> usize {
        self.iter()
            .fold(0, |acc, (key, value)| acc + key.size_of() + value.size_of())
    }
}

impl<K, V> TypeSize for HashMap<K, V>
where
    K: TypeSize,
    V: TypeSize,
{
    fn allocated_bytes(&self) -> usize {
        self.iter()
            .fold(0, |acc, (key, value)| acc + key.size_of() + value.size_of())
    }
}

impl<T: TypeSize> TypeSize for Vec<T> {
    #[inline]
    fn allocated_bytes(&self) -> usize {
        self.iter().fold(0, |acc, item| acc + item.size_of())
    }
}

impl TypeSize for bytes::Bytes {
    #[inline]
    fn allocated_bytes(&self) -> usize {
        self.len()
    }
}

impl TypeSize for bytes::BytesMut {
    #[inline]
    fn allocated_bytes(&self) -> usize {
        self.len()
    }
}
