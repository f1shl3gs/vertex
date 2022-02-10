#[macro_export]
macro_rules! tags {
    // Done without trailing comma
    ( $($x:expr => $y:expr),* ) => ({
        let mut _map: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
        $(
            _map.insert($x.into(), $y.into());
        )*
        _map
    });
    // Done with trailing comma
    ( $($x:expr => $y:expr,)* ) => (
        tags!{$($x => $y),*}
    );
}

#[macro_export]
macro_rules! fields {
    ( $($x:expr => $y:expr),* ) => ({
        let mut _map: std::collections::BTreeMap<String, $crate::Value> = std::collections::BTreeMap::new();
        $(
            _map.insert($x.into(), $y.into());
        )*
        _map
    });
    // Done with trailing comma
    ( $($x:expr => $y:expr,)* ) => (
        fields!{$($x => $y),*}
    );
}

#[macro_export]
macro_rules! buckets {
    ( $( $limit:expr => $count:expr),* ) => {
        vec![
            $( event::Bucket { upper: $limit, count: $count}, )*
        ]
    };
}

#[macro_export]
macro_rules! quantiles {
    ( $( $q:expr => $value:expr),* ) => {
        vec![
            $( event::Quantile { quantile: $q, value: $value }, )*
        ]
    };
}

/// A related trait to `PartialEq`, `EventDataEq` tests if two events
/// contain the same data, exclusive of the metadata. This is used to
/// test for events having the same values but potentially different
/// parts of the metadata that not fixed between runs, without removing
/// the ability to compare them for exact equality.
pub trait EventDataEq<Rhs: ?Sized = Self> {
    fn event_data_eq(&self, other: &Rhs) -> bool;
}

impl<T: EventDataEq> EventDataEq for &[T] {
    fn event_data_eq(&self, other: &Self) -> bool {
        self.len() == other.len()
            && self
                .iter()
                .zip(other.iter())
                .all(|(a, b)| a.event_data_eq(b))
    }
}

impl<T: EventDataEq> EventDataEq for Vec<T> {
    fn event_data_eq(&self, other: &Self) -> bool {
        self.as_slice().event_data_eq(&other.as_slice())
    }
}

#[macro_export]
macro_rules! assert_event_data_eq {
    ($left:expr, $right:expr, $message:expr) => {{
        use $crate::EventDataEq as _;
        match (&($left), &($right)) {
            (left, right) => {
                if !left.event_data_eq(right) {
                    panic!(
                        "assertion failed: {}\n\n{}\n",
                        $message,
                        pretty_assertions::Comparison::new(left, right)
                    );
                }
            }
        }
    }};
    ($left:expr, $right:expr,) => {
        $crate::assert_event_data_eq!($left, $right)
    };
    ($left:expr, $right:expr) => {
        $crate::assert_event_data_eq!($left, $right, "`left.event_data_eq(right)`")
    };
}
