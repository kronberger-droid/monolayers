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

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store() -> StateStore {
        let dir = tempfile::tempdir().unwrap();
        StateStore::open(dir.path()).unwrap()
    }

    #[test]
    fn get_missing_key_returns_none() {
        let store = temp_store();
        assert!(store.get("nonexistent.txt").is_none());
    }

    #[test]
    fn set_then_get() {
        let store = temp_store();
        store.set("docs/file.txt", "12345", true).unwrap();

        let (file_id, tagged) = store.get("docs/file.txt").unwrap();
        assert_eq!(file_id, "12345");
        assert!(tagged);
    }

    #[test]
    fn set_overwrites_previous() {
        let store = temp_store();
        store.set("file.txt", "100", true).unwrap();
        store.set("file.txt", "100", false).unwrap();

        let (_, tagged) = store.get("file.txt").unwrap();
        assert!(!tagged);
    }

    #[test]
    fn remove_deletes_entry() {
        let store = temp_store();
        store.set("file.txt", "100", true).unwrap();
        store.remove("file.txt").unwrap();

        assert!(store.get("file.txt").is_none());
    }

    #[test]
    fn remove_nonexistent_is_ok() {
        let store = temp_store();
        store.remove("ghost.txt").unwrap();
    }

    #[test]
    fn multiple_keys_independent() {
        let store = temp_store();
        store.set("a.txt", "1", true).unwrap();
        store.set("b.txt", "2", false).unwrap();

        let (id_a, tagged_a) = store.get("a.txt").unwrap();
        let (id_b, tagged_b) = store.get("b.txt").unwrap();

        assert_eq!(id_a, "1");
        assert!(tagged_a);
        assert_eq!(id_b, "2");
        assert!(!tagged_b);
    }
}
