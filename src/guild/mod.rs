use crate::audio::{Player, RepeatMode, Track};
use crate::config::Config;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

#[derive(Debug, Clone)]
pub struct GuildSession {
    pub guild_id: u64,
    pub voice_channel_id: Option<u64>,
    pub text_channel_id: Option<u64>,
    pub player: Player,
}

impl GuildSession {
    pub fn new(guild_id: u64, config: &Config) -> Self {
        Self {
            guild_id,
            voice_channel_id: None,
            text_channel_id: None,
            player: Player::new(config.max_queue_size, config.default_volume),
        }
    }
    pub fn enqueue(&mut self, track: Track) -> Result<bool, crate::audio::QueueError> {
        self.player.enqueue(track)
    }
    pub fn set_repeat(&mut self, mode: RepeatMode) {
        self.player.set_repeat(mode);
    }
}

#[derive(Clone, Default)]
pub struct SessionRegistry {
    sessions: Arc<RwLock<HashMap<u64, Arc<Mutex<GuildSession>>>>>,
}

impl SessionRegistry {
    pub async fn get_or_create(&self, guild_id: u64, config: &Config) -> Arc<Mutex<GuildSession>> {
        if let Some(session) = self.sessions.read().await.get(&guild_id).cloned() {
            return session;
        }
        let mut lock = self.sessions.write().await;
        lock.entry(guild_id)
            .or_insert_with(|| Arc::new(Mutex::new(GuildSession::new(guild_id, config))))
            .clone()
    }
    pub async fn remove(&self, guild_id: u64) -> Option<Arc<Mutex<GuildSession>>> {
        self.sessions.write().await.remove(&guild_id)
    }
    pub async fn len(&self) -> usize {
        self.sessions.read().await.len()
    }
}
