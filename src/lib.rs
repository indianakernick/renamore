//! More ways to rename files.
//!
//! ## Overview
//!
//! The Rust standard library offers [`std::fs::rename`] for renaming files.
//! Sometimes, that's not enough. Consider the example of renaming a file but
//! aborting the operation if something already exists at the destination path.
//! That can be achieved using the Rust standard library but ensuring that the
//! operation is atomic can only be achieved using platform-specific APIs.
//! Without using platform-specific APIs, a [TOCTTOU] bug can be introduced.
//! This library aims to provide a cross-platform interface to these APIs.
//!
//! [TOCTTOU]: https://en.wikipedia.org/wiki/Time-of-check_to_time-of-use
//!
//! ## Example
//!
//! Renaming a file without the possibility of accidentally overwriting anything
//! can be done using [`rename_exclusive`]. It should be noted that this feature
//! is not supported by all combinations of operation system and file system.
//! The return value will indicate whether the operation was performed
//! atomically or whether a non-atomic fallback was used.
//!
//! ```no_run
//! use std::io::Result;
//! use std::path::PathBuf;
//!
//! fn main() -> Result<()> {
//!     let from = PathBuf::from("old.txt");
//!     let to = PathBuf::from("new.txt");
//!
//!     if renamore::rename_exclusive(&from, &to)? {
//!         // `new.txt` will definitely not be overwritten.
//!         println!("The operation was atomic");
//!     } else {
//!         // `new.txt` will probably not be overwritten.
//!         println!("The operation was not atomic");
//!     }
//!
//!     Ok(())
//! }
//! ```

use std::path::Path;
use std::io::Result;

/// Rename a file without overwriting the destination path if it exists.
///
/// Unlike a combination of [`try_exists`] and [`rename`], this operation is
/// atomic on platforms that support it. A potential [TOCTTOU] bug is avoided.
/// There is no possibility of `to` coming into existence at just the wrong
/// moment and being overwritten.
///
/// [`try_exists`]: std::path::Path::try_exists
/// [`rename`]: std::fs::rename
/// [TOCTTOU]: https://en.wikipedia.org/wiki/Time-of-check_to_time-of-use
///
/// Performing this operation atomically is not supported on all platforms. If
/// the operation was successfully performed atomically, then `Ok(true)` will be
/// returned. Otherwise, `try_exists` and `rename` will be used and `Ok(false)`
/// will be returned.
///
/// # Platform-specific behaviour
///
/// On Linux, this calls `renameat2` with `RENAME_NOREPLACE`. On Darwin (macOS,
/// iOS, watchOS, tvOS), this calls `renamex_np` with `RENAME_EXCL`. On Windows,
/// this calls `MoveFileExW` with no flags. On all other platforms, this uses
/// `try_exists` and `rename` which means it will always return `Ok(false)` if
/// successful.
///
/// # Errors
///
/// If `to` exists, then [`ErrorKind::AlreadyExists`] will be returned. If the
/// operation was not atomic, returning an error in this case is not guaranteed.
///
/// [`ErrorKind::AlreadyExists`]: std::io::ErrorKind::AlreadyExists
pub fn rename_exclusive<F: AsRef<Path>, T: AsRef<Path>>(from: F, to: T) -> Result<bool> {
    sys::rename_exclusive(from.as_ref(), to.as_ref())
}

/// Determine whether [`rename_exclusive`] is atomic.
///
/// Support for performing this operation atomically depends on whether the
/// necessary functions are available at link-time, and the OS implements the
/// operation for the file system of the given path. If this function returns
/// `Ok(true)`, then a call to `rename_exclusive` at the same path is likely to
/// return `Ok(true)` rather than `Ok(false)` if it succeeds.
///
/// # Platform-specific behaviour
///
/// On Linux, this parses `/proc/version` to determine the kernel version and
/// calls `statfs` to determine the file system type. On Darwin (macOS, iOS,
/// watchOS, tvOS), this calls `getattrlist` to determine whether the volume at
/// the path lists `VOL_CAP_INT_RENAME_EXCL` as one of its capabilities. On
/// Windows, this always returns `Ok(true)` even though that may not be
/// technically true. On all other platforms, this always returns `Ok(false)`.
pub fn rename_exclusive_is_atomic<P: AsRef<Path>>(path: P) -> Result<bool> {
    sys::rename_exclusive_is_atomic(path.as_ref())
}

#[cfg(not(target_os = "windows"))]
fn rename_exclusive_non_atomic(from: &Path, to: &Path) -> Result<()> {
    if to.try_exists()? {
        return Err(std::io::Error::from(std::io::ErrorKind::AlreadyExists));
    }

    std::fs::rename(from, to)
}

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux as sys;

#[cfg(target_vendor = "apple")]
mod macos;
#[cfg(target_vendor = "apple")]
use macos as sys;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
use windows as sys;

#[cfg(not(any(
    target_os = "linux",
    target_vendor = "apple",
    target_os = "windows",
)))]
mod sys {
    use std::path::Path;
    use std::io::{Error, ErrorKind, Result};

    pub fn rename_exclusive(from: &Path, to: &Path) -> Result<bool> {
        rename_exclusive_non_atomic(from, to)?;
        Ok(false)
    }

    pub fn rename_exclusive_is_atomic(_path: &Path) -> Result<bool> {
        Ok(false)
    }
}

#[cfg(test)]
mod tests;
