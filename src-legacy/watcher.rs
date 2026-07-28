use notify::{Event, RecursiveMode, Watcher};
use std::path::Path;
use tokio::sync::mpsc;

pub fn start_watching(
    sync_path: &Path,
    sender: mpsc::UnboundedSender<Event>,
) -> Result<impl Watcher, Box<dyn std::error::Error>> {
    let mut watcher = notify::recommended_watcher(
        move |result: Result<Event, notify::Error>| {
            if let Ok(event) = result {
                let _ = sender.send(event);
            }
        },
    )?;

    watcher.watch(sync_path, RecursiveMode::Recursive)?;

    Ok(watcher)
}
