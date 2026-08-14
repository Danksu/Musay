use crate::audio::Track;
use crate::permissions::PermissionPolicy;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;
use tokio::fs;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GuildData {
    pub policy: PermissionPolicy,
    pub saved_playlists: Vec<SavedPlaylist>,
    pub default_volume: Option<u8>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedPlaylist {
    pub name: String,
    pub tracks: Vec<Track>,
}
#[derive(Debug, Error)]
pub enum PersistenceError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}
#[derive(Clone)]
pub struct JsonStore {
    path: PathBuf,
}
impl JsonStore {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }
    pub async fn load(&self) -> Result<Vec<GuildData>, PersistenceError> {
        match fs::read(&self.path).await {
            Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(e) => Err(e.into()),
        }
    }
    pub async fn save(&self, data: &[GuildData]) -> Result<(), PersistenceError> {
        let bytes = serde_json::to_vec_pretty(data)?;
        let tmp = self.path.with_extension("tmp");
        fs::write(&tmp, bytes).await?;
        fs::rename(tmp, &self.path).await?;
        Ok(())
    }
}
