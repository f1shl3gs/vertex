use std::fmt::{Display, Formatter};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::ops::Add;

use chrono::{DateTime, Duration, Utc};
use tokio::net::{ToSocketAddrs, UdpSocket};

const HEADER_SIZE: usize = 48;

const NTP_EPOCH_OFFSET: u64 = 2208988800;
const NANOS_PER_SEC: u64 = 1000000000;
const FRAC: u64 = 4294967296;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    ServerTickedBackwards,
    InvalidMode,
    InvalidTransmitTime,
    ServerResponseMismatch,
    KissOfDeath,
    InvalidStratum,
    ServerClockFreshness,
    InvalidDispersion,
    InvalidTime,
    InvalidLeapSecond,
}

impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(err) => err.fmt(f),
            Error::ServerTickedBackwards => f.write_str("server clock ticked backwards"),
            Error::InvalidMode => f.write_str("invalid mode in response"),
            Error::InvalidTransmitTime => f.write_str("invalid transmit time in response"),
            Error::ServerResponseMismatch => f.write_str("server response didn't match request"),
            Error::KissOfDeath => f.write_str("kiss of death received"),
            Error::InvalidStratum => f.write_str("invalid stratum in response"),
            Error::ServerClockFreshness => f.write_str("server clock not fresh"),
            Error::InvalidDispersion => f.write_str("invalid dispersion in response"),
            Error::InvalidTime => f.write_str("invalid time reported"),
            Error::InvalidLeapSecond => f.write_str("invalid leap second in response"),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
}

pub struct Client {
    bind: SocketAddr,
}

impl Default for Client {
    fn default() -> Self {
        Client {
            bind: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0),
        }
    }
}

impl Client {
    #[inline]
    pub fn new(bind: SocketAddr) -> Self {
        Client { bind }
    }

    pub async fn query<A: ToSocketAddrs>(&self, addr: A) -> Result<Response, Error> {
        let socket = UdpSocket::bind(self.bind).await?;
        socket.connect(addr).await?;

        let mut req = Header::default();
        req.set_client();
        req.precision = 0x20;

        // To help prevent spoofing and client fingerprinting, use a
        // cryptographically random 64-bit value for the TransmitTime. See:
        // https://www.ietf.org/archive/id/draft-ietf-ntp-data-minimization-04.txt
        req.transmit_time = rand::random::<u64>();

        // Transmit the query and keep track of when it was transmitted.
        let transmit_time = Utc::now();
        let _sent = socket.send(req.as_bytes()).await?;

        // Receive the response
        let mut buf = [0u8; HEADER_SIZE];
        let _received = socket.recv(&mut buf).await?;

        // Keep track of the time the response was received.
        let mut delta = Utc::now() - transmit_time;
        if delta < Duration::zero() {
            delta = Duration::zero();
        }
        let recv_time = transmit_time.add(delta);

        let mut resp = Header::from_bytes(buf);

        // check for invalid fields
        if resp.mode() != 4 {
            return Err(Error::InvalidMode);
        }
        if resp.transmit_time == 0 {
            return Err(Error::InvalidTransmitTime);
        }
        if resp.origin_time != req.transmit_time {
            return Err(Error::ServerResponseMismatch);
        }
        if resp.receive_time > resp.transmit_time {
            return Err(Error::ServerTickedBackwards);
        }

        resp.origin_time = transmit_time.timestamp_nanos_opt().unwrap() as u64;

        Ok(generate_response(
            resp,
            recv_time.timestamp_nanos_opt().unwrap() as u64,
        ))
    }
}

/// **Packet Header** - The most important state variables from an external point of view are the
/// packet header variables described here.
///
/// The NTP packet header consists of an integral number of 32-bit (4 octet) words in network byte
/// order. The packet format consists of three components: the header itself, one or more optional
/// extension fields, and an optional message authentication code (MAC).
///
/// ```ignore
/// +-----------+------------+-----------------------+
/// | Name      | Formula    | Description           |
/// +-----------+------------+-----------------------+
/// | leap      | leap       | leap indicator (LI)   |
/// | version   | version    | version number (VN)   |
/// | mode      | mode       | mode                  |
/// | stratum   | stratum    | stratum               |
/// | poll      | poll       | poll exponent         |
/// | precision | rho        | precision exponent    |
/// | rootdelay | delta_r    | root delay            |
/// | rootdisp  | epsilon_r  | root dispersion       |
/// | refid     | refid      | reference ID          |
/// | reftime   | reftime    | reference timestamp   |
/// | org       | T1         | origin timestamp      |
/// | rec       | T2         | receive timestamp     |
/// | xmt       | T3         | transmit timestamp    |
/// | dst       | T4         | destination timestamp |
/// | keyid     | keyid      | key ID                |
/// | dgst      | dgst       | message digest        |
/// +-----------+------------+-----------------------+
/// ```
///
/// ### Format
///
/// The NTP packet is a UDP datagram [RFC0768]. Some fields use multiple words and others are
/// packed in smaller fields within a word. The NTP packet header shown below has 12 words followed
/// by optional extension fields and finally an optional message authentication code (MAC)
/// consisting of the Key Identifier field and Message Digest field.
///
/// ```ignore
///  0                   1                   2                   3
///  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |LI | VN  |Mode |    Stratum     |     Poll      |  Precision   |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                         Root Delay                            |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                         Root Dispersion                       |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                          Reference ID                         |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                                                               |
/// +                     Reference Timestamp (64)                  +
/// |                                                               |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                                                               |
/// +                      Origin Timestamp (64)                    +
/// |                                                               |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                                                               |
/// +                      Receive Timestamp (64)                   +
/// |                                                               |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                                                               |
/// +                      Transmit Timestamp (64)                  +
/// |                                                               |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                                                               |
/// .                                                               .
/// .                    Extension Field 1 (variable)               .
/// .                                                               .
/// |                                                               |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                                                               |
/// .                                                               .
/// .                    Extension Field 2 (variable)               .
/// .                                                               .
/// |                                                               |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                          Key Identifier                       |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                                                               |
/// |                            dgst (128)                         |
/// |                                                               |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// ```
#[repr(C)]
#[derive(Debug, PartialEq, Default)]
struct Header {
    lvm: u8, // leap indicator(2) + Version(3) + Mode(3)
    stratum: u8,
    poll: i8,
    precision: i8,
    root_delay: u32,
    root_dispersion: u32,
    reference_id: u32, // KoD code if Stratum == 0
    reference_time: u64,
    origin_time: u64,
    receive_time: u64,
    transmit_time: u64,
}

impl Header {
    fn set_client(&mut self) {
        let mut lvm = 3u8; // set mode to client
        lvm = (lvm & 0xc7) | 4u8 << 3; // set version to v4
        lvm &= 0x3f; // set leap to LeapNoWarning

        self.lvm = lvm;
    }

    #[inline]
    fn mode(&self) -> u8 {
        self.lvm & 0x07
    }

    #[inline]
    fn leap(&self) -> u8 {
        (self.lvm >> 6) & 0x03
    }

    fn from_bytes(buf: [u8; HEADER_SIZE]) -> Self {
        fn convert(ts: u64) -> u64 {
            let (frac, sec) = ((ts >> 32) as u32, ts as u32);
            let sec = sec.to_be() as u64 - NTP_EPOCH_OFFSET;
            let frac = (frac.to_be() as u64 * FRAC) >> 32;

            sec * NANOS_PER_SEC + ((frac * NANOS_PER_SEC) >> 32)
        }

        fn short_time(duration: u32) -> u32 {
            let duration = (duration.to_be() as i64) << 16;
            let sec = (duration >> 32) as u32;
            let nanos = (((duration & 0xffffffff) * 1000000000) >> 32) as u32;

            sec * NANOS_PER_SEC as u32 + nanos
        }

        let mut header: Header = unsafe { std::mem::transmute(buf) };

        header.root_delay = short_time(header.root_delay);
        header.root_dispersion = short_time(header.root_dispersion);

        header.reference_time = convert(header.reference_time);
        // header.origin_time = convert(header.origin_time);
        header.receive_time = convert(header.receive_time);
        header.transmit_time = convert(header.transmit_time);

        header
    }

    #[inline]
    fn as_bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self as *const _ as *const u8, HEADER_SIZE) }
    }
}

#[derive(Debug)]
pub struct Response {
    /// time is the transmit time reported by the server just before it
    /// responded to the client's NTP query. You should not use this value
    /// for time synchronization purposes. Use the ClockOffset instead.
    pub time: DateTime<Utc>,

    /// clock_offset is the estimated offset of the local system clock
    /// relative to the server's clock. Add this value to subsequent local
    /// system time measurements in order to obtain a more accurate time.
    pub clock_offset: Duration,

    /// rtt is the measured round-trip-time delay estimate between the client
    /// and the server.
    pub rtt: Duration,

    /// precision is the reported precision of the server's clock.
    pub precision: Duration,

    /// stratum is the "stratum level" of the server. The smaller the number,
    /// the closer the server is to the reference clock. Stratum 1 servers are
    /// attached directly to the reference clock. A stratum value of 0
    /// indicates the "kiss of death", which typically occurs when the client
    /// issues too many requests to the server in a short period of time.
    pub stratum: u8,

    /// reference_id is  a32bit integer identifying the server or reference clock.
    /// For stratum 1 servers, this is typically a meaningful zero-padded
    /// ASCII-encoded string assigned to the clock. For stratum 2+servers, this
    /// is a reference identifier for the server and is either the server's
    /// IPv4 address or a hash of its IPv6 address. For kiss-of-death responses
    /// (stratum 0), this is the ASCII-encoded "kiss code".
    pub reference_id: u32,

    /// reference_time is the time when the server's system clock was last
    /// set or corrected.
    pub reference_time: DateTime<Utc>,

    /// root_delay is the server's estimated aggregate round-trip-time delay to
    /// the stratum 1 server.
    pub root_delay: Duration,

    /// root_dispersion is the server's estimated maximum measurement error
    /// relative to the stratum 1 server.
    pub root_dispersion: Duration,

    /// root_distance is an estimate of the total synchronization distance
    /// between the client and the stratum 1 server.
    pub root_distance: Duration,

    /// Leap indicates whether a leap second should be added or removed from
    /// the current month's last minute.
    pub leap: u8,

    /// min_err is a lower bound on the error between the client and server
    /// clocks. When the client and server are not synchronized to the same
    /// clock, the reported timestamps may appear to violate the principle of
    /// causality. In other words, the NTP server's response may indicate
    /// that a message was received before it was sent. In such cases, the
    /// minimum error may be useful.
    pub min_err: Duration,

    /// kiss_code is a 4-character string describing the reason for a
    /// "kiss of death" response (stratum=0). For a list of standard kiss
    /// codes, see https://tools.ietf.org/html/rfc5905#section-7.4.
    pub kiss_code: String,

    /// Poll is the maximum interval between successive NTP query messages to
    /// the server.
    pub poll: Duration,
}

impl Response {
    pub fn validate(&self) -> Result<(), Error> {
        if self.stratum == 0 {
            return Err(Error::KissOfDeath);
        }
        if self.stratum >= 16 {
            return Err(Error::InvalidStratum);
        }

        // Estimate the "freshness" of the time. If it exceeds the maximum
        // polling interval (~36 hours), then it cannot be considered "fresh"
        let freshness = self.time - self.reference_time;
        if freshness > Duration::seconds(1 << 17) {
            return Err(Error::ServerClockFreshness);
        }

        // Calculate the peer synchronization distance, lambda:
        //      lambda = RootDelay / 2 + RootDispersion
        // If this value exceeds MAXDISP (16s), then the time is not suitable
        // for synchronization purposes.
        // https://tools.ietf.org/html/rfc5905#appendix-A.5.1.1.
        let lambda = self.root_delay / 2 + self.root_dispersion;
        if lambda > Duration::seconds(17) {
            return Err(Error::InvalidDispersion);
        }

        // If the server's transmit time is before its reference time,
        // the response is invalid.
        if self.time < self.reference_time {
            return Err(Error::InvalidTime);
        }

        // Handle invalid leap second indicator
        if self.leap == 3 {
            // LeapNotInSync
            return Err(Error::InvalidLeapSecond);
        }

        Ok(())
    }
}

#[allow(clippy::comparison_chain)]
fn to_interval(t: i8) -> Duration {
    let duration = if t > 0 {
        NANOS_PER_SEC << t
    } else if t < 0 {
        NANOS_PER_SEC >> -t
    } else {
        NANOS_PER_SEC
    };

    Duration::nanoseconds(duration as i64)
}

fn generate_response(header: Header, dst: u64) -> Response {
    let clock_offset = ((header.receive_time as i64 - header.origin_time as i64)
        + (header.transmit_time as i64 - dst as i64))
        / 2;
    let rtt = (dst - header.origin_time) - (header.transmit_time - header.receive_time);
    let precision = to_interval(header.precision);
    let poll = to_interval(header.poll);

    Response {
        time: nanos_to_timestamp(header.transmit_time),
        clock_offset: Duration::nanoseconds(clock_offset),
        rtt: Duration::nanoseconds(rtt as i64),
        precision,
        stratum: header.stratum,
        reference_id: header.reference_id,
        reference_time: nanos_to_timestamp(header.reference_time),
        root_delay: Duration::nanoseconds(header.root_delay as i64),
        root_dispersion: Duration::nanoseconds(header.root_dispersion as i64),
        root_distance: Duration::nanoseconds(root_distance(
            rtt,
            header.root_delay as u64,
            header.root_dispersion as u64,
        ) as i64),
        leap: header.leap(),
        min_err: Duration::nanoseconds(min_err(
            header.origin_time,
            header.receive_time,
            header.transmit_time,
            dst,
        ) as i64),
        kiss_code: if header.stratum == 0 {
            kiss_code(header.reference_id)
        } else {
            String::new()
        },
        poll,
    }
}

fn nanos_to_timestamp(nanos: u64) -> DateTime<Utc> {
    let secs = nanos.div_euclid(NANOS_PER_SEC) as i64;
    let nsecs = nanos.rem_euclid(NANOS_PER_SEC) as u32;

    DateTime::from_timestamp(secs, nsecs).expect("success")
}

fn min_err(org: u64, rec: u64, xmt: u64, dst: u64) -> u64 {
    let mut err0 = 0;
    let mut err1 = 0;

    if org >= rec {
        err0 = org - rec;
    }
    if xmt >= dst {
        err1 = xmt - dst;
    }

    if err0 > err1 {
        err0
    } else {
        err1
    }
}

fn kiss_code(id: u32) -> String {
    let b: [u8; 4] = id.to_ne_bytes();
    for ch in b {
        if !(32..=126).contains(&ch) {
            return String::new();
        }
    }

    String::from_utf8_lossy(&b).to_string()
}

fn root_distance(rtt: u64, root_delay: u64, root_disp: u64) -> u64 {
    // The root distance is:
    // 	the maximum error due to all causes of the local clock
    //	relative to the primary server. It is defined as half the
    //	total delay plus total dispersion plus peer jitter.
    //	(https://tools.ietf.org/html/rfc5905#appendix-A.5.5.2)
    //
    // In the reference implementation, it is calculated as follows:
    //	rootDist = max(MINDISP, rootDelay + rtt)/2 + rootDisp
    //			+ peerDisp + PHI * (uptime - peerUptime)
    //			+ peerJitter
    // For an SNTP client which sends only a single packet, most of these
    // terms are irrelevant and become 0.
    let total_delay = rtt + root_delay;
    total_delay / 2 + root_disp
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn size() {
        assert_eq!(size_of::<Header>(), HEADER_SIZE)
    }

    #[test]
    fn root_delay() {
        let raw = [
            36, 2, 0, 231, 0, 0, 0, 7, 0, 0, 0, 67, 10, 137, 53, 7, 233, 59, 20, 190, 146, 209,
            120, 174, 67, 59, 149, 100, 3, 231, 49, 219, 233, 59, 20, 203, 154, 45, 255, 127, 233,
            59, 20, 203, 154, 52, 167, 43,
        ];
        let header = Header::from_bytes(raw);
        assert_eq!(header.root_delay, 106811);
        /*
        &{Time:2023-12-30 22:14:35.602365921 +0000 UTC ClockOffset:4.718672ms RTT:27.706179ms Precision:29ns Stratum:2 ReferenceID:176764167 ReferenceTime:2023-12-30 22:14:22.57350878 +0000 UTC RootDelay:106.812µs RootDispersion:1.022339ms RootDistance:14.928834ms Leap:0 MinError:0s KissCode: Poll:1s authErr:<nil>}
        */
    }
}
