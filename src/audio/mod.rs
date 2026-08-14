use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SourceKind {
    YouTube,
    SoundCloud,
    Spotify,
    DirectUrl,
    LocalFile,
    Radio,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Track {
    pub id: String,
    pub title: String,
    pub url: String,
    pub author: Option<String>,
    pub duration_secs: Option<u64>,
    pub source: SourceKind,
    pub requested_by: u64,
}
impl Track {
    pub fn new(title: impl Into<String>, url: impl Into<String>, requested_by: u64) -> Self {
        let url = url.into();
        Self {
            id: url.clone(),
            title: title.into(),
            url,
            author: None,
            duration_secs: None,
            source: SourceKind::Unknown,
            requested_by,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RepeatMode {
    Off,
    Track,
    Queue,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum QueueError {
    #[error("queue is empty")]
    Empty,
    #[error("index out of bounds")]
    InvalidIndex,
    #[error("queue limit reached")]
    Full,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackQueue {
    items: VecDeque<Track>,
    history: Vec<Track>,
    max_size: usize,
}
impl TrackQueue {
    pub fn new(max_size: usize) -> Self {
        Self {
            items: VecDeque::new(),
            history: Vec::new(),
            max_size,
        }
    }
    pub fn len(&self) -> usize {
        self.items.len()
    }
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
    pub fn items(&self) -> impl Iterator<Item = &Track> {
        self.items.iter()
    }
    pub fn history(&self) -> &[Track] {
        &self.history
    }
    pub fn push(&mut self, track: Track) -> Result<(), QueueError> {
        if self.items.len() >= self.max_size {
            return Err(QueueError::Full);
        }
        self.items.push_back(track);
        Ok(())
    }
    pub fn push_front(&mut self, track: Track) -> Result<(), QueueError> {
        if self.items.len() >= self.max_size {
            return Err(QueueError::Full);
        }
        self.items.push_front(track);
        Ok(())
    }
    pub fn pop(&mut self) -> Result<Track, QueueError> {
        self.items.pop_front().ok_or(QueueError::Empty)
    }
    pub fn remove(&mut self, index: usize) -> Result<Track, QueueError> {
        self.items.remove(index).ok_or(QueueError::InvalidIndex)
    }
    pub fn move_item(&mut self, from: usize, to: usize) -> Result<(), QueueError> {
        if from >= self.items.len() || to >= self.items.len() {
            return Err(QueueError::InvalidIndex);
        }
        let item = self.items.remove(from).ok_or(QueueError::InvalidIndex)?;
        self.items.insert(to, item);
        Ok(())
    }
    pub fn clear(&mut self) {
        self.items.clear();
    }
    pub fn shuffle(&mut self) {
        let mut v: Vec<_> = self.items.drain(..).collect();
        v.shuffle(&mut rand::thread_rng());
        self.items.extend(v);
    }
    pub fn record_history(&mut self, track: Track) {
        self.history.push(track);
        if self.history.len() > 100 {
            let _ = self.history.remove(0);
        }
    }
    pub fn previous(&mut self) -> Option<Track> {
        self.history.pop()
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PlayerState {
    Idle,
    Loading,
    Playing,
    Paused,
    Stopped,
    Recovering,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub state: PlayerState,
    pub current: Option<Track>,
    pub queue: TrackQueue,
    pub repeat: RepeatMode,
    pub volume: u8,
    pub muted: bool,
    pub position_secs: u64,
}
impl Player {
    pub fn new(max_queue: usize, volume: u8) -> Self {
        Self {
            state: PlayerState::Idle,
            current: None,
            queue: TrackQueue::new(max_queue),
            repeat: RepeatMode::Off,
            volume: volume.min(100),
            muted: false,
            position_secs: 0,
        }
    }
    pub fn enqueue(&mut self, track: Track) -> Result<bool, QueueError> {
        let start = self.current.is_none();
        self.queue.push(track)?;
        Ok(start)
    }
    pub fn start_next(&mut self) -> Option<Track> {
        if self.current.is_some() {
            return self.current.clone();
        }
        self.current = self.queue.pop().ok();
        self.position_secs = 0;
        self.state = if self.current.is_some() {
            PlayerState::Playing
        } else {
            PlayerState::Idle
        };
        if let Some(track) = self.current.clone() {
            self.queue.record_history(track);
        }
        self.current.clone()
    }
    pub fn finish_current(&mut self) -> Option<Track> {
        let previous = self.current.take();
        let next = match self.repeat {
            RepeatMode::Track => previous.or_else(|| self.queue.pop().ok()),
            RepeatMode::Queue => {
                if let Some(track) = previous {
                    let _ = self.queue.push(track);
                }
                self.queue.pop().ok()
            }
            RepeatMode::Off => self.queue.pop().ok(),
        };
        self.current = next.clone();
        self.position_secs = 0;
        self.state = if next.is_some() {
            PlayerState::Playing
        } else {
            PlayerState::Idle
        };
        if let Some(track) = next.clone() {
            self.queue.record_history(track);
        }
        next
    }
    pub fn pause(&mut self) -> bool {
        if self.state == PlayerState::Playing {
            self.state = PlayerState::Paused;
            true
        } else {
            false
        }
    }
    pub fn resume(&mut self) -> bool {
        if self.state == PlayerState::Paused {
            self.state = PlayerState::Playing;
            true
        } else {
            false
        }
    }
    pub fn stop(&mut self) {
        self.state = PlayerState::Stopped;
        self.current = None;
        self.position_secs = 0;
    }
    pub fn skip(&mut self) -> Option<Track> {
        self.current = None;
        self.start_next()
    }
    pub fn seek(&mut self, position: Duration) -> bool {
        if self.current.is_some() {
            self.position_secs = position.as_secs();
            true
        } else {
            false
        }
    }
    pub fn set_volume(&mut self, volume: u8) -> u8 {
        self.volume = volume.min(100);
        self.muted = self.volume == 0;
        self.volume
    }
    pub fn toggle_mute(&mut self) -> bool {
        self.muted = !self.muted;
        self.muted
    }
    pub fn set_repeat(&mut self, mode: RepeatMode) {
        self.repeat = mode;
    }
}
#[async_trait::async_trait]
pub trait AudioSource: Send + Sync {
    async fn resolve(&self, input: &str, requested_by: u64) -> Result<Vec<Track>, SourceError>;
}
#[derive(Debug, Error)]
pub enum SourceError {
    #[error("invalid source: {0}")]
    Invalid(String),
    #[error("source unavailable: {0}")]
    Unavailable(String),
    #[error("resolver error: {0}")]
    Resolver(String),
}
pub struct BasicResolver;
#[async_trait::async_trait]
impl AudioSource for BasicResolver {
    async fn resolve(&self, input: &str, requested_by: u64) -> Result<Vec<Track>, SourceError> {
        let value = input.trim();
        if value.is_empty() {
            return Err(SourceError::Invalid("empty query".into()));
        }
        if value.len() > 2048 || value.chars().any(char::is_control) {
            return Err(SourceError::Invalid("query exceeds input limits".into()));
        }
        let parsed = url::Url::parse(value).ok();
        if let Some(ref url) = parsed {
            let scheme = url.scheme();
            if scheme != "http" && scheme != "https" {
                return Err(SourceError::Invalid(
                    "only HTTP(S) sources are accepted".into(),
                ));
            }
            if url.username() != "" || url.password().is_some() {
                return Err(SourceError::Invalid(
                    "URL credentials are not accepted".into(),
                ));
            }
            if let Some(host) = url.host_str() {
                if let Ok(ip) = host.parse::<std::net::IpAddr>() {
                    let private = match ip {
                        std::net::IpAddr::V4(ip) => {
                            ip.is_private()
                                || ip.is_loopback()
                                || ip.is_link_local()
                                || ip.is_unspecified()
                                || ip.is_broadcast()
                                || ip.is_documentation()
                        }
                        std::net::IpAddr::V6(ip) => {
                            let first = ip.segments()[0];
                            ip.is_loopback()
                                || ip.is_unspecified()
                                || (first & 0xfe00) == 0xfc00
                                || (first & 0xffc0) == 0xfe80
                        }
                    };
                    if private {
                        return Err(SourceError::Invalid(
                            "private or local network destinations are not accepted".into(),
                        ));
                    }
                }
            }
        }
        let is_url = parsed.is_some();
        let resolved_url = if is_url {
            value.to_owned()
        } else {
            "search://".to_owned() + value
        };
        let mut track = Track::new(value, resolved_url, requested_by);
        track.source = if value.contains("youtube") {
            SourceKind::YouTube
        } else if value.contains("soundcloud") {
            SourceKind::SoundCloud
        } else if is_url {
            SourceKind::DirectUrl
        } else {
            SourceKind::Unknown
        };
        Ok(vec![track])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn t(n: &str) -> Track {
        Track::new(n, format!("https://example/{n}"), 1)
    }
    #[test]
    fn queue_ops() {
        let mut q = TrackQueue::new(3);
        q.push(t("a")).unwrap();
        q.push(t("b")).unwrap();
        q.move_item(0, 1).unwrap();
        assert_eq!(q.items().next().unwrap().title, "b");
        q.shuffle();
        assert_eq!(q.len(), 2);
    }
    #[test]
    fn player_state_and_repeat() {
        let mut p = Player::new(5, 120);
        p.enqueue(t("a")).unwrap();
        p.start_next();
        assert_eq!(p.state, PlayerState::Playing);
        assert!(p.pause());
        assert!(p.resume());
        p.set_repeat(RepeatMode::Track);
        assert_eq!(p.finish_current().unwrap().title, "a");
        p.set_repeat(RepeatMode::Queue);
        p.enqueue(t("b")).unwrap();
        assert_eq!(p.finish_current().unwrap().title, "b");
        assert_eq!(p.finish_current().unwrap().title, "a");
        assert_eq!(p.queue.len(), 1);
    }

    #[tokio::test]
    async fn resolver_rejects_dangerous_inputs() {
        let resolver = BasicResolver;
        assert!(resolver.resolve("file:///etc/passwd", 1).await.is_err());
        assert!(resolver.resolve("http://127.0.0.1/admin", 1).await.is_err());
        assert!(resolver.resolve(&"x".repeat(2049), 1).await.is_err());
        assert!(resolver
            .resolve("https://example.com/audio", 1)
            .await
            .is_ok());
    }
}
