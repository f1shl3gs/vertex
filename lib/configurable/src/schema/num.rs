use std::num::{
    NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI8, NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU8,
    NonZeroUsize,
};

use super::InstanceType;

pub const NUMERIC_ENFORCED_LOWER_BOUND: f64 = -9_007_199_254_740_991.0;
pub const NUMERIC_ENFORCED_UPPER_BOUND: f64 = 9_007_199_254_740_991.0;
// pub const ERR_NUMERIC_OUT_OF_RANGE: &str = "range bounds must be within -(2^53 - 1) and 2^53 - 1";

/// The class of a number type.
#[derive(Copy, Clone)]
pub enum NumberClass {
    /// A signed integer.
    Signed,

    /// An unsigned integer.
    Unsigned,

    /// A floating-point number.
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

    /// Whether or not a generated schema for this numeric type must explicitly
    /// disallow zero values.
    ///
    /// In some cases, such as `NonZero*` types from `std::num`, a numeric type
    /// may not support zero values for reasons of correctness and/or optimization.
    /// In some cases, we can simply adjust the normal minimum/maximum bounds in
    /// the schema to encode this. In other cases, such as signed versions like
    /// `NonZeroI64`, zero is a discrete value within the minimum and maximum
    /// bounds and must be excluded explicitly.
    fn requires_nonzero_exclusion() -> bool {
        false
    }

    /// Gets the minimum bound for this numeric type, limited by the representable
    /// range in JSON Schema.
    fn get_enforced_min_bound() -> f64;

    /// Gets the maximum bound for this numeric type, limited by the representable
    /// range in JSON Schema.
    fn get_enforced_max_bound() -> f64;
}

trait ToF64 {
    fn to_f64(&self) -> Option<f64>;
}

macro_rules! impl_to_f64_unchecked {
    ($($ty:ty),+) => {
        $(
            impl ToF64 for $ty {
                fn to_f64(&self) -> Option<f64> {
                    Some(*self as f64)
                }
            }
        )+
    }
}

macro_rules! impl_signed_to_f64_checked {
    ($($ty:ty),+) => {
        $(
            impl ToF64 for $ty {
                fn to_f64(&self) -> Option<f64> {
                    if f64::MIN as $ty > *self {
                        return None;
                    }

                    if f64::MAX as $ty < *self {
                        return None;
                    }

                    Some(*self as f64)
                }
            }
        )+
    };
}

macro_rules! impl_unsigned_to_f64_checked {
    ($($ty:ty),+) => {
        $(
            impl ToF64 for $ty {
                fn to_f64(&self) -> Option<f64> {
                    if f64::MAX as $ty < *self {
                        None
                    } else {
                        Some(*self as f64)
                    }
                }
            }
        )+
    };
}

impl_to_f64_unchecked!(i8, u8, i16, u16, i32, u32, f32, f64);
impl_signed_to_f64_checked!(i64, isize);
impl_unsigned_to_f64_checked!(u64, usize);

macro_rules! impl_bounded {
    ($ty:ty) => {
        fn get_enforced_min_bound() -> f64 {
            let mechanical_minimum = match (Self::is_nonzero(), Self::requires_nonzero_exclusion())
            {
                (false, _) | (true, true) => <$ty>::MIN,
                (true, false) => 1 as $ty,
            };

            let enforced_minimum = NUMERIC_ENFORCED_LOWER_BOUND;
            let mechanical_minimum = mechanical_minimum.to_f64().expect(
                "`Configurable` does not support numbers larger than an `f64` representation",
            );

            if mechanical_minimum < enforced_minimum {
                enforced_minimum
            } else {
                mechanical_minimum
            }
        }

        fn get_enforced_max_bound() -> f64 {
            let enforced_maximum = NUMERIC_ENFORCED_UPPER_BOUND;
            let mechanical_maximum = <$ty>::MAX.to_f64().expect(
                "`Configurable` does not support numbers larger than an `f64` representation",
            );

            if mechanical_maximum > enforced_maximum {
                enforced_maximum
            } else {
                mechanical_maximum
            }
        }
    };
}

macro_rules! impl_configurable_number {
	([$class:expr] $($ty:ty),+) => {
		$(
			impl ConfigurableNumber for $ty {
				type Numeric = $ty;

                fn class() -> NumberClass {
                    $class
                }

                impl_bounded! { $ty }
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

                impl_bounded! { $ity }
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

                impl_bounded! { $ity }
			}
		)+
	};
}

impl_configurable_number!([NumberClass::Unsigned] u8, u16, u32, u64, usize);
impl_configurable_number!([NumberClass::Signed] i8, i16, i32, i64, isize);
impl_configurable_number!([NumberClass::Float] f32, f64);
impl_configurable_number_nonzero!([NumberClass::Unsigned] NonZeroU8 => u8, NonZeroU16 => u16, NonZeroU32 => u32, NonZeroU64 => u64, NonZeroUsize => usize);
impl_configurable_number_nonzero!(with_exclusion, [NumberClass::Signed] NonZeroI8 => i8, NonZeroI16 => i16, NonZeroI32 => i32, NonZeroI64 => i64);
