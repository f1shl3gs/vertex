use std::collections::BTreeMap;
use std::iter::Peekable;

use crate::{
    Error, GroupKey, GroupKind, HistogramBucket, HistogramMetric, MetricGroup, MetricMap,
    SimpleMetric, SummaryMetric, SummaryQuantile,
};

/// Parsing prometheus's simple text-based exposition format
///
/// https://prometheus.io/docs/instrumenting/exposition_formats/#text-based-format
pub fn parse_text(input: &str) -> Result<Vec<MetricGroup>, Error> {
    let mut groups = vec![];
    let mut lines = input.lines().filter(|line| !line.is_empty()).peekable();

    loop {
        // parse help
        let description = match lines.next() {
            Some(line) => {
                if let Some(help) = line.strip_prefix("# HELP ") {
                    help
                } else {
                    // invalid HELP line, ignore it
                    continue;
                }
            }
            None => return Ok(groups),
        };

        // parse type line
        let (name, kind) = match lines.next() {
            Some(line) => {
                if let Some(value) = line.strip_prefix("# TYPE ") {
                    if let Some((name, kind)) = value.split_once(' ') {
                        (name, kind)
                    } else {
                        // invalid TYPE line, ignore it
                        continue;
                    }
                } else {
                    // invalid TYPE line, ignore it
                    continue;
                }
            }
            None => return Ok(groups),
        };

        let metrics = match kind {
            "counter" => {
                let group = parse_simple_metrics(name, &mut lines)?;
                GroupKind::Counter(group)
            }
            "gauge" => {
                let group = parse_simple_metrics(name, &mut lines)?;
                GroupKind::Gauge(group)
            }
            "histogram" => {
                let group = parse_histograms(&mut lines)?;
                GroupKind::Histogram(group)
            }
            "summary" => {
                let group = parse_summaries(&mut lines)?;
                GroupKind::Summary(group)
            }
            "untyped" => {
                let group = parse_simple_metrics(name, &mut lines)?;
                GroupKind::Untyped(group)
            }
            _ => return Err(Error::InvalidType(kind.to_string())),
        };

        groups.push(MetricGroup {
            name: name.to_string(),
            description: description
                .strip_prefix(name)
                .unwrap_or(description)
                .trim()
                .to_string(),
            metrics,
        })
    }
}

fn parse_simple_metrics<'a, I>(
    prefix: &str,
    lines: &mut Peekable<I>,
) -> Result<MetricMap<SimpleMetric>, Error>
where
    I: Iterator<Item = &'a str>,
{
    let mut group = MetricMap::<SimpleMetric>::new();

    loop {
        if let Some(line) = lines.peek() {
            if line.starts_with('#') {
                break;
            }
        } else {
            return Ok(group);
        }

        // The next line already peeked, so error should not happen.
        let line = lines.next().unwrap();
        match line.strip_prefix(prefix) {
            Some(stripped) => {
                let (gk, value) = parse_labels(stripped)?;
                group.insert(gk, value.into());
            }
            None => return Err(Error::InvalidMetric(line.into())),
        }
    }

    Ok(group)
}

fn parse_histograms<'a, I>(lines: &mut Peekable<I>) -> Result<MetricMap<HistogramMetric>, Error>
where
    I: Iterator<Item = &'a str>,
{
    let mut group = MetricMap::<HistogramMetric>::new();

    let mut hm = HistogramMetric::default();
    let mut current = GroupKey {
        timestamp: None,
        labels: Default::default(),
    };

    while let Some(line) = lines.peek() {
        if line.starts_with('#') {
            break;
        }

        // The next line already peeked, so error should not happened.
        let line = lines.next().unwrap();

        let (metric_name, mut gk, value) = parse_metric(line)?;
        if metric_name.ends_with("_bucket") {
            if let Some(s) = gk.labels.remove("le") {
                if gk.labels != current.labels {
                    group.insert(gk.clone(), hm);

                    current = gk;
                    hm = HistogramMetric::default();
                }

                let bucket = s.parse::<f64>().map_err(|err| Error::InvalidBucket {
                    line: line.into(),
                    err,
                })?;

                hm.buckets.push(HistogramBucket {
                    bucket,
                    count: value as u64,
                })
            } else {
                return Err(Error::MissingBucket(line.into()));
            }
        } else if metric_name.ends_with("_sum") {
            hm.sum = value;
        } else if metric_name.ends_with("_count") {
            hm.count = value as u32;
        } else {
            return Err(Error::InvalidMetric(line.into()));
        }
    }

    group.insert(current, hm);

    Ok(group)
}

fn parse_summaries<'a, I>(lines: &mut Peekable<I>) -> Result<MetricMap<SummaryMetric>, Error>
where
    I: Iterator<Item = &'a str>,
{
    let mut group = MetricMap::<SummaryMetric>::new();

    let mut sm = SummaryMetric::default();
    let mut labels = Default::default();

    loop {
        if let Some(line) = lines.peek() {
            if line.starts_with('#') {
                break;
            }
        } else {
            return Ok(group);
        }

        // The next line already peeked, so error should not happened.
        let line = lines.next().unwrap();

        let (metric_name, mut gk, value) = parse_metric(line)?;
        if metric_name.ends_with("_sum") {
            sm.sum = value;
        } else if metric_name.ends_with("_count") {
            sm.count = value as u32;
            group.insert(gk, sm);
            sm = SummaryMetric::default();
        } else {
            if let Some(s) = gk.labels.remove("quantile") {
                let quantile = s.parse::<f64>().map_err(|err| Error::InvalidQuantile {
                    line: line.to_string(),
                    err,
                })?;

                sm.quantiles.push(SummaryQuantile { quantile, value })
            } else {
                return Err(Error::MissingQuantile(line.into()));
            }

            if gk.labels != labels {
                group.insert(gk.clone(), sm);

                labels = gk.labels;
                sm = SummaryMetric::default();
            }
        }
    }

    Ok(group)
}

fn parse_labels(line: &str) -> Result<(GroupKey, f64), Error> {
    let length = line.len();

    if line.starts_with(' ') {
        // no labels
        let mut parts = line.split_whitespace();
        let value = parts.next().ok_or(Error::MissingValue)?.parse::<f64>()?;

        let timestamp = if let Some(value) = parts.next() {
            Some(value.parse::<i64>().map_err(Error::InvalidTimestamp)?)
        } else {
            None
        };

        return Ok((
            GroupKey {
                timestamp,
                labels: Default::default(),
            },
            value,
        ));
    }

    if line.starts_with('{') {
        // got some labels
        let mut pos = 1; // skip '{'
        let buf = line.as_bytes();
        let mut labels = BTreeMap::new();

        loop {
            // parse key
            let key_start = pos;
            while pos < length {
                let c = buf[pos];
                if c.is_ascii_alphanumeric() || c == b'_' {
                    pos += 1;
                    continue;
                }

                if c != b'=' {
                    return Err(Error::InvalidMetric(line.to_string()));
                }

                break;
            }

            let key_end = pos;
            pos += 2; // skip '="'
            if pos >= length {
                return Err(Error::InvalidMetric(line.into()));
            }

            let value_start = pos;
            while pos < length {
                let c = buf[pos];
                if c == b'\\' {
                    pos += 2; // skip '\' and next one which is escaped
                    if pos >= length {
                        return Err(Error::InvalidMetric(line.into()));
                    }

                    continue;
                }

                if c == b'"' {
                    break;
                }

                pos += 1;
            }

            let key = String::from_utf8_lossy(&buf[key_start..key_end]);
            let value = String::from_utf8_lossy(&buf[value_start..pos]);
            labels.insert(key.to_string(), value.to_string());

            if pos == length {
                break;
            }

            pos += 1; // skip '"'
            if buf[pos] == b'}' {
                pos += 1;
                break;
            }

            if buf[pos] != b',' {
                return Err(Error::InvalidMetric(line.into()));
            }

            // skip comma
            pos += 1;
        }

        let line = &line[pos..];

        let mut parts = line.split_whitespace();
        let value = parts.next().ok_or(Error::MissingValue)?.parse::<f64>()?;

        let timestamp = if let Some(value) = parts.next() {
            Some(value.parse::<i64>().map_err(Error::InvalidTimestamp)?)
        } else {
            None
        };

        return Ok((GroupKey { timestamp, labels }, value));
    }

    Err(Error::InvalidMetric(line.into()))
}

fn parse_metric(line: &str) -> Result<(&str, GroupKey, f64), Error> {
    let length = line.len();
    let buf = line.as_bytes();
    let mut pos = 0;

    // 1. Take metric name
    while pos < length {
        let c = buf[pos];
        if c.is_ascii_alphanumeric() || c == b'_' || c == b':' {
            pos += 1;
            continue;
        }

        if c != b'{' && c != b' ' {
            return Err(Error::InvalidMetric(line.into()));
        }

        break;
    }

    let name = &line[..pos];
    let line = &line[pos..];

    let (gk, value) = parse_labels(line)?;
    Ok((name, gk, value))
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! btreemap {
        () => (::std::collections::BTreeMap::new());

        // trailing comma case
        ($($key:expr => $value:expr,)+) => (btreemap!($($key => $value),+));

        ($($key:expr => $value:expr),*) => {
            ::std::collections::BTreeMap::from([
                $(
                    ($key.into(), $value.into()),
                )*
            ])
        };
    }

    macro_rules! labels {
        () => { BTreeMap::new() };
        ($($name:ident => $value:literal), *) => {{
            let mut result = BTreeMap::< String, String>::new();
            $ (result.insert(stringify ! ( $ name).into(), $ value.to_string()); ) *
            result
        }};
    }

    macro_rules! match_group {
        ($group: expr, $name: literal, $kind:ident => $inner:expr) => {{
            assert_eq!($group.name, $name);
            let inner = $inner;
            match &$group.metrics {
                GroupKind::$kind(metrics) => inner(metrics),
                _ => panic!("Invalid metric group type"),
            }
        }};
    }

    macro_rules! simple_metric {
        ($timestamp:expr, $labels:expr, $value:expr) => {
            (
                &GroupKey {
                    timestamp: $timestamp,
                    labels: $labels,
                },
                &SimpleMetric { value: $value },
            )
        };
    }

    #[test]
    fn parse_metrics() {
        let tests = [
            (r#"name{registry="} 1890"#, Err(Error::MissingValue)),
            (r#"name{registry=} 1890"#, Err(Error::MissingValue)),
            (
                r#"msdos_file_access_time_seconds{path="C:\\DIR\\FILE.TXT",error="Cannot find file:\n\"FILE.TXT\""} 1.458255915e9"#,
                Ok((
                    "msdos_file_access_time_seconds",
                    GroupKey {
                        timestamp: None,
                        labels: btreemap!(
                            "path" => r"C:\\DIR\\FILE.TXT",
                            "error" => r#"Cannot find file:\n\"FILE.TXT\""#
                        ),
                    },
                    1.458255915e9,
                )),
            ),
            (
                r#"node_hwmon_temp_auto_point1_pwm_celsius{chip="0000:00:03_1_0000:09:00_0",sensor="temp1"} 0.1"#,
                Ok((
                    "node_hwmon_temp_auto_point1_pwm_celsius",
                    GroupKey {
                        timestamp: None,
                        labels: btreemap!(
                            "chip" => "0000:00:03_1_0000:09:00_0",
                            "sensor" => "temp1",
                        ),
                    },
                    0.1,
                )),
            ),
            (
                "go_memstats_alloc_bytes 1.501088e+06",
                Ok((
                    "go_memstats_alloc_bytes",
                    GroupKey {
                        timestamp: None,
                        labels: btreemap!(),
                    },
                    1.501088e+06,
                )),
            ),
            (
                r#"go_gc_duration_seconds{quantile="1"} 0"#,
                Ok((
                    "go_gc_duration_seconds",
                    GroupKey {
                        timestamp: None,
                        labels: btreemap!(
                            "quantile" => "1"
                        ),
                    },
                    0.0,
                )),
            ),
            (
                r#"node_cpu_guest_seconds_total{cpu="0",mode="nice"} 1.1"#,
                Ok((
                    "node_cpu_guest_seconds_total",
                    GroupKey {
                        timestamp: None,
                        labels: btreemap!(
                            "cpu" => "0",
                            "mode" => "nice"
                        ),
                    },
                    1.1,
                )),
            ),
            (
                r#"some_negative -1"#,
                Ok((
                    "some_negative",
                    GroupKey {
                        timestamp: None,
                        labels: btreemap!(),
                    },
                    -1.0,
                )),
            ),
            (
                r##"some_negative -1.0"##,
                Ok((
                    "some_negative",
                    GroupKey {
                        timestamp: None,
                        labels: btreemap!(),
                    },
                    -1.0,
                )),
            ),
            (
                r##"some_negative -Inf"##,
                Ok((
                    "some_negative",
                    GroupKey {
                        timestamp: None,
                        labels: btreemap!(),
                    },
                    -f64::INFINITY,
                )),
            ),
            (
                r##"some_positive Inf"##,
                Ok((
                    "some_positive",
                    GroupKey {
                        timestamp: None,
                        labels: btreemap!(),
                    },
                    f64::INFINITY,
                )),
            ),
            (
                r##"some_positive +Inf"##,
                Ok((
                    "some_positive",
                    GroupKey {
                        timestamp: None,
                        labels: btreemap!(),
                    },
                    f64::INFINITY,
                )),
            ),
            (
                r##"some_positive +Inf 123456"##,
                Ok((
                    "some_positive",
                    GroupKey {
                        timestamp: Some(123456),
                        labels: btreemap!(),
                    },
                    f64::INFINITY,
                )),
            ),
            (
                r#"name{registry="default" content_type="html"} 1890"#,
                Err(Error::InvalidMetric(
                    r#"{registry="default" content_type="html"} 1890"#.into(),
                )),
            ),
            (
                r#"# TYPE a counte"#,
                Err(Error::InvalidMetric(r#"# TYPE a counte"#.into())),
            ),
            (
                r#"# TYPEabcd asdf"#,
                Err(Error::InvalidMetric(r#"# TYPEabcd asdf"#.into())),
            ),
            (r#"name{registry="} 1890"#, Err(Error::MissingValue)),
            (r#"name{registry=} 1890"#, Err(Error::MissingValue)),
            (
                r#"name abcd"#,
                Err(Error::InvalidMetricValue(
                    "abcd".parse::<f64>().unwrap_err(),
                )),
            ),
        ];

        for (input, want) in tests {
            let got = parse_metric(input);
            assert_eq!(want, got);
        }
    }

    #[test]
    fn node_exporter_output() {
        let data = std::fs::read_to_string("fixtures/node_exporter.txt").unwrap();
        let _n = parse_text(data.as_str()).unwrap();
    }

    #[test]
    fn with_metadata() {
        let data = std::fs::read_to_string("fixtures/prom.txt").unwrap();
        let _n = parse_text(data.as_str()).unwrap();
    }

    #[test]
    fn without_metadata() {
        let data = std::fs::read_to_string("fixtures/prom_nometa.txt").unwrap();
        let _n = parse_text(data.as_str()).unwrap();
    }

    #[test]
    fn all_kinds() {
        let input = r#"
# HELP http_requests_total The total number of HTTP requests.
# TYPE http_requests_total counter
http_requests_total{method="post",code="200"} 1027 1395066363000
http_requests_total{method="post",code="400"}    3 1395066363000

# Escaping in label values:
msdos_file_access_time_seconds{path="C:\\DIR\\FILE.TXT",error="Cannot find file:\n\"FILE.TXT\""} 1.458255915e9

# Minimalistic line:
metric_without_timestamp_and_labels 12.47

# A weird metric from before the epoch:
something_weird{problem="division by zero"} +Inf -3982045

# A histogram, which has a pretty complex representation in the text format:
# HELP http_request_duration_seconds A histogram of the request duration.
# TYPE http_request_duration_seconds histogram
http_request_duration_seconds_bucket{le="0.05"} 24054
http_request_duration_seconds_bucket{le="0.1"} 33444
http_request_duration_seconds_bucket{le="0.2"} 100392
http_request_duration_seconds_bucket{le="0.5"} 129389
http_request_duration_seconds_bucket{le="1"} 133988
http_request_duration_seconds_bucket{le="+Inf"} 144320
http_request_duration_seconds_sum 53423
http_request_duration_seconds_count 144320

# Finally a summary, which has a complex representation, too:
# HELP rpc_duration_seconds A summary of the RPC duration in seconds.
# TYPE rpc_duration_seconds summary
rpc_duration_seconds{quantile="0.01"} 3102
rpc_duration_seconds{quantile="0.05"} 3272
rpc_duration_seconds{quantile="0.5"} 4773
rpc_duration_seconds{quantile="0.9"} 9001
rpc_duration_seconds{quantile="0.99"} 76656
rpc_duration_seconds_sum 1.7560473e+07
rpc_duration_seconds_count 2693
"#;

        let groups = parse_text(input).unwrap();
        // msdos_file_access_time_seconds, metric_without_timestamp_and_labels and something_weird
        // are ignored cause we cannot detect the type of it.
        //
        // treat those metrics as Untyped
        assert_eq!(groups.len(), 3);
    }

    #[test]
    fn html() {
        // the returned page from web server could be html
        let content = r#"<!DOCTYPE html>
<html>
  <head>
    <script src="https://unpkg.com/react@18/umd/react.development.js" crossorigin></script>
    <script src="https://unpkg.com/react-dom@18/umd/react-dom.development.js" crossorigin></script>
    <script src="https://unpkg.com/@babel/standalone/babel.min.js"></script>
  </head>
  <body>

    <div id="mydiv"></div>

    <script type="text/babel">
      function Hello() {
        return <h1>Hello World!</h1>;
      }

      ReactDOM.render(<Hello />, document.getElementById('mydiv'))
    </script>

  </body>
</html>"#;

        let metrics = parse_text(content).unwrap();
        assert!(metrics.is_empty())
    }

    #[test]
    fn parse() {
        // Untyped not supported
        let input = r#"
# HELP http_requests_total The total number of HTTP requests.
# TYPE http_requests_total counter
http_requests_total{method="post",code="200"} 1027 1395066363000
http_requests_total{method="post",code="400"}    3 1395066363000

# Escaping in label values:
# msdos_file_access_time_seconds{path="C:\\DIR\\FILE.TXT",error="Cannot find file:\n\"FILE.TXT\""} 1.458255915e9

# Minimalistic line:
# metric_without_timestamp_and_labels 12.47

# A weird metric from before the epoch:
# something_weird{problem="division by zero"} +Inf -3982045

# A histogram, which has a pretty complex representation in the text format:
# HELP http_request_duration_seconds A histogram of the request duration.
# TYPE http_request_duration_seconds histogram
http_request_duration_seconds_bucket{le="0.05"} 24054
http_request_duration_seconds_bucket{le="0.1"} 33444
http_request_duration_seconds_bucket{le="0.2"} 100392
http_request_duration_seconds_bucket{le="0.5"} 129389
http_request_duration_seconds_bucket{le="1"} 133988
http_request_duration_seconds_bucket{le="+Inf"} 144320
http_request_duration_seconds_sum 53423
http_request_duration_seconds_count 144320

# Finally a summary, which has a complex representation, too:
# HELP rpc_duration_seconds A summary of the RPC duration in seconds.
# TYPE rpc_duration_seconds summary
rpc_duration_seconds{quantile="0.01"} 3102
rpc_duration_seconds{quantile="0.05"} 3272
rpc_duration_seconds{quantile="0.5"} 4773
rpc_duration_seconds{quantile="0.9"} 9001
rpc_duration_seconds{quantile="0.99"} 76656
rpc_duration_seconds_sum 1.7560473e+07
rpc_duration_seconds_count 2693
"#;

        let output = parse_text(input).unwrap();
        assert_eq!(output.len(), 3);
        match_group!(output[0], "http_requests_total", Counter => |metrics: &MetricMap<SimpleMetric>| {
            assert_eq!(metrics.len(), 2);
            assert_eq!(
                metrics.get_index(0).unwrap(),
                simple_metric!(Some(1395066363000), labels!(method => "post", code => 200), 1027.0)
            );
            assert_eq!(
                metrics.get_index(1).unwrap(),
                simple_metric!(Some(1395066363000), labels!(method => "post", code => 400), 3.0)
            );
        });
        /*
                match_group!(output[1], "msdos_file_access_time_seconds", Untyped => |metrics: &MetricMap<SimpleMetric>| {
                    assert_eq!(metrics.len(), 1);
                    assert_eq!(metrics.get_index(0).unwrap(), simple_metric!(
                        None,
                        labels!(path => "C:\\DIR\\FILE.TXT", error => "Cannot find file:\n\"FILE.TXT\""),
                        1.458255915e9
                    ));
                });

                match_group!(output[2], "metric_without_timestamp_and_labels", Untyped => |metrics: &MetricMap<SimpleMetric>| {
                    assert_eq!(metrics.len(), 1);
                    assert_eq!(metrics.get_index(0).unwrap(), simple_metric!(None, labels!(), 12.47));
                });

                match_group!(output[3], "something_weird", Untyped => |metrics: &MetricMap<SimpleMetric>| {
                    assert_eq!(metrics.len(), 1);
                    assert_eq!(
                        metrics.get_index(0).unwrap(),
                        simple_metric!(Some(-3982045), labels!(problem => "division by zero"), f64::INFINITY)
                    );
                });
        */
        match_group!(output[1], "http_request_duration_seconds", Histogram => |metrics: &MetricMap<HistogramMetric>| {
            assert_eq!(metrics.len(), 1, "length not match");
            assert_eq!(metrics.get_index(0).unwrap(), (
                &GroupKey {
                    timestamp: None,
                    labels: labels!(),
                },
                &HistogramMetric {
                    buckets: vec![
                        HistogramBucket { bucket: 0.05, count: 24054 },
                        HistogramBucket { bucket: 0.1, count: 33444 },
                        HistogramBucket { bucket: 0.2, count: 100392 },
                        HistogramBucket { bucket: 0.5, count: 129389 },
                        HistogramBucket { bucket: 1.0, count: 133988 },
                        HistogramBucket { bucket: f64::INFINITY, count: 144320 },
                    ],
                    count: 144320,
                    sum: 53423.0,
                },
            ));
        });

        match_group!(output[2], "rpc_duration_seconds", Summary => |metrics: &MetricMap<SummaryMetric>| {
            assert_eq!(metrics.len(), 1);
            assert_eq!(metrics.get_index(0).unwrap(), (
                &GroupKey {
                    timestamp: None,
                    labels: labels!(),
                },
                &SummaryMetric {
                    quantiles: vec![
                        SummaryQuantile { quantile: 0.01, value: 3102.0 },
                        SummaryQuantile { quantile: 0.05, value: 3272.0 },
                        SummaryQuantile { quantile: 0.5, value: 4773.0 },
                        SummaryQuantile { quantile: 0.9, value: 9001.0 },
                        SummaryQuantile { quantile: 0.99, value: 76656.0 },
                    ],
                    count: 2693,
                    sum: 1.7560473e+07,
                },
            ));
        });
    }
}
