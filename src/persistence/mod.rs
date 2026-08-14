use crate::audio::Track;
use crate::permissions::PermissionPolicy;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;
use tokio::fs;

const MAX_PERSISTED_BYTES: u64 = 16 * 1024 * 1024;

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
    #[error("persistence path is unsafe")]
    UnsafePath,
    #[error("persistence file exceeds the configured safety limit")]
    TooLarge,
}
#[derive(Clone)]
pub struct JsonStore {
    path: PathBuf,
}
impl JsonStore {
    pub fn new(path: impl AsRef<Path>) -> Result<Self, PersistenceError> {
        let path = path.as_ref().to_path_buf();
        if path.as_os_str().is_empty()
            || path.is_absolute()
            || path
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err(PersistenceError::UnsafePath);
        }
        Ok(Self { path })
    }
    pub async fn load(&self) -> Result<Vec<GuildData>, PersistenceError> {
        let metadata = match fs::symlink_metadata(&self.path).await {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(error.into()),
        };
        if metadata.file_type().is_symlink() || metadata.len() > MAX_PERSISTED_BYTES {
            return Err(if metadata.file_type().is_symlink() {
                PersistenceError::UnsafePath
            } else {
                PersistenceError::TooLarge
            });
        }
        let bytes = fs::read(&self.path).await?;
        Ok(serde_json::from_slice(&bytes)?)
    }
    pub async fn save(&self, data: &[GuildData]) -> Result<(), PersistenceError> {
        let bytes = serde_json::to_vec_pretty(data)?;
        if bytes.len() as u64 > MAX_PERSISTED_BYTES {
            return Err(PersistenceError::TooLarge);
        }
        let tmp = self.path.with_extension("tmp");
        fs::write(&tmp, bytes).await?;
        fs::rename(tmp, &self.path).await?;
        Ok(())
    }
}
