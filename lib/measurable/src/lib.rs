mod external;

use std::collections::BTreeMap;

use bytes::{Bytes, BytesMut};

pub trait ByteSizeOf {
    /// Returns the in-memory size of this type
    ///
    /// This function returns the total number of bytes that
    /// [`std::mem::size_of`] does in addition to any interior
    /// allocated bytes. It default implementation is `std::mem::size_of`
    /// + `ByteSizeOf::allocated_bytes`
    fn size_of(&self) -> usize {
        std::mem::size_of_val(self) + self.allocated_bytes()
    }

    /// Returns the allocated bytes of this type
    fn allocated_bytes(&self) -> usize;
}

macro_rules! impl_byte_size_of_for_num {
    ($typ:ident) => {
        impl ByteSizeOf for $typ {
            fn allocated_bytes(&self) -> usize {
                0
            }
        }
    };
}

impl_byte_size_of_for_num!(u8);
impl_byte_size_of_for_num!(u16);
impl_byte_size_of_for_num!(u32);
impl_byte_size_of_for_num!(u64);
impl_byte_size_of_for_num!(u128);
impl_byte_size_of_for_num!(i8);
impl_byte_size_of_for_num!(i16);
impl_byte_size_of_for_num!(i32);
impl_byte_size_of_for_num!(i64);
impl_byte_size_of_for_num!(i128);
impl_byte_size_of_for_num!(f32);
impl_byte_size_of_for_num!(f64);

impl ByteSizeOf for String {
    fn allocated_bytes(&self) -> usize {
        self.len()
    }
}

impl<K, V> ByteSizeOf for BTreeMap<K, V>
where
    K: ByteSizeOf,
    V: ByteSizeOf,
{
    fn allocated_bytes(&self) -> usize {
        self.iter()
            .fold(0, |acc, (k, v)| acc + k.size_of() + v.size_of())
    }
}

impl<T> ByteSizeOf for &[T]
where
    T: ByteSizeOf,
{
    fn allocated_bytes(&self) -> usize {
        self.iter().map(ByteSizeOf::size_of).sum()
    }
}

impl<T> ByteSizeOf for Vec<T>
where
    T: ByteSizeOf,
{
    fn allocated_bytes(&self) -> usize {
        self.iter().fold(0, |acc, i| acc + i.size_of())
    }
}

impl<T> ByteSizeOf for Option<T>
where
    T: ByteSizeOf,
{
    fn allocated_bytes(&self) -> usize {
        self.as_ref().map_or(0, ByteSizeOf::allocated_bytes)
    }
}

impl ByteSizeOf for Bytes {
    fn allocated_bytes(&self) -> usize {
        self.len()
    }
}

impl ByteSizeOf for BytesMut {
    fn allocated_bytes(&self) -> usize {
        self.len()
    }
}
