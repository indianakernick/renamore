use std::path::Path;
use std::io::Result;
use std::ffi::{c_char, c_int, c_uint, CString};
use std::os::unix::prelude::OsStrExt;

// Supported on Linux 3.15

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

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct Version(u64);

impl Version {
    const fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self(((major as u64) << 32) | ((minor as u64) << 16) | patch as u64)
    }
}

fn get_kernel_version() -> Result<Version> {
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

    Ok(Version::new(major, minor, patch))
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

const FS_EXT4: c_uint = 0xef53; // EXT4_SUPER_MAGIC
const FS_BTRFS: [c_uint; 2] = [
    0x9123683e, // BTRFS_SUPER_MAGIC
    0x73727279, // BTRFS_TEST_MAGIC
];
const FS_TMPFS: c_uint = 0x01021994; // TMPFS_MAGIC
const FS_CIFS: c_uint = 0xff534d42; // CIFS_MAGIC_NUMBER
const FS_XFS: c_uint = 0x58465342; // XFS_SUPER_MAGIC
// EXT2_SUPER_MAGIC is the same as EXT4_SUPER_MAGIC.
const FS_EXT2: c_uint = 0xef51; // EXT2_OLD_SUPER_MAGIC
const FS_MINIX: [c_uint; 5] = [
    0x137f, // MINIX_SUPER_MAGIC
    0x138f, // MINIX_SUPER_MAGIC2
    0x2468, // MINIX2_SUPER_MAGIC
    0x2478, // MINIX2_SUPER_MAGIC2
    0x4d5a, // MINIX3_SUPER_MAGIC
];
const FS_REISERFS: c_uint = 0x52654973; // REISERFS_SUPER_MAGIC
const FS_JFS: c_uint = 0x3153464a; // JFS_SUPER_MAGIC
// vfat was discovered experimentally. It doesn't appear in the man page or the
// magic.h header.
const FS_VFAT: c_uint = 0x7c7c6673;
const FS_BPF: c_uint = 0xcafe4a11; // BPF_FS_MAGIC

pub fn rename_exclusive_is_supported(path: &Path) -> Result<bool> {
    let kernel = get_kernel_version()?;
    let fs = get_filesystem_type(path)?;

    // The man page for renameat2 says this:
    //
    //  - ext4 (Linux 3.15);
    //  - btrfs, tmpfs, and cifs (Linux 3.17);
    //  - xfs (Linux 4.0);
    //  - Support for many other filesystems was added in Linux 4.9, including
    //    ext2, minix, reiserfs, jfs, vfat, and bpf.

    if kernel >= Version::new(3, 15, 0) {
        if fs == FS_EXT4 {
            return Ok(true);
        }
    }

    if kernel >= Version::new(3, 17, 0) {
        if FS_BTRFS.contains(&fs) || [FS_TMPFS, FS_CIFS].contains(&fs) {
            return Ok(true);
        }
    }

    if kernel >= Version::new(4, 0, 0) {
        if fs == FS_XFS {
            return Ok(true);
        }
    }

    if kernel >= Version::new(4, 9, 0) {
        // The man page says "including" which implies that this is not an
        // exhaustive list.
        if [FS_EXT2, FS_REISERFS, FS_JFS, FS_VFAT, FS_BPF].contains(&fs) || FS_MINIX.contains(&fs) {
            return Ok(true);
        }
    }

    Ok(false)
}
