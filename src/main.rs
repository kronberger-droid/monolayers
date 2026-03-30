use std::path::Path;

use crate::{api::NextcloudClient, config::NextcloudConfig};

mod api;
mod config;
mod reconciler;
mod watcher;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = NextcloudConfig::load(Path::new("config/test_config.toml"))?;

    let client = NextcloudClient::new(config.base_url, config.user_credentials);

    let tag_id = client.ensure_tag("immutable").await?;

    reconciler::reconcile(
        &client,
        &tag_id,
        &config.local_sync_path,
        &config.exempt_folder_names,
    )
    .await?;

    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();

    let _watcher = watcher::start_watching(&config.local_sync_path, sender)?;

    while let Some(event) = receiver.recv().await {
        println!("{event:?}");
    }

    Ok(())
}
