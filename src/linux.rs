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

struct KernelVersion {
    major: u16,
    minor: u16,
    patch: u16,
}

fn get_kernel_version() -> Result<KernelVersion> {
    let invalid = std::io::ErrorKind::InvalidData;
    let version = std::fs::read_to_string("/proc/version")?;
    let version_bytes = version.as_bytes();

    let major_begin = version_bytes.iter()
        .position(|c| c.is_ascii_digit())
        .ok_or(invalid)?;
    let major_end = major_begin + version_bytes[major_begin..].iter()
        .position(|c| *c == b'.')
        .ok_or(invalid)?;

    if major_end == version_bytes.len() - 1 {
        return Err(invalid.into());
    }

    let minor_begin = major_end + 1;
    let minor_end = minor_begin + version_bytes[minor_begin..].iter()
        .position(|c| *c == b'.')
        .ok_or(invalid)?;

    if minor_end == version_bytes.len() - 1 {
        return Err(invalid.into());
    }

    let patch_begin = minor_end + 1;
    let patch_end = patch_begin + version_bytes[patch_begin..].iter()
        .position(|c| !c.is_ascii_digit())
        .ok_or(invalid)?;

    let major = u16::from_str_radix(&version[major_begin..major_end], 10)
        .map_err(|_| invalid)?;
    let minor = u16::from_str_radix(&version[minor_begin..minor_end], 10)
        .map_err(|_| invalid)?;
    let patch = u16::from_str_radix(&version[patch_begin..patch_end], 10)
        .map_err(|_| invalid)?;

    Ok(KernelVersion { major, minor, patch })
}

#[repr(C)]
struct statfs {
    f_type: c_uint,
    // We don't care about the rest.
    padding: [u64; 16],
}

extern "C" {
    fn statfs(path: *const c_char, buf: *mut statfs) -> c_int;
}

fn get_filesystem_type(path: &Path) -> Result<u32> {
    let path_str = CString::new(path.as_os_str().as_bytes())?;
    let mut buf = std::mem::MaybeUninit::<statfs>::uninit();
    let ret = unsafe { statfs(path_str.as_ptr(), buf.as_mut_ptr()) };

    if ret == -1 {
        return Err(std::io::Error::last_os_error());
    }

    Ok(unsafe { buf.assume_init() }.f_type)
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
