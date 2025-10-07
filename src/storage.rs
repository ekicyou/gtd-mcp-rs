use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};
use crate::gtd::GtdData;

pub struct Storage {
    file_path: PathBuf,
}

impl Storage {
    pub fn new(file_path: impl AsRef<Path>) -> Self {
        Self {
            file_path: file_path.as_ref().to_path_buf(),
        }
    }

    pub fn load(&self) -> Result<GtdData> {
        if !self.file_path.exists() {
            return Ok(GtdData::new());
        }

        let content = fs::read_to_string(&self.file_path)?;
        let data: GtdData = toml::from_str(&content)?;
        Ok(data)
    }

    pub fn save(&self, data: &GtdData) -> Result<()> {
        let content = toml::to_string_pretty(data)?;
        fs::write(&self.file_path, content)?;
        Ok(())
    }
}
