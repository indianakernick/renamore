use std::path::Path;
use std::io::Result;

/// Rename `from` to `to` without overwriting `to` if it exists.
///
/// This operation is atomic on platforms that support it. This avoids a
/// potential [TOCTTOU] bug that could arise from first checking for existence,
/// and then renaming.
///
/// If the platform doesn't expose an API for performing the operation
/// atomically, then a non-atomic fallback will be used. Even if the API is
/// exposed, the operation might still be non-atomic if the file system doesn't
/// support it. See [`rename_exclusive_is_atomic`](rename_exclusive_is_atomic)
/// to check for atomicity.
///
/// [TOCTTOU]: https://en.wikipedia.org/wiki/Time-of-check_to_time-of-use
pub fn rename_exclusive<F: AsRef<Path>, T: AsRef<Path>>(from: F, to: T) -> Result<()> {
    sys::rename_exclusive(from.as_ref(), to.as_ref())
}

// Is "atomic" the right word here?

/// Determine whether [`rename_exclusive`](rename_exclusive) is an atomic
/// operation.
///
/// This will return `true` if the OS exposes the necessary API and the file
/// system being used at the given path supports it.
pub fn rename_exclusive_is_atomic<P: AsRef<Path>>(path: P) -> Result<bool> {
    sys::rename_exclusive_is_atomic(path.as_ref())
}

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux as sys;

#[cfg(any(target_os = "macos", target_os = "ios"))]
mod macos;
#[cfg(any(target_os = "macos", target_os = "ios"))]
use macos as sys;

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "ios")))]
mod sys {
    use std::path::Path;
    use std::io::Result;

    pub fn rename_exclusive(from: &Path, to: &Path) -> Result<()> {
        if to.try_exists()? {
            return Err(std::io::Error::from(std::io::ErrorKind::AlreadyExists));
        }

        std::fs::rename(from, to)
    }

    pub fn rename_exclusive_is_atomic(_path: &Path) -> Result<bool> {
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    struct CurrentDirectory {
        previous: PathBuf,
    }

    impl CurrentDirectory {
        fn set<T: AsRef<Path>>(to: T) -> std::io::Result<Self> {
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

    fn is_exists_error(result: std::io::Result<()>) -> bool {
        if let Err(e) = result {
            e.kind() == std::io::ErrorKind::AlreadyExists
        } else {
            false
        }
    }

    fn parent_join(path: &Path) -> PathBuf {
        let mut parent = PathBuf::new();
        parent.push(std::path::Component::ParentDir);
        parent.push(path);
        parent
    }

    #[test]
    fn rename_exclusive_abs() -> std::io::Result<()> {
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
    fn rename_exclusive_rel() -> std::io::Result<()> {
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
    fn rename_exclusive_is_atomic() -> std::io::Result<()> {
        let is_atomic = super::rename_exclusive_is_atomic(std::env::current_dir()?)?;

        if is_atomic {
            println!("rename_exclusive is atomic");
        } else {
            println!("rename_exclusive is not atomic");
        }

        Ok(())
    }
}
