use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use tokio::io::unix::AsyncFd;

const REQUEST: u16 = 1;
const DUMP: u16 = 0x100 | 0x200;

/// A multi-part message, terminated by Done on the last message
const FLAG_MULTI: u16 = 2;

const CMD_GET_INTERFACE: u8 = 0x5;
const CMD_GET_SCAN: u8 = 0x20;
const CMD_GET_STATION: u8 = 0x11;

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Default)]
pub enum InterfaceType {
    // InterfaceTypeUnspecified indicates that an interface's type is unspecified
    // and the driver determines its function.
    #[default]
    Unspecified = 0,

    // AdHoc indicates that an interface is part of an independent
    // basic service set (BSS) of client devices without a controlling access
    // point.
    AdHoc = 1,

    // Station indicates that an interface is part of a managed
    // basic service set (BSS) of client devices with a controlling access point.
    Station = 2,

    // AP indicates that an interface is an access point.
    AP = 3,

    // APVLAN indicates that an interface is a VLAN interface
    // associated with an access point.
    APVLAN = 4,

    // WDS indicates that an interface is a wireless distribution
    // interface, used as part of a network of multiple access points.
    WDS = 5,

    // Monitor indicates that an interface is a monitor interface,
    // receiving all frames from all clients in a given network.
    Monitor = 6,

    // MeshPoint indicates that an interface is part of a wireless
    // mesh network.
    MeshPoint = 7,

    // P2PClient indicates that an interface is a client within
    // a peer-to-peer network.
    P2PClient = 8,

    // P2PGroupOwner indicates that an interface is the group
    // owner within a peer-to-peer network.
    P2PGroupOwner = 9,

    // P2PDevice indicates that an interface is a device within
    // a peer-to-peer client network.
    P2PDevice = 10,

    // OCB indicates that an interface is outside the context
    // of a basic service set (BSS).
    OCB = 11,

    // NAN indicates that an interface is part of a near-me
    // area network (NAN).
    NAN = 12,
}

impl InterfaceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            InterfaceType::Unspecified => "unspecified",
            InterfaceType::AdHoc => "ad-hoc",
            InterfaceType::Station => "station",
            InterfaceType::AP => "access point",
            InterfaceType::APVLAN => "access point with VLAN",
            InterfaceType::WDS => "wireless distribution",
            InterfaceType::Monitor => "monitor",
            InterfaceType::MeshPoint => "mesh point",
            InterfaceType::P2PClient => "P2P client",
            InterfaceType::P2PGroupOwner => "P2P group owner",
            InterfaceType::P2PDevice => "P2P device",
            InterfaceType::OCB => "outside context of BSS",
            InterfaceType::NAN => "near-me area network",
        }
    }
}

impl From<u32> for InterfaceType {
    fn from(value: u32) -> Self {
        match value {
            1 => InterfaceType::AdHoc,
            2 => InterfaceType::Station,
            3 => InterfaceType::AP,
            4 => InterfaceType::APVLAN,
            5 => InterfaceType::WDS,
            6 => InterfaceType::Monitor,
            7 => InterfaceType::MeshPoint,
            8 => InterfaceType::P2PClient,
            9 => InterfaceType::P2PGroupOwner,
            10 => InterfaceType::P2PDevice,
            11 => InterfaceType::OCB,
            12 => InterfaceType::NAN,
            _ => InterfaceType::Unspecified,
        }
    }
}

/// A Wi-Fi network interface
#[derive(Debug, Default)]
pub struct Interface {
    /// The index of the interface
    pub index: u32,

    /// The name of the interface
    pub name: String,

    /// The hardware address of the interface
    pub mac: Vec<u8>,

    /// The physical device that this interface belongs to
    pub phy: u32,

    /// The virtual device number of this interface within a PHY
    pub device: u64,

    /// The operating mode of the interface
    pub typ: InterfaceType,

    /// The interface's wireless frequency in MHz
    pub frequency: u32,
}

const NL80211_BSS_BSSID: u16 = 0x1;
const NL80211_BSS_FREQUENCY: u16 = 0x2;
const NL80211_BSS_BEACON_INTERVAL: u16 = 0x4;
const NL80211_BSS_INFORMATION_ELEMENTS: u16 = 0x6;
const NL80211_BSS_STATUS: u16 = 0x9;
const NL80211_BSS_SEEN_MS_AGO: u16 = 0xa;

/// A BSS is an 802.11 basic service set. It contains information about a
/// wireless network associated with an Interface
#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Default)]
pub struct BSS {
    /// The service set identifier, or "network name"
    pub ssid: String,

    /// In infrastructure mode, this is the hardware address of the wireless
    /// access point that a client is associated with.
    pub mac: Vec<u8>,

    /// The frequency used by the BSS, in Mhz
    pub frequency: u32,

    /// The time interval between beacon transmissions for this BSS
    pub beacon_interval: Duration,

    /// The time since the client last scanned this BSS's information.
    pub last_seen: Duration,

    /// The status of the client within the BSS
    pub status: u32,
}

/// Statistics about a WiFi interface operating in station mode
#[derive(Debug, Default)]
pub struct StationInfo {
    /// The time since the station last connected
    pub connected: Duration,

    /// The hardware address of the station
    pub mac: Vec<u8>,

    /// The time since wireless activity last occurred
    pub inactive: Duration,

    /// The current data receive bitrate, in bits/second
    pub receive_bitrate: u32,

    /// The current data transmit bitrate, in bits/second
    pub transmit_bitrate: u32,

    /// The number of bytes received by this station
    pub received_bytes: u64,

    /// The number of bytes transmitted by this station
    pub transmitted_bytes: u64,

    /// The signal strength of the last received PPDU, in dBm
    pub signal: i32,

    /// The number of times the station has had to retry while sending a packet
    pub transmit_retries: u32,

    /// The number of times a packet transmission failed
    pub transmit_failed: u32,

    /// The number of times a beacon loss was detected
    pub beacon_loss: u32,

    /// The number of packets transmitted by this station
    pub transmitted_packets: u32,

    /// The number of packets received by this station
    pub received_packets: u32,
}

// No action was taken
// const HEADER_TYPE_NOOP: u16 = 1;
/// An error code is present, which is also used to indicate success
/// when the code is 0
const HEADER_TYPE_ERROR: u16 = 2;
/// End of a multi-part message
const HEADER_TYPE_DONE: u16 = 3;
// Data was lost from this message
// const HEADER_TYPE_OVERRUN: u16 = 4;

#[derive(Default, Debug)]
#[repr(C)]
struct Header {
    length: u32,
    typ: u16,
    flags: u16,
    sequence: u32,
    pid: u32,
}

const HEADER_SIZE: usize = size_of::<Header>();

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("invalid information element")]
    InvalidIE,

    #[error("response sequence is not match the request")]
    SequenceMismatched,

    #[error("response is too short")]
    TooShort,

    #[error("device not exists")]
    NotExists,

    #[error("{0}")]
    Api(String),
}

pub struct Client {
    fd: AsyncFd<OwnedFd>,

    family_id: u16,
    family_version: u8,

    pid: u32,
    seq: AtomicU32,
}

impl Client {
    pub async fn connect() -> Result<Self, Error> {
        let fd = unsafe {
            let fd = libc::socket(libc::AF_NETLINK, libc::SOCK_RAW, libc::NETLINK_GENERIC);
            if fd == -1 {
                return Err(Error::Io(std::io::Error::last_os_error()));
            }

            let enable = 1;
            let ret = libc::ioctl(fd, libc::FIONBIO, &enable as *const _);
            if ret != 0 {
                return Err(Error::Io(std::io::Error::last_os_error()));
            }

            let ret = libc::setsockopt(
                fd,
                libc::SOL_NETLINK,
                libc::NETLINK_EXT_ACK,
                &enable as *const i32 as *const libc::c_void,
                4,
            );
            if ret == -1 {
                return Err(Error::Io(std::io::Error::last_os_error()));
            }

            // NETLINK_GET_STRICT_CHK
            let ret = libc::setsockopt(
                fd,
                libc::SOL_NETLINK,
                libc::NETLINK_GET_STRICT_CHK,
                &enable as *const i32 as *const libc::c_void,
                4,
            );
            if ret == -1 {
                return Err(Error::Io(std::io::Error::last_os_error()));
            }

            OwnedFd::from_raw_fd(fd)
        };

        let pid = std::process::id();
        let mut client = Client {
            fd: AsyncFd::new(fd)?,
            family_id: 16, // GENL_ID_CTRL
            family_version: 1,

            pid,
            seq: AtomicU32::new(1),
        };

        // get family
        #[rustfmt::skip]
        let buf: [u8; 12] = [
            12, 0, // length
            2, 0, // type
            110, 108, 56, 48, // "nl80"
            50, 49, 49, 0     // "211\0"
        ];

        let mut family_id = 0;
        let mut family_version = 0;

        client
            .execute(3, 0, &buf, |_header, data| {
                let attrs = AttributeIterator {
                    data: &data[4..],
                    pos: 0,
                };

                for (typ, data) in attrs {
                    match typ {
                        // libc::CTRL_ATTR_FAMILY_ID
                        1 => {
                            if data.len() < 2 {
                                continue;
                            }
                            let a = data[0];
                            let b = data[1];

                            family_id = ((b as u16) << 8) | a as u16;
                        }
                        // libc::CTRL_ATTR_FAMILY_VERSION
                        3 => {
                            if data.len() < 4 {
                                continue;
                            }

                            family_version = u32::from_ne_bytes(data.try_into().unwrap()) as u8;
                        }
                        _ => {}
                    }
                }

                Ok(())
            })
            .await?;

        client.family_id = family_id;
        client.family_version = family_version;

        Ok(client)
    }

    pub async fn interfaces(&self) -> Result<Vec<Interface>, Error> {
        let mut interfaces = Vec::new();

        self.execute(CMD_GET_INTERFACE, DUMP, &[], |_header, data| {
            let attrs = AttributeIterator {
                data: &data[4..],
                pos: 0,
            };

            let mut interface = Interface::default();
            for (typ, data) in attrs {
                match typ {
                    3 => {
                        interface.index = u32::from_ne_bytes(data.try_into().unwrap());
                    }
                    4 => {
                        let data = data.strip_suffix(b"\0").unwrap_or(data);
                        interface.name = String::from_utf8_lossy(data).to_string();
                    }
                    6 => {
                        interface.mac = data.to_vec();
                    }
                    1 => {
                        interface.phy = u32::from_ne_bytes(data.try_into().unwrap());
                    }
                    5 => {
                        let value = u32::from_ne_bytes(data.try_into().unwrap());
                        interface.typ = InterfaceType::from(value);
                    }
                    153 => {
                        interface.device = u64::from_ne_bytes(data.try_into().unwrap());
                    }
                    38 => {
                        interface.frequency = u32::from_ne_bytes(data.try_into().unwrap());
                    }
                    _ => {}
                }
            }

            interfaces.push(interface);

            Ok(())
        })
        .await?;

        Ok(interfaces)
    }

    async fn execute<F>(&self, cmd: u8, flags: u16, attrs: &[u8], mut f: F) -> Result<(), Error>
    where
        F: FnMut(&Header, &[u8]) -> Result<(), Error>,
    {
        let len = HEADER_SIZE + 4 + attrs.len();
        let mut buf = Vec::with_capacity(len);

        // header
        buf.extend((len as u32).to_ne_bytes());
        buf.extend(self.family_id.to_ne_bytes());
        buf.extend((flags | REQUEST).to_ne_bytes());
        let sequence = self.next_seq();
        buf.extend(sequence.to_ne_bytes());
        buf.extend(self.pid.to_ne_bytes());

        // payload
        buf.extend([cmd, self.family_version, 0, 0]);
        buf.extend(attrs);

        self.send(&buf).await?;

        loop {
            let resp = self.receive().await?;

            let mut multiple = false;
            let msgs = MessageIterator {
                data: &resp,
                pos: 0,
            };
            for (header, data) in msgs {
                check_message(header, data)?;

                // skip the final message with multi-part done indicator if present
                if header.flags & FLAG_MULTI != 0 && header.typ == HEADER_TYPE_DONE {
                    continue;
                }

                if header.sequence != sequence {
                    return Err(Error::SequenceMismatched);
                }

                f(header, data)?;

                if header.flags & 2 == 0 {
                    // No, check the next messages
                    continue;
                }

                // Does this message indicate the last message in a series of multi-part
                // messages from a single read?
                multiple = header.typ != 3
            }

            if !multiple {
                // no more messages coming
                break;
            }
        }

        Ok(())
    }

    pub async fn bss(&self, interface: &Interface) -> Result<BSS, Error> {
        let mut buf = Vec::with_capacity(20);

        // length, type, data
        buf.extend([8, 0, NL80211_ATTR_IFINDEX as u8, 0]);
        buf.extend(interface.index.to_ne_bytes());

        // length, type, data
        buf.extend([10, 0, NL80211_ATTR_MAC as u8, 0]); // length do not contain the padding
        buf.extend(interface.mac.as_slice());
        buf.extend([0, 0]); // padding

        let mut bss = None;
        self.execute(CMD_GET_SCAN, DUMP, buf.as_ref(), |_header, data| {
            let mut temp = BSS::default();
            let mut ignore = true;

            let attrs = AttributeIterator {
                data: &data[4..], // skip command, version and two padding
                pos: 0,
            };

            for (typ, data) in attrs {
                if typ != NL80211_ATTR_BSS {
                    continue;
                }

                let attrs = AttributeIterator { data, pos: 0 };
                for (typ, data) in attrs {
                    match typ {
                        NL80211_BSS_BSSID => {
                            temp.mac = data.to_vec();
                        }
                        NL80211_BSS_FREQUENCY => {
                            temp.frequency = u32::from_ne_bytes(data.try_into().unwrap());
                        }
                        NL80211_BSS_BEACON_INTERVAL => {
                            // Raw value is in "Time Units (TU)".  See:
                            // https://en.wikipedia.org/wiki/Beacon_frame
                            let value = u16::from_ne_bytes(data.try_into().unwrap());
                            temp.beacon_interval = Duration::from_nanos(value as u64) * 1024 * 1000;
                        }
                        NL80211_BSS_SEEN_MS_AGO => {
                            // * @NL80211_BSS_SEEN_MS_AGO: age of this BSS entry in ms
                            let value =
                                u32::from_ne_bytes(data.try_into().map_err(|_| Error::TooShort)?);
                            temp.last_seen = 1000 * Duration::from_nanos(value as u64);
                        }
                        NL80211_BSS_STATUS => {
                            // The BSS which is associated with an interface will have a
                            // status attribute
                            ignore = false;

                            // NOTE: BSSStatus copies the ordering of nl80211's BSS status
                            // constants.  This may not be the case on other operating systems.
                            temp.status = u32::from_ne_bytes(data.try_into().unwrap());
                        }
                        NL80211_BSS_INFORMATION_ELEMENTS => {
                            for (id, data) in parse_ies(data)? {
                                if id == 0 {
                                    temp.ssid = String::from_utf8_lossy(data).to_string();
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }

            if !ignore {
                bss = Some(temp);
            }

            Ok(())
        })
        .await?;

        match bss {
            Some(bss) => Ok(bss),
            None => Err(Error::NotExists),
        }
    }

    pub async fn station_info(&self, interface: &Interface) -> Result<Vec<StationInfo>, Error> {
        let mut buf = Vec::with_capacity(20);

        // attributes
        // length, type, data
        buf.extend([8, 0, NL80211_ATTR_IFINDEX as u8, 0]);
        buf.extend(interface.index.to_ne_bytes().as_ref());

        // length, type, data
        buf.extend([10, 0, NL80211_ATTR_MAC as u8, 0]);
        buf.extend(interface.mac.as_slice());
        buf.extend([0, 0]); // padding

        let mut infos = Vec::new();
        let mut msgs = 0;

        self.execute(CMD_GET_STATION, DUMP, &buf, |_header, data| {
            msgs += 1;

            infos.push(parse_station_info(&data[4..])?);

            Ok(())
        })
        .await?;

        if msgs == 0 {
            return Err(Error::NotExists);
        }

        Ok(infos)
    }

    async fn send(&self, buf: &[u8]) -> Result<usize, Error> {
        self.fd
            .writable()
            .await?
            .try_io(|fd| {
                let mut sa = unsafe { std::mem::zeroed::<libc::sockaddr_nl>() };
                sa.nl_family = libc::AF_NETLINK as u16;
                let mut iovec = libc::iovec {
                    iov_base: buf.as_ptr() as *mut _,
                    iov_len: buf.len(),
                };
                let msghdr = libc::msghdr {
                    msg_name: (&mut sa) as *mut _ as *mut _,
                    msg_namelen: size_of::<libc::sockaddr_nl>() as _,
                    msg_iov: &mut iovec,
                    msg_iovlen: 1,
                    msg_control: std::ptr::null_mut(),
                    msg_controllen: 0,
                    msg_flags: 0,
                };

                let ret = unsafe { libc::sendmsg(fd.as_raw_fd(), &msghdr, 0) };
                if ret == -1 {
                    return Err(std::io::Error::last_os_error());
                }

                Ok(ret as usize)
            })
            .expect("sendmsg success")
            .map_err(Into::into)
    }

    async fn receive(&self) -> Result<Vec<u8>, Error> {
        // For most systems, 4096 is one PageSize
        let mut buf = Vec::<u8>::with_capacity(4096);

        loop {
            // peek at the buffer to see how many bytes are available.
            let read = self
                .fd
                .readable()
                .await?
                .try_io(|fd| {
                    let mut iov = libc::iovec {
                        iov_base: buf.as_mut_ptr() as *mut _,
                        iov_len: buf.capacity(),
                    };
                    let mut sa = unsafe { std::mem::zeroed::<libc::sockaddr_nl>() };
                    sa.nl_family = libc::AF_NETLINK as u16;
                    let mut msghdr = libc::msghdr {
                        msg_name: (&mut sa) as *mut _ as *mut _,
                        msg_namelen: size_of::<libc::sockaddr_nl>() as _,
                        msg_iov: &mut iov,
                        msg_iovlen: 1,
                        msg_control: std::ptr::null_mut(),
                        msg_controllen: 0,
                        msg_flags: libc::MSG_PEEK,
                    };

                    let ret = unsafe { libc::recvmsg(fd.as_raw_fd(), &mut msghdr, libc::MSG_PEEK) };
                    if ret == -1 {
                        return Err(std::io::Error::last_os_error());
                    }

                    Ok(ret as usize)
                })
                .expect("peek response size")?;

            if buf.capacity() > read {
                unsafe { buf.set_len(read) };
                break;
            }

            // double in size if not enough bytes
            buf.reserve(buf.capacity());
        }

        let _read = self
            .fd
            .readable()
            .await?
            .try_io(|fd| {
                let mut iov = libc::iovec {
                    iov_base: buf.as_mut_ptr() as *mut _,
                    iov_len: buf.capacity(),
                };
                let mut sa = unsafe { std::mem::zeroed::<libc::sockaddr_nl>() };
                sa.nl_family = libc::AF_NETLINK as u16;
                let mut msghdr = libc::msghdr {
                    msg_name: (&mut sa) as *mut _ as *mut _,
                    msg_namelen: size_of::<libc::sockaddr_nl>() as _,
                    msg_iov: &mut iov,
                    msg_iovlen: 1,
                    msg_control: std::ptr::null_mut(),
                    msg_controllen: 0,
                    msg_flags: 0,
                };

                let ret = unsafe { libc::recvmsg(fd.as_raw_fd(), &mut msghdr, 0) };
                if ret == -1 {
                    return Err(std::io::Error::last_os_error());
                }

                Ok(ret as usize)
            })
            .expect("recvmsg success")?;

        Ok(buf)
    }

    #[inline]
    fn next_seq(&self) -> u32 {
        self.seq.fetch_add(1, Ordering::SeqCst)
    }
}

// parseIEs parses zero or more ies from a byte slice.
// Reference:
//
//	https://www.safaribooksonline.com/library/view/80211-wireless-networks/0596100523/ch04.html#wireless802dot112-CHP-4-FIG-31
fn parse_ies(data: &[u8]) -> Result<Vec<(u8, &[u8])>, Error> {
    let mut pos = 0;
    let mut ies = vec![];

    while data.len() - pos > 2 {
        let id = data[pos];
        let len = data[pos + 1];
        pos += 2;

        if data.len() - pos < len as usize {
            return Err(Error::InvalidIE);
        }

        let info = &data[pos..pos + len as usize];
        pos += len as usize;

        ies.push((id, info))
    }

    Ok(ies)
}

const NL80211_ATTR_IFINDEX: u16 = 0x3;
const NL80211_ATTR_MAC: u16 = 0x6;
const NL80211_ATTR_STA_INFO: u16 = 0x15;
const NL80211_ATTR_BSS: u16 = 0x2f;

const NL80211_STA_INFO_INACTIVE_TIME: u16 = 0x1;
const NL80211_STA_INFO_SIGNAL: u16 = 0x7;
const NL80211_STA_INFO_RX_PACKETS: u16 = 0x9;
const NL80211_STA_INFO_CONNECTED_TIME: u16 = 0x10;
const NL80211_STA_INFO_RX_BYTES64: u16 = 0x17;
const NL80211_STA_INFO_TX_BYTES64: u16 = 0x18;
const NL80211_STA_INFO_TX_PACKETS: u16 = 0xa;
const NL80211_STA_INFO_TX_RETRIES: u16 = 0xb;
const NL80211_STA_INFO_TX_FAILED: u16 = 0xc;
const NL80211_STA_INFO_BEACON_LOSS: u16 = 0x12;
const NL80211_STA_INFO_RX_BITRATE: u16 = 0xe;
const NL80211_STA_INFO_TX_BITRATE: u16 = 0x8;
const NL80211_STA_INFO_RX_BYTES: u16 = 0x2;
const NL80211_STA_INFO_TX_BYTES: u16 = 0x3;

fn parse_station_info(data: &[u8]) -> Result<StationInfo, Error> {
    let attrs = AttributeIterator { data, pos: 0 };
    let mut info = StationInfo::default();

    for (typ, data) in attrs {
        match typ {
            NL80211_ATTR_IFINDEX => {
                if data.len() < 4 {
                    return Err(Error::TooShort);
                }
            }
            NL80211_ATTR_MAC => {
                info.mac = data.to_vec();
            }
            NL80211_ATTR_STA_INFO => {
                let attrs = AttributeIterator { data, pos: 0 };

                for (typ, data) in attrs {
                    match typ {
                        NL80211_STA_INFO_CONNECTED_TIME => {
                            let value =
                                u32::from_ne_bytes(data.try_into().map_err(|_| Error::TooShort)?);
                            info.connected = Duration::from_secs(value as u64);
                        }
                        NL80211_STA_INFO_INACTIVE_TIME => {
                            let value =
                                u32::from_ne_bytes(data.try_into().map_err(|_| Error::TooShort)?);
                            info.inactive = Duration::from_millis(value as u64);
                        }
                        NL80211_STA_INFO_RX_BYTES64 => {
                            info.received_bytes =
                                u64::from_ne_bytes(data.try_into().map_err(|_| Error::TooShort)?);
                        }
                        NL80211_STA_INFO_TX_BYTES64 => {
                            info.transmitted_bytes =
                                u64::from_ne_bytes(data.try_into().map_err(|_| Error::TooShort)?);
                        }
                        NL80211_STA_INFO_SIGNAL => {
                            if data.is_empty() {
                                return Err(Error::TooShort);
                            }

                            info.signal = data[0] as i8 as i32;
                        }
                        NL80211_STA_INFO_RX_PACKETS => {
                            info.received_packets =
                                u32::from_ne_bytes(data.try_into().map_err(|_| Error::TooShort)?);
                        }
                        NL80211_STA_INFO_TX_PACKETS => {
                            info.transmitted_packets =
                                u32::from_ne_bytes(data.try_into().map_err(|_| Error::TooShort)?);
                        }
                        NL80211_STA_INFO_TX_RETRIES => {
                            info.transmit_retries =
                                u32::from_ne_bytes(data.try_into().map_err(|_| Error::TooShort)?);
                        }
                        NL80211_STA_INFO_TX_FAILED => {
                            info.transmit_failed =
                                u32::from_ne_bytes(data.try_into().map_err(|_| Error::TooShort)?);
                        }
                        NL80211_STA_INFO_BEACON_LOSS => {
                            info.beacon_loss =
                                u32::from_ne_bytes(data.try_into().map_err(|_| Error::TooShort)?);
                        }
                        NL80211_STA_INFO_RX_BITRATE => {
                            info.receive_bitrate = parse_rate_info(data)?;
                        }
                        NL80211_STA_INFO_TX_BITRATE => {
                            info.transmit_bitrate = parse_rate_info(data)?;
                        }
                        NL80211_STA_INFO_RX_BYTES if info.received_bytes == 0 => {
                            let value =
                                u32::from_ne_bytes(data.try_into().map_err(|_| Error::TooShort)?);
                            info.received_bytes = value as u64;
                        }
                        NL80211_STA_INFO_TX_BYTES if info.transmitted_bytes == 0 => {
                            let value =
                                u32::from_ne_bytes(data.try_into().map_err(|_| Error::TooShort)?);
                            info.transmitted_bytes = value as u64;
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    Ok(info)
}

const NL80211_RATE_INFO_BITRATE32: u16 = 0x5;
const NL80211_RATE_INFO_BITRATE: u16 = 0x1;

fn parse_rate_info(data: &[u8]) -> Result<u32, Error> {
    let attrs = AttributeIterator { data, pos: 0 };
    let mut rate = 0;

    for (typ, data) in attrs {
        if typ == NL80211_RATE_INFO_BITRATE32 {
            rate = u32::from_ne_bytes(data.try_into().map_err(|_| Error::TooShort)?);
        }

        if rate == 0 && typ == NL80211_RATE_INFO_BITRATE {
            rate = u16::from_ne_bytes(data.try_into().map_err(|_| Error::TooShort)?) as u32;
        }
    }

    // Scale bitrate to bits/second as base unit instead of 100kbits/second.
    Ok(rate * 100 * 1000)
}

fn check_message(header: &Header, data: &[u8]) -> Result<(), Error> {
    let mut has_header = false;
    if header.typ == HEADER_TYPE_ERROR {
        has_header = true;
    } else if header.typ == HEADER_TYPE_DONE && header.flags & FLAG_MULTI != 0 {
        if data.is_empty() {
            return Ok(());
        }
    } else {
        return Ok(());
    }

    if data.len() < 4 {
        return Err(Error::TooShort);
    }

    let code = i32::from_ne_bytes((&data[..4]).try_into().unwrap());
    if code == 0 {
        // no error
        return Ok(());
    }

    if header.flags & 0x200 == 0 {
        // No extended acknowledgement
        return Err(Error::Io(std::io::Error::from_raw_os_error(code)));
    }

    let mut offset = 4u32;
    let mut msg = String::new();

    if has_header {
        if data.len() < 4 + HEADER_SIZE {
            return Err(Error::TooShort);
        }

        let a = data[4];
        let b = data[5];

        offset = ((4 + ((b as u16) << 8)) | a as u16) as u32;

        if data.len() < offset as usize {
            return Err(Error::TooShort);
        }
    }

    let attrs = AttributeIterator {
        data: &data[offset as usize..],
        pos: 0,
    };
    for (typ, data) in attrs {
        // NLMSGERR_ATTR_MSG
        if typ == 1 {
            msg = String::from_utf8_lossy(data).to_string();
            break;
        }
    }

    Err(Error::Api(msg))
}

struct MessageIterator<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Iterator for MessageIterator<'a> {
    type Item = (&'a Header, &'a [u8]);

    fn next(&mut self) -> Option<Self::Item> {
        if self.data.len() - self.pos < HEADER_SIZE {
            return None;
        }

        let header: &'a Header = unsafe {
            let ptr = self.data.as_ptr().add(self.pos);
            &*(ptr as *const Header)
        };

        let data = &self.data[self.pos + HEADER_SIZE..self.pos + header.length as usize];
        self.pos += header.length as usize;

        Some((header, data))
    }
}

struct AttributeIterator<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Iterator for AttributeIterator<'a> {
    type Item = (u16, &'a [u8]);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.data.len() - self.pos < 4 {
                return None;
            }

            let a = self.data[self.pos];
            let b = self.data[self.pos + 1];
            let c = self.data[self.pos + 2];
            let d = self.data[self.pos + 3];

            let len = ((b as u16) << 8) | a as u16;
            let typ = ((d as u16) << 8) | c as u16;

            if len >= 4 {
                let data = &self.data[self.pos + 4..self.pos + len as usize];
                self.pos += align(len) as usize;

                return Some((typ, data));
            }

            // ignore zero-length attribute, and advance
            self.pos += 4;
        }
    }
}

#[inline]
fn align(len: u16) -> i16 {
    ((len as i16) + 3) & (-4)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size() {
        assert_eq!(HEADER_SIZE, 16);
        assert_eq!(size_of::<Header>(), HEADER_SIZE);
    }

    #[ignore]
    #[tokio::test]
    async fn dump() {
        let client = Client::connect().await.unwrap();

        let infos = client.interfaces().await.unwrap();

        for info in infos {
            if info.name.is_empty() {
                continue;
            }

            match client.bss(&info).await {
                Ok(bss) => {
                    println!("bss: {:?}", bss);
                }
                Err(err) => {
                    println!("bss err: {}", err);
                }
            }

            match client.station_info(&info).await {
                Ok(stations) => {
                    for station in stations {
                        println!("station: {:?}", station);
                    }
                }
                Err(err) => {
                    println!("station_info err: {}", err);
                }
            }
        }
    }
}
