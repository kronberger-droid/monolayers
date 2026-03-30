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
