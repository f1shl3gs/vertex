use crate::error::Error;
use crate::typedparam::sys::virTypedParameterPtr;

#[link(name = "virt")]
extern "C" {
    fn virTypedParamsGetInt(
        params: virTypedParameterPtr,
        nparams: libc::c_uint,
        name: *const libc::c_char,
        value: *mut libc::c_int,
    ) -> libc::c_int;
    fn virTypedParamsGetUInt(
        params: virTypedParameterPtr,
        nparams: libc::c_uint,
        name: *const libc::c_char,
        value: *mut libc::c_uint,
    ) -> libc::c_int;
    fn virTypedParamsGetULLong(
        params: virTypedParameterPtr,
        nparams: libc::c_uint,
        name: *const libc::c_char,
        value: *mut libc::c_ulonglong,
    ) -> libc::c_int;
    fn virTypedParamsGetString(
        params: virTypedParameterPtr,
        nparams: libc::c_uint,
        name: *const libc::c_char,
        value: *mut *mut libc::c_char,
    ) -> libc::c_int;
}

pub fn get_int(params: virTypedParameterPtr, nparams: u32, name: &str) -> Result<i32, Error> {
    unsafe {
        let mut value: libc::c_int = 0;
        if virTypedParamsGetInt(params, nparams, string_to_c_chars!(name), &mut value) == -1 {
            return Err(Error::new());
        }

        return Ok(value as i32);
    }
}

pub fn get_u64(params: virTypedParameterPtr, nparams: u32, name: &str) -> Result<u64, Error> {
    unsafe {
        let mut value: libc::c_ulonglong = 0;
        if virTypedParamsGetULLong(params, nparams, string_to_c_chars!(name), &mut value) == -1 {
            return Err(Error::new());
        }

        return Ok(value as u64);
    }
}

pub fn get_u32(params: virTypedParameterPtr, nparams: u32, name: &str) -> Result<u32, Error> {
    unsafe {
        let mut value: libc::c_uint = 0;
        if virTypedParamsGetUInt(params, nparams, string_to_c_chars!(name), &mut value) == -1 {
            return Err(Error::new());
        }

        return Ok(value as u32);
    }
}

pub fn get_string(params: virTypedParameterPtr, nparams: u32, name: &str) -> Result<String, Error> {
    unsafe {
        let mut value = std::ptr::null_mut();

        if virTypedParamsGetString(params, nparams, string_to_c_chars!(name), &mut value) == -1 {
            return Err(Error::new());
        }

        Ok(c_chars_to_string!(value))
    }
}
