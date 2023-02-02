use schemars::schema::InstanceType;
use std::num::{
    NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI8, NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU8,
    NonZeroUsize,
};

use serde::Serialize;

#[derive(Copy, Clone, Serialize)]
pub enum NumberClass {
    Signed,

    Unsigned,

    Float,
}

impl NumberClass {
    /// Gets the equivalent instance type of this number class.
    pub fn as_instance_type(self) -> InstanceType {
        match self {
            Self::Signed | Self::Unsigned => InstanceType::Integer,
            Self::Float => InstanceType::Number,
        }
    }
}

/// A numeric type that can be represented correctly in a JSON Schema document.
pub trait ConfigurableNumber {
    /// The integral numeric type.
    ///
    /// We parameterize the "integral" numberic type in this way to allow generating
    /// the schema for wrapper type such as `NonZeroU64`, where the overall type must
    /// be represented as `NonZeroU64` but the integral numeric type that we're
    /// constraining against is `u64`.
    type Numeric;

    /// Gets the class of this numeric type.
    fn class() -> NumberClass;

    /// Whether or not this numeric type disallows nonzero values.
    fn is_nonzero() -> bool {
        false
    }

    /// Whether or not a generated schema for this numeric type must explicitly disallow zero values.
    ///
    /// In some cases, such as `NonZero*` types from `std::num`, a numeric type may not support zero values for reasons
    /// of correctness and/or optimization. In some cases, we can simply adjust the normal minimum/maximum bounds in the
    /// schema to encode this. In other cases, such as signed versions like `NonZeroI64`, zero is a discrete value
    /// within the minimum and maximum bounds and must be excluded explicitly.
    fn requires_nonzero_exclusion() -> bool {
        false
    }
}

macro_rules! impl_configurable_number {
	([$class:expr] $($ty:ty),+) => {
		$(
			impl ConfigurableNumber for $ty {
				type Numeric = $ty;

                fn class() -> NumberClass {
                    $class
                }
			}
		)+
	};
}

macro_rules! impl_configurable_number_nonzero {
	([$class:expr] $($aty:ty => $ity:ty),+) => {
		$(
			impl ConfigurableNumber for $aty {
				type Numeric = $ity;

				fn is_nonzero() -> bool {
					true
				}

                fn class() -> NumberClass {
                    $class
                }
			}
		)+
	};

	(with_exclusion, [$class:expr] $($aty:ty => $ity:ty),+) => {
		$(
			impl ConfigurableNumber for $aty {
				type Numeric = $ity;

				fn is_nonzero() -> bool {
					true
				}

				fn requires_nonzero_exclusion() -> bool {
					true
				}

                fn class() -> NumberClass {
                    $class
                }
			}
		)+
	};
}

impl_configurable_number!([NumberClass::Unsigned] u8, u16, u32, u64, usize);
impl_configurable_number!([NumberClass::Signed] i8, i16, i32, i64, isize);
impl_configurable_number!([NumberClass::Float] f32, f64);
impl_configurable_number_nonzero!([NumberClass::Unsigned] NonZeroU8 => u8, NonZeroU16 => u16, NonZeroU32 => u32, NonZeroU64 => u64, NonZeroUsize => usize);
impl_configurable_number_nonzero!(with_exclusion, [NumberClass::Signed] NonZeroI8 => i8, NonZeroI16 => i16, NonZeroI32 => i32, NonZeroI64 => i64);
