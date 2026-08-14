use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;
use thiserror::Error;

const MAX_PREFIX_BYTES: usize = 16;
const MAX_QUEUE_SIZE: usize = 10_000;
const MAX_LEAVE_ON_EMPTY_SECS: u64 = 86_400;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub discord_token: String,
    pub command_prefix: String,
    pub default_volume: u8,
    pub max_queue_size: usize,
    pub leave_on_empty_secs: u64,
    pub database_path: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error("DISCORD_TOKEN is missing")]
    MissingToken,
    #[error("invalid numeric configuration: {0}")]
    InvalidNumber(String),
    #[error("configuration value is out of range: {0}")]
    OutOfRange(String),
    #[error("configuration value cannot be empty: {0}")]
    Empty(String),
    #[error("database path must be a relative file path without parent traversal")]
    UnsafeDatabasePath,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let token = env::var("DISCORD_TOKEN").map_err(|_| ConfigError::MissingToken)?;
        Self::from_values(token)
    }

    pub fn for_self_check() -> Result<Self, ConfigError> {
        Self::from_values("self-check-token".to_owned())
    }

    fn from_values(token: String) -> Result<Self, ConfigError> {
        let command_prefix = env::var("COMMAND_PREFIX").unwrap_or_else(|_| "!".into());
        if command_prefix.is_empty() || command_prefix.len() > MAX_PREFIX_BYTES {
            return Err(ConfigError::OutOfRange("COMMAND_PREFIX".into()));
        }
        let parse = |key: &str, default: &str| {
            env::var(key)
                .unwrap_or_else(|_| default.into())
                .parse::<u64>()
                .map_err(|_| ConfigError::InvalidNumber(key.into()))
        };
        let default_volume = parse("DEFAULT_VOLUME", "75")?;
        let max_queue_size = parse("MAX_QUEUE_SIZE", "100")?;
        let leave_on_empty_secs = parse("LEAVE_ON_EMPTY_SECS", "300")?;
        if default_volume > 100 {
            return Err(ConfigError::OutOfRange("DEFAULT_VOLUME".into()));
        }
        if !(1..=MAX_QUEUE_SIZE as u64).contains(&max_queue_size) {
            return Err(ConfigError::OutOfRange("MAX_QUEUE_SIZE".into()));
        }
        if leave_on_empty_secs > MAX_LEAVE_ON_EMPTY_SECS {
            return Err(ConfigError::OutOfRange("LEAVE_ON_EMPTY_SECS".into()));
        }
        let database_path = env::var("DATABASE_PATH").unwrap_or_else(|_| "musay.json".into());
        let path = PathBuf::from(&database_path);
        if database_path.is_empty()
            || path.is_absolute()
            || path
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err(ConfigError::UnsafeDatabasePath);
        }
        Ok(Self {
            discord_token: token,
            command_prefix,
            default_volume: default_volume as u8,
            max_queue_size: max_queue_size as usize,
            leave_on_empty_secs,
            database_path,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn self_check_has_safe_defaults() {
        let c = Config::for_self_check().unwrap();
        assert_eq!(c.default_volume, 75);
        assert!(c.max_queue_size > 0);
    }
}
