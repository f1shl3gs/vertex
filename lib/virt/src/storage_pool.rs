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

use crate::error::Error;

pub mod sys {
    extern crate libc;

    #[repr(C)]
    pub struct virStoragePool {}

    pub type virStoragePoolPtr = *mut virStoragePool;

    #[repr(C)]
    #[derive(Default)]
    pub struct virStoragePoolInfo {
        pub state: libc::c_int,
        pub capacity: libc::c_ulonglong,
        pub allocation: libc::c_ulonglong,
        pub available: libc::c_ulonglong,
    }

    pub type virStoragePoolInfoPtr = *mut virStoragePoolInfo;
}

#[link(name = "virt")]
extern "C" {
    fn virStoragePoolRefresh(ptr: sys::virStoragePoolPtr, flags: libc::c_uint) -> libc::c_int;
    fn virStoragePoolFree(ptr: sys::virStoragePoolPtr) -> libc::c_int;
    fn virStoragePoolIsActive(ptr: sys::virStoragePoolPtr) -> libc::c_int;
    fn virStoragePoolIsPersistent(ptr: sys::virStoragePoolPtr) -> libc::c_int;
    fn virStoragePoolGetName(ptr: sys::virStoragePoolPtr) -> *const libc::c_char;
    fn virStoragePoolGetUUIDString(
        ptr: sys::virStoragePoolPtr,
        uuid: *mut libc::c_char,
    ) -> libc::c_int;
    fn virStoragePoolGetInfo(
        ptr: sys::virStoragePoolPtr,
        info: sys::virStoragePoolInfoPtr,
    ) -> libc::c_int;
}

pub type StoragePoolXMLFlags = self::libc::c_uint;
pub const VIR_STORAGE_POOL_XML_INACTIVE: StoragePoolXMLFlags = 1 << 0;

pub type StoragePoolCreateFlags = self::libc::c_uint;
pub const STORAGE_POOL_CREATE_NORMAL: StoragePoolCreateFlags = 0;
pub const STORAGE_POOL_CREATE_WITH_BUILD: StoragePoolCreateFlags = 1 << 0;
pub const STORAGE_POOL_CREATE_WITH_BUILD_OVERWRITE: StoragePoolCreateFlags = 1 << 1;
pub const STORAGE_POOL_CREATE_WITH_BUILD_NO_OVERWRITE: StoragePoolCreateFlags = 1 << 2;

pub type StoragePoolState = self::libc::c_uint;
pub const VIR_STORAGE_POOL_INACTIVE: StoragePoolState = 0;
pub const VIR_STORAGE_POOL_BUILDING: StoragePoolState = 1;
pub const VIR_STORAGE_POOL_RUNNING: StoragePoolState = 2;
pub const VIR_STORAGE_POOL_DEGRADED: StoragePoolState = 3;
pub const VIR_STORAGE_POOL_INACCESSIBLE: StoragePoolState = 4;

pub type StoragePoolListAllVolumesFlags = self::libc::c_uint;
pub const VIR_STORAGE_POOL_DEFAULT: StoragePoolListAllVolumesFlags = 0;

#[derive(Clone, Debug)]
pub struct StoragePoolInfo {
    /// A `StoragePoolState` flags
    pub state: u32,
    /// Logical size bytes.
    pub capacity: u64,
    /// Current allocation bytes.
    pub allocation: u64,
    /// Remaining free space bytes.
    pub available: u64,
}

impl StoragePoolInfo {
    pub fn from_ptr(ptr: sys::virStoragePoolInfoPtr) -> StoragePoolInfo {
        unsafe {
            StoragePoolInfo {
                state: (*ptr).state as StoragePoolState,
                capacity: (*ptr).capacity as u64,
                allocation: (*ptr).allocation as u64,
                available: (*ptr).available as u64,
            }
        }
    }
}

/// Provides APIs for the management of storage pools.
///
/// See http://libvirt.org/html/libvirt-libvirt-storage.html
#[derive(Debug)]
pub struct StoragePool {
    ptr: Option<sys::virStoragePoolPtr>,
}

impl Drop for StoragePool {
    fn drop(&mut self) {
        if self.ptr.is_some() {
            if let Err(e) = self.free() {
                panic!(
                    "Unable to drop memory for StoragePool, code {}, message: {}",
                    e.code, e.message
                )
            }
        }
    }
}

impl StoragePool {
    pub fn new(ptr: sys::virStoragePoolPtr) -> StoragePool {
        return StoragePool { ptr: Some(ptr) };
    }

    pub fn as_ptr(&self) -> sys::virStoragePoolPtr {
        self.ptr.unwrap()
    }

    pub fn get_name(&self) -> Result<String, Error> {
        unsafe {
            let n = virStoragePoolGetName(self.as_ptr());
            if n.is_null() {
                return Err(Error::new());
            }
            return Ok(c_chars_to_string!(n, nofree));
        }
    }

    pub fn get_uuid_string(&self) -> Result<String, Error> {
        unsafe {
            let mut uuid: [libc::c_char; 37] = [0; 37];
            if virStoragePoolGetUUIDString(self.as_ptr(), uuid.as_mut_ptr()) == -1 {
                return Err(Error::new());
            }
            return Ok(c_chars_to_string!(uuid.as_ptr(), nofree));
        }
    }

    pub fn free(&mut self) -> Result<(), Error> {
        unsafe {
            if virStoragePoolFree(self.as_ptr()) == -1 {
                return Err(Error::new());
            }
            self.ptr = None;
            return Ok(());
        }
    }

    pub fn is_active(&self) -> Result<bool, Error> {
        unsafe {
            let ret = virStoragePoolIsActive(self.as_ptr());
            if ret == -1 {
                return Err(Error::new());
            }
            return Ok(ret == 1);
        }
    }

    pub fn is_persistent(&self) -> Result<bool, Error> {
        unsafe {
            let ret = virStoragePoolIsPersistent(self.as_ptr());
            if ret == -1 {
                return Err(Error::new());
            }
            return Ok(ret == 1);
        }
    }

    pub fn refresh(&self, flags: u32) -> Result<u32, Error> {
        unsafe {
            let ret = virStoragePoolRefresh(self.as_ptr(), flags as libc::c_uint);
            if ret == -1 {
                return Err(Error::new());
            }
            return Ok(ret as u32);
        }
    }

    pub fn get_info(&self) -> Result<StoragePoolInfo, Error> {
        unsafe {
            let pinfo = &mut sys::virStoragePoolInfo::default();
            let res = virStoragePoolGetInfo(self.as_ptr(), pinfo);
            if res == -1 {
                return Err(Error::new());
            }
            return Ok(StoragePoolInfo::from_ptr(pinfo));
        }
    }
}
