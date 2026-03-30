use std::io;
use std::path::Path;

pub trait FilePolicyBackend {
    fn set_readonly(&self, path: &Path, readonly: bool) -> io::Result<()>;
}

pub struct LinuxBackend;

impl FilePolicyBackend for LinuxBackend {
    fn set_readonly(&self, path: &Path, readonly: bool) -> io::Result<()> {
        use std::os::unix::fs::PermissionsExt;

        let metadata = std::fs::metadata(path)?;
        let mut perms = metadata.permissions();

        if readonly {
            perms.set_mode(perms.mode() & !0o222);
        } else {
            perms.set_mode(perms.mode() | 0o200);
        }

        std::fs::set_permissions(path, perms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn set_readonly_removes_write_bits() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, "hello").unwrap();

        LinuxBackend.set_readonly(&file, true).unwrap();

        let mode = std::fs::metadata(&file).unwrap().permissions().mode();
        assert_eq!(mode & 0o222, 0, "write bits should be cleared");
    }

    #[test]
    fn set_writable_restores_owner_write() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, "hello").unwrap();

        LinuxBackend.set_readonly(&file, true).unwrap();
        LinuxBackend.set_readonly(&file, false).unwrap();

        let mode = std::fs::metadata(&file).unwrap().permissions().mode();
        assert_ne!(mode & 0o200, 0, "owner write bit should be set");
    }

    #[test]
    fn set_readonly_preserves_read_bits() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, "hello").unwrap();

        let original_mode = std::fs::metadata(&file).unwrap().permissions().mode();
        LinuxBackend.set_readonly(&file, true).unwrap();

        let new_mode = std::fs::metadata(&file).unwrap().permissions().mode();
        assert_eq!(
            original_mode & 0o555,
            new_mode & 0o555,
            "read/execute bits should be unchanged"
        );
    }

    #[test]
    fn set_readonly_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, "hello").unwrap();

        LinuxBackend.set_readonly(&file, true).unwrap();
        let mode1 = std::fs::metadata(&file).unwrap().permissions().mode();

        LinuxBackend.set_readonly(&file, true).unwrap();
        let mode2 = std::fs::metadata(&file).unwrap().permissions().mode();

        assert_eq!(mode1, mode2);
    }
}
