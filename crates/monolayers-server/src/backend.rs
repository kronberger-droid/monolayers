use std::io;
use std::path::Path;
use std::process::Command;

use monolayers_core::backend::FilePolicyBackend;

/// Server-side backend: uses `chattr +i` / `chattr -i` for kernel-level immutability.
/// Requires `CAP_LINUX_IMMUTABLE` capability on the process.
pub struct ChattrBackend;

impl FilePolicyBackend for ChattrBackend {
    fn set_readonly(&self, path: &Path, readonly: bool) -> io::Result<()> {
        let flag = if readonly { "+i" } else { "-i" };

        let output = Command::new("chattr").arg(flag).arg(path).output()?;

        if !output.status.success() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                String::from_utf8_lossy(&output.stderr).into_owned(),
            ));
        }

        Ok(())
    }
}
