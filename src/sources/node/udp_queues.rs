use event::{tags, Metric};
use tokio::io::AsyncBufReadExt;

use super::Error;

/// Exposes UDP total lengths of the rx_queue and tx_queue
/// from `/proc/net/udp` and `/proc/net/udp6`

pub async fn gather(proc_path: &str) -> Result<Vec<Metric>, Error> {
    let mut metrics = Vec::new();
    if let Ok(v4) = net_udp_summary(proc_path).await {
        metrics.extend_from_slice(&[
            Metric::gauge_with_tags(
                "node_udp_queues",
                "Number of allocated memory in the kernel for UDP datagrams in bytes.",
                v4.tx_queue_length as f64,
                tags! {
                    "queue" => "tx",
                    "ip" => "v4"
                },
            ),
            Metric::gauge_with_tags(
                "node_udp_queues",
                "Number of allocated memory in the kernel for UDP datagrams in bytes.",
                v4.rx_queue_length as f64,
                tags! {
                    "queue" => "rx",
                    "ip" => "v4"
                },
            ),
        ]);
    }

    if let Ok(v6) = net_udp6_summary(proc_path).await {
        metrics.extend_from_slice(&[
            Metric::gauge_with_tags(
                "node_udp_queues",
                "Number of allocated memory in the kernel for UDP datagrams in bytes.",
                v6.tx_queue_length as f64,
                tags! {
                    "queue" => "tx",
                    "ip" => "v6"
                },
            ),
            Metric::gauge_with_tags(
                "node_udp_queues",
                "Number of allocated memory in the kernel for UDP datagrams in bytes.",
                v6.rx_queue_length as f64,
                tags! {
                    "queue" => "rx",
                    "ip" => "v6"
                },
            ),
        ]);
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

async fn net_udp_summary(root: &str) -> Result<NetIPSocketSummary, Error> {
    let path = format!("{}/net/udp", root);
    net_ip_socket_summary(&path).await
}

async fn net_udp6_summary(root: &str) -> Result<NetIPSocketSummary, Error> {
    let path = format!("{}/net/udp6", root);
    net_ip_socket_summary(&path).await
}

/// NetIPSocketLine represents the fields parsed from a single line
/// in /proc/net/{t,u}dp{,6}. Fields which are not used by IPSocket are skipped.
/// For the proc file format details, see https://linux.die.net/man/5/proc.
#[allow(dead_code)]
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

async fn net_ip_socket_summary(path: &str) -> Result<NetIPSocketSummary, Error> {
    let f = tokio::fs::File::open(path).await?;
    let reader = tokio::io::BufReader::new(f);
    let mut lines = reader.lines();
    let mut summary = NetIPSocketSummary::default();

    while let Some(line) = lines.next_line().await? {
        // skip the head line
        if line.starts_with("  sl") {
            continue;
        }

        let (tx, rx) = parse_net_ip_socket_queues(&line)?;
        summary.used_sockets += 1;
        summary.tx_queue_length += tx;
        summary.rx_queue_length += rx;
    }

    Ok(summary)
}

fn parse_net_ip_socket_queues(line: &str) -> Result<(u64, u64), Error> {
    // the content looks like
    // sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode ref pointer drops
    //    73: 0100007F:0143 00000000:0000 07 00000000:00000000 00:00000000 00000000     0        0 36799 2 0000000000000000 0
    let fields = line
        .split_ascii_whitespace()
        .nth(4)
        .ok_or_else(|| Error::new_invalid("invalid field"))?;

    let txq = u64::from_str_radix(&fields[..8], 16)?;
    let rxq = u64::from_str_radix(&fields[9..], 16)?;

    Ok((txq, rxq))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_net_ip_socket_queues() {
        let line = "   73: 0100007F:0143 00000000:0000 07 00000010:00000005 00:00000000 00000000     0        0 36799 2 0000000000000000 0 ";
        let (tx, rx) = parse_net_ip_socket_queues(line).unwrap();

        assert_eq!(tx, 16);
        assert_eq!(rx, 5);
    }

    #[tokio::test]
    async fn test_net_ip_socket_summary() {
        struct TestCase {
            name: String,
            file: String,
            want: NetIPSocketSummary,
            want_err: bool,
        }

        let cases = vec![
            TestCase {
                name: "udp file found, no error should come up".to_string(),
                file: "tests/fixtures/proc/net/udp".to_string(),
                want: NetIPSocketSummary {
                    tx_queue_length: 2,
                    rx_queue_length: 2,
                    used_sockets: 3,
                },
                want_err: false,
            },
            TestCase {
                name: "udp6 file found, no error should come up".to_string(),
                file: "tests/fixtures/proc/net/udp6".to_string(),
                want: NetIPSocketSummary {
                    tx_queue_length: 0,
                    rx_queue_length: 0,
                    used_sockets: 2,
                },
                want_err: false,
            },
            TestCase {
                name: "error case - file not found".to_string(),
                file: "somewhere over the rainbow".to_string(),
                want: NetIPSocketSummary::default(),
                want_err: true,
            },
            TestCase {
                name: "error case - parse error".to_string(),
                file: "tests/fixtures/proc/net/udp_broken".to_string(),
                want: Default::default(),
                want_err: true,
            },
        ];

        for case in cases {
            let result = net_ip_socket_summary(&case.file).await;
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
