use std::path::PathBuf;

use event::{Metric, tags};

use super::Error;

/// Exposes UDP total lengths of the rx_queue and tx_queue
/// from `/proc/net/udp` and `/proc/net/udp6`
pub async fn gather(proc_path: PathBuf) -> Result<Vec<Metric>, Error> {
    let mut metrics = Vec::with_capacity(4);

    for (ip, path) in [("v4", "net/udp"), ("v6", "net/udp6")] {
        let path = proc_path.join(path);
        if !path.exists() {
            continue;
        }

        match net_ip_socket_summary(path) {
            Ok(summary) => {
                metrics.extend([
                    Metric::gauge_with_tags(
                        "node_udp_queues",
                        "Number of allocated memory in the kernel for UDP datagrams in bytes.",
                        summary.tx_queue_length,
                        tags! {
                            "ip" => ip,
                            "queue" => "tx",
                        },
                    ),
                    Metric::gauge_with_tags(
                        "node_udp_queues",
                        "Number of allocated memory in the kernel for UDP datagrams in bytes.",
                        summary.rx_queue_length,
                        tags! {
                            "ip" => ip,
                            "queue" => "rx",
                        },
                    ),
                ]);
            }
            Err(err) => {
                warn!(
                    message = "couldn't get udp queue stats",
                    %err,
                );
            }
        }
    }

    Ok(metrics)
}

/// NetIPSocketSummary provides already computed values like the
/// total queue lengths or the total number of used sockets. In contrast to
/// NetIPSocket it does not collect the parsed lines into a slice.
#[derive(Default, Debug, PartialEq)]
struct NetIPSocketSummary {
    // tx_queue_length shows the total queue length of all parsed tx_queue lengths
    tx_queue_length: u64,

    // rx_queue_length shows the total queue length of all parsed rx_queue lengths
    rx_queue_length: u64,

    // used_sockets shows the total number of parsed lines representing the number
    // of used sockets
    used_sockets: u64,
}

/// NetIPSocketLine represents the fields parsed from a single line
/// in /proc/net/{t,u}dp{,6}. Fields which are not used by IPSocket are skipped.
/// For the proc file format details, see https://linux.die.net/man/5/proc.
struct NetIPSocketLine {
    sl: u64,
    local_addr: String,
    local_port: u64,
    remote_addr: String,
    remote_port: u64,
    st: u64,
    tx_queue: u64,
    rx_queue: u64,
    uid: u64,
    inode: u64,
}

fn net_ip_socket_summary(path: PathBuf) -> Result<NetIPSocketSummary, Error> {
    let data = std::fs::read_to_string(path)?;
    let mut summary = NetIPSocketSummary::default();

    // skip first header line
    for line in data.lines().skip(1) {
        let (tx, rx) = parse_net_ip_socket_queues(line)?;

        summary.used_sockets += 1;
        summary.tx_queue_length += tx;
        summary.rx_queue_length += rx;
    }

    Ok(summary)
}

fn parse_net_ip_socket_queues(line: &str) -> Result<(u64, u64), Error> {
    // the content looks like
    // sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode ref pointer drops
    // 1560: 00000000:B2A8 00000000:0000 07 00000000:00000000 00:00000000 00000000  1000        0 49584 2 00000000d54e19cb 195
    // 3674: 00000000:BAEA 00000000:0000 07 00000000:00000000 00:00000000 00000000  1000        0 53882 2 00000000e53720bf 0
    let fields = line
        .split_ascii_whitespace()
        .nth(4)
        .ok_or_else(|| Error::from("invalid field"))?;

    let txq = u64::from_str_radix(&fields[..8], 16)?;
    let rxq = u64::from_str_radix(&fields[9..], 16)?;

    Ok((txq, rxq))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse() {
        let line = "   73: 0100007F:0143 00000000:0000 07 00000010:00000005 00:00000000 00000000     0        0 36799 2 0000000000000000 0 ";
        let (tx, rx) = parse_net_ip_socket_queues(line).unwrap();

        assert_eq!(tx, 16);
        assert_eq!(rx, 5);
    }

    #[test]
    fn socket_summary() {
        struct TestCase {
            name: String,
            file: PathBuf,
            want: NetIPSocketSummary,
            want_err: bool,
        }

        let cases = vec![
            TestCase {
                name: "udp file found, no error should come up".to_string(),
                file: "tests/node/proc/net/udp".into(),
                want: NetIPSocketSummary {
                    tx_queue_length: 2,
                    rx_queue_length: 2,
                    used_sockets: 3,
                },
                want_err: false,
            },
            TestCase {
                name: "udp6 file found, no error should come up".to_string(),
                file: "tests/node/proc/net/udp6".into(),
                want: NetIPSocketSummary {
                    tx_queue_length: 0,
                    rx_queue_length: 0,
                    used_sockets: 2,
                },
                want_err: false,
            },
            TestCase {
                name: "error case - file not found".to_string(),
                file: "somewhere over the rainbow".into(),
                want: NetIPSocketSummary::default(),
                want_err: true,
            },
            TestCase {
                name: "error case - parse error".to_string(),
                file: "tests/node/proc/net/udp_broken".into(),
                want: Default::default(),
                want_err: true,
            },
        ];

        for case in cases {
            let result = net_ip_socket_summary(case.file);
            if case.want_err {
                if result.is_err() {
                    continue;
                }

                panic!("error expected, but not come up")
            }

            let actual = result.unwrap();
            assert_eq!(actual, case.want, "{}", case.name)
        }
    }
}
