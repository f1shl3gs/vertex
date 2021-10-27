
#[macro_export]
macro_rules! tags {
    ( $($x:expr => $y:expr),* ) => ({
        let mut _map: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
        $(
            _map.insert($x.into(), $y.into());
        )*
        _map
    });
    ( $($x:expr => $y:expr,)* ) => (
        tags!{$($x => $y),*}
    );
}

#[macro_export]
macro_rules! fields {
    ( $($x:expr => $y:expr),* ) => ({
        let mut _map: std::collections::BTreeMap<String, event::Value> = std::collections::BTreeMap::new();
        $(
            _map.insert($x.into(), $y.into());
        )*
        _map
    });
    ( $($x:expr => $y:expr,)* ) => (
        tags!{$($x => $y),*}
    );
}


#[macro_export]
macro_rules! gauge_metric {
    ($name: expr, $desc: expr, $value: expr, $( $k: expr => $v: expr),* ) => {
        Metric{
            name: $name.into(),
            description: Some($desc.into()),
            tags: tags!(
                $($k => $v,)*
            ),
            unit: None,
            timestamp: None,
            value: event::MetricValue::Gauge($value)
        }
    };
    ($name: expr, $desc: expr, $value: expr) => {
        Metric{
            name: $name.into(),
            description: Some($desc.into()),
            tags: Default::default(),
            unit: None,
            timestamp: None,
            value: event::MetricValue::Gauge($value)
        }
    };
}

#[macro_export]
macro_rules! sum_metric {
    ($name: expr, $desc: expr, $value: expr, $( $k: expr => $v: expr),* ) => {
        Metric{
            name: $name.into(),
            description: Some($desc.into()),
            tags: tags!(
                $($k => $v,)*
            ),
            unit: None,
            timestamp: None,
            value: event::MetricValue::Sum($value.into())
        }
    };

    ($name: expr, $desc: expr, $value: expr) => {
        Metric{
            name: $name.into(),
            description: Some($desc.into()),
            tags: Default::default(),
            unit: None,
            timestamp: None,
            value: event::MetricValue::Sum($value)
        }
    };
}