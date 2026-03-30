use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct FileState {
    file_id: String,
    tagged: bool,
}

pub struct StateStore {
    db: sled::Db,
}

impl StateStore {
    pub fn open(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            db: sled::open(path)?,
        })
    }

    pub fn get(&self, relative_path: &str) -> Option<(String, bool)> {
        let bytes = self.db.get(relative_path.as_bytes()).ok()??;
        let state: FileState = serde_json::from_slice(&bytes).ok()?;
        Some((state.file_id, state.tagged))
    }

    pub fn set(
        &self,
        relative_path: &str,
        file_id: &str,
        tagged: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let state = FileState {
            file_id: file_id.to_string(),
            tagged,
        };
        self.db
            .insert(relative_path.as_bytes(), serde_json::to_vec(&state)?)?;
        Ok(())
    }

    pub fn remove(&self, relative_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.db.remove(relative_path.as_bytes())?;
        Ok(())
    }
}
