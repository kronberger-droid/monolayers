use std::path::Path;

use notify::Event;

use crate::{api::NextcloudClient, config::is_exempt};

pub async fn handle_event(
    event: &Event,
    client: &NextcloudClient,
    tag_id: &str,
    sync_path: &Path,
    exempt_names: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    match event.kind {
        notify::EventKind::Create(notify::event::CreateKind::File) => {
            if !is_exempt(&event.paths[0], exempt_names) {
                let relative_path = event.paths[0].strip_prefix(sync_path)?;
                let file_id =
                    client.get_file_id(relative_path.to_str().unwrap()).await?;
                client.apply_tag(&file_id, tag_id).await?;
            }
        }
        notify::EventKind::Modify(notify::event::ModifyKind::Name(
            notify::event::RenameMode::Both,
        )) => {
            let (old_path, new_path) = (&event.paths[0], &event.paths[1]);
            let (old_exempt, new_exempt) = (
                is_exempt(old_path, exempt_names),
                is_exempt(new_path, exempt_names),
            );
            match (old_exempt, new_exempt) {
                (true, false) => {
                    let relative_path = new_path.strip_prefix(sync_path)?;
                    let file_id = client
                        .get_file_id(relative_path.to_str().unwrap())
                        .await?;
                    client.apply_tag(&file_id, tag_id).await?;
                }
                (false, true) => {
                    let relative_path = new_path.strip_prefix(sync_path)?;
                    let file_id = client
                        .get_file_id(relative_path.to_str().unwrap())
                        .await?;
                    client.delete_tag(&file_id, tag_id).await?;
                }
                _ => {}
            }
        }
        _ => {}
    }
    Ok(())
}
