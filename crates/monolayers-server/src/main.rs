use std::path::Path;

use monolayers_core::store::StateStore;

mod api;
mod backend;
mod config;
mod policy;
mod reconciler;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = config::ServerConfig::load(Path::new("config/server_config.toml"))?;

    let store = StateStore::open(&config.state_db_path)?;
    let backend = backend::ChattrBackend;

    // TODO: startup reconciler (walk /srv/files, ensure chattr state matches store)

    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();

    let _watcher = monolayers_core::watcher::start_watching(&config.watch_path, sender)?;

    while let Some(event) = receiver.recv().await {
        if let Err(e) = policy::handle_event(
            &event,
            &config.watch_path,
            &config.exempt_folder_names,
            &store,
            &backend,
        )
        .await
        {
            eprintln!("error handling event: {e}");
        }
    }

    Ok(())
}
