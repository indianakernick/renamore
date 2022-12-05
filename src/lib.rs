//! More ways to rename files.
//!
//! The Rust standard library offers [`std::fs::rename`] for renaming files.
//! Sometimes, that's not enough. Consider the example of renaming a file but
//! aborting the operation if something already exists at the destination path.
//! That can be achieved using the Rust standard library but ensuring that the
//! operation is atomic can only be achieved using platform-specific APIs.
//! Without using platform-specific APIs, a [TOCTTOU] bug can be introduced.
//!
//! [TOCTTOU]: https://en.wikipedia.org/wiki/Time-of-check_to_time-of-use

use std::path::Path;
use std::io::{Error, ErrorKind, Result};

/// Rename `from` to `to` without overwriting `to` if it exists.
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
/// `false`.
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
mod tests {
    use std::path::{Component, Path, PathBuf};
    use std::io::{ErrorKind, Result};

    struct CurrentDirectory {
        previous: PathBuf,
    }

    impl CurrentDirectory {
        fn set<T: AsRef<Path>>(to: T) -> Result<Self> {
            let previous = std::env::current_dir()?;
            std::env::set_current_dir(to)?;
            Ok(Self { previous })
        }
    }

    impl Drop for CurrentDirectory {
        fn drop(&mut self) {
            std::env::set_current_dir(&self.previous).unwrap();
        }
    }

    fn is_exists_error(result: Result<()>) -> bool {
        if let Err(e) = result {
            e.kind() == ErrorKind::AlreadyExists
        } else {
            false
        }
    }

    fn parent_join(path: &Path) -> PathBuf {
        let mut parent = PathBuf::new();
        parent.push(Component::ParentDir);
        parent.push(path);
        parent
    }

    #[test]
    fn rename_exclusive_abs() -> Result<()> {
        let dir = tempfile::tempdir()?;

        let path_a = dir.path().join("a");
        let path_b = dir.path().join("b");
        let path_c = dir.path().join("c");

        std::fs::write(&path_a, "a")?;
        std::fs::create_dir(&path_b)?;

        // Rename a file to a non-existent path.
        super::rename_exclusive(&path_a, &path_c)?;
        assert!(!path_a.try_exists()?);
        assert!(path_c.try_exists()?);
        assert_eq!(std::fs::read_to_string(&path_c)?, "a");

        // Rename a directory to a non-existent path.
        super::rename_exclusive(&path_b, &path_a)?;
        assert!(!path_b.try_exists()?);
        assert!(path_a.try_exists()?);
        assert!(std::fs::metadata(&path_a)?.is_dir());

        // Rename a file to an existing directory.
        assert!(is_exists_error(super::rename_exclusive(&path_c, &path_a)));
        assert!(path_c.try_exists()?);

        // Rename a directory to an existing file.
        assert!(is_exists_error(super::rename_exclusive(&path_a, &path_c)));
        assert!(path_a.try_exists()?);

        Ok(())
    }

    #[test]
    fn rename_exclusive_rel() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let _curr = CurrentDirectory::set(dir.path())?;

        let path_a = PathBuf::from("a");
        let path_b = PathBuf::from("b");
        let path_c = PathBuf::from("c");

        std::fs::write(&path_a, "a")?;
        std::fs::write(&path_b, "b")?;
        std::fs::create_dir(&path_c)?;

        // Rename a file to a non-existent path inside a directory.
        let path_c_b = path_c.join(&path_b);
        super::rename_exclusive(&path_a, &path_c_b)?;
        assert!(!path_a.try_exists()?);
        assert!(path_c_b.try_exists()?);
        assert_eq!(std::fs::read_to_string(&path_c_b)?, "a");

        // Rename a directory to a non-existent path.
        super::rename_exclusive(&path_c, &path_a)?;
        assert!(!path_c.try_exists()?);
        assert!(path_a.try_exists()?);
        assert!(std::fs::metadata(&path_a)?.is_dir());

        {
            let _curr = CurrentDirectory::set(&path_a)?;

            let path_up_b = parent_join(&path_b);

            // Rename a file to an existing file in the parent directory.
            assert!(is_exists_error(super::rename_exclusive(&path_b, &path_up_b)));
            assert!(path_b.try_exists()?);

            // Rename a file in a parent directory to a non-existent path.
            super::rename_exclusive(&path_up_b, &path_a)?;
            assert!(!path_up_b.try_exists()?);
            assert!(path_a.try_exists()?);
            assert_eq!(std::fs::read_to_string(&path_a)?, "b");

            // Rename a file to a non-existent path in the parent directory.
            super::rename_exclusive(&path_b, &path_up_b)?;
            assert!(!path_b.try_exists()?);
            assert!(path_up_b.try_exists()?);
            assert_eq!(std::fs::read_to_string(&path_up_b)?, "a");
        }

        Ok(())
    }

    #[test]
    fn rename_exclusive_is_supported() -> Result<()> {
        let is_supported = super::rename_exclusive_is_supported(std::env::current_dir()?)?;

        if is_supported {
            println!("rename_exclusive is supported");
        } else {
            println!("rename_exclusive is not supported");
        }

        Ok(())
    }
}
