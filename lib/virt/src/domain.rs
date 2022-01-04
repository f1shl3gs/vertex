/*
 * This library is free software; you can redistribute it and/or
 * modify it under the terms of the GNU Lesser General Public
 * License as published by the Free Software Foundation; either
 * version 2.1 of the License, or (at your option) any later version.
 *
 * This library is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
 * Lesser General Public License for more details.
 *
 * You should have received a copy of the GNU Lesser General Public
 * License along with this library.  If not, see
 * <http://www.gnu.org/licenses/>.
 *
 * Sahid Orentino Ferdjaoui <sahid.ferdjaoui@redhat.com>
 */

extern crate libc;

use crate::common::{get_string, get_u32, get_u64};
use std::{ptr, str};

use crate::domain::sys::{virDomainMemoryStats, virVcpuInfo, virVcpuInfoPtr};
use crate::error::Error;
use crate::typedparam::sys::{virTypedParameter, virTypedParameterPtr};

pub mod sys {
    extern crate libc;

    use crate::typedparam::sys::virTypedParameterPtr;

    #[repr(C)]
    pub struct virDomain {}

    pub type virDomainPtr = *mut virDomain;

    #[repr(C)]
    #[derive(Default)]
    pub struct virDomainInfo {
        pub state: libc::c_ulong,
        pub maxMem: libc::c_ulong,
        pub memory: libc::c_ulong,
        pub nrVirtCpu: libc::c_uint,
        pub cpuTime: libc::c_ulong,
    }

    pub type virDomainInfoPtr = *mut virDomainInfo;

    #[repr(C)]
    #[derive(Debug, Default)]
    pub struct virVcpuInfo {
        pub number: libc::c_uint,
        // virtual CPU number
        pub state: libc::c_int,
        // value from virVcpuState
        pub cpuTime: libc::c_ulonglong,
        // CPU time used, in nanoseconds
        pub cpu: libc::c_int, // real CPU number, or one of the values from virVcpuHostCpuState
    }

    pub type virVcpuInfoPtr = *mut virVcpuInfo;

    #[repr(C)]
    pub struct virDomainStatsRecord {
        pub dom: virDomainPtr,
        pub params: virTypedParameterPtr,
        pub nparams: libc::c_uint,
    }

    pub type virDomainStatsRecordPtr = *mut virDomainStatsRecord;

    #[repr(C)]
    #[derive(Default)]
    pub struct virDomainBlockInfo {
        pub capacity: libc::c_ulonglong,
        pub allocation: libc::c_ulonglong,
        pub physical: libc::c_ulonglong,
    }

    pub type virDomainBlockInfoPtr = *mut virDomainBlockInfo;

    #[repr(C)]
    pub struct virDomainIPAddress {
        pub typed: libc::c_int,
        pub addr: *mut libc::c_char,
        pub prefix: libc::c_uint,
    }

    pub type virDomainIPAddressPtr = *mut virDomainIPAddress;

    #[repr(C)]
    pub struct virDomainInterface {
        pub name: *mut libc::c_char,
        pub hwaddr: *mut libc::c_char,
        pub naddrs: libc::c_uint,
        pub addrs: virDomainIPAddressPtr,
    }

    pub type virDomainInterfacePtr = *mut virDomainInterface;

    #[repr(C)]
    #[derive(Default)]
    pub struct virDomainInterfaceStats {
        pub rx_bytes: libc::c_longlong,
        pub rx_packets: libc::c_longlong,
        pub rx_errs: libc::c_longlong,
        pub rx_drop: libc::c_longlong,
        pub tx_bytes: libc::c_longlong,
        pub tx_packets: libc::c_longlong,
        pub tx_errs: libc::c_longlong,
        pub tx_drop: libc::c_longlong,
    }

    pub type virDomainInterfaceStatsPtr = *mut virDomainInterfaceStats;

    #[repr(C)]
    #[derive(Default)]
    pub struct virDomainMemoryStats {
        pub tag: libc::c_int,
        pub val: libc::c_ulonglong,
    }

    pub type virDomainMemoryStatsPtr = *mut virDomainMemoryStats;
}

#[link(name = "virt")]
extern "C" {
    fn virDomainFree(ptr: sys::virDomainPtr) -> libc::c_int;
    fn virDomainGetName(ptr: sys::virDomainPtr) -> *const libc::c_char;
    fn virDomainGetState(
        ptr: sys::virDomainPtr,
        state: *mut libc::c_int,
        reason: *mut libc::c_int,
        flags: libc::c_uint,
    ) -> libc::c_int;
    fn virDomainGetUUIDString(ptr: sys::virDomainPtr, uuid: *mut libc::c_char) -> libc::c_int;
    fn virDomainGetXMLDesc(ptr: sys::virDomainPtr, flags: libc::c_uint) -> *mut libc::c_char;
    fn virDomainGetVcpus(
        ptr: sys::virDomainPtr,
        info: sys::virVcpuInfoPtr,
        maxinfo: libc::c_int,
        cpumaps: *const libc::c_uchar,
        maplen: libc::c_int,
    ) -> libc::c_int;
    fn virDomainGetInfo(ptr: sys::virDomainPtr, ninfo: sys::virDomainInfoPtr) -> libc::c_int;
    fn virDomainMemoryStats(
        ptr: sys::virDomainPtr,
        // stats: sys::virDomainMemoryStatsPtr,
        stats: *mut sys::virDomainMemoryStats,
        nr_stats: libc::c_uint,
        flags: libc::c_uint,
    ) -> libc::c_int;
    fn virDomainGetBlockIoTune(
        ptr: sys::virDomainPtr,
        disk: *const libc::c_char,
        params: virTypedParameterPtr,
        nparams: *const libc::c_uint,
        flags: libc::c_uint,
    ) -> libc::c_int;
}

pub type DomainCreateFlags = self::libc::c_uint;

pub const VIR_DOMAIN_NONE: DomainCreateFlags = 0;
pub const VIR_DOMAIN_START_PAUSED: DomainCreateFlags = 1 << 0;
pub const VIR_DOMAIN_START_AUTODESTROY: DomainCreateFlags = 1 << 1;
pub const VIR_DOMAIN_START_BYPASS_CACHE: DomainCreateFlags = 1 << 2;
pub const VIR_DOMAIN_START_FORCE_BOOT: DomainCreateFlags = 1 << 3;
pub const VIR_DOMAIN_START_VALIDATE: DomainCreateFlags = 1 << 4;

pub type DomainNumatuneMemMode = self::libc::c_int;

pub const VIR_DOMAIN_NUMATUNE_MEM_STRICT: DomainNumatuneMemMode = 0;
pub const VIR_DOMAIN_NUMATUNE_MEM_PREFERRED: DomainNumatuneMemMode = 1;
pub const VIR_DOMAIN_NUMATUNE_MEM_INTERLEAVE: DomainNumatuneMemMode = 2;

pub type DomainState = self::libc::c_uint;

pub const VIR_DOMAIN_NOSTATE: DomainState = 0;
pub const VIR_DOMAIN_RUNNING: DomainState = 1;
pub const VIR_DOMAIN_BLOCKED: DomainState = 2;
pub const VIR_DOMAIN_PAUSED: DomainState = 3;
pub const VIR_DOMAIN_SHUTDOWN: DomainState = 4;
pub const VIR_DOMAIN_SHUTOFF: DomainState = 5;
pub const VIR_DOMAIN_CRASHED: DomainState = 6;
pub const VIR_DOMAIN_PMSUSPENDED: DomainState = 7;

pub type DomainRebootFlags = self::libc::c_uint;

pub const VIR_DOMAIN_REBOOT_DEFAULT: DomainRebootFlags = 0;
pub const VIR_DOMAIN_REBOOT_ACPI_POWER_BTN: DomainRebootFlags = 1 << 0;
pub const VIR_DOMAIN_REBOOT_GUEST_AGENT: DomainRebootFlags = 1 << 1;
pub const VIR_DOMAIN_REBOOT_INITCTL: DomainRebootFlags = 1 << 2;
pub const VIR_DOMAIN_REBOOT_SIGNAL: DomainRebootFlags = 1 << 3;
pub const VIR_DOMAIN_REBOOT_PARAVIRT: DomainRebootFlags = 1 << 4;

pub type KeycodeSet = self::libc::c_uint;

pub const VIR_KEYCODE_SET_LINUX: KeycodeSet = 0;
pub const VIR_KEYCODE_SET_XT: KeycodeSet = 1;
pub const VIR_KEYCODE_SET_ATSET1: KeycodeSet = 2;
pub const VIR_KEYCODE_SET_ATSET2: KeycodeSet = 3;
pub const VIR_KEYCODE_SET_ATSET3: KeycodeSet = 4;
pub const VIR_KEYCODE_SET_OSX: KeycodeSet = 5;
pub const VIR_KEYCODE_SET_XT_KBD: KeycodeSet = 6;
pub const VIR_KEYCODE_SET_USB: KeycodeSet = 7;
pub const VIR_KEYCODE_SET_WIN32: KeycodeSet = 8;
pub const VIR_KEYCODE_SET_QNUM: KeycodeSet = 9;
pub const VIR_KEYCODE_SET_LAST: KeycodeSet = 10;

pub type DomainInterfaceAddressesSource = self::libc::c_uint;

pub const VIR_DOMAIN_INTERFACE_ADDRESSES_SRC_LEASE: DomainInterfaceAddressesSource = 0;
pub const VIR_DOMAIN_INTERFACE_ADDRESSES_SRC_AGENT: DomainInterfaceAddressesSource = 1;
pub const VIR_DOMAIN_INTERFACE_ADDRESSES_SRC_ARP: DomainInterfaceAddressesSource = 2;
pub const VIR_DOMAIN_INTERFACE_ADDRESSES_SRC_LAST: DomainInterfaceAddressesSource = 3;

#[derive(Clone, Debug)]
pub struct DomainInfo {
    /// The running state, one of virDomainState.
    pub state: DomainState,
    /// The maximum memory in KBytes allowed.
    pub max_mem: u64,
    /// The memory in KBytes used by the domain.
    pub memory: u64,
    /// The number of virtual CPUs for the domain.
    pub nr_virt_cpu: u32,
    /// The CPU time used in nanoseconds.
    pub cpu_time: u64,
}

impl DomainInfo {
    pub fn from_ptr(ptr: sys::virDomainInfoPtr) -> DomainInfo {
        unsafe {
            DomainInfo {
                state: (*ptr).state as DomainState,
                max_mem: (*ptr).maxMem as u64,
                memory: (*ptr).memory as u64,
                nr_virt_cpu: (*ptr).nrVirtCpu as u32,
                cpu_time: (*ptr).cpuTime as u64,
            }
        }
    }
}

#[derive(Default)]
pub struct DomainStatsBalloonStats {
    pub current: u64,
    pub maximum: u64,
    pub swap_in: u64,
    pub swap_out: u64,
    pub major_fault: u64,
    pub minor_fault: u64,
    pub unused: u64,
    pub available: u64,
    pub actual: u64,
    pub rss: u64,
    pub usable: u64,
    pub last_update: u64,
    pub disk_caches: u64,
    pub huge_tlb_pg_alloc: u64,
    pub huge_tlb_pg_fail: u64,
}

pub struct BlockIoTuneParameters {
    pub total_bytes_sec: u64,
    pub read_bytes_sec: u64,
    pub write_bytes_sec: u64,
    pub total_iops_sec: u64,
    pub read_iops_sec: u64,
    pub write_iops_sec: u64,
    pub total_bytes_sec_max: u64,
    pub read_bytes_sec_max: u64,
    pub write_bytes_sec_max: u64,
    pub total_iops_sec_max: u64,
    pub read_iops_sec_max: u64,
    pub write_iops_sec_max: u64,
    pub total_bytes_sec_max_length: u64,
    pub read_bytes_sec_max_length: u64,
    pub write_bytes_sec_max_length: u64,
    pub total_iops_sec_max_length: u64,
    pub read_iops_sec_max_length: u64,
    pub write_iops_sec_max_length: u64,
    pub size_iops_sec: u64,
}

pub struct DomainStatsRecord {
    // TODO(sahid): needs to be implemented
    pub ptr: sys::virDomainStatsRecordPtr,
}

impl DomainStatsRecord {
    pub fn balloon(&self) -> Result<DomainStatsBalloonStats, Error> {
        let stats = unsafe {
            DomainStatsBalloonStats {
                current: get_u64((*self.ptr).params, (*self.ptr).nparams, "balloon.current")?,
                maximum: get_u64((*self.ptr).params, (*self.ptr).nparams, "balloon.maximum")?,
                swap_in: get_u64((*self.ptr).params, (*self.ptr).nparams, "balloon.swap_in")?,
                swap_out: get_u64((*self.ptr).params, (*self.ptr).nparams, "balloon.swap_out")?,
                major_fault: get_u64(
                    (*self.ptr).params,
                    (*self.ptr).nparams,
                    "balloon.major_fault",
                )?,
                minor_fault: get_u64(
                    (*self.ptr).params,
                    (*self.ptr).nparams,
                    "balloon.minor_fault",
                )?,
                unused: get_u64((*self.ptr).params, (*self.ptr).nparams, "balloon.unused")?,
                available: get_u64((*self.ptr).params, (*self.ptr).nparams, "balloon.available")?,
                actual: get_u64((*self.ptr).params, (*self.ptr).nparams, "balloon.actual")?,
                rss: get_u64((*self.ptr).params, (*self.ptr).nparams, "balloon.rss")?,
                usable: get_u64((*self.ptr).params, (*self.ptr).nparams, "balloon.usable")?,
                last_update: get_u64(
                    (*self.ptr).params,
                    (*self.ptr).nparams,
                    "balloon.last-update",
                )?,
                disk_caches: get_u64(
                    (*self.ptr).params,
                    (*self.ptr).nparams,
                    "balloon.disk_caches",
                )?,
                huge_tlb_pg_alloc: get_u64(
                    (*self.ptr).params,
                    (*self.ptr).nparams,
                    "balloon.hugetlb_pgalloc",
                )?,
                huge_tlb_pg_fail: get_u64(
                    (*self.ptr).params,
                    (*self.ptr).nparams,
                    "balloon.hugetlb_pgfail",
                )?,
            }
        };

        Ok(stats)
    }

    pub fn vcpu_wait_and_delay(&self, vcpu: u32) -> Result<(u64, u64), Error> {
        unsafe {
            let params = (*self.ptr).params;
            let nparams = (*self.ptr).nparams;

            let wait = get_u64(params, nparams, &format!("vcpu.{}.wait", vcpu))?;
            let delay = get_u64(params, nparams, &format!("vcpu.{}.delay", vcpu))?;

            Ok((wait, delay))
        }
    }

    pub fn block_count(&self) -> Result<u32, Error> {
        unsafe {
            get_u32(
                (*self.ptr).params,
                (*self.ptr).nparams,
                &format!("block.count"),
            )
        }
    }

    pub fn block_stats(&self) -> Result<Vec<BlockInfo>, Error> {
        unsafe {
            let n = get_u32(
                (*self.ptr).params,
                (*self.ptr).nparams,
                &format!("block.count"),
            )?;

            let mut array = Vec::with_capacity(n as usize);
            for i in 0..n {
                array.push(BlockInfo::new(
                    (*self.ptr).params,
                    (*self.ptr).nparams,
                    i as i32,
                )?)
            }

            Ok(array)
        }
    }

    pub fn block_io_tune(&self, disk: &str) -> Result<BlockIoTuneParameters, Error> {
        unsafe {
            let params = std::ptr::null_mut();
            let nparams = 0 as libc::c_uint;

            let ret = virDomainGetBlockIoTune(
                (*self.ptr).dom,
                string_to_c_chars!(disk),
                params,
                &nparams,
                0,
            );

            if ret == -1 {
                return Err(Error::new());
            }

            let params = libc::malloc(std::mem::size_of::<virTypedParameter>() * (nparams as usize))
                as virTypedParameterPtr;
            // let params = &mut virTypedParameter::default();
            let ret = virDomainGetBlockIoTune(
                (*self.ptr).dom,
                string_to_c_chars!(disk),
                params,
                &nparams,
                0,
            );
            if ret == -1 {
                return Err(Error::new());
            }

            // TODO: free params properly
            let tune = BlockIoTuneParameters {
                total_bytes_sec: get_u64(params, nparams, "total_bytes_sec")?,
                read_bytes_sec: get_u64(params, nparams, "read_bytes_sec")?,
                write_bytes_sec: get_u64(params, nparams, "write_bytes_sec")?,
                total_iops_sec: get_u64(params, nparams, "total_iops_sec")?,
                read_iops_sec: get_u64(params, nparams, "read_iops_sec")?,
                write_iops_sec: get_u64(params, nparams, "write_iops_sec")?,
                total_bytes_sec_max: get_u64(params, nparams, "total_bytes_sec_max")?,
                read_bytes_sec_max: get_u64(params, nparams, "read_bytes_sec_max")?,
                write_bytes_sec_max: get_u64(params, nparams, "write_bytes_sec_max")?,
                total_iops_sec_max: get_u64(params, nparams, "total_iops_sec_max")?,
                read_iops_sec_max: get_u64(params, nparams, "read_iops_sec_max")?,
                write_iops_sec_max: get_u64(params, nparams, "write_iops_sec_max")?,
                total_bytes_sec_max_length: get_u64(params, nparams, "total_bytes_sec_max_length")?,
                read_bytes_sec_max_length: get_u64(params, nparams, "read_bytes_sec_max_length")?,
                write_bytes_sec_max_length: get_u64(params, nparams, "write_bytes_sec_max_length")?,
                total_iops_sec_max_length: get_u64(params, nparams, "total_iops_sec_max_length")?,
                read_iops_sec_max_length: get_u64(params, nparams, "read_iops_sec_max_length")?,
                write_iops_sec_max_length: get_u64(params, nparams, "write_iops_sec_max_length")?,
                size_iops_sec: get_u64(params, nparams, "size_iops_sec")?,
            };

            libc::free(params as *mut libc::c_void);

            Ok(tune)
        }
    }

    pub fn network_stats(&self) -> Result<Vec<InterfaceStats>, Error> {
        unsafe {
            let n = get_u32(
                (*self.ptr).params,
                (*self.ptr).nparams,
                &format!("net.count"),
            )?;

            let mut array = vec![];
            for i in 0..n {
                array.push(InterfaceStats::new(
                    (*self.ptr).params,
                    (*self.ptr).nparams,
                    i,
                )?)
            }

            Ok(array)
        }
    }
}

#[derive(Clone, Debug)]
pub struct BlockInfo {
    pub name: String,
    pub backing_index: u32,
    pub path: String,
    pub read_requests: u64,
    pub read_bytes: u64,
    pub read_time: u64,
    pub write_requests: u64,
    pub write_bytes: u64,
    pub write_time: u64,
    pub flush_requests: u64,
    pub flush_time: u64,
    pub errors: u64,

    /// Logical size in bytes of the image (how much storage the guest
    /// will see).
    pub capacity: u64,
    /// Host storage in bytes occupied by the image (such as highest
    /// allocated extent if there are no holes, similar to 'du').
    pub allocation: u64,
    /// Host physical size in bytes of the image container (last
    /// offset, similar to 'ls')
    pub physical: u64,
}

impl BlockInfo {
    pub fn new(params: virTypedParameterPtr, nparams: u32, i: i32) -> Result<BlockInfo, Error> {
        Ok(Self {
            name: get_string(params, nparams, &format!("block.{}.name", i))?,
            backing_index: get_u32(params, nparams, &format!("block.{}.backingIndex", i))?,
            path: get_string(params, nparams, &format!("block.{}.path", i))?,
            read_requests: get_u64(params, nparams, &format!("block.{}.rd.reqs", i))?,
            read_bytes: get_u64(params, nparams, &format!("block.{}.rd.bytes", i))?,
            read_time: get_u64(params, nparams, &format!("block.{}.rd.times", i))?,
            write_requests: get_u64(params, nparams, &format!("block.{}.wr.reqs", i))?,
            write_bytes: get_u64(params, nparams, &format!("block.{}.wr.bytes", i))?,
            write_time: get_u64(params, nparams, &format!("block.{}.wr.times", i))?,
            flush_requests: get_u64(params, nparams, &format!("block.{}.fl.reqs", i))?,
            flush_time: get_u64(params, nparams, &format!("block.{}.fl.times", i))?,
            errors: get_u64(params, nparams, &format!("block.{}.errors", i))?,
            allocation: get_u64(params, nparams, &format!("block.{}.allocation", i))?,
            capacity: get_u64(params, nparams, &format!("block.{}.capacity", i))?,
            physical: get_u64(params, nparams, &format!("block.{}.physical", i))?,
        })
    }
}

#[derive(Clone, Debug)]
pub struct InterfaceStats {
    pub name: String,
    pub rx_bytes: u64,
    pub rx_packets: u64,
    pub rx_errs: u64,
    pub rx_drop: u64,
    pub tx_bytes: u64,
    pub tx_packets: u64,
    pub tx_errs: u64,
    pub tx_drop: u64,
}

impl InterfaceStats {
    pub fn new(params: virTypedParameterPtr, nparams: u32, i: u32) -> Result<Self, Error> {
        let stats = Self {
            name: get_string(params, nparams, &format!("net.{}.name", i))?,
            rx_bytes: get_u64(params, nparams, &format!("net.{}.rx.bytes", i))?,
            rx_packets: get_u64(params, nparams, &format!("net.{}.rx.pkts", i))?,
            rx_errs: get_u64(params, nparams, &format!("net.{}.rx.errs", i))?,
            rx_drop: get_u64(params, nparams, &format!("net.{}.rx.drop", i))?,
            tx_bytes: get_u64(params, nparams, &format!("net.{}.tx.bytes", i))?,
            tx_packets: get_u64(params, nparams, &format!("net.{}.tx.pkts", i))?,
            tx_errs: get_u64(params, nparams, &format!("net.{}.tx.errs", i))?,
            tx_drop: get_u64(params, nparams, &format!("net.{}.tx.drop", i))?,
        };

        Ok(stats)
    }
}

#[derive(Clone, Debug)]
pub struct MemoryStats {
    pub tag: i32,
    pub val: u64,
}

impl MemoryStats {
    pub fn from_ptr(ptr: sys::virDomainMemoryStatsPtr) -> MemoryStats {
        unsafe {
            MemoryStats {
                tag: (*ptr).tag as i32,
                val: (*ptr).val as u64,
            }
        }
    }
}

/// Provides APIs for the management of domains.
///
/// See http://libvirt.org/html/libvirt-libvirt-domain.html
#[derive(Debug)]
pub struct Domain {
    ptr: Option<sys::virDomainPtr>,
}

impl Drop for Domain {
    fn drop(&mut self) {
        if self.ptr.is_some() {
            if let Err(e) = self.free() {
                panic!(
                    "Unable to drop memory for Domain, code {}, message: {}",
                    e.code, e.message
                )
            }
        }
    }
}

#[derive(Debug)]
pub struct VcpuInfo {
    pub number: u32,
    // virtual CPU number
    pub state: i32,
    // value from virVcpuState
    pub cpu: i32,
    // real CPU number, or one of the value
    pub cpu_time: u64, // CPU time used, in nanoseco
}

impl From<virVcpuInfoPtr> for VcpuInfo {
    fn from(info_ptr: virVcpuInfoPtr) -> Self {
        let info = unsafe {
            Self {
                number: (*info_ptr).number as u32,
                state: (*info_ptr).state as i32,
                cpu: (*info_ptr).cpu as i32,
                cpu_time: (*info_ptr).cpuTime as u64,
            }
        };

        info
    }
}

impl Domain {
    pub fn new(ptr: sys::virDomainPtr) -> Domain {
        return Domain { ptr: Some(ptr) };
    }

    pub fn as_ptr(&self) -> sys::virDomainPtr {
        self.ptr.unwrap()
    }

    /// Extracts domain state.
    ///
    /// Each state can be accompanied with a reason (if known) which
    /// led to the state.
    pub fn get_state(&self) -> Result<(DomainState, i32), Error> {
        unsafe {
            let mut state: libc::c_int = -1;
            let mut reason: libc::c_int = -1;
            let ret = virDomainGetState(self.as_ptr(), &mut state, &mut reason, 0);
            if ret == -1 {
                return Err(Error::new());
            }
            return Ok((state as DomainState, reason as i32));
        }
    }

    /// Get the public name of the domain.
    pub fn get_name(&self) -> Result<String, Error> {
        unsafe {
            let n = virDomainGetName(self.as_ptr());
            if n.is_null() {
                return Err(Error::new());
            }
            return Ok(c_chars_to_string!(n, nofree));
        }
    }

    /// Get the UUID for a domain as string.
    ///
    /// For more information about UUID see RFC4122.
    pub fn get_uuid_string(&self) -> Result<String, Error> {
        unsafe {
            let mut uuid: [libc::c_char; 37] = [0; 37];
            if virDomainGetUUIDString(self.as_ptr(), uuid.as_mut_ptr()) == -1 {
                return Err(Error::new());
            }
            return Ok(c_chars_to_string!(uuid.as_ptr(), nofree));
        }
    }

    /// Provide an XML description of the domain. The description may
    /// be reused later to relaunch the domain with `create_xml()`.
    pub fn get_xml_desc(&self, flags: DomainCreateFlags) -> Result<String, Error> {
        unsafe {
            let xml = virDomainGetXMLDesc(self.as_ptr(), flags);
            if xml.is_null() {
                return Err(Error::new());
            }
            return Ok(c_chars_to_string!(xml));
        }
    }

    /// Extract information about a domain. Note that if the
    /// connection used to get the domain is limited only a partial
    /// set of the information can be extracted.
    pub fn get_info(&self) -> Result<DomainInfo, Error> {
        unsafe {
            let pinfo = &mut sys::virDomainInfo::default();
            let res = virDomainGetInfo(self.as_ptr(), pinfo);
            if res == -1 {
                return Err(Error::new());
            }
            return Ok(DomainInfo::from_ptr(pinfo));
        }
    }

    /// Free the domain object.
    ///
    /// The running instance is kept alive. The data structure is
    /// freed and should not be used thereafter.
    pub fn free(&mut self) -> Result<(), Error> {
        unsafe {
            if virDomainFree(self.as_ptr()) == -1 {
                return Err(Error::new());
            }
            self.ptr = None;
            return Ok(());
        }
    }

    pub fn get_vcpus(&self) -> Result<Vec<VcpuInfo>, Error> {
        let node_info = self.get_info()?;
        let maxinfo = node_info.nr_virt_cpu;
        unsafe {
            let mut vcpus: Vec<virVcpuInfo> = Vec::with_capacity(maxinfo as usize);
            vcpus.set_len(maxinfo as usize);
            let ret = virDomainGetVcpus(
                self.as_ptr(),
                vcpus.as_mut_ptr(),
                maxinfo as libc::c_int,
                ptr::null(),
                0 as libc::c_int,
            );
            if ret == -1 {
                return Err(Error::new());
            }

            let mut array = Vec::with_capacity(ret as usize);
            for i in 0..ret as usize {
                array.push(VcpuInfo {
                    number: vcpus[i].number,
                    state: vcpus[i].state,
                    cpu: vcpus[i].cpu,
                    cpu_time: vcpus[i].cpuTime,
                })
            }

            Ok(array)
        }
    }

    pub fn memory_stats(&self, nr_stats: u32, flags: u32) -> Result<Vec<MemoryStats>, Error> {
        unsafe {
            let mut infos: Vec<virDomainMemoryStats> = Vec::with_capacity(nr_stats as usize);
            infos.set_len(infos.capacity());
            let ret = virDomainMemoryStats(
                self.as_ptr(),
                infos.as_mut_ptr(),
                nr_stats as libc::c_uint,
                flags as libc::c_uint,
            );
            if ret == -1 {
                return Err(Error::new());
            }

            let mut array = Vec::with_capacity(ret as usize);
            for x in 0..ret as usize {
                array.push(MemoryStats {
                    tag: infos[x].tag,
                    val: infos[x].val,
                })
            }

            Ok(array)
        }
    }
}
