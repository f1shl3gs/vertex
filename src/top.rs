#![allow(clippy::print_stdout)]

use argh::FromArgs;
use bytes::Buf;
use chrono::{DateTime, Local};
use exitcode::ExitCode;
use framework::config::{ComponentKey, ProxyConfig};
use framework::http::HttpClient;
use framework::tls::TlsSettings;
use http::{Method, Request};
use hyper::Body;
use tokio::time::MissedTickBehavior;
use tracing::warn;
use vertex::extensions::zpages::TopStats;

fn default_interval() -> String {
    "1s".to_string()
}

fn default_uri() -> String {
    "http://127.0.0.1:56888/top".to_string()
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

        let interval =
            humanize::duration::parse_duration(&self.interval).map_err(|_err| exitcode::CONFIG)?;
        let uri = self.uri.clone();
        rt.block_on(async move {
            let mut ticker = tokio::time::interval(interval);
            ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

            let uri = &uri;
            let interval = &humanize::duration::duration(&interval);

            loop {
                ticker.tick().await;

                let stats = match fetch(uri).await {
                    Ok(stats) => stats,
                    Err(err) => {
                        warn!(message = "fetch top stats failed", ?err);
                        continue;
                    }
                };

                print_stats(interval, uri, &stats);
            }
        });

        Ok(())
    }
}

async fn fetch(uri: &str) -> framework::Result<TopStats> {
    let tls = TlsSettings::from_options(&None)?;
    let client = HttpClient::new(tls, &ProxyConfig::default())?;

    let req = Request::builder()
        .method(Method::GET)
        .uri(uri)
        .body(Body::empty())?;

    let resp = client.send(req).await?;
    let (_parts, body) = resp.into_parts();
    let body = hyper::body::aggregate(body).await?;

    serde_json::from_reader(body.reader()).map_err(Into::into)
}

fn print_stats(interval: &str, uri: &str, stats: &TopStats) {
    // Clear the console
    print!("{esc}[2J{esc}[1;1H", esc = 27 as char);

    print_header(interval, uri, stats);

    println!("{:?}", stats);

    println!("press q to exit");
}

fn print_header(interval: &str, uri: &str, stats: &TopStats) {
    let now = Local::now();
    println!(
        "top - {} uri: {uri}, interval: {interval}",
        now.format("%H:%M:%s")
    );
    println!(
        "Tasks: {} total, {} sources, {} transforms, {} sinks",
        stats.sources.len() + stats.transforms.len() + stats.sinks.len(),
        stats.sources.len(),
        stats.transforms.len(),
        stats.sinks.len()
    );
    println!();
}

mod term {
    use libc::{c_ushort, ioctl, STDOUT_FILENO, TIOCGWINSZ};

    /// A representation of the size of the current terminal
    #[repr(C)]
    #[derive(Debug)]
    pub struct UnixSize {
        /// number of rows
        pub rows: c_ushort,
        /// number of columns
        pub cols: c_ushort,
        x: c_ushort,
        y: c_ushort,
    }

    /// Gets the current terminal size
    pub fn get() -> Option<(u16, u16)> {
        // http://rosettacode.org/wiki/Terminal_control/Dimensions#Library:_BSD_libc
        if atty::isnt(atty::Stream::Stdout) {
            return None;
        }
        let us = UnixSize {
            rows: 0,
            cols: 0,
            x: 0,
            y: 0,
        };
        let r = unsafe { ioctl(STDOUT_FILENO, TIOCGWINSZ.into(), &us) };
        if r == 0 {
            Some((us.rows, us.cols))
        } else {
            None
        }
    }
}

struct Row {
    key: String,
    kind: String,

    // processed_events: u64,
    // processed_events_rate: f64,
    // processed_bytes: u64,
    // processed_bytes_rate: f64,

    received_events_total: u64,
    received_bytes_total: u64,
    sent_events_total: u64,
    sent_bytes_total: u64,
}

struct Table {
    uri: String,
    interval: String,

    prev: Option<PrevStats>,
}

struct PrevStats {
    timestamp: DateTime<Local>,
    stats: TopStats,
}

impl Table {
    fn print(&self, stats: TopStats) {

    }

    fn rows(&mut self, stats: TopStats) -> Vec<Row> {
        let mut rows = vec![];

        stats.sources
            .iter()
            .for_each(|(key, throughput)| {
                rows.push(Row {
                    key: key.to_string(),
                    kind: "source".to_string(),
                    received_events_total: 0,
                    received_bytes_total: 0,
                    sent_events_total: 0,
                    sent_bytes_total: 0,
                })
            })

        rows
    }
}
