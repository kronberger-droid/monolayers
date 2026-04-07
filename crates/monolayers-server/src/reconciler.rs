// Legacy Nextcloud reconciler — kept for reference.
// The new server reconciler will walk the watch path and ensure chattr state
// matches the store, without any API calls.

use std::path::Path;

use monolayers_core::{backend::FilePolicyBackend, config::is_exempt, store::StateStore};

/// Walk the watch directory and ensure every file's chattr state matches policy.
/// Files in exempt folders are made writable; all others get chattr +i.
pub fn reconcile(
    watch_path: &Path,
    exempt_names: &[String],
    store: &StateStore,
    backend: &dyn FilePolicyBackend,
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in walkdir::WalkDir::new(watch_path) {
        let entry = entry?;

        if entry.file_type().is_dir() {
            continue;
        }

        let local_path = entry.path();
        let relative_path = local_path
            .strip_prefix(watch_path)?
            .to_str()
            .ok_or("non-utf8 path")?;

        let exempt = is_exempt(local_path, exempt_names);

        if exempt {
            backend.set_readonly(local_path, false)?;
            store.remove(relative_path)?;
        } else {
            backend.set_readonly(local_path, true)?;
            store.set(relative_path, relative_path, true)?;
        }
    }

    Ok(())
}
