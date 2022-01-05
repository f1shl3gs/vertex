#[derive(Debug)]
#[repr(u32)]
pub enum ErrorCode {
    OK = 0,
    /// internal error
    InternalError = 1,
    /// memory allocation failure
    NoMemory = 2,
    /// no support for this function
    NoSupport = 3,
    /// could not resolve hostname
    UnknownHost = 4,
    /// can't connect to hypervisor
    NoConnect = 5,
    /// invalid connection object
    InvalidConn = 6,
    /// invalid domain object
    InvalidDomain = 7,
    /// invalid function argument
    InvalidArg = 8,
    /// a command to hypervisor failed
    OperationFailed = 9,
    /// a HTTP GET command to failed
    GetFailed = 10,
    /// a HTTP POST command to failed
    PostFailed = 11,
    /// unexpected HTTP error code
    HttpError = 12,
    /// failure to serialize an S-Expr
    SexprSerial = 13,
    /// could not open Xen hypervisor control
    NoXen = 14,
    /// failure doing an hypervisor call
    XenCall = 15,
    /// unknown OS type
    OsType = 16,
    /// missing kernel information
    NoKernel = 17,
    /// missing root device information
    NoRoot = 18,
    /// missing source device information
    NoSource = 19,
    /// missing target device information
    NoTarget = 20,
    /// missing domain name information
    NoName = 21,
    /// missing domain OS information
    NoOs = 22,
    /// missing domain devices information
    NoDevice = 23,
    /// could not open Xen Store control
    NoXenstore = 24,
    /// too many drivers registered
    DriverFull = 25,
    /// not supported by the drivers (DEPRECATED)
    CallFailed = 26,
    /// an XML description is not well formed or broken
    XmlError = 27,
    /// the domain already exist
    DomExist = 28,
    /// operation forbidden on read-only connections
    OperationDenied = 29,
    /// failed to open a conf file
    OpenFailed = 30,
    /// failed to read a conf file
    ReadFailed = 31,
    /// failed to parse a conf file
    ParseFailed = 32,
    /// failed to parse the syntax of a conf file
    ConfSyntax = 33,
    /// failed to write a conf file
    WriteFailed = 34,
    /// detail of an XML error
    XmlDetail = 35,
    /// invalid network object
    InvalidNetwork = 36,
    /// the network already exist
    NetworkExist = 37,
    /// general system call failure
    SystemError = 38,
    /// some sort of RPC error
    Rpc = 39,
    /// error from a GNUTLS call
    GnutlsError = 40,
    /// failed to start network
    VirWarNoNetwork = 41,
    /// domain not found or unexpectedly disappeared
    NoDomain = 42,
    /// network not found
    NoNetwork = 43,
    /// invalid MAC address
    InvalidMac = 44,
    /// authentication failed
    AuthFailed = 45,
    /// invalid storage pool object
    InvalidStoragePool = 46,
    /// invalid storage vol object
    InvalidStorageVol = 47,
    /// failed to start storage
    VirWarNoStorage = 48,
    /// storage pool not found
    NoStoragePool = 49,
    /// storage volume not found
    NoStorageVol = 50,
    /// failed to start node driver
    VirWarNoNode = 51,
    /// invalid node device object
    InvalidNodeDevice = 52,
    /// node device not found
    NoNodeDevice = 53,
    /// security model not found
    NoSecurityModel = 54,
    /// operation is not applicable at this time
    OperationInvalid = 55,
    /// failed to start interface driver
    VirWarNoInterface = 56,
    /// interface driver not running
    NoInterface = 57,
    /// invalid interface object
    InvalidInterface = 58,
    /// more than one matching interface found
    MultipleInterfaces = 59,
    /// failed to start nwfilter driver
    VirWarNoNwfilter = 60,
    /// invalid nwfilter object
    InvalidNwfilter = 61,
    /// nw filter pool not found
    NoNwfilter = 62,
    /// nw filter pool not found
    BuildFirewall = 63,
    /// failed to start secret storage
    VirWarNoSecret = 64,
    /// invalid secret
    InvalidSecret = 65,
    /// secret not found
    NoSecret = 66,
    /// unsupported configuration construct
    ConfigUnsupported = 67,
    /// timeout occurred during operation
    OperationTimeout = 68,
    /// a migration worked, but making the VM persist on the dest host failed
    MigratePersistFailed = 69,
    /// a synchronous hook script failed
    HookScriptFailed = 70,
    /// invalid domain snapshot
    InvalidDomainSnapshot = 71,
    /// domain snapshot not found
    NoDomainSnapshot = 72,
    /// stream pointer not valid
    InvalidStream = 73,
    /// valid API use but unsupported by the given driver
    ArgumentUnsupported = 74,
    /// storage pool probe failed
    StorageProbeFailed = 75,
    /// storage pool already built
    StoragePoolBuilt = 76,
    /// force was not requested for a risky domain snapshot revert
    SnapshotRevertRisky = 77,
    /// operation on a domain was canceled/aborted by user
    OperationAborted = 78,
    /// authentication cancelled
    AuthCancelled = 79,
    /// The metadata is not present
    NoDomainMetadata = 80,
    /// Migration is not safe
    MigrateUnsafe = 81,
    /// integer overflow
    Overflow = 82,
    /// action prevented by block copy job
    BlockCopyActive = 83,
    /// The requested operation is not supported
    OperationUnsupported = 84,
    /// error in ssh transport driver
    Ssh = 85,
    /// guest agent is unresponsive, not running or not usable
    AgentUnresponsive = 86,
    /// resource is already in use
    ResourceBusy = 87,
    /// operation on the object/resource was denied
    AccessDenied = 88,
    /// error from a dbus service
    DbusService = 89,
    /// the storage vol already exists
    StorageVolExist = 90,
    /// given CPU is incompatible with host CP
    CpuIncompatible = 91,
    /// XML document doesn't validate against schema
    XmlInvalidSchema = 92,
    /// Finish API succeeded but it is expected to return NULL
    MigrateFinishOk = 93,
    /// authentication unavailable
    AuthUnavailable = 94,
    /// Server was not found
    NoServer = 95,
    /// Client was not found
    NoClient = 96,
    /// guest agent replies with wrong id to guest-sync command
    AgentUnsynced = 97,
    /// error in libssh transport driver
    Libssh = 98,
}

impl From<i32> for ErrorCode {
    fn from(v: i32) -> Self {
        unsafe { ::std::mem::transmute(v) }
    }
}

#[derive(Debug)]
#[repr(u32)]
pub enum ErrorDomain {
    None = 0,
    /// Error at Xen hypervisor layer
    Xen = 1,
    /// Error at connection with xend daemon
    Xend = 2,
    /// Error at connection with xen store
    Xenstore = 3,
    /// Error in the S-Expression code
    Sexpr = 4,
    /// Error in the XML code
    Xml = 5,
    /// Error when operating on a domain
    Dom = 6,
    /// Error in the XML-RPC code
    Rpc = 7,
    /// Error in the proxy code; unused since 0.8.6
    Proxy = 8,
    /// Error in the configuration file handling
    Conf = 9,
    /// Error at the QEMU daemon
    Qemu = 10,
    /// Error when operating on a network
    Net = 11,
    /// Error from test driver
    Test = 12,
    /// Error from remote driver
    Remote = 13,
    /// Error from OpenVZ driver
    Openvz = 14,
    /// Error at Xen XM layer
    Xenxm = 15,
    /// Error in the Linux Stats code
    StatsLinux = 16,
    /// Error from Linux Container driver
    Lxc = 17,
    /// Error from storage driver
    Storage = 18,
    /// Error from network config
    Network = 19,
    /// Error from domain config
    Domain = 20,
    /// Error at the UML driver
    Uml = 21,
    /// Error from node device monitor
    Nodedev = 22,
    /// Error from xen inotify layer
    XenInotify = 23,
    /// Error from security framework
    Security = 24,
    /// Error from VirtualBox driver
    Vbox = 25,
    /// Error when operating on an interface
    Interface = 26,
    /// The OpenNebula driver no longer exists. Retained for ABI/API compat only
    One = 27,
    /// Error from ESX driver
    Esx = 28,
    /// Error from IBM power hypervisor
    Phyp = 29,
    /// Error from secret storage
    Secret = 30,
    /// Error from CPU driver
    Cpu = 31,
    /// Error from XenAPI
    Xenapi = 32,
    /// Error from network filter driver
    Nwfilter = 33,
    /// Error from Synchronous hooks
    Hook = 34,
    /// Error from domain snapshot
    DomainSnapshot = 35,
    /// Error from auditing subsystem
    Audit = 36,
    /// Error from sysinfo/SMBIOS
    Sysinfo = 37,
    /// Error from I/O streams
    Streams = 38,
    /// Error from VMware driver
    Vmware = 39,
    /// Error from event loop impl
    Event = 40,
    /// Error from libxenlight driver
    Libxl = 41,
    /// Error from lock manager
    Locking = 42,
    /// Error from Hyper-V driver
    Hyperv = 43,
    /// Error from capabilities
    Capabilities = 44,
    /// Error from URI handling
    Uri = 45,
    /// Error from auth handling
    Auth = 46,
    /// Error from DBus
    Dbus = 47,
    /// Error from Parallels
    Parallels = 48,
    /// Error from Device
    Device = 49,
    /// Error from libssh2 connection transport
    Ssh = 50,
    /// Error from lockspace
    Lockspace = 51,
    /// Error from initctl device communication
    Initctl = 52,
    /// Error from identity code
    Identity = 53,
    /// Error from cgroups
    Cgroup = 54,
    /// Error from access control manager
    Access = 55,
    /// Error from systemd code
    Systemd = 56,
    /// Error from bhyve driver
    Bhyve = 57,
    /// Error from crypto code
    Crypto = 58,
    /// Error from firewall
    Firewall = 59,
    /// Error from polkit code
    Polkit = 60,
    /// Error from thread utils
    Thread = 61,
    /// Error from admin backend
    Admin = 62,
    /// Error from log manager
    Logging = 63,
    /// Error from Xen xl config code
    Xenxl = 64,
    /// Error from perf
    Perf = 65,
    /// Error from libssh connection transport
    Libssh = 66,
}

impl From<i32> for ErrorDomain {
    fn from(v: i32) -> Self {
        unsafe { ::std::mem::transmute(v) }
    }
}

pub mod generated {
    //! This module is generated from protocol files.
    //!
    //! It follows original naming convention
    #![allow(non_camel_case_types)]
    #![allow(dead_code)]
    #![allow(non_snake_case)]
    #![allow(unused_assignments)]

    use super::{ErrorCode, ErrorDomain};
    use ::xdr_codec;

    include!(concat!(env!("OUT_DIR"), "/virnetprotocol_xdr.rs"));
    include!(concat!(env!("OUT_DIR"), "/remote_protocol_xdr.rs"));

    impl virNetMessageError {
        pub fn code(&self) -> ErrorCode {
            ErrorCode::from(self.code)
        }

        pub fn domain(&self) -> ErrorDomain {
            ErrorDomain::from(self.domain)
        }
    }

    impl Default for virNetMessageHeader {
        fn default() -> Self {
            virNetMessageHeader {
                prog: 0x20008086,
                vers: 1,
                proc_: 0,
                type_: virNetMessageType::VIR_NET_CALL,
                serial: 0,
                status: virNetMessageStatus::VIR_NET_OK,
            }
        }
    }
}

pub use generated::{
    remote_procedure, virNetMessageError, virNetMessageHeader, virNetMessageStatus,
};

#[derive(Debug)]
pub struct LibvirtMessage<P> {
    pub header: generated::virNetMessageHeader,
    pub payload: P,
}

impl<P: xdr_codec::Pack<Out>, Out: xdr_codec::Write> xdr_codec::Pack<Out> for LibvirtMessage<P> {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        let mut sz: usize = 0;
        sz += self.header.pack(out)?;
        sz += self.payload.pack(out)?;
        Ok(sz)
    }
}

pub trait LibvirtRpc<R: ::std::io::Read> {
    const PROCEDURE: remote_procedure;
    type Response: Send + ::xdr_codec::Unpack<R>;
}

macro_rules! delegate_pack_impl {
    ($t:ty) => {
        impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for $t {
            fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
                self.0.pack(out)
            }
        }
    };
}

macro_rules! delegate_unpack_impl {
    ($t:ty) => {
        impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for $t {
            fn unpack(input: &mut In) -> xdr_codec::Result<(Self, usize)> {
                let (inner, len) = xdr_codec::Unpack::unpack(input)?;
                let mut pkt: $t = unsafe { ::std::mem::zeroed() };
                pkt.0 = inner;
                Ok((pkt, len))
            }
        }
    };
}

macro_rules! req {
    ($name: ident) => {
        #[derive(Debug)]
        pub struct $name(());
        delegate_pack_impl!($name);

        impl $name {
            pub fn new() -> Self {
                $name(())
            }
        }
    };

    ($name:ident : $inner:ident { $($f:ident : $t:ty => $e: expr),+ }) => {
        #[derive(Debug)]
        pub struct $name($inner);
        delegate_pack_impl!($name);

        impl $name {
            pub fn new($( $f: $t,)+) -> Self {
                let inner = $inner {
                    $(
                        $f: $e,
                    )+
                };
                $name(inner)
            }
        }
    };

    ($name:ident : $inner:ident { $($f:ident as $arg:ident : $t:ty => $e: expr),+ }) => {
        #[derive(Debug)]
        pub struct $name($inner);
        delegate_pack_impl!($name);

        impl $name {
            pub fn new($( $arg: $t,)+) -> Self {
                let inner = $inner {
                    $(
                        $f: $e,
                    )+
                };
                $name(inner)
            }
        }
    };



    ($name:ident : $inner:ident { $($f: ident => $e: expr),+ }) => {
        #[derive(Debug)]
        pub struct $name($inner);
        delegate_pack_impl!($name);

        impl $name {
            pub fn new() -> Self {
                let inner = $inner {
                    $(
                        $f: $e,
                    )+
                };
                $name(inner)
            }
        }
    };


    ($name:ident : $inner:ident { $($f: ident : $t: ty),+ }) => {
        #[derive(Debug)]
        pub struct $name($inner);
        delegate_pack_impl!($name);

        impl $name {
            pub fn new($( $f: $t,)+) -> Self {
                let inner = $inner {
                    $(
                        $f,
                    )+
                };
                $name(inner)
            }
        }
    };

    // argument renaming
    ($name:ident : $inner:ident { $($f: ident as $arg: ident : $t: ty),+ }) => {
        #[derive(Debug)]
        pub struct $name($inner);
        delegate_pack_impl!($name);

        impl $name {
            pub fn new($( $arg: $t,)+) -> Self {
                let inner = $inner {
                    $(
                        $f: $arg,
                    )+
                };
                $name(inner)
            }
        }
    };
}

macro_rules! resp {
    ($name: ident) => {
        #[derive(Debug)]
        pub struct $name(());
        delegate_unpack_impl!($name);

        impl Into<()> for $name {
            fn into(self) -> () {
                ()
            }
        }
    };

    ($name: ident : $inner: ty) => {
        #[derive(Debug)]
        pub struct $name($inner);
        delegate_unpack_impl!($name);
    };
}

macro_rules! rpc {
    ($id:path, $req:ident => $resp:ident) => {
        impl<R: ::std::io::Read> LibvirtRpc<R> for $req {
            const PROCEDURE: remote_procedure = $id;
            type Response = $resp;
        }
    };
}

/// VM instance
#[derive(Debug, Clone)]
pub struct Domain(generated::remote_nonnull_domain);

impl Domain {
    /// positive integer, unique amongst running guest domains on a single host. An inactive domain does not have an ID.
    pub fn id(&self) -> i32 {
        self.0.id
    }

    /// short string, unique amongst all guest domains on a single host, both running and inactive.
    pub fn name(&self) -> String {
        self.0.name.0.clone()
    }

    /// guaranteed to be unique amongst all guest domains on any host.
    pub fn uuid(&self) -> ::uuid::Uuid {
        let bytes = self.0.uuid.0;
        uuid::Uuid::from_slice(&bytes).unwrap()
    }
}

/// Version request
req!(GetLibVersionRequest);
resp!(GetLibVersionResponse: generated::remote_connect_get_lib_version_ret);
rpc!(remote_procedure::REMOTE_PROC_CONNECT_GET_LIB_VERSION, GetLibVersionRequest => GetLibVersionResponse);

impl GetLibVersionResponse {
    pub fn version(&self) -> (u32, u32, u32) {
        let v = (self.0).lib_ver;

        (
            (v / 1000 / 1000 % 1000) as u32,
            (v / 1000 % 1000) as u32,
            (v % 1000) as u32,
        )
    }
}

/// Auth list request must be the first request
req!(AuthListRequest);
resp!(AuthListResponse: generated::remote_auth_list_ret);
rpc!(remote_procedure::REMOTE_PROC_AUTH_LIST, AuthListRequest => AuthListResponse);

/// Connect open request
use generated::remote_connect_open_args;
req!(ConnectOpenRequest: remote_connect_open_args {
     name => Some(generated::remote_nonnull_string("qemu:///system".to_string())),
     flags => 0
});
resp!(ConnectOpenResponse);
rpc!(remote_procedure::REMOTE_PROC_CONNECT_OPEN, ConnectOpenRequest => ConnectOpenResponse);

/// List all domains
use bitflags::bitflags;
bitflags! {
    pub struct ListAllDomainsFlags: u32 {
        const DOMAINS_ACTIVE	=	1;
        const DOMAINS_INACTIVE	=	2;
        const DOMAINS_PERSISTENT	=	4;
        const DOMAINS_TRANSIENT	=	8;
        const DOMAINS_RUNNING	=	16;
        const DOMAINS_PAUSED	=	32;
        const DOMAINS_SHUTOFF	=	64;
        const DOMAINS_OTHER	=	128;
        const DOMAINS_MANAGEDSAVE	=	256;
        const DOMAINS_NO_MANAGEDSAVE	=	512;
        const DOMAINS_AUTOSTART	=	1024;
        const DOMAINS_NO_AUTOSTART	=	2048;
        const DOMAINS_HAS_SNAPSHOT	=	4096;
        const DOMAINS_NO_SNAPSHOT	=	8192;
    }
}

#[derive(Debug)]
pub struct ListAllDomainsRequest(generated::remote_connect_list_all_domains_args);

impl ListAllDomainsRequest {
    pub fn new(flags: ListAllDomainsFlags) -> Self {
        let payload = generated::remote_connect_list_all_domains_args {
            need_results: 1,
            flags: flags.bits(),
        };
        ListAllDomainsRequest(payload)
    }
}

delegate_pack_impl!(ListAllDomainsRequest);

#[derive(Debug)]
pub struct ListAllDomainsResponse(generated::remote_connect_list_all_domains_ret);

impl ::std::convert::Into<Vec<Domain>> for ListAllDomainsResponse {
    fn into(self) -> Vec<Domain> {
        let mut domains = Vec::new();
        for dom in &(self.0).domains {
            domains.push(Domain(dom.clone()))
        }
        domains
    }
}

delegate_unpack_impl!(ListAllDomainsResponse);

impl<R: ::std::io::Read> LibvirtRpc<R> for ListAllDomainsRequest {
    const PROCEDURE: remote_procedure = remote_procedure::REMOTE_PROC_CONNECT_LIST_ALL_DOMAINS;
    type Response = ListAllDomainsResponse;
}
