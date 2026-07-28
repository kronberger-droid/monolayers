use std::{collections::HashSet, path::Path};

use crate::{
    api::NextcloudClient,
    backend::FilePolicyBackend,
    config::is_exempt,
    store::StateStore,
};

pub async fn reconcile(
    client: &NextcloudClient,
    tag_id: &str,
    sync_path: &Path,
    exempt_names: &[String],
    store: &StateStore,
    backend: &dyn FilePolicyBackend,
) -> Result<(), Box<dyn std::error::Error>> {
    let tagged_files: HashSet<String> =
        client.get_tagged_files(tag_id).await?.into_iter().collect();

    for entry in walkdir::WalkDir::new(sync_path) {
        let entry = entry?;

        if entry.file_type().is_dir() {
            continue;
        }

        let local_path = entry.path();

        let relative_path = local_path
            .strip_prefix(sync_path)?
            .to_str()
            .ok_or("non-utf8 path")?;

        let exempt = is_exempt(local_path, exempt_names);
        let tagged = tagged_files.contains(relative_path);

        match (exempt, tagged) {
            (true, true) => {
                let file_id = client.get_file_id(relative_path).await?;
                client.delete_tag(&file_id, tag_id).await?;
                store.set(relative_path, &file_id, false)?;
                backend.set_readonly(local_path, false)?;
            }
            (false, false) => {
                let file_id = client.get_file_id(relative_path).await?;
                client.apply_tag(&file_id, tag_id).await?;
                store.set(relative_path, &file_id, true)?;
                backend.set_readonly(local_path, true)?;
            }
            (false, true) => {
                let file_id = client.get_file_id(relative_path).await?;
                store.set(relative_path, &file_id, true)?;
                backend.set_readonly(local_path, true)?;
            }
            (true, false) => {
                backend.set_readonly(local_path, false)?;
            }
        }
    }

    Ok(())
}
