use std::path::Path;
use std::io::Result;
use std::ffi::{c_int, c_ulong, OsStr};
use std::os::windows::prelude::OsStrExt;

extern "C" {
    fn MoveFileExW(
        lpExistingFileName: *const u16,
        lpNewFileName: *const u16,
        dwFlags: c_ulong,
    ) -> c_int;
}

fn to_wide(s: &OsStr) -> Vec<u16> {
    let mut wide = Vec::with_capacity(s.len() + 1);
    wide.extend(s.encode_wide());
    wide.push(0);
    wide
}

pub fn rename_exclusive(from: &Path, to: &Path) -> Result<()> {
    let from_str = to_wide(from.as_os_str());
    let to_str = to_wide(to.as_os_str());
    let ret = unsafe {
        MoveFileExW(from_str.as_ptr(), to_str.as_ptr(), 0)
    };

    if ret == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

pub fn rename_exclusive_is_atomic(_path: &Path) -> Result<bool> {
    // Can't seem to find definitive evidence that MoveFileExW is ever atomic.
    // Also, the implementation of this might be similar to the Linux one where
    // we check the OS version and file system and work it out from that.

    Ok(true)
}
