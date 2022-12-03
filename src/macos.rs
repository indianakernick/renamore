#![allow(non_camel_case_types)]

use std::path::Path;
use std::io::Result;
use std::ffi::{c_char, c_int, c_uint, CString, c_ulong};
use std::os::unix::prelude::OsStrExt;

// Supported on:
//  - macOS 10.12
//  - iOS 10.0
//  - tvOS 10.0
//  - watchOS 3.0

extern "C" {
    fn renamex_np(from: *const c_char, to: *const c_char, flags: c_uint) -> c_int;
}

// const RENAME_SWAP: c_uint = 2;
const RENAME_EXCL: c_uint = 4;

pub fn rename_exclusive(from: &Path, to: &Path) -> Result<()> {
    let from_str = CString::new(from.as_os_str().as_bytes())?;
    let to_str = CString::new(to.as_os_str().as_bytes())?;
    let ret = unsafe {
        renamex_np(from_str.as_ptr(), to_str.as_ptr(), RENAME_EXCL)
    };

    if ret == -1 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[repr(C)]
struct attrlist {
    bitmapcount: u16,
    reserved: u16,
    commonattr: u32,
    volattr: u32,
    dirattr: u32,
    fileattr: u32,
    forkattr: u32,
}

const ATTR_BIT_MAP_COUNT: u16 = 5;
const ATTR_VOL_CAPABILITIES: u32 = 0x00020000;

type vol_capabilities_set_t = [u32; 4];

const VOL_CAPABILITIES_INTERFACES: usize = 1;

#[repr(C)]
struct vol_capabilities_attr_t {
    capabilities: vol_capabilities_set_t,
    valid: vol_capabilities_set_t,
}

// const VOL_CAP_INT_RENAME_SWAP: u32 = 0x00040000;
const VOL_CAP_INT_RENAME_EXCL: u32 = 0x00080000;

#[repr(C)]
struct AttributeBuf {
    length: u32,
    volume: vol_capabilities_attr_t,
}

extern "C" {
    fn getattrlist(
        path: *const c_char,
        attrList: *mut attrlist,
        attrBuf: *mut AttributeBuf,
        attrBufSize: usize,
        options: c_ulong,
    ) -> c_int;
}

pub fn rename_exclusive_is_atomic(path: &Path) -> Result<bool> {
    let path_str = CString::new(path.as_os_str().as_bytes())?;
    let mut list = attrlist {
        bitmapcount: ATTR_BIT_MAP_COUNT,
        reserved: 0,
        commonattr: 0,
        volattr: ATTR_VOL_CAPABILITIES,
        dirattr: 0,
        fileattr: 0,
        forkattr: 0,
    };
    let mut buf = std::mem::MaybeUninit::<AttributeBuf>::uninit();

    let ret = unsafe {
        getattrlist(
            path_str.as_ptr(),
            std::ptr::addr_of_mut!(list),
            buf.as_mut_ptr(),
            std::mem::size_of::<AttributeBuf>(),
            0
        )
    };

    if ret == -1 {
        return Err(std::io::Error::last_os_error());
    }

    let attrs = unsafe { buf.assume_init_ref() };
    let capabilities = attrs.volume.capabilities[VOL_CAPABILITIES_INTERFACES];

    Ok(capabilities & VOL_CAP_INT_RENAME_EXCL != 0)
}
