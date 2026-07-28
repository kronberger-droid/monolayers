use std::path::Path;

use notify::Event;

use crate::{
    api::NextcloudClient,
    backend::FilePolicyBackend,
    config::is_exempt,
    store::StateStore,
};

pub async fn handle_event(
    event: &Event,
    client: &NextcloudClient,
    tag_id: &str,
    sync_path: &Path,
    exempt_names: &[String],
    store: &StateStore,
    backend: &dyn FilePolicyBackend,
) -> Result<(), Box<dyn std::error::Error>> {
    match event.kind {
        notify::EventKind::Create(notify::event::CreateKind::File) => {
            let local_path = &event.paths[0];
            if !is_exempt(local_path, exempt_names) {
                let relative_path = local_path
                    .strip_prefix(sync_path)?
                    .to_str()
                    .ok_or("non-utf8 path")?;

                let file_id = resolve_file_id(store, client, relative_path).await?;
                client.apply_tag(&file_id, tag_id).await?;
                store.set(relative_path, &file_id, true)?;
                backend.set_readonly(local_path, true)?;
            }
        }
        notify::EventKind::Modify(notify::event::ModifyKind::Name(
            notify::event::RenameMode::Both,
        )) => {
            let (old_path, new_path) = (&event.paths[0], &event.paths[1]);

            // Clean up old store entry
            if let Some(old_relative) = old_path
                .strip_prefix(sync_path)
                .ok()
                .and_then(|p| p.to_str())
            {
                store.remove(old_relative)?;
            }

            let (old_exempt, new_exempt) = (
                is_exempt(old_path, exempt_names),
                is_exempt(new_path, exempt_names),
            );

            let new_relative = new_path
                .strip_prefix(sync_path)?
                .to_str()
                .ok_or("non-utf8 path")?;

            match (old_exempt, new_exempt) {
                (true, false) => {
                    let file_id =
                        resolve_file_id(store, client, new_relative).await?;
                    client.apply_tag(&file_id, tag_id).await?;
                    store.set(new_relative, &file_id, true)?;
                    backend.set_readonly(new_path, true)?;
                }
                (false, true) => {
                    let file_id =
                        resolve_file_id(store, client, new_relative).await?;
                    client.delete_tag(&file_id, tag_id).await?;
                    store.set(new_relative, &file_id, false)?;
                    backend.set_readonly(new_path, false)?;
                }
                (false, false) => {
                    let file_id =
                        resolve_file_id(store, client, new_relative).await?;
                    store.set(new_relative, &file_id, true)?;
                }
                (true, true) => {}
            }
        }
        // RenameMode::To — treat as a new file appearing (same as Create)
        notify::EventKind::Modify(notify::event::ModifyKind::Name(
            notify::event::RenameMode::To,
        )) => {
            let local_path = &event.paths[0];
            let relative_path = local_path
                .strip_prefix(sync_path)?
                .to_str()
                .ok_or("non-utf8 path")?;

            if is_exempt(local_path, exempt_names) {
                let file_id =
                    resolve_file_id(store, client, relative_path).await?;
                client.delete_tag(&file_id, tag_id).await?;
                store.set(relative_path, &file_id, false)?;
                backend.set_readonly(local_path, false)?;
            } else {
                let file_id =
                    resolve_file_id(store, client, relative_path).await?;
                client.apply_tag(&file_id, tag_id).await?;
                store.set(relative_path, &file_id, true)?;
                backend.set_readonly(local_path, true)?;
            }
        }
        // RenameMode::From or Remove — file gone, clean up store
        notify::EventKind::Modify(notify::event::ModifyKind::Name(
            notify::event::RenameMode::From,
        ))
        | notify::EventKind::Remove(_) => {
            let local_path = &event.paths[0];
            if let Some(relative_path) = local_path
                .strip_prefix(sync_path)
                .ok()
                .and_then(|p| p.to_str())
            {
                store.remove(relative_path)?;
            }
        }
        _ => {}
    }
    Ok(())
}

async fn resolve_file_id(
    store: &StateStore,
    client: &NextcloudClient,
    relative_path: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    if let Some((file_id, _)) = store.get(relative_path) {
        return Ok(file_id);
    }
    client.get_file_id(relative_path).await
}
