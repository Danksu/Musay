use crate::config::Config;
use crate::discord::{parse_command, Command, CommandService};
use serenity::all::{Context, EventHandler, GatewayIntents, GuildId, Message, Ready, UserId};
use serenity::async_trait;
use songbird::input::YoutubeDl;
use songbird::tracks::TrackHandle;
use songbird::SerenityInit;
use std::collections::HashMap;
use std::env;
use std::path::Path;
use std::sync::Arc;
use tokio::process::Command as ProcessCommand;
use tokio::sync::Mutex;
use tracing::{info, warn};

#[derive(Clone)]
pub struct BotRuntime {
    pub service: CommandService,
    pub http: reqwest::Client,
    active_tracks: Arc<Mutex<HashMap<GuildId, TrackHandle>>>,
}

impl BotRuntime {
    pub fn new(config: Config) -> Self {
        Self {
            service: CommandService::new(config),
            http: reqwest::Client::new(),
            active_tracks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn run(self) -> Result<(), String> {
        self.ensure_local_tools().await?;
        let intents = GatewayIntents::GUILDS
            | GatewayIntents::GUILD_MESSAGES
            | GatewayIntents::MESSAGE_CONTENT
            | GatewayIntents::GUILD_VOICE_STATES;
        let handler = Handler {
            runtime: self.clone(),
        };
        let mut client = serenity::Client::builder(&self.service.config.discord_token, intents)
            .event_handler(handler)
            .register_songbird()
            .await
            .map_err(|error| format!("falha ao construir cliente Discord: {error}"))?;

        tokio::select! {
            result = client.start() => result.map_err(|error| format!("cliente Discord encerrou com erro: {error}")),
            signal = tokio::signal::ctrl_c() => {
                signal.map_err(|error| format!("falha ao aguardar Ctrl+C: {error}"))?;
                info!("Ctrl+C recebido; encerrando shards e chamadas de voz");
                client.shard_manager.shutdown_all().await;
                Ok(())
            }
        }
    }

    async fn ensure_local_tools(&self) -> Result<(), String> {
        let executable_dir = env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(Path::to_path_buf));
        if let Some(dir) = executable_dir {
            let candidate = if cfg!(windows) {
                dir.join("yt-dlp.exe")
            } else {
                dir.join("yt-dlp")
            };
            if candidate.is_file() {
                let current_path = env::var_os("PATH").unwrap_or_default();
                let mut paths = vec![dir];
                paths.extend(env::split_paths(&current_path));
                let joined = env::join_paths(paths)
                    .map_err(|error| format!("PATH local inválido: {error}"))?;
                env::set_var("PATH", joined);
            }
        }
        let output = ProcessCommand::new(if cfg!(windows) {
            "yt-dlp.exe"
        } else {
            "yt-dlp"
        })
        .arg("--version")
        .output()
        .await
        .map_err(|_| {
            "yt-dlp não foi encontrado. Coloque yt-dlp/yt-dlp.exe ao lado do executável ou no PATH"
                .to_owned()
        })?;
        if !output.status.success() {
            return Err("yt-dlp foi encontrado, mas não conseguiu executar --version".to_owned());
        }
        info!(version = %String::from_utf8_lossy(&output.stdout).trim(), "yt-dlp disponível");
        if ProcessCommand::new(if cfg!(windows) {
            "ffmpeg.exe"
        } else {
            "ffmpeg"
        })
        .arg("-version")
        .output()
        .await
        .is_err()
        {
            warn!("FFmpeg não foi encontrado; algumas fontes e formatos podem não reproduzir");
        }
        Ok(())
    }

    async fn voice_channel_for(
        &self,
        ctx: &Context,
        message: &Message,
    ) -> Result<serenity::all::ChannelId, String> {
        let guild_id = message
            .guild_id
            .ok_or_else(|| "este comando só funciona em um servidor".to_owned())?;
        let voice_state = guild_id
            .get_user_voice_state(&ctx.http, UserId::new(message.author.id.get()))
            .await
            .map_err(|error| format!("não foi possível consultar seu canal de voz: {error}"))?;
        voice_state
            .channel_id
            .ok_or_else(|| "entre em um canal de voz antes de usar este comando".to_owned())
    }

    async fn play(
        &self,
        ctx: &Context,
        message: &Message,
        query: String,
    ) -> Result<String, String> {
        let guild_id = message
            .guild_id
            .ok_or_else(|| "este comando só funciona em um servidor".to_owned())?;
        let voice_channel = self.voice_channel_for(ctx, message).await?;
        self.stop_active(guild_id).await;
        let session = self
            .service
            .sessions
            .get_or_create(guild_id.get(), &self.service.config)
            .await;
        session.lock().await.player.stop();
        self.service
            .play(guild_id.get(), message.author.id.get(), &query)
            .await?;
        let manager = songbird::get(ctx)
            .await
            .ok_or_else(|| "gerenciador de voz indisponível".to_owned())?
            .clone();
        let call = manager
            .join(guild_id, voice_channel)
            .await
            .map_err(|error| format!("não foi possível entrar no canal de voz: {error}"))?;
        let source = if url::Url::parse(&query)
            .map(|url| url.scheme() == "http" || url.scheme() == "https")
            .unwrap_or(false)
        {
            YoutubeDl::new(self.http.clone(), query.clone()).into()
        } else {
            YoutubeDl::new_search(self.http.clone(), query.clone()).into()
        };
        let handle = call.lock().await.play_input(source);
        if let Some(previous) = self.active_tracks.lock().await.insert(guild_id, handle) {
            let _ = previous.stop();
        }
        Ok(format!("tocando `{query}`"))
    }

    async fn reply(&self, message: &Message, content: impl Into<String>, ctx: &Context) {
        if let Err(error) = message.channel_id.say(&ctx.http, content.into()).await {
            warn!(?error, "falha ao enviar resposta Discord");
        }
    }

    async fn dispatch(&self, ctx: &Context, message: &Message, command: Command) {
        let guild_id = message.guild_id;
        let result: Result<String, String> = match command {
            Command::Play(query) => self.play(ctx, message, query).await,
            Command::Pause => self.control_active(guild_id, |track| track.pause(), "pausado").await,
            Command::Resume => self.control_active(guild_id, |track| track.play(), "retomado").await,
                        Command::Stop => {
                if let Some(id) = guild_id {
                    self.stop_active(id).await;
                    let _ = self.mutate_session(Some(id), |session| { session.player.stop(); Ok(String::new()) }).await;
                }
                Ok("parado".into())
            }
            Command::Skip => {
                if let Some(id) = guild_id {
                    self.stop_active(id).await;
                    let _ = self.mutate_session(Some(id), |session| { session.player.stop(); Ok(String::new()) }).await;
                }
                Ok("faixa pulada; use `!play` para iniciar outra".into())
            }
            Command::Volume(value) => self.control_active(guild_id, |track| track.set_volume(value as f32 / 100.0), &format!("volume ajustado para {value}%")).await,
            Command::Mute => self.control_active(guild_id, |track| track.set_volume(0.0), "mutado").await,
            Command::Queue => self.queue(guild_id).await,
            Command::NowPlaying => self.now_playing(guild_id).await,
            Command::Shuffle => self.mutate_session(guild_id, |session| { session.player.queue.shuffle(); Ok("fila embaralhada".into()) }).await,
            Command::Repeat(mode) => self.mutate_session(guild_id, move |session| { session.player.set_repeat(mode); Ok(format!("repeat definido para {mode:?}")) }).await,
            Command::Remove(index) => self.mutate_session(guild_id, move |session| session.player.queue.remove(index).map(|track| format!("removido `{}`", track.title)).map_err(|error| error.to_string())).await,
            Command::Clear => self.mutate_session(guild_id, |session| { session.player.queue.clear(); Ok("fila limpa".into()) }).await,
            Command::Previous => self.mutate_session(guild_id, |session| { session.player.queue.previous().map(|track| { let _ = session.player.queue.push_front(track.clone()); format!("faixa anterior recuperada: `{}`", track.title) }).ok_or_else(|| "não há histórico".into()) }).await,
            Command::Help => Ok("comandos: `!play <busca/url>`, `!pause`, `!resume`, `!stop`, `!skip`, `!queue`, `!nowplaying`, `!shuffle`, `!volume <0-100>`, `!mute`, `!repeat off|track|queue`, `!remove <índice>`, `!clear`".into()),
        };
        match result {
            Ok(text) => self.reply(message, text, ctx).await,
            Err(error) => self.reply(message, format!("erro: {error}"), ctx).await,
        }
    }

    async fn control_active<F>(
        &self,
        guild_id: Option<GuildId>,
        operation: F,
        success: &str,
    ) -> Result<String, String>
    where
        F: FnOnce(&TrackHandle) -> songbird::tracks::TrackResult<()>,
    {
        let id = guild_id.ok_or_else(|| "este comando só funciona em um servidor".to_owned())?;
        let tracks = self.active_tracks.lock().await;
        let track = tracks
            .get(&id)
            .ok_or_else(|| "não há faixa ativa".to_owned())?;
        operation(track).map_err(|error| error.to_string())?;
        Ok(success.into())
    }

    async fn stop_active(&self, guild_id: GuildId) {
        if let Some(track) = self.active_tracks.lock().await.remove(&guild_id) {
            let _ = track.stop();
        }
    }

    async fn mutate_session<F>(
        &self,
        guild_id: Option<GuildId>,
        operation: F,
    ) -> Result<String, String>
    where
        F: FnOnce(&mut crate::guild::GuildSession) -> Result<String, String>,
    {
        let id = guild_id.ok_or_else(|| "este comando só funciona em um servidor".to_owned())?;
        let session = self
            .service
            .sessions
            .get_or_create(id.get(), &self.service.config)
            .await;
        let mut session = session.lock().await;
        operation(&mut session)
    }

    async fn queue(&self, guild_id: Option<GuildId>) -> Result<String, String> {
        let id = guild_id.ok_or_else(|| "este comando só funciona em um servidor".to_owned())?;
        let session = self
            .service
            .sessions
            .get_or_create(id.get(), &self.service.config)
            .await;
        let session = session.lock().await;
        let current = session
            .player
            .current
            .as_ref()
            .map(|track| track.title.as_str())
            .unwrap_or("nada");
        let next = session
            .player
            .queue
            .items()
            .take(10)
            .map(|track| track.title.clone())
            .collect::<Vec<_>>();
        Ok(format!(
            "atual: `{current}`; fila: {}",
            if next.is_empty() {
                "vazia".into()
            } else {
                next.join(", ")
            }
        ))
    }

    async fn now_playing(&self, guild_id: Option<GuildId>) -> Result<String, String> {
        let id = guild_id.ok_or_else(|| "este comando só funciona em um servidor".to_owned())?;
        let session = self
            .service
            .sessions
            .get_or_create(id.get(), &self.service.config)
            .await;
        let session = session.lock().await;
        let track = session
            .player
            .current
            .as_ref()
            .ok_or_else(|| "nada está tocando".to_owned())?;
        Ok(format!(
            "tocando `{}`; volume {}%",
            track.title, session.player.volume
        ))
    }
}

struct Handler {
    runtime: BotRuntime,
}
#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        info!(user = %ready.user.name, "bot conectado ao Discord");
    }
    async fn message(&self, ctx: Context, message: Message) {
        if message.author.bot
            || !message
                .content
                .starts_with(&self.runtime.service.config.command_prefix)
        {
            return;
        }
        match parse_command(
            &message.content,
            &self.runtime.service.config.command_prefix,
        ) {
            Ok(command) => self.runtime.dispatch(&ctx, &message, command).await,
            Err(error) => {
                self.runtime
                    .reply(&message, format!("erro: {error}"), &ctx)
                    .await
            }
        }
    }
}
