use std::io;
use std::path::Path;

pub trait FilePolicyBackend {
    fn set_readonly(&self, path: &Path, readonly: bool) -> io::Result<()>;
}
