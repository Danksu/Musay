use serde::{Deserialize, Serialize};
use std::env;
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub discord_token: String,
    pub command_prefix: String,
    pub default_volume: u8,
    pub max_queue_size: usize,
    pub leave_on_empty_secs: u64,
    pub database_path: String,
}
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("DISCORD_TOKEN is missing")]
    MissingToken,
    #[error("invalid numeric configuration: {0}")]
    InvalidNumber(String),
}
impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let token = env::var("DISCORD_TOKEN").map_err(|_| ConfigError::MissingToken)?;
        let parse = |key: &str, default: &str| {
            env::var(key)
                .unwrap_or_else(|_| default.into())
                .parse::<u64>()
                .map_err(|_| ConfigError::InvalidNumber(key.into()))
        };
        Ok(Self {
            discord_token: token,
            command_prefix: env::var("COMMAND_PREFIX").unwrap_or_else(|_| "!".into()),
            default_volume: parse("DEFAULT_VOLUME", "75")?.min(100) as u8,
            max_queue_size: parse("MAX_QUEUE_SIZE", "100")? as usize,
            leave_on_empty_secs: parse("LEAVE_ON_EMPTY_SECS", "300")?,
            database_path: env::var("DATABASE_PATH").unwrap_or_else(|_| "musay.json".into()),
        })
    }
}
