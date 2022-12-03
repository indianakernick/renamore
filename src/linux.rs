use std::path::Path;
use std::io::Result;
use std::ffi::{c_char, c_int, c_uint, CString};
use std::os::unix::prelude::OsStrExt;

// Supported on Linux 3.15 with glibc 2.38

extern "C" {
    fn renameat2(
        olddirfd: c_int,
        oldpath: *const c_char,
        newdirfd: c_int,
        newpath: *const c_char,
        flags: c_uint,
    ) -> c_int;
}

const AT_FDCWD: c_int = -100;
const RENAME_NOREPLACE: c_uint = 1;
// const RENAME_EXCHANGE: c_uint = 2;

pub fn rename_exclusive(from: &Path, to: &Path) -> Result<()> {
    let from_str = CString::new(from.as_os_str().as_bytes())?;
    let to_str = CString::new(to.as_os_str().as_bytes())?;
    let ret = unsafe {
        renameat2(AT_FDCWD, from_str.as_ptr(), AT_FDCWD, to_str.as_ptr(), RENAME_NOREPLACE)
    };

    if ret == -1 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

pub fn rename_exclusive_is_atomic(_path: &Path) -> Result<bool> {
    // Not sure how to implement this.

    // The man page for renameat2 says this:
    //
    //  - ext4 (Linux 3.15);
    //  - btrfs, tmpfs, and cifs (Linux 3.17);
    //  - xfs (Linux 4.0);
    //  - Support for many other filesystems was added in Linux 4.9, including
    //    ext2, minix, reiserfs, jfs, vfat, and bpf.
    //
    // statfs can be used to get the file system type.
    // uname can be used to get the kernel version.
    //
    // Surely there's a more direct way!

    Ok(true)
}
