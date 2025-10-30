use std::collections::BTreeMap;
use std::time::Instant;

use argh::FromArgs;
use bytes::Buf;
use bytes::Bytes;
use chrono::Local;
use exitcode::ExitCode;
use framework::config::ProxyConfig;
use framework::http::HttpClient;
use http::{Method, Request, StatusCode};
use http_body_util::{BodyExt, Full};
use serde::Deserialize;
use tokio::time::MissedTickBehavior;
use tracing::warn;

#[derive(Debug, Deserialize)]
struct Point {
    attrs: BTreeMap<String, String>,
    value: f64,
}

#[derive(Debug, Deserialize)]
struct Metric {
    name: String,
    // description: String,
    points: Vec<Point>,
}

fn default_interval() -> String {
    "1s".to_string()
}

fn default_uri() -> String {
    "http://127.0.0.1:11000/stats".to_string()
}

#[derive(Debug, FromArgs)]
#[argh(
    subcommand,
    name = "top",
    description = "Display vertex components stats",
    help_triggers("-h", "--help")
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
        description = "vertex remote_tap endpoint",
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
                warn!(message = "build tokio runtime failed", %err);
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

                let stats = fetch(&table.uri).await.ok();

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
struct Throughput {
    received: u64,
    received_bytes: u64,
    sent: u64,
    sent_bytes: u64,
}

async fn fetch(uri: &str) -> framework::Result<TopStats> {
    let client = HttpClient::new(None, &ProxyConfig::default())?;

    let req = Request::builder()
        .method(Method::GET)
        .uri(uri)
        .body(Full::<Bytes>::default())?;

    let resp = client.send(req).await?;
    let (parts, incoming) = resp.into_parts();
    if parts.status != StatusCode::OK {
        return Err(format!("unexpected status code {}", parts.status).into());
    }

    let body = incoming.collect().await?.to_bytes();

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

struct Row {
    key: String,
    typ: String,

    events_in: String,
    events_out: String,
    bytes_in: String,
    bytes_out: String,
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

const MINIMUM_PADDING: usize = 1;

fn format_row(total: u64, throughput: u64) -> String {
    if total == 0 {
        return "N/A".to_string();
    }

    let mut index = 0;
    let mut value = total as f64;
    let units = ["", "K", "M", "G", "T"];
    while value > 1024.0 {
        value /= 1024.0;
        if index != 4 {
            index += 1;
        }
    }

    format!("{throughput}ps | {value:.2}{}", units[index])
}

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
        let rows = stats
            .0
            .iter()
            .map(|(component, throughput)| {
                let (events_in, events_out, bytes_in, bytes_out) =
                    if let Some(prev_stats) = &self.prev {
                        let duration = (now - prev_stats.timestamp).as_secs_f64().ceil() as u64;

                        match prev_stats.stats.0.get(component) {
                            Some(prev_throughput) => {
                                let events_in = format_row(
                                    throughput.received,
                                    (throughput.received - prev_throughput.received) / duration,
                                );
                                let events_out = format_row(
                                    throughput.sent,
                                    (throughput.sent - prev_throughput.sent) / duration,
                                );
                                let bytes_in = format_row(
                                    throughput.received_bytes,
                                    (throughput.received_bytes - prev_throughput.received_bytes)
                                        / duration,
                                );
                                let bytes_out = format_row(
                                    throughput.sent_bytes,
                                    (throughput.sent_bytes - prev_throughput.sent_bytes) / duration,
                                );

                                (events_in, events_out, bytes_in, bytes_out)
                            }

                            // previous stats exist, but the component stats is not
                            None => (
                                "N/A".to_string(),
                                "N/A".to_string(),
                                "N/A".to_string(),
                                "N/A".to_string(),
                            ),
                        }
                    } else {
                        // previous stats is not exist
                        (
                            "N/A".to_string(),
                            "N/A".to_string(),
                            "N/A".to_string(),
                            "N/A".to_string(),
                        )
                    };

                Row {
                    key: component.name.clone(),
                    typ: format!("{}/{}", component.kind, component.typ),
                    events_in,
                    events_out,
                    bytes_in,
                    bytes_out,
                }
            })
            .collect::<Vec<_>>();

        let (
            mut key_width,
            mut typ_width,
            mut events_in_width,
            mut events_out_width,
            mut bytes_in_width,
            mut bytes_out_width,
        ) = rows.iter().fold((0, 0, 0, 0, 0, 0), |mut acc, row| {
            acc.0 = acc.0.max(row.key.len() + MINIMUM_PADDING);
            acc.1 = acc.1.max(row.typ.len() + MINIMUM_PADDING);
            acc.2 = acc.2.max(row.events_in.len() + MINIMUM_PADDING);
            acc.3 = acc.3.max(row.events_out.len() + MINIMUM_PADDING);
            acc.4 = acc.4.max(row.bytes_in.len() + MINIMUM_PADDING);
            acc.5 = acc.5.max(row.bytes_out.len());
            acc
        });

        // if total width less than the window width, then add some extra padding to each column
        #[cfg(target_os = "linux")]
        if let Some((_max_height, max_width)) = termsize()
            && let Some(mut extra) = max_width.checked_sub(
                key_width
                    + typ_width
                    + events_in_width
                    + events_out_width
                    + bytes_in_width
                    + bytes_out_width,
            )
        {
            let padding = extra % 6;
            extra /= 6;

            key_width += extra;
            typ_width += extra;
            events_in_width += extra;
            events_out_width += extra;
            bytes_in_width += extra;
            bytes_out_width += extra + padding;
        }

        // header
        println!(
            "\x1b[7m{:key_width$}{:typ_width$}{:events_in_width$}{:events_out_width$}{:bytes_in_width$}{:bytes_out_width$}\x1b[0m",
            "ID", "Type", "Events In", "Events Out", "Bytes In", "Bytes Out"
        );

        rows.iter().for_each(|row| {
            println!(
                "{:key_width$}{:typ_width$}{:events_in_width$}{:events_out_width$}{:bytes_in_width$}{:bytes_out_width$}",
                row.key, row.typ, row.events_in, row.events_out, row.bytes_in, row.bytes_out
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

/// Gets the current terminal size
#[cfg(target_os = "linux")]
fn termsize() -> Option<(usize, usize)> {
    use std::io::IsTerminal;

    use libc::{STDOUT_FILENO, TIOCGWINSZ, ioctl};

    // http://rosettacode.org/wiki/Terminal_control/Dimensions#Library:_BSD_libc
    if !std::io::stdout().is_terminal() {
        return None;
    }

    // A representation of the size of the current terminal
    //
    // https://man7.org/linux/man-pages/man2/TIOCSWINSZ.2const.html
    let mut us = libc::winsize {
        ws_row: 0,
        ws_col: 0,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    let ret = unsafe { ioctl(STDOUT_FILENO, TIOCGWINSZ, &mut us) };
    if ret == 0 {
        Some((us.ws_row as usize, us.ws_col as usize))
    } else {
        None
    }
}
