#[cfg(feature = "discord")]
pub mod runtime;

use crate::{
    audio::{AudioSource, BasicResolver, RepeatMode},
    config::Config,
    guild::SessionRegistry,
};
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Play(String),
    Pause,
    Resume,
    Stop,
    Skip,
    Previous,
    Shuffle,
    Queue,
    NowPlaying,
    Volume(u8),
    Mute,
    Repeat(RepeatMode),
    Remove(usize),
    Clear,
    Help,
}
#[derive(Debug, Error, PartialEq, Eq)]
pub enum CommandError {
    #[error("unknown command")]
    Unknown,
    #[error("missing argument")]
    MissingArgument,
    #[error("invalid number")]
    InvalidNumber,
    #[error("invalid input")]
    InvalidInput,
}
pub fn parse_command(input: &str, prefix: &str) -> Result<Command, CommandError> {
    if input.len() > 2048 || prefix.len() > 16 {
        return Err(CommandError::InvalidInput);
    }
    let text = input.trim().strip_prefix(prefix).unwrap_or(input.trim());
    let mut p = text.splitn(2, ' ');
    let name = p.next().unwrap_or("").to_ascii_lowercase();
    let arg = p.next().map(str::trim).filter(|s| !s.is_empty());
    match name.as_str() {
        "play" | "p" => Ok(Command::Play(
            arg.ok_or(CommandError::MissingArgument)?.into(),
        )),
        "pause" => Ok(Command::Pause),
        "resume" | "unpause" => Ok(Command::Resume),
        "stop" => Ok(Command::Stop),
        "skip" | "next" => Ok(Command::Skip),
        "previous" | "back" => Ok(Command::Previous),
        "shuffle" => Ok(Command::Shuffle),
        "queue" | "q" => Ok(Command::Queue),
        "nowplaying" | "np" => Ok(Command::NowPlaying),
        "volume" | "vol" => Ok(Command::Volume(
            arg.ok_or(CommandError::MissingArgument)?
                .parse()
                .map_err(|_| CommandError::InvalidNumber)?,
        )),
        "mute" => Ok(Command::Mute),
        "repeat" | "loop" => match arg.unwrap_or("off") {
            "track" => Ok(Command::Repeat(RepeatMode::Track)),
            "queue" => Ok(Command::Repeat(RepeatMode::Queue)),
            "off" => Ok(Command::Repeat(RepeatMode::Off)),
            _ => Err(CommandError::Unknown),
        },
        "remove" => Ok(Command::Remove(
            arg.ok_or(CommandError::MissingArgument)?
                .parse()
                .map_err(|_| CommandError::InvalidNumber)?,
        )),
        "clear" => Ok(Command::Clear),
        "help" => Ok(Command::Help),
        _ => Err(CommandError::Unknown),
    }
}

#[derive(Clone)]
pub struct CommandService {
    pub config: Config,
    pub sessions: SessionRegistry,
    pub resolver: Arc<dyn AudioSource>,
}
impl CommandService {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            sessions: SessionRegistry::default(),
            resolver: Arc::new(BasicResolver),
        }
    }
    pub async fn play(&self, guild_id: u64, user_id: u64, query: &str) -> Result<String, String> {
        let tracks = self
            .resolver
            .resolve(query, user_id)
            .await
            .map_err(|e| e.to_string())?;
        let session = self.sessions.get_or_create(guild_id, &self.config).await;
        let mut session = session.lock().await;
        let count = tracks.len();
        for t in tracks {
            session.enqueue(t).map_err(|e| e.to_string())?;
        }
        if session.player.current.is_none() {
            let _ = session.player.start_next();
        }
        Ok(format!("{} faixa(s) adicionada(s)", count))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_commands() {
        assert_eq!(parse_command("!volume 40", "!"), Ok(Command::Volume(40)));
        assert_eq!(
            parse_command("!repeat queue", "!"),
            Ok(Command::Repeat(RepeatMode::Queue))
        );
        assert!(parse_command(&("!play ".to_owned() + &"x".repeat(2048)), "!").is_err());
    }

    #[tokio::test]
    async fn concurrent_play_operations_preserve_both_tracks() {
        let service = CommandService::new(Config::for_self_check().unwrap());
        let first = service.play(7, 1, "first");
        let second = service.play(7, 2, "second");
        let (a, b) = tokio::join!(first, second);
        assert!(a.is_ok() && b.is_ok());
        let session = service.sessions.get_or_create(7, &service.config).await;
        let session = session.lock().await;
        assert_eq!(session.player.queue.len(), 1);
        assert!(session.player.current.is_some());
    }
}
