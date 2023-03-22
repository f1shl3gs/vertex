use std::collections::BTreeMap;
use std::iter::Peekable;

use crate::{
    Error, GroupKey, GroupKind, HistogramBucket, HistogramMetric, MetricGroup, MetricKind,
    SummaryMetric, SummaryQuantile,
};

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
                    if let Some((name, kind)) = value.trim().split_once(' ') {
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

        let mut group = match kind {
            "counter" => GroupKind::new(MetricKind::Counter),
            "histogram" => GroupKind::new(MetricKind::Histogram),
            "gauge" => GroupKind::new(MetricKind::Gauge),
            "summary" => GroupKind::new(MetricKind::Summary),
            "untyped" => GroupKind::new(MetricKind::Untyped),
            _ => return Err(Error::InvalidType(kind.to_string())),
        };

        // parse metric lines
        match group.metric_kind() {
            MetricKind::Counter | MetricKind::Gauge | MetricKind::Untyped => {
                parse_simple_metrics(name, &mut group, &mut lines)?;
            }
            MetricKind::Histogram => {
                parse_histograms(&mut group, &mut lines)?;
            }
            MetricKind::Summary => {
                parse_summaries(&mut group, &mut lines)?;
            }
        };

        groups.push(MetricGroup {
            name: name.to_string(),
            description: description
                .strip_prefix(name)
                .unwrap_or(description)
                .trim()
                .to_string(),
            metrics: group,
        })
    }
}

fn parse_simple_metrics<'a, I>(
    prefix: &str,
    group: &mut GroupKind,
    lines: &mut Peekable<I>,
) -> Result<(), Error>
where
    I: Iterator<Item = &'a str>,
{
    loop {
        if let Some(line) = lines.peek() {
            if line.starts_with('#') {
                break;
            }
        } else {
            return Ok(());
        }

        // The next line already peeked, so error should not happened.
        let line = lines.next().unwrap();
        match line.strip_prefix(prefix) {
            Some(stripped) => {
                let (gk, value) = parse_labels(stripped)?;
                group.push(gk, value);
            }
            None => return Err(Error::InvalidMetric(line.into())),
        }
    }

    Ok(())
}

fn parse_histograms<'a, I>(group: &mut GroupKind, lines: &mut Peekable<I>) -> Result<(), Error>
where
    I: Iterator<Item = &'a str>,
{
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
                    group.push_histogram(gk.clone(), hm);

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

    group.push_histogram(current, hm);

    Ok(())
}

fn parse_summaries<'a, I>(group: &mut GroupKind, lines: &mut Peekable<I>) -> Result<(), Error>
where
    I: Iterator<Item = &'a str>,
{
    let mut sm = SummaryMetric::default();
    let mut labels = Default::default();

    loop {
        if let Some(line) = lines.peek() {
            if line.starts_with('#') {
                break;
            }
        } else {
            return Ok(());
        }

        // The next line already peeked, so error should not happened.
        let line = lines.next().unwrap();

        let (metric_name, mut gk, value) = parse_metric(line)?;
        if metric_name.ends_with("_sum") {
            sm.sum = value;
        } else if metric_name.ends_with("_count") {
            sm.count = value as u32;
            group.push_summary(gk, sm);
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
                group.push_summary(gk.clone(), sm);

                labels = gk.labels;
                sm = SummaryMetric::default();
            }
        }
    }

    Ok(())
}

fn parse_labels(line: &str) -> Result<(GroupKey, f64), Error> {
    let length = line.len();

    return if line.starts_with(' ') {
        // no labels
        let mut parts = line.split_whitespace();
        let value = parts.next().ok_or(Error::MissingValue)?.parse::<f64>()?;

        let timestamp = if let Some(value) = parts.next() {
            Some(value.parse::<i64>().map_err(Error::InvalidTimestamp)?)
        } else {
            None
        };

        Ok((
            GroupKey {
                timestamp,
                labels: Default::default(),
            },
            value,
        ))
    } else if line.starts_with('{') {
        // got some labels
        let mut pos = 1; // skip '{'
        let buf = line.as_bytes();
        let mut labels = BTreeMap::new();

        loop {
            // parse key
            let key_start = pos;
            while pos < length {
                let c = buf[pos];
                if c.is_ascii_alphabetic() || c == b'_' {
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

            let value = String::from_utf8_lossy(&buf[value_start..pos]).to_string();
            let key = String::from_utf8_lossy(&buf[key_start..key_end]).to_string();
            labels.insert(key, value);

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

        let line = line[pos..].trim();

        let mut parts = line.split_whitespace();
        let value = parts.next().ok_or(Error::MissingValue)?.parse::<f64>()?;

        let timestamp = if let Some(value) = parts.next() {
            Some(value.parse::<i64>().map_err(Error::InvalidTimestamp)?)
        } else {
            None
        };

        Ok((GroupKey { timestamp, labels }, value))
    } else {
        Err(Error::InvalidMetric(line.into()))
    };
}

fn parse_metric(line: &str) -> Result<(&str, GroupKey, f64), Error> {
    let length = line.len();
    let buf = line.as_bytes();
    let mut pos = 0;

    // 1. Take metric name
    while pos < length {
        let c = buf[pos];
        if c.is_ascii_alphanumeric() || c == b'_' {
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

    #[test]
    fn test_parse_metric() {
        let tests = [
            (r#"name{registry="} 1890"#, Err(Error::MissingValue)),
            (r#"name{registry=} 1890"#, Err(Error::MissingValue)),
            (
                r##"msdos_file_access_time_seconds{path="C:\\DIR\\FILE.TXT",error="Cannot find file:\n\"FILE.TXT\""} 1.458255915e9"##,
                Ok((
                    "msdos_file_access_time_seconds",
                    GroupKey {
                        timestamp: None,
                        labels: btreemap!(
                            "path" => r##"C:\\DIR\\FILE.TXT"##,
                            "error" => r##"Cannot find file:\n\"FILE.TXT\""##
                        ),
                    },
                    1.458255915e9,
                )),
            ),
            (
                r##"node_hwmon_temp_auto_point1_pwm_celsius{chip="0000:00:03_1_0000:09:00_0",sensor="temp1"} 0.1"##,
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
                r##"go_gc_duration_seconds{quantile="1"} 0"##,
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
                r##"node_cpu_guest_seconds_total{cpu="0",mode="nice"} 1.1"##,
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
                r##"some_negative -1"##,
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
    fn test_parse() {
        let input = r##"
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
"##;

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
}
