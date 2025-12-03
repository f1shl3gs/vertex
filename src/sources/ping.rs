use std::collections::BTreeMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use chrono::Utc;
use configurable::configurable_component;
use event::{Metric, tags};
use framework::config::default_interval;
use framework::config::{OutputType, SourceConfig, SourceContext};
use framework::{Pipeline, ShutdownSignal, Source};
use parking_lot::Mutex;
use rand::{Rng, rng};
use socket2::{Domain, Protocol, Socket, Type};
use tokio::net::UdpSocket;
use tokio::task::JoinSet;

// default value from `net.ipv4.ip_default_ttl`
const fn default_ttl() -> u32 {
    64
}

/// Configuration of this source to do ICMP/PING request, and gather
/// metrics.
#[configurable_component(source, name = "ping")]
struct Config {
    targets: Vec<IpAddr>,

    /// The interval between each metrics sending
    #[serde(default = "default_interval", with = "humanize::duration::serde")]
    interval: Duration,

    /// TTL to the UDP socket
    #[serde(default = "default_ttl")]
    ttl: u32,
}

#[async_trait::async_trait]
#[typetag::serde(name = "ping")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::ICMPV4))?;
        socket.set_nonblocking(true)?;
        socket.set_ttl_v4(self.ttl)?;

        // TODO: bind the source address
        let socket = UdpSocket::from_std(socket.into()).map(Arc::new)?;

        let flush_interval = self.interval;
        let shutdown = cx.shutdown;
        let output = cx.output;
        let stats = self
            .targets
            .iter()
            .map(|addr| {
                let tracker_id = rng().random::<u128>();

                (
                    tracker_id,
                    Arc::new(Stat {
                        target: *addr,
                        send: AtomicUsize::new(0),
                        recv: AtomicUsize::new(0),
                        errs: AtomicUsize::new(0),
                        count_sketch: Mutex::new(CountSketch::default()),
                    }),
                )
            })
            .collect::<BTreeMap<_, _>>();

        Ok(Box::pin(async move {
            let mut tasks = JoinSet::new();

            for (identifier, (tracker_id, stat)) in stats.iter().enumerate() {
                tasks.spawn(send_loop(
                    Arc::clone(&socket),
                    stat.target,
                    Duration::from_secs(1),
                    identifier as u16,
                    *tracker_id,
                    Arc::clone(stat),
                    shutdown.clone(),
                ));
            }

            tasks.spawn(recv_loop(
                Arc::clone(&socket),
                stats.clone(),
                shutdown.clone(),
            ));

            tasks.spawn(flush_metrics(flush_interval, stats, output, shutdown));

            let _ = tasks.join_all().await;

            Ok(())
        }))
    }

    fn outputs(&self) -> Vec<OutputType> {
        vec![OutputType::metric()]
    }

    fn can_acknowledge(&self) -> bool {
        false
    }
}

async fn flush_metrics(
    interval: Duration,
    stats: BTreeMap<u128, Arc<Stat>>,
    mut output: Pipeline,
    mut shutdown: ShutdownSignal,
) {
    let mut ticker = tokio::time::interval(interval);

    loop {
        tokio::select! {
            _ = &mut shutdown => break,
            _ = ticker.tick() => {}
        }

        let mut metrics = Vec::with_capacity(stats.len() * 7);
        for (_, stat) in stats.iter() {
            let (max, min, mean, stddev) = stat.count_sketch.lock().compute();
            let sent = stat.send.load(Ordering::Acquire);
            let recv = stat.recv.load(Ordering::Acquire);

            metrics.extend([
                Metric::sum_with_tags(
                    "ping_send_packet_total",
                    "The number of packet send",
                    sent,
                    tags!(
                        "target" => stat.target.to_string(),
                    ),
                ),
                Metric::sum_with_tags(
                    "ping_recv_packet_total",
                    "The number of packet received",
                    recv,
                    tags!(
                        "target" => stat.target.to_string(),
                    ),
                ),
                Metric::sum_with_tags(
                    "ping_lost_packet_total",
                    "The number of packet lost",
                    sent - recv,
                    tags!(
                        "target" => stat.target.to_string(),
                    ),
                ),
                Metric::gauge_with_tags(
                    "ping_rtt_best_seconds",
                    "Best round trip time in seconds",
                    min as f64 / 1000000000.0,
                    tags!(
                        "target" => stat.target.to_string(),
                    ),
                ),
                Metric::gauge_with_tags(
                    "ping_rtt_worst_seconds",
                    "Worst round trip time in seconds",
                    max as f64 / 1000000000.0,
                    tags!(
                        "target" => stat.target.to_string(),
                    ),
                ),
                Metric::gauge_with_tags(
                    "ping_rtt_mean_seconds",
                    "Mean round trip time in seconds",
                    mean / 1000000000.0,
                    tags!(
                        "target" => stat.target.to_string(),
                    ),
                ),
                Metric::gauge_with_tags(
                    "ping_rtt_std_deviation_seconds",
                    "Standard deviation round trip time in seconds",
                    stddev / 1000000000.0,
                    tags!(
                        "target" => stat.target.to_string(),
                    ),
                ),
            ]);
        }

        if let Err(_err) = output.send(metrics).await {
            break;
        }
    }
}

#[derive(Debug, Default)]
struct CountSketch {
    rtts: Vec<i64>,
}

impl CountSketch {
    fn compute(&mut self) -> (i64, i64, f64, f64) {
        if self.rtts.is_empty() {
            self.rtts.clear();
            return (0, 0, 0.0, 0.0);
        }

        let mut max = 0;
        let mut min = u16::MAX as i64;
        let mut total = 0;
        for i in &self.rtts {
            let rtt = *i;

            if rtt > max {
                max = rtt;
            }

            if rtt < min {
                min = rtt;
            }

            total += rtt;
        }

        let length = self.rtts.len();

        let mean = total as f64 / length as f64;

        let mut squares = 0f64;
        for i in &self.rtts {
            let rtt = *i;

            squares += (rtt as f64 - mean).powi(2)
        }
        let stddev = f64::sqrt(squares / length as f64);

        self.rtts.clear();
        (max, min, mean, stddev)
    }
}

struct Stat {
    target: IpAddr,

    send: AtomicUsize,
    recv: AtomicUsize,
    errs: AtomicUsize,

    count_sketch: Mutex<CountSketch>,
}

async fn recv_loop(
    socket: Arc<UdpSocket>,
    states: BTreeMap<u128, Arc<Stat>>,
    mut shutdown: ShutdownSignal,
) {
    let mut buf = [0u8; 1024];

    loop {
        let (size, peer) = tokio::select! {
            _ = &mut shutdown => {
                break
            },
            result = socket.recv_from(&mut buf) => match result {
                Ok(pair) => pair,
                Err(err) => {
                    warn!(
                        message = "Error receiving from socket",
                        %err,
                        internal_log_rate_limit = 30
                    );

                    continue
                }
            }
        };

        let resp = &buf[..size];
        if resp.len() != 32 {
            continue;
        }

        let now = Utc::now();
        let sent = i64::from_ne_bytes(resp[8..16].try_into().unwrap());
        let rtt = now.timestamp_nanos_opt().unwrap() - sent;
        let tracker_id = u128::from_ne_bytes(resp[16..32].try_into().unwrap());

        match states.get(&tracker_id) {
            Some(stat) => {
                stat.recv.fetch_add(1, Ordering::SeqCst);
                stat.count_sketch.lock().rtts.push(rtt);

                // info!(message = "recv", bytes = size, ?peer, rtt);
            }
            None => {
                // unknown EchoReply
                warn!(
                    message = "unknown tracker id in the EchoReply response",
                    ?peer,
                    internal_log_rate_limit = 30
                );
            }
        }
    }
}

async fn send_loop(
    socket: Arc<UdpSocket>,
    target: IpAddr,
    interval: Duration,
    identifier: u16,
    tracker_id: u128,
    stats: Arc<Stat>,
    mut shutdown: ShutdownSignal,
) {
    #[rustfmt::skip]
    let mut req = [
        // icmp type EchoRequest
        0x08,
        // icmp code
        0x00,
        // checksum
        0x00, 0x00,

        // identifier
        0x00, 0x00,
        // sequence
        0x00, 0x00,

        // timestamp, i64
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        // tracker id
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];
    req[4..6].copy_from_slice(&identifier.to_ne_bytes());
    req[16..32].copy_from_slice(&tracker_id.to_ne_bytes());

    let mut sequence = 0u16;
    let mut ticker = tokio::time::interval(interval);
    loop {
        sequence = if sequence == 65535 { 1 } else { sequence + 1 };

        tokio::select! {
            _ = &mut shutdown => break,
            _ = ticker.tick() => {}
        }

        // prepare request
        req[6..8].copy_from_slice(&sequence.to_ne_bytes());
        let start = Utc::now().timestamp_nanos_opt().unwrap();
        req[8..16].copy_from_slice(&start.to_ne_bytes());
        if target.is_ipv4() {
            let checksum = calculate_checksum(&req);
            req[2..4].copy_from_slice(&checksum.to_ne_bytes());
        }

        if let Err(err) = socket.send_to(&req, SocketAddr::new(target, 0)).await {
            debug!(message = "unable to send ping request", ?err, ?target);

            stats.errs.fetch_add(1, Ordering::SeqCst);
            continue;
        }

        stats.send.fetch_add(1, Ordering::SeqCst);
    }
}

fn calculate_checksum(buf: &[u8]) -> u16 {
    let csumcv = buf.len() - 1;
    let mut s = 0;

    for i in (0..csumcv).step_by(2) {
        // skip checksum placeholder
        if i == 2 {
            continue;
        }

        s += (u32::from(buf[i + 1]) << 8) | u32::from(buf[i]);
    }

    if csumcv & 1 == 0 {
        s += u32::from(buf[csumcv]);
    }

    s = (s >> 16) + (s & 0xFFFF);
    s = s + (s >> 16);

    (s as u16) ^ 0xFFFF
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>();
    }

    #[test]
    fn checksum() {
        let input = [
            30, 119, 10, 10, 60, 31, 200, 87, 169, 36, 250, 118, 59, 138, 178, 61, 60, 255, 234,
            166, 44, 14, 120, 191, 9, 200, 174, 139, 71, 175, 93, 110,
        ];
        let c = calculate_checksum(&input);

        assert_eq!(c, 51489);
    }
}
