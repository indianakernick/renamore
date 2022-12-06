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
