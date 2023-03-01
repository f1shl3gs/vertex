use std::fs::Permissions;
use std::io;
use std::io::{Cursor, Read, Write};
use std::net::Shutdown;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use bytes::Buf;
use configurable::configurable_component;
use event::{tags, Metric};
use framework::config::{default_interval, DataType, Output, SourceConfig, SourceContext};
use framework::{Pipeline, ShutdownSignal, Source};
use tokio::net::UnixDatagram;
use tokio::time::timeout;

const BUFFER_SIZE: usize = 104;

const PROTOCOL_VERSION: u8 = 6;
const PACKET_REQUEST: u8 = 1;
const PACKET_RESPONSE: u8 = 2;

fn default_address() -> PathBuf {
    "/var/run/chrony/chronyd.sock".into()
}

const fn default_timeout() -> Duration {
    Duration::from_secs(10)
}

/// The [chrony] source is a pure rust implementation of the command
/// `chronyc tracking` to allow for portability across systems and
/// platforms. All of the data that would typically be captured by
/// the tracking command is made available in this source.
#[configurable_component(source, name = "chrony")]
#[derive(Clone, Debug)]
struct Config {
    /// The Address on where to communicate to `chronyd`.
    ///
    /// Make sure vertex has the right permission to access this address.
    #[serde(default = "default_address")]
    #[configurable(required)]
    address: PathBuf,

    /// How frequent this source should poll.
    #[serde(with = "humanize::duration::serde", default = "default_interval")]
    interval: Duration,

    /// The total amount of time allowed to read and process the data from chronyd.
    #[serde(with = "humanize::duration::serde", default = "default_timeout")]
    timeout: Duration,
}

#[async_trait]
#[typetag::serde(name = "chrony")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> framework::Result<Source> {
        let shutdown = cx.shutdown;
        let output = cx.output;

        Ok(Box::pin(run(self.clone(), output, shutdown)))
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Metric)]
    }
}

async fn run(config: Config, mut output: Pipeline, mut shutdown: ShutdownSignal) -> Result<(), ()> {
    let mut ticker = tokio::time::interval(config.interval);

    loop {
        tokio::select! {
            biased;

            _ = &mut shutdown => break,
            _ = ticker.tick() => {}
        }

        let start = Instant::now();
        let result = match timeout(config.timeout, scrape(&config.address)).await {
            Ok(result) => result,
            Err(_err) => Err(Error::Timeout),
        };
        let elapsed = start.elapsed().as_secs_f64();
        let up = result.is_ok();
        let mut metrics = vec![
            Metric::gauge("ntp_up", "Could the chronyd be accessible", up),
            Metric::gauge("ntp_scrape_duration_seconds", "", elapsed),
        ];

        match result {
            Ok(scraped) => metrics.extend(scraped),
            Err(err) => {
                warn!(message = "scrape failed", ?err);
            }
        }

        if let Err(err) = output.send(metrics).await {
            warn!(message = "Send chrony metrics to closed output", ?err);
            break;
        }
    }

    Ok(())
}

#[derive(Debug)]
enum Error {
    Io(io::Error),

    Timeout,

    Protocol(String),
}

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Error::Io(value)
    }
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
    stream: UnixDatagram,
}

impl Drop for Client {
    fn drop(&mut self) {
        if let Ok(addr) = self.stream.local_addr() {
            if let Some(path) = addr.as_pathname() {
                let _ = self.stream.shutdown(Shutdown::Both);
                let _ = std::fs::remove_file(path);
            }
        }
    }
}

impl Client {
    fn connect(addr: &PathBuf) -> io::Result<Self> {
        let pid = std::process::id();
        let path = format!("/var/run/chrony/vertex-{pid}.sock");

        let stream = UnixDatagram::bind(&path)?;
        info!("binded");

        std::fs::set_permissions(&path, Permissions::from_mode(0o777))?;

        stream.connect(addr)?;
        info!("connected");

        Ok(Self { stream })
    }

    async fn get_tracking_data(&self) -> Result<TrackingResponse, Error> {
        info!("gtd");
        let req = TrackingRequest::new();
        let data = req.encode()?;
        info!("data len {}", data.len());
        let n = self.stream.send(&data).await?;
        info!("write {} bytes", n);

        let mut resp = [0u8; BUFFER_SIZE];
        let n = self.stream.recv(&mut resp).await?;
        info!("read {} bytes", n);

        TrackingResponse::decode(Cursor::new(resp)).map_err(Into::into)
    }
}

async fn scrape(addr: &PathBuf) -> Result<Vec<Metric>, Error> {
    /*    let tracking = get_tracking_data(addr).await?; */

    let client = Client::connect(addr)?;
    let tracking = client.get_tracking_data().await?;
    let tags = tags!(
        "leap_status" => tracking.leap_status()
    );

    Ok(vec![
        Metric::gauge(
            "ntp_stratum",
            "The number of hops away from the reference system keeping the reference time",
            tracking.stratum,
        ),
        Metric::gauge_with_tags(
            "ntp_time_correction_seconds",
            "The number of seconds difference between the system's clock and the reference clock",
            tracking.current_correction,
            tags.clone()
        ),
        Metric::gauge_with_tags(
            "ntp_time_last_offset_seconds",
            "The estimated local offset on the last clock update",
            tracking.last_offset,
            tags.clone()
        ),
        Metric::gauge_with_tags(
            "ntp_time_rms_offset_seconds",
            "the long term average of the offset value",
            tracking.rms_offset,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "ntp_frequency_offset_ppm",
            "The frequency is the rate by which the system s clock would be wrong if chronyd was not correcting it.",
            tracking.freq_ppm,
            tags.clone()
        ),
        Metric::gauge_with_tags(
            "ntp_skew_ppm",
            "This is the estimated error bound on the frequency.",
            tracking.skew_ppm,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "ntp_time_root_delay_seconds",
            "This is the total of the network path delays to the stratum-1 system from which the system is ultimately synchronised.",
            tracking.root_delay,
            tags
        )
    ])
}

trait WriteExt: Write {
    fn write_u8(&mut self, n: u8) -> io::Result<usize> {
        self.write(&n.to_be_bytes())
    }

    fn write_u16(&mut self, n: u16) -> io::Result<usize> {
        self.write(&n.to_be_bytes())
    }

    fn write_u32(&mut self, n: u32) -> io::Result<usize> {
        self.write(&n.to_be_bytes())
    }

    fn write_u64(&mut self, n: u64) -> io::Result<usize> {
        self.write(&n.to_be_bytes())
    }
}

impl<T: Write> WriteExt for T {}

struct TrackingRequest {
    /// Protocol version.
    version: u8,
    /// What sort of packet this is
    pkt_type: u8,
    _res1: u8,
    _res2: u8,
    command: u16, // always 33
    attempt: u16,

    /// Client's sequence number.
    sequence: u32,
    _pad1: u32,
    _pad2: u32,
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
                } data; /* Command specific parameters */

                /* Padding used to prevent traffic amplification.  It only defines the
                   maximum size of the packet, there is no hole after the data field. */
                uint8_t padding[MAX_PADDING_LENGTH];

            } CMD_Request;
        */

        let mut w = Cursor::new([0u8; BUFFER_SIZE]);

        w.write_u8(self.version)?;
        w.write_u8(self.pkt_type)?;
        w.write_u16(0)?; // _res1 & _res2
        w.write_u16(self.command)?; // command
        w.write_u16(self.attempt)?;
        w.write_u32(self.sequence)?;
        w.write_u64(0)?; // _pad1 & _pad2

        Ok(w.into_inner())
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

trait ReadExt: Read {
    fn read_u8(&mut self) -> io::Result<u8> {
        let mut buf = [0u8; 1];
        self.read_exact(&mut buf)?;
        Ok(u8::from_be_bytes(buf))
    }

    fn read_u16(&mut self) -> io::Result<u16> {
        let mut buf = [0u8; 2];
        self.read_exact(&mut buf)?;
        Ok(u16::from_be_bytes(buf))
    }

    fn read_u32(&mut self) -> io::Result<u32> {
        let mut buf = [0u8; 4];
        self.read_exact(&mut buf)?;
        Ok(u32::from_be_bytes(buf))
    }

    fn read_i32(&mut self) -> io::Result<i32> {
        let mut buf = [0u8; 4];
        self.read_exact(&mut buf)?;
        Ok(i32::from_be_bytes(buf))
    }
}

impl<T: Read> ReadExt for T {}

#[allow(dead_code)]
struct TrackingResponse {
    // head
    version: u8,
    pkt_type: u8,
    command: u16,
    reply: u16,
    status: u16,
    sequence: u32,

    // content
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
    fn decode(mut r: impl ReadExt + Buf) -> Result<Self, Error> {
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
        let version = r.read_u8()?;
        if version != PROTOCOL_VERSION {
            return Err(format!("unknown protocol version {version}").into());
        }

        let pkt_type = r.read_u8()?;
        if pkt_type != PACKET_RESPONSE {
            return Err("expect to receive response packet".into());
        }

        let _ = r.read_u16()?;
        let command = r.read_u16()?;
        let reply = r.read_u16()?;
        if reply != 5 {
            // https://github.com/mlichvar/chrony/blob/4.3/candm.h#L502
            return Err(format!("unknown reply code from chronyd: {reply}").into());
        }

        let status = r.read_u16()?;
        if status != 0 {
            return Err("request failed".into());
        }

        let _pad1 = r.read_u16()?;
        let _pad2 = r.read_u16()?;
        let _pad3 = r.read_u16()?;
        let sequence = r.read_u32()?;
        let _pad4 = r.read_u32()?;
        let _pad5 = r.read_u32()?;

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
        // N.B. chrony's Float is i32 not normal f64, see: https://github.com/mlichvar/chrony/blob/master/candm.h#L125
        let ref_id = r.read_u32()?;
        r.advance(16 + 2 + 2); // ip_addr
        let stratum = r.read_u16()?;
        let leap_status = r.read_u16()?;
        r.advance(3 * 4); // Timespec: 3 * u32
        let current_correction = to_std_f32(r.read_i32()?);
        let last_offset = to_std_f32(r.read_i32()?);
        let rms_offset = to_std_f32(r.read_i32()?);
        let freq_ppm = to_std_f32(r.read_i32()?);
        let _resid_freq_ppm = r.read_i32()?;
        let skew_ppm = to_std_f32(r.read_i32()?);
        let root_delay = to_std_f32(r.read_i32()?);
        let _root_dispersion = r.read_i32()?;
        let _last_update_interval = r.read_i32()?;

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
        crate::testing::test_generate_config::<Config>()
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
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
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
        //
        // ReplyHead: ReplyHead{
        //     Version:  protoVersionNumber,
        //     PKTType:  pktTypeCmdReply,
        //     Res1:     0,
        //     Res2:     0,
        //     Command:  reqTracking,
        //     Reply:    rpyTracking,
        //     Status:   sttSuccess,
        //     Sequence: 2,
        // },
        // Tracking: Tracking{
        // 	   RefID:              3861235310,
        // 	   IPAddr:             net.IP{36, 1, 219, 0, 49, 16, 33, 50, 250, 206, 0, 0, 0, 142, 0, 0},
        // 	   Stratum:            3,
        // 	   LeapStatus:         0,
        // 	   RefTime:            time.Unix(0, 1631117697915705301),
        // 	   CurrentCorrection:  -3.4395072816550964e-06,
        // 	   LastOffset:         -2.823539716700907e-06,
        // 	   RMSOffset:          1.405413968313951e-05,
        // 	   FreqPPM:            -1.5478190183639526,
        // 	   ResidFreqPPM:       -0.00012660636275541037,
        // 	   SkewPPM:            0.005385049618780613,
        // 	   RootDelay:          0.00022063794312998652,
        // 	   RootDispersion:     0.0010384710039943457,
        // 	   LastUpdateInterval: 520.4907836914062,
        // },

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
