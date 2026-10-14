//! Sibling-file replacement mechanics, independent of domain errors and locking.
use std::{io::Write, path::Path};

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Error {
    Create,
    #[cfg(unix)]
    Permissions,
    Write,
    Sync,
    Replace,
}

/// Replace from a unique sibling, secured to 0600 before writing on Unix.
/// Syncs the file, not its parent directory; does not promise power-loss durability.
pub(crate) fn replace(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(|_| Error::Create)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        temporary
            .as_file()
            .set_permissions(std::fs::Permissions::from_mode(0o600))
            .map_err(|_| Error::Permissions)?;
    }
    temporary.write_all(bytes).map_err(|_| Error::Write)?;
    temporary.as_file().sync_all().map_err(|_| Error::Sync)?;
    temporary.persist(path).map_err(|_| Error::Replace)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn creates_and_replaces_without_leaving_temporary_files() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("output");
        replace(&path, b"first").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"first");
        replace(&path, b"replacement").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"replacement");
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
    }

    #[test]
    fn replacement_failure_preserves_destination_and_cleans_staging() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("output");
        fs::create_dir(&path).unwrap();
        fs::write(path.join("keep"), b"original").unwrap();
        assert_eq!(replace(&path, b"replacement"), Err(Error::Replace));
        assert_eq!(fs::read(path.join("keep")).unwrap(), b"original");
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
    }

    #[test]
    fn missing_parent_is_not_created() {
        let root = tempfile::tempdir().unwrap();
        let parent = root.path().join("missing");
        assert_eq!(
            replace(&parent.join("output"), b"bytes"),
            Err(Error::Create)
        );
        assert!(!parent.exists());
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
    }

    #[test]
    fn basename_destination_uses_current_directory_without_changing_it() {
        let path = tempfile::NamedTempFile::new_in(".")
            .unwrap()
            .into_temp_path();
        replace(Path::new(path.file_name().unwrap()), b"relative").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"relative");
    }

    #[cfg(unix)]
    #[test]
    fn created_and_replaced_files_have_mode_0600() {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("output");
        replace(&path, b"secret").unwrap();
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        replace(&path, b"new secret").unwrap();
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}
