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

use std::{mem, ptr, str};

use crate::domain::sys::virDomainStatsRecordPtr;
use crate::storage_pool::sys::virStoragePoolPtr;

use crate::domain::DomainStatsRecord;
use crate::error::Error;
use crate::storage_pool::StoragePool;

pub mod sys {
    extern crate libc;

    #[repr(C)]
    pub struct virConnect {}

    pub type virConnectPtr = *mut virConnect;

    #[repr(C)]
    pub struct virConnectCredential {
        pub typed: libc::c_int,
        pub prompt: *const libc::c_char,
        pub challenge: *const libc::c_char,
        pub defresult: *const libc::c_char,
        pub result: *mut libc::c_char,
        pub resultlen: libc::c_uint,
    }

    pub type virConnectCredentialPtr = *mut virConnectCredential;

    pub type virConnectAuthCallbackPtr =
        unsafe extern "C" fn(virConnectCredentialPtr, libc::c_uint, *mut libc::c_void) -> i32;

    #[repr(C)]
    pub struct virConnectAuth {
        pub credtype: *mut libc::c_int,
        pub ncredtype: libc::c_uint,
        pub cb: virConnectAuthCallbackPtr,
        pub cbdata: *mut libc::c_void,
    }

    pub type virConnectAuthPtr = *mut virConnectAuth;

    #[repr(C)]
    #[derive(Default)]
    pub struct virNodeInfo {
        pub model: [libc::c_char; 32],
        pub memory: libc::c_ulong,
        pub cpus: libc::c_uint,
        pub mhz: libc::c_uint,
        pub nodes: libc::c_uint,
        pub sockets: libc::c_uint,
        pub cores: libc::c_uint,
        pub threads: libc::c_uint,
    }

    pub type virNodeInfoPtr = *mut virNodeInfo;
}

#[link(name = "virt")]
extern "C" {
    fn virGetVersion(
        hyver: *const libc::c_ulong,
        ctype: *const libc::c_char,
        typever: *const libc::c_ulong,
    ) -> libc::c_int;
    fn virConnectOpen(uri: *const libc::c_char) -> sys::virConnectPtr;
    fn virConnectOpenReadOnly(uri: *const libc::c_char) -> sys::virConnectPtr;
    fn virConnectOpenAuth(
        uri: *const libc::c_char,
        auth: sys::virConnectAuthPtr,
        flags: libc::c_uint,
    ) -> sys::virConnectPtr;
    fn virConnectClose(ptr: sys::virConnectPtr) -> libc::c_int;
    fn virConnectGetVersion(ptr: sys::virConnectPtr, hyver: *mut libc::c_ulong) -> libc::c_int;
    fn virConnectGetLibVersion(ptr: sys::virConnectPtr, ver: *mut libc::c_ulong) -> libc::c_int;
    fn virConnectListAllStoragePools(
        ptr: sys::virConnectPtr,
        storages: *mut *mut virStoragePoolPtr,
        flags: libc::c_uint,
    ) -> libc::c_int;
    fn virConnectGetAllDomainStats(
        ptr: sys::virConnectPtr,
        stats: libc::c_uint,
        ret: *mut *mut virDomainStatsRecordPtr,
        flags: libc::c_uint,
    ) -> libc::c_int;
    fn virConnectSetKeepAlive(
        ptr: sys::virConnectPtr,
        interval: libc::c_int,
        count: libc::c_uint,
    ) -> libc::c_int;
}

extern "C" fn connectCallback(
    ccreds: sys::virConnectCredentialPtr,
    ncred: libc::c_uint,
    cbdata: *mut libc::c_void,
) -> libc::c_int {
    let callback: ConnectAuthCallback = unsafe {
        // Safe because connectCallback is private and only used by
        // Connect::open_auth(). In open_auth() we transmute the
        // callback allocate in *void.
        mem::transmute(cbdata)
    };
    let mut rcreds: Vec<ConnectCredential> = Vec::new();
    for i in 0..ncred as isize {
        unsafe {
            // Safe because ccreds is allocated.
            let c = ConnectCredential::from_ptr(ccreds.offset(i));
            rcreds.push(c);
        }
    }
    callback(&mut rcreds);
    for i in 0..ncred as isize {
        if rcreds[i as usize].result.is_some() {
            if let Some(ref result) = rcreds[i as usize].result {
                unsafe {
                    // Safe because ccreds is allocated and the result
                    // is comming from Rust calls.
                    (*ccreds.offset(i)).resultlen = result.len() as libc::c_uint;
                    (*ccreds.offset(i)).result = string_to_mut_c_chars!(result.clone());
                }
            }
        }
    }
    0
}

pub type ConnectFlags = self::libc::c_uint;
pub const VIR_CONNECT_RO: ConnectFlags = 1 << 0;
pub const VIR_CONNECT_NO_ALIASES: ConnectFlags = 1 << 1;

pub type ConnectListAllNodeDeviceFlags = self::libc::c_uint;
pub const VIR_CONNECT_LIST_NODE_DEVICES_CAP_SYSTEM: ConnectListAllNodeDeviceFlags = 1 << 0;
pub const VIR_CONNECT_LIST_NODE_DEVICES_CAP_PCI_DEV: ConnectListAllNodeDeviceFlags = 1 << 1;
pub const VIR_CONNECT_LIST_NODE_DEVICES_CAP_USB_DEV: ConnectListAllNodeDeviceFlags = 1 << 2;
pub const VIR_CONNECT_LIST_NODE_DEVICES_CAP_USB_INTERFACE: ConnectListAllNodeDeviceFlags = 1 << 3;
pub const VIR_CONNECT_LIST_NODE_DEVICES_CAP_NET: ConnectListAllNodeDeviceFlags = 1 << 4;
pub const VIR_CONNECT_LIST_NODE_DEVICES_CAP_SCSI_HOST: ConnectListAllNodeDeviceFlags = 1 << 5;
pub const VIR_CONNECT_LIST_NODE_DEVICES_CAP_SCSI_TARGET: ConnectListAllNodeDeviceFlags = 1 << 6;
pub const VIR_CONNECT_LIST_NODE_DEVICES_CAP_SCSI: ConnectListAllNodeDeviceFlags = 1 << 7;
pub const VIR_CONNECT_LIST_NODE_DEVICES_CAP_STORAGE: ConnectListAllNodeDeviceFlags = 1 << 8;
pub const VIR_CONNECT_LIST_NODE_DEVICES_CAP_FC_HOST: ConnectListAllNodeDeviceFlags = 1 << 9;
pub const VIR_CONNECT_LIST_NODE_DEVICES_CAP_VPORTS: ConnectListAllNodeDeviceFlags = 1 << 10;
pub const VIR_CONNECT_LIST_NODE_DEVICES_CAP_SCSI_GENERIC: ConnectListAllNodeDeviceFlags = 1 << 11;
pub const VIR_CONNECT_LIST_NODE_DEVICES_CAP_DRM: ConnectListAllNodeDeviceFlags = 1 << 12;

pub type ConnectListAllSecretsFlags = self::libc::c_uint;
pub const VIR_CONNECT_LIST_SECRETS_EPHEMERAL: ConnectListAllSecretsFlags = 1 << 0;
pub const VIR_CONNECT_LIST_SECRETS_NO_EPHEMERAL: ConnectListAllSecretsFlags = 1 << 1;
pub const VIR_CONNECT_LIST_SECRETS_PRIVATE: ConnectListAllSecretsFlags = 1 << 2;
pub const VIR_CONNECT_LIST_SECRETS_NO_PRIVATE: ConnectListAllSecretsFlags = 1 << 3;

pub type ConnectListAllDomainsFlags = self::libc::c_uint;
pub const VIR_CONNECT_LIST_DOMAINS_ACTIVE: ConnectListAllDomainsFlags = 1 << 0;
pub const VIR_CONNECT_LIST_DOMAINS_INACTIVE: ConnectListAllDomainsFlags = 1 << 1;
pub const VIR_CONNECT_LIST_DOMAINS_PERSISTENT: ConnectListAllDomainsFlags = 1 << 2;
pub const VIR_CONNECT_LIST_DOMAINS_TRANSIENT: ConnectListAllDomainsFlags = 1 << 3;
pub const VIR_CONNECT_LIST_DOMAINS_RUNNING: ConnectListAllDomainsFlags = 1 << 4;
pub const VIR_CONNECT_LIST_DOMAINS_PAUSED: ConnectListAllDomainsFlags = 1 << 5;
pub const VIR_CONNECT_LIST_DOMAINS_SHUTOFF: ConnectListAllDomainsFlags = 1 << 6;
pub const VIR_CONNECT_LIST_DOMAINS_OTHER: ConnectListAllDomainsFlags = 1 << 7;
pub const VIR_CONNECT_LIST_DOMAINS_MANAGEDSAVE: ConnectListAllDomainsFlags = 1 << 8;
pub const VIR_CONNECT_LIST_DOMAINS_NO_MANAGEDSAVE: ConnectListAllDomainsFlags = 1 << 9;
pub const VIR_CONNECT_LIST_DOMAINS_AUTOSTART: ConnectListAllDomainsFlags = 1 << 10;
pub const VIR_CONNECT_LIST_DOMAINS_NO_AUTOSTART: ConnectListAllDomainsFlags = 1 << 11;
pub const VIR_CONNECT_LIST_DOMAINS_HAS_SNAPSHOT: ConnectListAllDomainsFlags = 1 << 12;
pub const VIR_CONNECT_LIST_DOMAINS_NO_SNAPSHOT: ConnectListAllDomainsFlags = 1 << 13;

pub type ConnectListAllNetworksFlags = self::libc::c_uint;
pub const VIR_CONNECT_LIST_NETWORKS_INACTIVE: ConnectListAllNetworksFlags = 1 << 0;
pub const VIR_CONNECT_LIST_NETWORKS_ACTIVE: ConnectListAllNetworksFlags = 1 << 1;
pub const VIR_CONNECT_LIST_NETWORKS_PERSISTENT: ConnectListAllNetworksFlags = 1 << 2;
pub const VIR_CONNECT_LIST_NETWORKS_TRANSIENT: ConnectListAllNetworksFlags = 1 << 3;
pub const VIR_CONNECT_LIST_NETWORKS_AUTOSTART: ConnectListAllNetworksFlags = 1 << 4;
pub const VIR_CONNECT_LIST_NETWORKS_NO_AUTOSTART: ConnectListAllNetworksFlags = 1 << 5;

pub type ConnectListAllInterfacesFlags = self::libc::c_uint;
pub const VIR_CONNECT_LIST_INTERFACES_INACTIVE: ConnectListAllInterfacesFlags = 1 << 0;
pub const VIR_CONNECT_LIST_INTERFACES_ACTIVE: ConnectListAllInterfacesFlags = 1 << 1;

pub type ConnectListAllStoragePoolsFlags = self::libc::c_uint;
pub const VIR_CONNECT_LIST_STORAGE_POOLS_INACTIVE: ConnectListAllStoragePoolsFlags = 1 << 0;
pub const VIR_CONNECT_LIST_STORAGE_POOLS_ACTIVE: ConnectListAllStoragePoolsFlags = 1 << 1;
pub const VIR_CONNECT_LIST_STORAGE_POOLS_PERSISTENT: ConnectListAllStoragePoolsFlags = 1 << 2;
pub const VIR_CONNECT_LIST_STORAGE_POOLS_TRANSIENT: ConnectListAllStoragePoolsFlags = 1 << 3;
pub const VIR_CONNECT_LIST_STORAGE_POOLS_AUTOSTART: ConnectListAllStoragePoolsFlags = 1 << 4;
pub const VIR_CONNECT_LIST_STORAGE_POOLS_NO_AUTOSTART: ConnectListAllStoragePoolsFlags = 1 << 5;
pub const VIR_CONNECT_LIST_STORAGE_POOLS_DIR: ConnectListAllStoragePoolsFlags = 1 << 6;
pub const VIR_CONNECT_LIST_STORAGE_POOLS_FS: ConnectListAllStoragePoolsFlags = 1 << 7;
pub const VIR_CONNECT_LIST_STORAGE_POOLS_NETFS: ConnectListAllStoragePoolsFlags = 1 << 8;
pub const VIR_CONNECT_LIST_STORAGE_POOLS_LOGICAL: ConnectListAllStoragePoolsFlags = 1 << 9;
pub const VIR_CONNECT_LIST_STORAGE_POOLS_DISK: ConnectListAllStoragePoolsFlags = 1 << 10;
pub const VIR_CONNECT_LIST_STORAGE_POOLS_ISCSI: ConnectListAllStoragePoolsFlags = 1 << 11;
pub const VIR_CONNECT_LIST_STORAGE_POOLS_SCSI: ConnectListAllStoragePoolsFlags = 1 << 12;
pub const VIR_CONNECT_LIST_STORAGE_POOLS_MPATH: ConnectListAllStoragePoolsFlags = 1 << 13;
pub const VIR_CONNECT_LIST_STORAGE_POOLS_RBD: ConnectListAllStoragePoolsFlags = 1 << 14;
pub const VIR_CONNECT_LIST_STORAGE_POOLS_SHEEPDOG: ConnectListAllStoragePoolsFlags = 1 << 15;
pub const VIR_CONNECT_LIST_STORAGE_POOLS_GLUSTER: ConnectListAllStoragePoolsFlags = 1 << 16;
pub const VIR_CONNECT_LIST_STORAGE_POOLS_ZFS: ConnectListAllStoragePoolsFlags = 1 << 17;
pub const VIR_CONNECT_LIST_STORAGE_POOLS_VSTORAGE: ConnectListAllStoragePoolsFlags = 1 << 18;

pub type ConnectCompareCPUFlags = self::libc::c_uint;
pub const VIR_CONNECT_COMPARE_CPU_FAIL_INCOMPATIBLE: ConnectCompareCPUFlags = 1 << 0;

pub type CPUCompareResult = self::libc::c_int;
pub const VIR_CPU_COMPARE_ERROR: CPUCompareResult = -1;
pub const VIR_CPU_COMPARE_INCOMPATIBLE: CPUCompareResult = 0;
pub const VIR_CPU_COMPARE_IDENTICAL: CPUCompareResult = 1;
pub const VIR_CPU_COMPARE_SUPERSET: CPUCompareResult = 2;

pub type BaselineCPUFlags = self::libc::c_int;
pub const VIR_CONNECT_BASELINE_CPU_EXPAND_FEATURES: BaselineCPUFlags = 1 << 0;
pub const VIR_CONNECT_BASELINE_CPU_MIGRATABLE: BaselineCPUFlags = 1 << 1;

pub type ConnectCredentialType = self::libc::c_int;
pub const VIR_CRED_USERNAME: ConnectCredentialType = 1;
pub const VIR_CRED_AUTHNAME: ConnectCredentialType = 2;
pub const VIR_CRED_LANGUAGE: ConnectCredentialType = 3;
pub const VIR_CRED_CNONCE: ConnectCredentialType = 4;
pub const VIR_CRED_PASSPHRASE: ConnectCredentialType = 5;
pub const VIR_CRED_ECHOPROMPT: ConnectCredentialType = 6;
pub const VIR_CRED_NOECHOPROMPT: ConnectCredentialType = 7;
pub const VIR_CRED_REALM: ConnectCredentialType = 8;
pub const VIR_CRED_EXTERNAL: ConnectCredentialType = 9;

#[derive(Clone, Debug)]
pub struct NodeInfo {
    /// Indicating the CPU model.
    pub model: String,
    /// Memory size in kilobytes.
    pub memory: u64,
    /// The number of active CPUs.
    pub cpus: u32,
    /// expected CPU frequency, 0 if not known or on unusual
    /// architectures.
    pub mhz: u32,
    /// The number of NUMA cell, 1 for unusual NUMA topologies or
    /// uniform memory access; check capabilities XML for the actual
    /// NUMA topology
    pub nodes: u32,
    /// Number of CPU sockets per node if nodes > 1, 1 in case of
    /// unusual NUMA topology.
    pub sockets: u32,
    /// Number of cores per socket, total number of processors in case
    /// of unusual NUMA topology
    pub cores: u32,
    /// Number of threads per core, 1 in case of unusual numa topology
    pub threads: u32,
}

// TODO(sahid): should support closure
pub type ConnectAuthCallback = fn(creds: &mut Vec<ConnectCredential>);

#[derive(Clone, Debug)]
pub struct ConnectCredential {
    /// One of `ConnectCredentialType` constants
    pub typed: i32,
    /// Prompt to show to user.
    pub prompt: String,
    /// Additional challenge to show.
    pub challenge: String,
    /// Optional default result.
    pub def_result: String,
    /// Result to be filled with user response (or def_result).
    pub result: Option<String>,
}

impl ConnectCredential {
    pub fn from_ptr(cred: sys::virConnectCredentialPtr) -> ConnectCredential {
        unsafe {
            let mut default: String = String::from("");
            if !(*cred).defresult.is_null() {
                default = c_chars_to_string!((*cred).defresult, nofree);
            }
            ConnectCredential {
                typed: (*cred).typed,
                prompt: c_chars_to_string!((*cred).prompt, nofree),
                challenge: c_chars_to_string!((*cred).challenge, nofree),
                def_result: default,
                result: None,
            }
        }
    }
}

pub struct ConnectAuth {
    /// List of supported `ConnectCredentialType` values.
    creds: Vec<ConnectCredentialType>,
    /// Callback used to collect credentials.
    callback: ConnectAuthCallback,
}

impl ConnectAuth {
    pub fn new(creds: Vec<ConnectCredentialType>, callback: ConnectAuthCallback) -> ConnectAuth {
        ConnectAuth {
            creds: creds,
            callback: callback,
        }
    }
}

/// Provides APIs for the management of hosts.
///
/// See http://libvirt.org/html/libvirt-libvirt-host.html
#[derive(Debug)]
pub struct Connect {
    ptr: Option<sys::virConnectPtr>,
}

impl Connect {
    pub fn as_ptr(&self) -> sys::virConnectPtr {
        self.ptr.unwrap()
    }

    pub fn new(ptr: sys::virConnectPtr) -> Connect {
        return Connect { ptr: Some(ptr) };
    }

    pub fn get_version() -> Result<u32, Error> {
        unsafe {
            let ver: libc::c_ulong = 0;
            if virGetVersion(&ver, ptr::null(), ptr::null()) == -1 {
                return Err(Error::new());
            }
            return Ok(ver as u32);
        }
    }

    /// This function should be called first to get a connection to
    /// the Hypervisor and xen store.
    ///
    /// If @uri is "", if the LIBVIRT_DEFAULT_URI environment
    /// variable is set, then it will be used. Otherwise if the client
    /// configuration file has the "uri_default" parameter set, then
    /// it will be used. Finally probing will be done to determine a
    /// suitable default driver to activate. This involves trying each
    /// hypervisor in turn until one successfully opens.
    ///
    /// If connecting to an unprivileged hypervisor driver which
    /// requires the libvirtd daemon to be active, it will
    /// automatically be launched if not already running. This can be
    /// prevented by setting the environment variable
    /// LIBVIRT_AUTOSTART=0
    ///
    /// URIs are documented at http://libvirt.org/uri.html
    ///
    /// Connect.close should be used to release the resources after the
    /// connection is no longer needed.
    ///
    /// # Examples
    ///
    /// ```
    /// use virt::connect::Connect;
    ///
    /// match Connect::open("test:///default") {
    ///   Ok(mut conn) => {
    ///       assert_eq!(Ok(0), conn.close());
    ///   },
    ///   Err(e) => panic!(
    ///     "failed with code {}, message: {}", e.code, e.message)
    /// }
    /// ```
    pub fn open(uri: &str) -> Result<Connect, Error> {
        unsafe {
            let c = virConnectOpen(string_to_c_chars!(uri));
            if c.is_null() {
                return Err(Error::new());
            }
            return Ok(Connect::new(c));
        }
    }

    /// This function should be called first to get a restricted
    /// connection to the library functionalities. The set of APIs
    /// usable are then restricted on the available methods to control
    /// the domains.
    ///
    /// See 'new' for notes about environment variables which can have
    /// an effect on opening drivers and freeing the connection
    /// resources.
    ///
    /// # Examples
    ///
    /// ```
    /// use virt::connect::Connect;
    ///
    /// match Connect::open_read_only("test:///default") {
    ///   Ok(mut conn) => {
    ///     assert_eq!(Ok(0), conn.close());
    ///   },
    ///   Err(e) => panic!(
    ///     "failed with code {}, message: {}", e.code, e.message)
    /// }
    /// ```
    pub fn open_read_only(uri: &str) -> Result<Connect, Error> {
        unsafe {
            let c = virConnectOpenReadOnly(string_to_c_chars!(uri));
            if c.is_null() {
                return Err(Error::new());
            }
            return Ok(Connect::new(c));
        }
    }

    pub fn open_auth(
        uri: &str,
        auth: &mut ConnectAuth,
        flags: ConnectFlags,
    ) -> Result<Connect, Error> {
        let mut cauth = unsafe {
            // Safe because Rust forces to allocate all attributes of
            // the struct ConnectAuth.
            sys::virConnectAuth {
                credtype: &mut auth.creds[0],
                ncredtype: auth.creds.len() as libc::c_uint,
                cb: connectCallback,
                cbdata: mem::transmute(auth.callback),
            }
        };
        let c = unsafe {
            virConnectOpenAuth(string_to_c_chars!(uri), &mut cauth, flags as libc::c_uint)
        };
        if c.is_null() {
            return Err(Error::new());
        }
        return Ok(Connect::new(c));
    }

    /// This function closes the connection to the hypervisor. This
    /// should not be called if further interaction with the
    /// hypervisor are needed especially if there is running domain
    /// which need further monitoring by the application.
    pub fn close(&mut self) -> Result<i32, Error> {
        unsafe {
            let ret = virConnectClose(self.as_ptr());
            if ret == -1 {
                return Err(Error::new());
            }
            if ret == 0 {
                self.ptr = None;
            }
            Ok(ret)
        }
    }

    pub fn get_lib_version(&self) -> Result<u32, Error> {
        unsafe {
            let mut ver: libc::c_ulong = 0;
            if virConnectGetLibVersion(self.as_ptr(), &mut ver) == -1 {
                return Err(Error::new());
            }
            return Ok(ver as u32);
        }
    }

    pub fn list_all_storage_pools(
        &self,
        flags: ConnectListAllStoragePoolsFlags,
    ) -> Result<Vec<StoragePool>, Error> {
        unsafe {
            let mut storages: *mut virStoragePoolPtr = ptr::null_mut();
            let size =
                virConnectListAllStoragePools(self.as_ptr(), &mut storages, flags as libc::c_uint);
            if size == -1 {
                return Err(Error::new());
            }

            let mut array: Vec<StoragePool> = Vec::new();
            for x in 0..size as isize {
                array.push(StoragePool::new(*storages.offset(x)));
            }
            libc::free(storages as *mut libc::c_void);

            return Ok(array);
        }
    }

    /// Connect.close should be used to release the resources after the
    /// connection is no longer needed.
    ///
    /// # Examples
    ///
    /// ```
    /// use virt::connect::Connect;
    ///
    /// match Connect::open("test:///default") {
    ///   Ok(mut conn) => {
    ///       match conn.get_hyp_version() {
    ///         Ok(hyver) => assert_eq!(2, hyver),
    ///         Err(e) => panic!(
    ///           "failed with code {}, message: {}", e.code, e.message)
    ///       }
    ///       assert_eq!(Ok(0), conn.close());
    ///   },
    ///   Err(e) => panic!(
    ///     "failed with code {}, message: {}", e.code, e.message)
    /// }
    /// ```
    pub fn get_hyp_version(&self) -> Result<u32, Error> {
        unsafe {
            let mut hyver: libc::c_ulong = 0;
            if virConnectGetVersion(self.as_ptr(), &mut hyver) == -1 {
                return Err(Error::new());
            }
            return Ok(hyver as u32);
        }
    }

    pub fn get_all_domain_stats(
        &self,
        stats: u32,
        flags: u32,
    ) -> Result<Vec<DomainStatsRecord>, Error> {
        unsafe {
            let mut record: *mut virDomainStatsRecordPtr = ptr::null_mut();
            let size = virConnectGetAllDomainStats(
                self.as_ptr(),
                stats as libc::c_uint,
                &mut record,
                flags as libc::c_uint,
            );
            if size == -1 {
                return Err(Error::new());
            }

            let mut array: Vec<DomainStatsRecord> = Vec::new();
            for x in 0..size as isize {
                array.push(DomainStatsRecord {
                    ptr: *record.offset(x),
                });
            }
            libc::free(record as *mut libc::c_void);

            return Ok(array);
        }
    }

    // See also. https://libvirt.org/html/libvirt-libvirt-host.html#virConnectSetKeepAlive
    pub fn set_keep_alive(&self, interval: i32, count: u32) -> Result<i32, Error> {
        unsafe {
            let ret = virConnectSetKeepAlive(
                self.as_ptr(),
                interval as libc::c_int,
                count as libc::c_uint,
            );
            if ret == -1 {
                return Err(Error::new());
            }
            Ok(ret as i32)
        }
    }
}

impl Drop for Connect {
    fn drop(&mut self) {
        self.close()
            .expect("connect should be closed when drop");
    }
}