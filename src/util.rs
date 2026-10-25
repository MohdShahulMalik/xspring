//! Small helpers shared across the crate.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

/// Reads an environment variable, treating a blank value as unset.
pub fn env_non_empty(key: &str) -> Option<String> {
    match std::env::var(key) {
        Ok(value) if !value.trim().is_empty() => Some(value),
        _ => None,
    }
}

/// Current time as a unix timestamp in seconds.
pub fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0)
}

/// Writes `bytes` to `path` atomically, keeping the file's existing permissions.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    atomic_write_full(path, bytes, None)
}

/// Writes `bytes` to `path` atomically.
///
/// A crash mid-write must never leave a truncated file behind, and a reader on
/// another thread sees either the old contents or the new ones, never a mix.
///
/// The temporary file lives next to `path` (so the final `rename` stays on one
/// filesystem) and carries the process id, so concurrent runs do not trample
/// each other's scratch file. On unix:
///
/// * `mode` forces the new file's permissions, which the cache uses to keep its
///   contents readable only by the owner;
/// * without `mode`, an existing file keeps its permissions, which build-file
///   edits expect (`fs::write` on the real path behaves the same way).
pub fn atomic_write_full(path: &Path, bytes: &[u8], mode: Option<u32>) -> io::Result<()> {
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no file name"))?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    // On unix, an explicit `mode` wins; otherwise an existing file keeps its
    // permissions, which build-file edits expect (`fs::write` to the real path
    // behaves the same way).
    #[cfg(unix)]
    let existing_mode: Option<u32> = {
        use std::os::unix::fs::MetadataExt;

        std::fs::metadata(path).ok().map(|meta| meta.mode() & 0o777)
    };
    #[cfg(not(unix))]
    let existing_mode: Option<u32> = None;

    #[cfg(unix)]
    let forced_mode = match mode {
        Some(bits) => Some(bits & 0o777),
        None => existing_mode,
    };
    #[cfg(not(unix))]
    let _ = mode;
    #[cfg(not(unix))]
    let forced_mode: Option<u32> = None;

    let tmp = path.with_file_name(format!(".{file_name}.{}.tmp", std::process::id()));

    let result = (|| -> io::Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .apply_mode(forced_mode)
            .open(&tmp)?;
        file.write_all(bytes)?;
        file.flush()?;
        fs::rename(&tmp, path)
    })();

    match result {
        Ok(()) => Ok(()),
        Err(err) => {
            let _ = fs::remove_file(&tmp);
            Err(err)
        }
    }
}

/// Permissions for the file about to be opened.
trait ApplyMode {
    fn apply_mode(&mut self, mode: Option<u32>) -> &mut Self;
}

#[cfg(unix)]
impl ApplyMode for OpenOptions {
    fn apply_mode(&mut self, mode: Option<u32>) -> &mut Self {
        use std::os::unix::fs::OpenOptionsExt;

        match mode {
            Some(bits) => self.mode(bits),
            None => self,
        }
    }
}

#[cfg(not(unix))]
impl ApplyMode for OpenOptions {
    fn apply_mode(&mut self, _mode: Option<u32>) -> &mut Self {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::{atomic_write, atomic_write_full, env_non_empty, now_secs};
    use std::path::Path;

    #[test]
    fn blank_env_vars_count_as_unset() {
        // The test runner's environment must not actually define this variable.
        assert!(env_non_empty("XSPRING_DEFINITELY_NOT_SET").is_none());
    }

    #[test]
    fn writes_the_file_at_once() {
        let dir = std::env::temp_dir().join("xspring-util-atomic-write-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("target.txt");

        atomic_write(&path, b"first").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
        atomic_write(&path, b"second").unwrap();

        assert_eq!(std::fs::read_to_string(&path).unwrap(), "second");
        assert_eq!(
            std::fs::read_dir(&dir).unwrap().count(),
            1,
            "no tmp files left behind"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    /// An existing file keeps its permissions when it is replaced, matching
    /// what `fs::write` to the real path would have done.
    #[cfg(unix)]
    #[test]
    fn an_existing_file_keeps_its_permissions() {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};

        let dir = std::env::temp_dir().join("xspring-util-atomic-perms-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("target.txt");

        std::fs::write(&path, b"original").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o640)).unwrap();

        atomic_write(&path, b"updated").unwrap();

        assert_eq!(std::fs::read_to_string(&path).unwrap(), "updated");
        assert_eq!(std::fs::metadata(&path).unwrap().mode() & 0o777, 0o640);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[cfg(unix)]
    #[test]
    fn a_forced_mode_wins() {
        use std::os::unix::fs::MetadataExt;

        let dir = std::env::temp_dir().join("xspring-util-atomic-mode-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("secret.txt");

        atomic_write_full(&path, b"contents", Some(0o600)).unwrap();

        assert_eq!(std::fs::read_to_string(&path).unwrap(), "contents");
        assert_eq!(std::fs::metadata(&path).unwrap().mode() & 0o777, 0o600);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_missing_directory_is_created() {
        let dir = std::env::temp_dir().join("xspring-util-atomic-mkdir-test/nested/deeper");
        let path = dir.join("target.txt");

        atomic_write(&path, b"contents").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "contents");

        std::fs::remove_dir_all(std::env::temp_dir().join("xspring-util-atomic-mkdir-test")).ok();
    }

    #[test]
    fn a_fileless_path_is_rejected() {
        let result = atomic_write(Path::new("/"), b"nope");
        assert!(result.is_err());
    }

    #[test]
    fn now_secs_is_monotonic() {
        let first = now_secs();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let second = now_secs();
        assert!(second >= first);
    }
}
