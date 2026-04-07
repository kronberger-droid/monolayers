use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Serialize)]
pub struct ServerConfig {
    pub watch_path: PathBuf,
    pub state_db_path: PathBuf,
    pub exempt_folder_names: Vec<String>,
}

impl ServerConfig {
    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let file_content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&file_content)?;
        Ok(config)
    }
}
