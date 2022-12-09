//! More ways to rename files.
//!
//! ## Overview
//!
//! The Rust standard library offers [`std::fs::rename`] for renaming files.
//! Sometimes, that's not enough. Consider the example of renaming a file but
//! aborting the operation if something already exists at the destination path.
//! That can be achieved using the Rust standard library but ensuring that the
//! operation is atomic requires platform-specific APIs. Without using
//! platform-specific APIs, a [TOCTTOU] bug can be introduced. This library aims
//! to provide a cross-platform interface to these APIs.
//!
//! [TOCTTOU]: https://en.wikipedia.org/wiki/Time-of-check_to_time-of-use
//!
//! ## Examples
//!
//! Renaming a file without the possibility of accidentally overwriting anything
//! can be done using [`rename_exclusive`]. It should be noted that this feature
//! is not supported by all combinations of operation system and file system.
//! `rename_exclusive` will fail if it can't be done atomically.
//!
//! ```no_run
//! use std::io::Result;
//!
//! fn main() -> Result<()> {
//!     renamore::rename_exclusive("old.txt", "new.txt")
//! }
//! ```
//!
//! Alternatively, [`rename_exclusive_fallback`] can be used. This will try to
//! perform the operation atomically, and use a non-atomic fallback if that's
//! not supported. The return value will indicate what happened.
//!
//! ```no_run
//! use std::io::Result;
//!
//! fn main() -> Result<()> {
//!     if renamore::rename_exclusive_fallback("old.txt", "new.txt")? {
//!         // `new.txt` was definitely not overwritten.
//!         println!("The operation was atomic");
//!     } else {
//!         // `new.txt` was probably not overwritten.
//!         println!("The operation was not atomic");
//!     }
//!
//!     Ok(())
//! }
//! ```

use std::path::Path;
use std::io::{Error, ErrorKind, Result};

/// Rename a file without overwriting the destination path if it exists.
///
/// Unlike a combination of [`try_exists`] and [`rename`], this operation is
/// atomic. A potential [TOCTTOU] bug is avoided. There is no possibility of
/// `to` coming into existence at just the wrong moment and being overwritten.
///
/// [`try_exists`]: std::path::Path::try_exists
/// [`rename`]: std::fs::rename
/// [TOCTTOU]: https://en.wikipedia.org/wiki/Time-of-check_to_time-of-use
///
/// # Platform-specific behaviour
///
/// On Linux, this calls `renameat2` with `RENAME_NOREPLACE`. On Darwin (macOS,
/// iOS, watchOS, tvOS), this calls `renamex_np` with `RENAME_EXCL`. On Windows,
/// this calls `MoveFileExW` with no flags. On all other platforms, this returns
/// [`ErrorKind::Unsupported`] unconditionally.
///
/// # Errors
///
/// Performing this operation atomically is not supported on all platforms. If
/// it's not supported but the rename request is otherwise valid, then
/// [`ErrorKind::Unsupported`] will be returned. If the operation is supported
/// but a file at `to` exists, then [`ErrorKind::AlreadyExists`] will be
/// returned.
///
/// [`ErrorKind::Unsupported`]: std::io::ErrorKind::Unsupported
/// [`ErrorKind::AlreadyExists`]: std::io::ErrorKind::AlreadyExists
pub fn rename_exclusive<F: AsRef<Path>, T: AsRef<Path>>(from: F, to: T) -> Result<()> {
    sys::rename_exclusive(from.as_ref(), to.as_ref())
}

/// Determine whether an atomic [`rename_exclusive`] is supported.
///
/// Support for performing this operation atomically depends on whether the
/// necessary functions are available at link-time, and the OS implements the
/// operation for the file system of the given path. If this function returns
/// `Ok(true)`, then a call to `rename_exclusive` at the same path is unlikely
/// to return [`ErrorKind::Unsupported`] if it fails.
///
/// [`ErrorKind::Unsupported`]: std::io::ErrorKind::Unsupported
///
/// # Platform-specific behaviour
///
/// On Linux, this parses `/proc/version` to determine the kernel version and
/// calls `statfs` to determine the file system type. On Darwin (macOS, iOS,
/// watchOS, tvOS), this calls `getattrlist` to determine whether the volume at
/// the path lists `VOL_CAP_INT_RENAME_EXCL` as one of its capabilities. On
/// Windows, this always returns `Ok(true)` even though that may not be
/// technically true. On all other platforms, this always returns `Ok(false)`.
///
/// # Examples
///
/// ```no_run
/// # use std::io::Result;
/// # fn main() -> Result<()> {
/// if !renamore::rename_exclusive_is_atomic(".")? {
///     println!("Warning: atomically renaming without overwriting is not supported!");
/// }
/// # Ok(())
/// # }
/// ```
pub fn rename_exclusive_is_atomic<P: AsRef<Path>>(path: P) -> Result<bool> {
    sys::rename_exclusive_is_atomic(path.as_ref())
}

/// Rename a file without overwriting the destination path if it exists, using a
/// non-atomic fallback if necessary.
///
/// This is similar to [`rename_exclusive`] except that if performing the
/// operation atomically is not supported, then a non-atomic fallback
/// implementation based on [`try_exists`] and [`rename`] will be used.
///
/// # Examples
///
/// ```no_run
/// # fn main() -> std::io::Result<()> {
/// if renamore::rename_exclusive_fallback("old.txt", "new.txt")? {
///     // `new.txt` was definitely not overwritten.
///     println!("The operation was atomic");
/// } else {
///     // `new.txt` was probably not overwritten.
///     println!("The operation was not atomic");
/// }
/// # Ok(())
/// # }
/// ```
///
/// [`try_exists`]: std::path::Path::try_exists
/// [`rename`]: std::fs::rename
pub fn rename_exclusive_fallback<F: AsRef<Path>, T: AsRef<Path>>(from: F, to: T) -> Result<bool> {
    fn inner(from: &Path, to: &Path) -> Result<bool> {
        if let Err(e) = sys::rename_exclusive(from, to) {
            if e.kind() == ErrorKind::Unsupported {
                rename_exclusive_non_atomic(from, to)?;
                return Ok(false);
            }
            Err(e)
        } else {
            Ok(true)
        }
    }
    inner(from.as_ref(), to.as_ref())
}

fn rename_exclusive_non_atomic(from: &Path, to: &Path) -> Result<()> {
    if to.try_exists()? {
        return Err(Error::from(ErrorKind::AlreadyExists));
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

    pub fn rename_exclusive(from: &Path, to: &Path) -> Result<()> {
        Err(Error::from(ErrorKind::Unsupported))
    }

    pub fn rename_exclusive_is_atomic(_path: &Path) -> Result<bool> {
        Ok(false)
    }
}

#[cfg(test)]
mod tests;
