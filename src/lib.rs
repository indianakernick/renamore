//! More ways to rename files.
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
//! can be done using [`rename_exclusive`].
//!
//! ```no_run
//! use std::io::Result;
//! use std::path::PathBuf;
//!
//! fn main() -> Result<()> {
//!     let from = PathBuf::from("old.txt");
//!     let to = PathBuf::from("new.txt");
//!
//!     renamore::rename_exclusive(&from, &to)
//! }
//! ```
//!
//! It should be noted that this feature is not supported by all combinations of
//! operating system and file system. Support can be checked by calling
//! [`rename_exclusive_is_supported`]. If the feature is not supported, then
//! [`rename_exclusive_non_atomic`] can be used. As the same suggests, this is a
//! non-atomic version of `rename_exclusive`.
//!
//! ```no_run
//! use std::io::Result;
//! use std::path::PathBuf;
//!
//! fn main() -> Result<()> {
//!     let from = PathBuf::from("old.txt");
//!     let to = PathBuf::from("new.txt");
//!
//!     // Checking if rename_exclusive is supported by the current OS version
//!     // using the file system of the current directory.
//!     if renamore::rename_exclusive_is_supported(".") {
//!         // It's supported!
//!         // `new.txt` will definitely not be overwritten.
//!         renamore::rename_exclusive(&from, &to)
//!     } else {
//!         // Oh no!
//!         // `new.txt` will probably not be overwritten.
//!         renamore::rename_exclusive_non_atomic(&from, &to)
//!     }
//! }
//! ```
//!
//! Doing this check can be a little bit verbose. For this reason, there is
//! [`rename_exclusive_checked`] which will check for support and switch between
//! the atomic and non-atomic implementations.
//!
//! ```no_run
//! use std::io::Result;
//! use std::path::PathBuf;
//!
//! fn main() -> Result<()> {
//!     let from = PathBuf::from("old.txt");
//!     let to = PathBuf::from("new.txt");
//!
//!     renamore::rename_exclusive_checked(&from, &to)
//! }
//! ```
//!
//! For doing a bulk-rename rather than a one-off, it may be more efficient to
//! do the check for support manually rather than using
//! `rename_exclusive_checked`. That would mean doing the check once rather than
//! for every rename.

use std::path::Path;
use std::io::{Error, ErrorKind, Result};

/// Rename a file without overwriting the destination path if it exists.
///
/// Unlike a combination of [`try_exists`](std::path::Path::try_exists) and
/// [`rename`](std::fs::rename), this operation is atomic on platforms that
/// support it. A potential [TOCTTOU] bug is avoided. If `to` exists, then
/// [`ErrorKind::AlreadyExists`](std::io::ErrorKind::AlreadyExists) will be
/// returned. There is no possibility of `to` coming into existence at just the
/// wrong moment and being overwritten.
///
/// Before this function can be called, support should be checked with
/// [`rename_exclusive_is_supported`]. If this function is called when it is not
/// supported, then it may behave the same as `rename`, or it might
/// non-atomically try to avoid overwriting `to`, or it might crash.
///
/// [TOCTTOU]: https://en.wikipedia.org/wiki/Time-of-check_to_time-of-use
pub fn rename_exclusive<F: AsRef<Path>, T: AsRef<Path>>(from: F, to: T) -> Result<()> {
    sys::rename_exclusive(from.as_ref(), to.as_ref())
}

/// Determine whether [`rename_exclusive`] is supported.
///
/// Support depends on whether the necessary functions are available at
/// link-time, and the OS implements the operation for the file system of the
/// given path. `rename_exclusive` should not be called if this function returns
/// `Ok(false)`.
pub fn rename_exclusive_is_supported<P: AsRef<Path>>(path: P) -> Result<bool> {
    sys::rename_exclusive_is_supported(path.as_ref())
}

/// A non-atomic version of [`rename_exclusive`].
///
/// This is supported on all platforms but of course this operation is not
/// atomic. This is meant to be used as a fallback for when `rename_exclusive`
/// is not supported.
pub fn rename_exclusive_non_atomic<F: AsRef<Path>, T: AsRef<Path>>(from: F, to: T) -> Result<()> {
    if to.as_ref().try_exists()? {
        return Err(Error::from(ErrorKind::AlreadyExists));
    }

    std::fs::rename(from, to)
}

/// Check whether [`rename_exclusive`] is supported and call
/// it if so, otherwise fall back on [`rename_exclusive_non_atomic`].
///
/// Checking whether the operation is supported may not be particularly fast.
/// When doing multiple renames under consistent conditions (same OS, same file
/// system), it may be more efficient to check for support once using
/// [`rename_exclusive_is_supported`] and then choose which function to use for
/// the renames.
pub fn rename_exclusive_checked<F: AsRef<Path>, T: AsRef<Path>>(from: F, to: T) -> Result<()> {
    // Probably need to check if `from` and `to` are on the same volume.
    if sys::rename_exclusive_is_supported(from.as_ref())? {
        sys::rename_exclusive(from.as_ref(), to.as_ref())
    } else {
        rename_exclusive_non_atomic(from, to)
    }
}

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux as sys;

#[cfg(any(target_os = "macos", target_os = "ios"))]
mod macos;
#[cfg(any(target_os = "macos", target_os = "ios"))]
use macos as sys;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
use windows as sys;

#[cfg(not(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "ios",
    target_os = "windows"
)))]
mod sys {
    use std::path::Path;
    use std::io::{Error, ErrorKind, Result};

    pub fn rename_exclusive(from: &Path, to: &Path) -> Result<()> {
        panic!("rename_exclusive is not supported on this platform");
    }

    pub fn rename_exclusive_is_supported(_path: &Path) -> Result<bool> {
        Ok(false)
    }
}

#[cfg(test)]
mod tests;
