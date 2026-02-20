use std::fmt;
use std::io::Write;

use event::{Metric, MetricValue};
use framework::sink::encoding::{Encoder, TrackedWriter};

// https://docs.influxdata.com/influxdb/cloud/reference/syntax/line-protocol/#special-characters
const COMMA_EQ_SPACE: [char; 3] = [',', '=', ' '];
const COMMA_SPACE: [char; 2] = [',', ' '];

/// InfluxDB Line Protocol encoder
///
/// See https://docs.influxdata.com/influxdb/v2/reference/syntax/line-protocol/
pub struct LineProtocolEncoder;

impl Encoder<Vec<Metric>> for LineProtocolEncoder {
    fn encode(&self, metrics: Vec<Metric>, writer: &mut dyn Write) -> std::io::Result<usize> {
        let mut writer = TrackedWriter::new(writer);

        for metric in &metrics {
            write!(writer, "{}", escape(metric.name(), COMMA_SPACE))?;

            for (key, value) in &metric.tags {
                let key = escape(key.as_str(), COMMA_EQ_SPACE);
                let value = value.to_string();
                if value.is_empty() {
                    continue;
                }

                let value = escape(&value, COMMA_EQ_SPACE);
                write!(writer, ",{key}={value}")?;
            }

            writer.write_all(b" ")?;

            match &metric.value {
                MetricValue::Sum(value) => {
                    writeln!(writer, "counter={value}")?;
                }
                MetricValue::Gauge(value) => {
                    writeln!(writer, "gauge={value}")?;
                }
                MetricValue::Histogram {
                    buckets,
                    count,
                    sum,
                } => {
                    for (index, bucket) in buckets.iter().enumerate() {
                        if index != 0 {
                            writer.write_all(b",")?;
                        }

                        if bucket.upper == f64::MAX {
                            write!(writer, "+Inf={}", bucket.count)?;
                        } else {
                            write!(writer, "{}={}", bucket.upper, bucket.count)?;
                        }
                    }
                    write!(writer, ",count={count}")?;
                    writeln!(writer, ",sum={sum}")?;
                }
                MetricValue::Summary {
                    quantiles,
                    count,
                    sum,
                } => {
                    for (index, quantile) in quantiles.iter().enumerate() {
                        if index == 0 {
                            write!(writer, "{}={}", quantile.quantile, quantile.value)?;
                        } else {
                            write!(writer, ",{}={}", quantile.quantile, quantile.value)?;
                        }
                    }
                    write!(writer, ",count={count}")?;
                    writeln!(writer, ",sum={sum}")?;
                }
            }
        }

        Ok(writer.written())
    }
}

// Return a [`fmt::Display`] that renders string while escaping any characters in the `special_characters` array
// with a `\`
fn escape<const N: usize>(src: &str, special_characters: [char; N]) -> Escaped<'_, N> {
    Escaped {
        src,
        special_characters,
    }
}

struct Escaped<'a, const N: usize> {
    src: &'a str,
    special_characters: [char; N],
}

impl<const N: usize> fmt::Display for Escaped<'_, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for ch in self.src.chars() {
            if self.special_characters.contains(&ch) || ch == '\\' {
                write!(f, "\\")?;
            }
            write!(f, "{ch}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOUBLE_QUOTE: [char; 1] = ['"'];

    #[test]
    fn string_escape() {
        assert_eq!(
            format!("\"{}\"", escape(r#"foo"#, DOUBLE_QUOTE)),
            r#""foo""#
        );
        assert_eq!(
            format!("\"{}\"", escape(r"foo \ bar", DOUBLE_QUOTE)),
            r#""foo \\ bar""#
        );
        assert_eq!(
            format!("\"{}\"", escape(r#"foo " bar"#, DOUBLE_QUOTE)),
            r#""foo \" bar""#
        );
        assert_eq!(
            format!("\"{}\"", escape(r#"foo \" bar"#, DOUBLE_QUOTE)),
            r#""foo \\\" bar""#
        );
    }
}
