use std::io;
use std::io::Cursor;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use bytes::Buf;
use configurable::{Configurable, configurable_component};
use event::{Metric, tags};
use framework::config::{OutputType, SourceConfig, SourceContext, default_interval};
use framework::{Pipeline, ShutdownSignal, Source};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UdpSocket, UnixSocket};
use tokio::time::timeout;

const BUFFER_SIZE: usize = 108;

const PROTOCOL_VERSION: u8 = 6;
const PACKET_REQUEST: u8 = 1;
const PACKET_RESPONSE: u8 = 2;

#[derive(Clone, Debug, Deserialize, Serialize, Configurable)]
#[serde(untagged)]
enum ServerAddr {
    // 127.0.0.1:323
    SocketAddr(SocketAddr),

    // /var/run/chrony/chronyd.sock
    Unix(PathBuf),
}

impl Default for ServerAddr {
    fn default() -> Self {
        ServerAddr::SocketAddr(SocketAddr::from(([127, 0, 0, 1], 323)))
    }
}

const fn default_timeout() -> Duration {
    Duration::from_secs(10)
}

/// The [chrony] source is a pure rust implementation of the command
/// `chronyc tracking` to allow for portability across systems and
/// platforms. All of the data that would typically be captured by
/// the tracking command is made available in this source.
#[configurable_component(source, name = "chrony")]
#[derive(Clone)]
struct Config {
    /// The Address on where to communicate to `chronyd`.
    ///
    /// Make sure vertex has the right permission to access this address.
    #[serde(default)]
    #[configurable(required)]
    address: ServerAddr,

    /// How frequent this source should poll.
    #[serde(with = "humanize::duration::serde", default = "default_interval")]
    interval: Duration,

    /// The total amount of time allowed to read and process the data from chronyd.
    #[serde(with = "humanize::duration::serde", default = "default_timeout")]
    timeout: Duration,
}

#[async_trait::async_trait]
#[typetag::serde(name = "chrony")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> framework::Result<Source> {
        let shutdown = cx.shutdown;
        let output = cx.output;

        Ok(Box::pin(run(self.clone(), output, shutdown)))
    }

    fn outputs(&self) -> Vec<OutputType> {
        vec![OutputType::metric()]
    }

    fn can_acknowledge(&self) -> bool {
        false
    }
}

async fn run(config: Config, mut output: Pipeline, mut shutdown: ShutdownSignal) -> Result<(), ()> {
    let client = Client::new(config.address);
    let mut ticker = tokio::time::interval(config.interval);

    loop {
        tokio::select! {
            biased;

            _ = &mut shutdown => break,
            _ = ticker.tick() => {}
        }

        let start = Instant::now();
        let result = timeout(config.timeout, scrape(&client))
            .await
            .unwrap_or_else(|_err| Err(Error::Timeout));
        let elapsed = start.elapsed();
        let up = result.is_ok();
        let mut metrics = vec![
            Metric::gauge("chrony_up", "Could the chronyd be accessible", up),
            Metric::gauge("chrony_scrape_duration_seconds", "", elapsed),
        ];

        match result {
            Ok(scraped) => metrics.extend(scraped),
            Err(err) => {
                warn!(message = "scrape failed", %err);
            }
        }

        if let Err(err) = output.send(metrics).await {
            warn!(message = "Send chrony metrics to closed output", %err);
            break;
        }
    }

    Ok(())
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),

    #[error("timeout")]
    Timeout,

    #[error("protocol error {0}")]
    Protocol(String),
}

impl From<&'static str> for Error {
    fn from(value: &'static str) -> Self {
        Error::Protocol(value.to_string())
    }
}

impl From<String> for Error {
    fn from(value: String) -> Self {
        Error::Protocol(value)
    }
}

struct Client {
    addr: ServerAddr,
}

impl Client {
    fn new(addr: ServerAddr) -> Self {
        Client { addr }
    }

    async fn get_tracking_data(&self) -> Result<TrackingResponse, Error> {
        let req = TrackingRequest::new();
        let mut buf = req.encode()?;

        match &self.addr {
            ServerAddr::SocketAddr(addr) => {
                // UNSPECIFIED
                let bind_addr = SocketAddr::from(([0, 0, 0, 0], 0));
                let sock = UdpSocket::bind(bind_addr).await?;
                sock.connect(addr).await?;

                // tracking request does not have payload, but we still need
                // some space to make sure chronyd can handle it properly.
                sock.send(&buf).await?;
                sock.recv(&mut buf).await?;
            }
            ServerAddr::Unix(path) => {
                let sock = UnixSocket::new_datagram()?;
                let mut stream = sock.connect(path).await?;

                stream.write_all(&buf).await?;
                stream.read_exact(&mut buf).await?;
            }
        }

        TrackingResponse::decode(Cursor::new(buf))
    }
}

async fn scrape(client: &Client) -> Result<Vec<Metric>, Error> {
    let tracking = client.get_tracking_data().await?;
    let tags = tags!(
        "leap_status" => tracking.leap_status()
    );

    Ok(vec![
        Metric::gauge(
            "chrony_stratum",
            "The number of hops away from the reference system keeping the reference time",
            tracking.stratum,
        ),
        Metric::gauge_with_tags(
            "chrony_time_correction_seconds",
            "The number of seconds difference between the system's clock and the reference clock",
            tracking.current_correction,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "chrony_time_last_offset_seconds",
            "The estimated local offset on the last clock update",
            tracking.last_offset,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "chrony_time_rms_offset_seconds",
            "the long term average of the offset value",
            tracking.rms_offset,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "chrony_frequency_offset_ppm",
            "The frequency is the rate by which the system s clock would be wrong if chronyd was not correcting it.",
            tracking.freq_ppm,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "chrony_skew_ppm",
            "This is the estimated error bound on the frequency.",
            tracking.skew_ppm,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "chrony_time_root_delay_seconds",
            "This is the total of the network path delays to the stratum-1 system from which the system is ultimately synchronised.",
            tracking.root_delay,
            tags,
        ),
    ])
}

struct TrackingRequest {
    // Header
    version: u8,  // Protocol version.
    pkt_type: u8, // What sort of packet this is
    _res1: u8,
    _res2: u8,
    command: u16,  // which command is being issued, always 33 for our use case
    attempt: u16,  // how many resends the client has done
    sequence: u32, // client's sequence number
    _pad1: u32,
    _pad2: u32,
    // no payload
}

impl TrackingRequest {
    const fn new() -> Self {
        Self {
            version: PROTOCOL_VERSION,
            pkt_type: PACKET_REQUEST, // 1 for request
            _res1: 0,
            _res2: 0,
            command: 33,
            attempt: 1,
            sequence: 1,
            _pad1: 0,
            _pad2: 0,
        }
    }

    fn encode(self) -> Result<[u8; BUFFER_SIZE], Error> {
        /*
        typedef struct {
          uint8_t version; /* Protocol version */
          uint8_t pkt_type; /* What sort of packet this is */
          uint8_t res1;
          uint8_t res2;
          uint16_t command; /* Which command is being issued */
          uint16_t attempt; /* How many resends the client has done
                                     (count up from zero for same sequence
                                     number) */
          uint32_t sequence; /* Client's sequence number */
          uint32_t pad1;
          uint32_t pad2;

          union {
            REQ_Null null;
            REQ_Online online;
            REQ_Offline offline;
            REQ_Burst burst;
            REQ_Modify_Minpoll modify_minpoll;
            REQ_Modify_Maxpoll modify_maxpoll;
            REQ_Dump dump;
            REQ_Modify_Maxdelay modify_maxdelay;
            REQ_Modify_Maxdelayratio modify_maxdelayratio;
            REQ_Modify_Maxdelaydevratio modify_maxdelaydevratio;
            REQ_Modify_Minstratum modify_minstratum;
            REQ_Modify_Polltarget modify_polltarget;
            REQ_Modify_Maxupdateskew modify_maxupdateskew;
            REQ_Modify_Makestep modify_makestep;
            REQ_Logon logon;
            REQ_Settime settime;
            REQ_Local local;
            REQ_Manual manual;
            REQ_Source_Data source_data;
            REQ_Allow_Deny allow_deny;
            REQ_Ac_Check ac_check;
            REQ_NTP_Source ntp_source;
            REQ_Del_Source del_source;
            REQ_Dfreq dfreq;
            REQ_Doffset doffset;
            REQ_Sourcestats sourcestats;
            REQ_ClientAccessesByIndex client_accesses_by_index;
            REQ_ManualDelete manual_delete;
            REQ_ReselectDistance reselect_distance;
            REQ_SmoothTime smoothtime;
            REQ_NTPData ntp_data;
            REQ_NTPSourceName ntp_source_name;
            REQ_AuthData auth_data;
            REQ_SelectData select_data;
            REQ_SelectData select_data;
            REQ_Modify_SelectOpts modify_select_opts;
            REQ_Modify_Offset modify_offset;
          } data; /* Command specific parameters */

          /* Padding used to prevent traffic amplification.  It only defines the
             maximum size of the packet, there is no hole after the data field. */
          uint8_t padding[MAX_PADDING_LENGTH];

          } CMD_Request;
        */

        let mut buf = [0u8; BUFFER_SIZE];

        buf[..1].copy_from_slice(&self.version.to_be_bytes());
        buf[1..2].copy_from_slice(&self.pkt_type.to_be_bytes());
        // _res1 & _res2 is zero so we don't need to set the value
        buf[4..6].copy_from_slice(&self.command.to_be_bytes());
        buf[6..8].copy_from_slice(&self.attempt.to_be_bytes());
        buf[8..12].copy_from_slice(&self.sequence.to_be_bytes());
        // _pad1 & _pad2 is zero so we don't need to set the value

        Ok(buf)
    }
}

// magic numbers to convert chrony Float to normal float
const FLOAT_EXP_BITS: u32 = 7;
const FLOAT_COEF_BITS: u32 = 4 * 8 - 7;

/// chrony Float's underlying data is i32.
///
/// 32-bit floating-point format consisting of 7-bit signed exponent
/// and 25-bit signed coefficient without hidden bit.
/// The result is calculated as: 2^(exp - 25) * coef
///
/// https://github.com/mlichvar/chrony/blob/master/candm.h#L128
fn to_std_f32(v: i32) -> f32 {
    let x = v as u32;

    let mut exp = (x >> FLOAT_COEF_BITS) as i32;
    if exp >= 1 << (FLOAT_EXP_BITS - 1) {
        exp -= 1 << FLOAT_EXP_BITS;
    }

    exp -= FLOAT_COEF_BITS as i32;

    let mut coef = (x % (1 << FLOAT_COEF_BITS)) as i32;
    if coef >= 1 << (FLOAT_COEF_BITS - 1) {
        coef -= 1 << FLOAT_COEF_BITS;
    }

    coef as f32 * 2.0f32.powf(exp as f32)
}

#[repr(C)]
struct TrackingResponse {
    // header
    version: u8,
    pkt_type: u8,
    command: u16,  // Which command is being replied to
    reply: u16,    // which format of replay this is
    status: u16,   // status of command processing
    sequence: u32, // Echo of client's sequence number

    // payload
    ref_id: u32,
    stratum: u16,
    leap_status: u16,
    current_correction: f32,
    last_offset: f32,
    rms_offset: f32,
    freq_ppm: f32,
    skew_ppm: f32,
    root_delay: f32,
}

impl TrackingResponse {
    fn decode(mut r: impl Buf) -> Result<Self, Error> {
        // decode header first, see: https://github.com/mlichvar/chrony/blob/master/candm.h#L797
        //
        // typedef struct {
        //   uint8_t version;
        //   uint8_t pkt_type;
        //   uint8_t res1;
        //   uint8_t res2;
        //   uint16_t command; /* Which command is being replied to */
        //   uint16_t reply; /* Which format of reply this is */
        //   uint16_t status; /* Status of command processing */
        //   uint16_t pad1; /* Padding for compatibility and 4 byte alignment */
        //   uint16_t pad2;
        //   uint16_t pad3;
        //   uint32_t sequence; /* Echo of client's sequence number */
        //   uint32_t pad4;
        //   uint32_t pad5;
        //
        //   union {
        //     RPY_Null null;
        //     RPY_N_Sources n_sources;
        //     RPY_Source_Data source_data;
        //     RPY_ManualTimestamp manual_timestamp;
        //     RPY_Tracking tracking;
        //     RPY_Sourcestats sourcestats;
        //     RPY_Rtc rtc;
        //     RPY_ClientAccessesByIndex client_accesses_by_index;
        //     RPY_ServerStats server_stats;
        //     RPY_ManualList manual_list;
        //     RPY_Activity activity;
        //     RPY_Smoothing smoothing;
        //     RPY_NTPData ntp_data;
        //     RPY_NTPSourceName ntp_source_name;
        //     RPY_AuthData auth_data;
        //     RPY_SelectData select_data;
        //   } data; /* Reply specific parameters */
        //
        // } CMD_Reply;
        let version = r.get_u8();
        if version != PROTOCOL_VERSION {
            return Err(format!("unknown protocol version {version}").into());
        }

        let pkt_type = r.get_u8();
        if pkt_type != PACKET_RESPONSE {
            return Err("expect to receive response packet".into());
        }

        let _ = r.get_u16();
        let command = r.get_u16();
        let reply = r.get_u16();
        if reply != 5 {
            // https://github.com/mlichvar/chrony/blob/4.3/candm.h#L502
            return Err(format!("unknown reply code from chronyd: {reply}").into());
        }

        let status = r.get_u16();
        if status != 0 {
            return Err("request failed".into());
        }

        // skip pad1, pad2 and pad3
        r.advance(3 * 2);
        let sequence = r.get_u32();
        // skip pad4 and pad5
        r.advance(2 * 4);

        // decode content part, see: https://github.com/mlichvar/chrony/blob/master/candm.h#L593
        //
        // typedef struct {
        //   uint32_t ref_id;
        //   IPAddr ip_addr;     ip(16 * u8) + family(u16) + pad(u16)
        //   uint16_t stratum;
        //   uint16_t leap_status;
        //   Timespec ref_time;
        //   Float current_correction;
        //   Float last_offset;
        //   Float rms_offset;
        //   Float freq_ppm;
        //   Float resid_freq_ppm;
        //   Float skew_ppm;
        //   Float root_delay;
        //   Float root_dispersion;
        //   Float last_update_interval;
        //   int32_t EOR;
        // } RPY_Tracking;
        //
        // N.B. chrony's Float is i32 not normal f64,
        // see: https://github.com/mlichvar/chrony/blob/master/candm.h#L125
        let ref_id = r.get_u32();
        r.advance(16 + 2 + 2); // ip_addr
        let stratum = r.get_u16();
        let leap_status = r.get_u16();
        r.advance(3 * 4); // Timespec: 3 * u32
        let current_correction = to_std_f32(r.get_i32());
        let last_offset = to_std_f32(r.get_i32());
        let rms_offset = to_std_f32(r.get_i32());
        let freq_ppm = to_std_f32(r.get_i32());
        // skip resid_freq_ppm
        r.advance(4);
        let skew_ppm = to_std_f32(r.get_i32());
        let root_delay = to_std_f32(r.get_i32());
        // skip root_dispersion and last_update_interval
        r.advance(2 * 4);

        Ok(Self {
            // head
            version,
            pkt_type,
            command,
            reply,
            status,
            sequence,

            // content
            ref_id,
            stratum,
            leap_status,
            current_correction,
            last_offset,
            rms_offset,
            freq_ppm,
            skew_ppm,
            root_delay,
        })
    }

    const fn leap_status(&self) -> &'static str {
        match self.leap_status + 1 {
            1 => "normal",
            2 => "insert_second",
            3 => "delete_second",
            4 => "unsynchronised",
            _ => "unknown",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>()
    }

    #[test]
    fn to_f32() {
        let precision = 0.000001;

        for (input, want) in [(0, 0.0), (17091950, -0.490620), (-90077357, 0.039435696)] {
            let got = to_std_f32(input);

            let delta = want - got;
            assert!(delta >= -precision && delta < precision);
        }
    }

    #[test]
    fn encode() {
        let want = [
            0x06, 0x01, 0x00, 0x00, 0x00, 0x21, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let req = TrackingRequest::new();
        let got = req.encode().unwrap();
        assert_eq!(want, got);
    }

    #[test]
    #[allow(clippy::excessive_precision)]
    fn decode() {
        // This test ported from
        // https://github.com/facebook/time/blob/main/ntp/chrony/packet_test.go#L151
        let data = [
            0x06, 0x02, 0x00, 0x00, 0x00, 0x21, 0x00, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0xe6, 0x25, 0xc6, 0x6e, 0x24, 0x01, 0xdb, 0x00, 0x31, 0x10, 0x21, 0x32, 0xfa, 0xce,
            0x00, 0x00, 0x00, 0x8e, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x61, 0x38, 0xe1, 0x81, 0x36, 0x94, 0x8d, 0xd5, 0xdf, 0x19,
            0x2d, 0xb7, 0xdf, 0x42, 0x83, 0xf5, 0xe2, 0xeb, 0xca, 0x12, 0x05, 0x39, 0xe1, 0x11,
            0xeb, 0x7b, 0x3e, 0x5d, 0xf4, 0xb0, 0x75, 0x12, 0xea, 0xe7, 0x5b, 0x0c, 0xf0, 0x88,
            0x1d, 0x4e, 0x16, 0x82, 0x1f, 0x69,
        ];

        let tracking = TrackingResponse::decode(Cursor::new(data)).expect("ok");
        assert_eq!(tracking.command, 33);
        assert_eq!(tracking.reply, 5);
        assert_eq!(tracking.status, 0);
        assert_eq!(tracking.sequence, 2);

        assert_eq!(tracking.ref_id, 3861235310);
        assert_eq!(tracking.stratum, 3);
        assert_eq!(tracking.leap_status, 0);
        assert_eq!(tracking.current_correction, -3.4395072816550964e-06);
        assert_eq!(tracking.last_offset, -2.823539716700907e-06);
        assert_eq!(tracking.rms_offset, 1.405413968313951e-05);
        assert_eq!(tracking.freq_ppm, -1.5478190183639526);
        assert_eq!(tracking.skew_ppm, 0.005385049618780613);
        assert_eq!(tracking.root_delay, 0.00022063794312998652);
    }
}
