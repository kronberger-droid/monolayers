use std::path::Path;

use notify::Event;

use monolayers_core::{backend::FilePolicyBackend, config::is_exempt, store::StateStore};

pub async fn handle_event(
    event: &Event,
    watch_path: &Path,
    exempt_names: &[String],
    store: &StateStore,
    backend: &dyn FilePolicyBackend,
) -> Result<(), Box<dyn std::error::Error>> {
    match event.kind {
        notify::EventKind::Create(notify::event::CreateKind::File) => {
            let local_path = &event.paths[0];
            if !is_exempt(local_path, exempt_names) {
                let relative_path = local_path
                    .strip_prefix(watch_path)?
                    .to_str()
                    .ok_or("non-utf8 path")?;

                backend.set_readonly(local_path, true)?;
                store.set(relative_path, relative_path, true)?;
            }
        }
        notify::EventKind::Modify(notify::event::ModifyKind::Name(
            notify::event::RenameMode::Both,
        )) => {
            let (old_path, new_path) = (&event.paths[0], &event.paths[1]);

            if let Some(old_relative) = old_path
                .strip_prefix(watch_path)
                .ok()
                .and_then(|p| p.to_str())
            {
                store.remove(old_relative)?;
            }

            let new_relative = new_path
                .strip_prefix(watch_path)?
                .to_str()
                .ok_or("non-utf8 path")?;

            let (old_exempt, new_exempt) = (
                is_exempt(old_path, exempt_names),
                is_exempt(new_path, exempt_names),
            );

            match (old_exempt, new_exempt) {
                (true, false) => {
                    backend.set_readonly(new_path, true)?;
                    store.set(new_relative, new_relative, true)?;
                }
                (false, true) => {
                    backend.set_readonly(new_path, false)?;
                    store.set(new_relative, new_relative, false)?;
                }
                (false, false) => {
                    store.set(new_relative, new_relative, true)?;
                }
                (true, true) => {}
            }
        }
        notify::EventKind::Modify(notify::event::ModifyKind::Name(
            notify::event::RenameMode::To,
        )) => {
            let local_path = &event.paths[0];
            let relative_path = local_path
                .strip_prefix(watch_path)?
                .to_str()
                .ok_or("non-utf8 path")?;

            if is_exempt(local_path, exempt_names) {
                backend.set_readonly(local_path, false)?;
                store.set(relative_path, relative_path, false)?;
            } else {
                backend.set_readonly(local_path, true)?;
                store.set(relative_path, relative_path, true)?;
            }
        }
        notify::EventKind::Modify(notify::event::ModifyKind::Name(
            notify::event::RenameMode::From,
        ))
        | notify::EventKind::Remove(_) => {
            let local_path = &event.paths[0];
            if let Some(relative_path) = local_path
                .strip_prefix(watch_path)
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
