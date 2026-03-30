pub struct StateStore {
    db: sled::Db,
}

impl StateStore {
    pub fn open(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            db: sled::open(path)?,
        })
    }
}
