use std::collections::BTreeMap;
use std::time::Instant;

use argh::FromArgs;
use bytes::Buf;
use chrono::Local;
use exitcode::ExitCode;
use framework::config::ProxyConfig;
use framework::http::HttpClient;
use http::{Method, Request, StatusCode};
use hyper::Body;
use serde::Deserialize;
use tokio::time::MissedTickBehavior;
use tracing::warn;

#[derive(Debug, Deserialize)]
pub struct Point {
    pub attrs: BTreeMap<String, String>,
    pub value: f64,
}

#[derive(Debug, Deserialize)]
pub struct Metric {
    pub name: String,
    pub description: String,
    pub points: Vec<Point>,
}

fn default_interval() -> String {
    "1s".to_string()
}

fn default_uri() -> String {
    "http://127.0.0.1:56888/statsz".to_string()
}

#[derive(Debug, FromArgs)]
#[argh(
    subcommand,
    name = "top",
    description = "Display vertex components stats"
)]
pub struct Top {
    #[argh(
        option,
        description = "interval of update data",
        default = "default_interval()"
    )]
    interval: String,

    #[argh(
        option,
        description = "vertex zpages endpoint",
        default = "default_uri()"
    )]
    uri: String,
}

impl Top {
    pub fn run(&self) -> Result<(), ExitCode> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|err| {
                warn!(message = "build tokio runtime failed", ?err);
                exitcode::OSERR
            })?;

        let interval = self.interval.clone();
        let period =
            humanize::duration::parse_duration(&interval).map_err(|_err| exitcode::CONFIG)?;
        let uri = self.uri.clone();
        rt.block_on(async move {
            let mut ticker = tokio::time::interval(period);
            ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

            let mut table = Table::new(uri, interval);

            loop {
                ticker.tick().await;

                let stats = match fetch(&table.uri).await {
                    Ok(stats) => Some(stats),
                    Err(_err) => None,
                };

                table.render(stats);
            }
        });

        Ok(())
    }
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd)]
struct Component {
    name: String,
    kind: String,
    typ: String,
}

fn into_parts(point: &Point) -> Option<(Component, u64)> {
    let name = point.attrs.get("component")?.to_string();
    let kind = point.attrs.get("component_kind")?.to_string();
    let typ = point.attrs.get("component_type")?.to_string();

    Some((Component { name, kind, typ }, point.value as u64))
}

#[derive(Default, Debug)]
pub struct Throughput {
    received: u64,
    received_bytes: u64,
    sent: u64,
    sent_bytes: u64,
}

async fn fetch(uri: &str) -> framework::Result<TopStats> {
    let client = HttpClient::new(&None, &ProxyConfig::default())?;

    let req = Request::builder()
        .method(Method::GET)
        .uri(uri)
        .body(Body::empty())?;

    let resp = client.send(req).await?;
    let (parts, body) = resp.into_parts();
    if parts.status != StatusCode::OK {
        return Err(format!("unexpected status code {}", parts.status).into());
    }

    let body = hyper::body::aggregate(body).await?;

    let metrics: Vec<Metric> = serde_json::from_reader(body.reader())?;
    let mut top = BTreeMap::new();
    metrics.iter().for_each(|metric| {
        metric.points.iter().for_each(|point| {
            if let Some((component, value)) = into_parts(point) {
                let throughput = top.entry(component).or_insert_with(Throughput::default);
                match metric.name.as_str() {
                    "component_received_event_bytes_total" => throughput.received_bytes = value,
                    "component_received_events_total" => throughput.received = value,
                    "component_sent_event_bytes_total" => throughput.sent_bytes = value,
                    "component_sent_events_total" => throughput.sent = value,
                    _ => {}
                }
            }
        })
    });

    Ok(TopStats(top))
}

#[allow(dead_code)]
struct Row {
    key: String,
    kind: String,
    typ: String,

    received: String,
    received_rate: f64,
    received_bytes: String,
    received_bytes_rate: f64,
    sent: String,
    sent_rate: f64,
    sent_bytes: String,
    sent_bytes_rate: f64,
}

struct TopStats(BTreeMap<Component, Throughput>);

impl TopStats {
    fn sources(&self) -> usize {
        self.0
            .keys()
            .filter(|component| component.kind == "source")
            .count()
    }

    fn transforms(&self) -> usize {
        self.0
            .keys()
            .filter(|component| component.kind == "transform")
            .count()
    }

    fn sinks(&self) -> usize {
        self.0
            .keys()
            .filter(|component| component.kind == "sink")
            .count()
    }
}

struct PrevStats {
    timestamp: Instant,
    stats: TopStats,
}

const MINIMUM_PADDING_RIGHT: usize = 1;

struct Table {
    uri: String,
    interval: String,

    prev: Option<PrevStats>,
}

#[allow(clippy::print_stdout)]
impl Table {
    fn new(uri: String, interval: String) -> Self {
        Self {
            uri,
            interval,
            prev: None,
        }
    }

    fn render(&mut self, stats: Option<TopStats>) {
        // Clear the console
        print!("\x1b[2J\x1b[1;1H");

        self.print_summary(stats.as_ref());

        let stats = match stats {
            Some(stats) => stats,
            None => {
                self.prev = None;
                return;
            }
        };

        let now = Instant::now();
        let mut rows = stats
            .0
            .iter()
            .map(|(component, throughput)| {
                let (received, received_rate, sent, sent_rate) = if let Some(prev_stats) =
                    &self.prev
                {
                    let duration = (now - prev_stats.timestamp).as_secs_f64();

                    match prev_stats.stats.0.get(component) {
                        Some(prev_throughput) => {
                            let received_rate =
                                (throughput.received - prev_throughput.received) as f64 / duration;
                            let received =
                                format!("{} ({:.2}/s)", throughput.received, received_rate);

                            let sent_rate =
                                (throughput.sent - prev_throughput.sent) as f64 / duration;
                            let sent = format!("{} ({:.2}/s)", throughput.sent, sent_rate);

                            (received, received_rate, sent, sent_rate)
                        }

                        // previous stats exist, but the component stats is not
                        None => (
                            format!("{} (--/s)", throughput.received),
                            0.0,
                            format!("{} (--/s)", throughput.sent),
                            0.0,
                        ),
                    }
                } else {
                    // previous stats is not exist
                    (
                        format!("{} (--/s)", throughput.received),
                        0.0,
                        format!("{} (--/s)", throughput.sent),
                        0.0,
                    )
                };

                Row {
                    key: component.name.clone(),
                    kind: component.kind.clone(),
                    typ: component.typ.clone(),
                    received,
                    received_rate,
                    received_bytes: "".into(),
                    received_bytes_rate: 0.0,
                    sent,
                    sent_rate,
                    sent_bytes: "".into(),
                    sent_bytes_rate: 0.0,
                }
            })
            .collect::<Vec<_>>();

        let (mut key_width, mut kind_width, mut typ_width, mut received_width, mut sent_width) =
            rows.iter().fold((0, 0, 0, 0, 0), |mut acc, row| {
                acc.0 = acc.0.max(row.key.len() + MINIMUM_PADDING_RIGHT);
                acc.1 = acc.1.max(row.kind.len() + MINIMUM_PADDING_RIGHT);
                acc.2 = acc.2.max(row.typ.len() + MINIMUM_PADDING_RIGHT);
                acc.3 = acc.3.max(row.received.len() + MINIMUM_PADDING_RIGHT);
                acc.4 = acc.4.max(row.sent.len() + MINIMUM_PADDING_RIGHT);
                acc
            });

        if let Some((_max_height, max_width)) = termsize::get() {
            if let Some(mut n) = max_width
                .checked_sub(key_width + kind_width + typ_width + received_width + sent_width)
            {
                let padding = n % 5;
                n /= 5;

                key_width += n;
                kind_width += n;
                typ_width += n;
                received_width += n;
                sent_width += n + padding;
            }
        }

        // header
        println!("\x1b[7m{:key_width$}{:kind_width$}{:typ_width$}{:received_width$}{:sent_width$}\x1b[0m",
                 "ID", "Kind", "Type", "Received", "Sent");

        // sort by received rate for now
        rows.sort_by(|a, b| b.received_rate.total_cmp(&a.received_rate));
        rows.iter().for_each(|row| {
            println!(
                "{:key_width$}{:kind_width$}{:typ_width$}{:received_width$}{:sent_width$}",
                row.key, row.kind, row.typ, row.received, row.sent
            );
        });

        self.prev = Some(PrevStats {
            timestamp: now,
            stats,
        });
    }

    fn print_summary(&self, stats: Option<&TopStats>) {
        let now = Local::now();
        println!(
            "top - {} | {} | {} | {}",
            now.format("%H:%M:%S"),
            self.uri,
            self.interval,
            if stats.is_some() {
                "\x1b[32mConnected\x1b[0m"
            } else {
                "\x1b[31mUnconnected\x1b[0m"
            }
        );

        let (sources, transforms, sinks) = match stats {
            Some(stats) => (stats.sources(), stats.transforms(), stats.sinks()),
            None => (0, 0, 0),
        };

        println!(
            "Tasks: {} total, {} sources, {} transforms, {} sinks",
            sources + transforms + sinks,
            sources,
            transforms,
            sinks
        );
        println!();
    }
}

mod termsize {
    use libc::{c_ushort, ioctl, STDOUT_FILENO, TIOCGWINSZ};
    use std::io::IsTerminal;

    /// A representation of the size of the current terminal
    #[repr(C)]
    #[derive(Debug)]
    struct UnixSize {
        /// number of rows
        pub rows: c_ushort,
        /// number of columns
        pub cols: c_ushort,
        x: c_ushort,
        y: c_ushort,
    }

    /// Gets the current terminal size
    pub fn get() -> Option<(usize, usize)> {
        // http://rosettacode.org/wiki/Terminal_control/Dimensions#Library:_BSD_libc
        if !std::io::stdout().is_terminal() {
            return None;
        }
        let us = UnixSize {
            rows: 0,
            cols: 0,
            x: 0,
            y: 0,
        };
        let r = unsafe { ioctl(STDOUT_FILENO, TIOCGWINSZ, &us) };
        if r == 0 {
            Some((us.rows as usize, us.cols as usize))
        } else {
            None
        }
    }
}
