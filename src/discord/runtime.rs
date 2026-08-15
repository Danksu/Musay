use crate::config::Config;
use crate::discord::{parse_command, Command, CommandService};
use serenity::all::{Context, EventHandler, GatewayIntents, GuildId, Message, Ready, UserId};
use serenity::async_trait;
use songbird::events::{Event, EventContext, EventHandler as SongbirdEventHandler, TrackEvent};
use songbird::input::YoutubeDl;
use songbird::tracks::TrackHandle;
use songbird::SerenityInit;
use std::collections::HashMap;
use std::env;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Command as ProcessCommand;
use tokio::sync::{Mutex, RwLock};
use tracing::{info, warn};

#[derive(Clone)]
pub struct BotRuntime {
    pub service: CommandService,
    pub http: reqwest::Client,
    active_tracks: Arc<Mutex<HashMap<GuildId, TrackHandle>>>,
    voice_manager: Arc<RwLock<Option<Arc<songbird::Songbird>>>>,
}

impl BotRuntime {
    pub fn new(config: Config) -> Self {
        Self {
            service: CommandService::new(config),
            http: reqwest::Client::builder()
                .connect_timeout(Duration::from_secs(10))
                .timeout(Duration::from_secs(90))
                .user_agent("Musay/0.1")
                .build()
                .expect("configuração estática do cliente HTTP deve ser válida"),
            active_tracks: Arc::new(Mutex::new(HashMap::new())),
            voice_manager: Arc::new(RwLock::new(None)),
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
        let output = tokio::time::timeout(
            Duration::from_secs(10),
            ProcessCommand::new(if cfg!(windows) {
                "yt-dlp.exe"
            } else {
                "yt-dlp"
            })
            .arg("--version")
            .output(),
        )
        .await
        .map_err(|_| "yt-dlp demorou demais para responder".to_owned())?
        .map_err(|_| {
            "yt-dlp não foi encontrado. Coloque yt-dlp/yt-dlp.exe ao lado do executável ou no PATH"
                .to_owned()
        })?;
        if !output.status.success() {
            return Err("yt-dlp foi encontrado, mas não conseguiu executar --version".to_owned());
        }
        info!(version = %String::from_utf8_lossy(&output.stdout).trim(), "yt-dlp disponível");
        let deno_output = tokio::time::timeout(
            Duration::from_secs(10),
            ProcessCommand::new(if cfg!(windows) { "deno.exe" } else { "deno" })
                .arg("--version")
                .output(),
        )
        .await
        .map_err(|_| "Deno demorou demais para responder".to_owned())?
        .map_err(|_| {
            "Deno não foi encontrado. Coloque deno/deno.exe ao lado do executável ou no PATH"
                .to_owned()
        })?;
        if !deno_output.status.success() {
            return Err("Deno foi encontrado, mas não conseguiu executar --version".to_owned());
        }
        info!(version = %String::from_utf8_lossy(&deno_output.stdout).lines().next().unwrap_or("desconhecida"), "Deno disponível");
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

    fn ytdlp_args() -> Vec<String> {
        [
            "--socket-timeout",
            "15",
            "--retries",
            "2",
            "--fragment-retries",
            "2",
            "--no-playlist",
            "--js-runtimes",
            "deno",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect()
    }

    fn source_for_track(&self, track: &crate::audio::Track) -> songbird::input::Input {
        if let Some(query) = track.url.strip_prefix("search://") {
            YoutubeDl::new_search(self.http.clone(), query.to_owned())
                .user_args(Self::ytdlp_args())
                .into()
        } else {
            YoutubeDl::new(self.http.clone(), track.url.clone())
                .user_args(Self::ytdlp_args())
                .into()
        }
    }

    async fn attach_end_handler(
        &self,
        guild_id: GuildId,
        handle: &TrackHandle,
    ) -> Result<(), String> {
        let track_id = handle.uuid().to_string();
        handle
            .add_event(
                Event::Track(TrackEvent::End),
                EndHandler {
                    runtime: self.clone(),
                    guild_id,
                    track_id: track_id.clone(),
                },
            )
            .map_err(|error| format!("não foi possível registrar evento de término: {error}"))?;
        handle
            .add_event(
                Event::Track(TrackEvent::Error),
                TrackErrorHandler { guild_id, track_id },
            )
            .map_err(|error| format!("não foi possível registrar evento de erro: {error}"))
            .map(|_| ())
    }

    async fn start_track(
        &self,
        guild_id: GuildId,
        track: &crate::audio::Track,
    ) -> Result<(), String> {
        let manager = self
            .voice_manager
            .read()
            .await
            .clone()
            .ok_or_else(|| "gerenciador de voz indisponível".to_owned())?;
        let call = manager
            .get(guild_id)
            .ok_or_else(|| "chamada de voz não encontrada".to_owned())?;
        let handle = call.lock().await.play_input(self.source_for_track(track));
        self.attach_end_handler(guild_id, &handle).await?;
        if let Some(previous) = self
            .active_tracks
            .lock()
            .await
            .insert(guild_id, handle.clone())
        {
            let _ = previous.stop();
        }
        if let Err(error) = handle.make_playable_async().await {
            self.stop_active(guild_id).await;
            let session = self
                .service
                .sessions
                .get_or_create(guild_id.get(), &self.service.config)
                .await;
            session.lock().await.player.stop();
            return Err(format!("falha ao preparar o áudio: {error}"));
        }
        Ok(())
    }

    async fn play_next(&self, guild_id: GuildId) -> Result<(), String> {
        let next = {
            let session = self
                .service
                .sessions
                .get_or_create(guild_id.get(), &self.service.config)
                .await;
            let mut session = session.lock().await;
            session.player.finish_current()
        };
        let Some(track) = next else {
            self.active_tracks.lock().await.remove(&guild_id);
            return Ok(());
        };
        self.start_track(guild_id, &track).await
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
        let active = self.active_tracks.lock().await.contains_key(&guild_id);
        let tracks = self
            .service
            .enqueue(guild_id.get(), message.author.id.get(), &query)
            .await?;
        if active {
            return Ok(format!("faixa adicionada à fila: `{query}`"));
        }
        let track = {
            let session = self
                .service
                .sessions
                .get_or_create(guild_id.get(), &self.service.config)
                .await;
            let current = {
                let session = session.lock().await;
                session.player.current.clone()
            };
            current
                .or_else(|| tracks.first().cloned())
                .ok_or_else(|| "nenhuma faixa foi resolvida".to_owned())?
        };
        let voice_channel = self.voice_channel_for(ctx, message).await?;
        let manager = songbird::get(ctx)
            .await
            .ok_or_else(|| "gerenciador de voz indisponível".to_owned())?
            .clone();
        *self.voice_manager.write().await = Some(manager.clone());
        manager
            .join(guild_id, voice_channel)
            .await
            .map_err(|error| format!("não foi possível entrar no canal de voz: {error}"))?;
        self.start_track(guild_id, &track).await?;
        Ok(format!("tocando `{}`", track.title))
    }

    async fn join(&self, ctx: &Context, message: &Message) -> Result<String, String> {
        let guild_id = message
            .guild_id
            .ok_or_else(|| "este comando só funciona em um servidor".to_owned())?;
        let channel_id = self.voice_channel_for(ctx, message).await?;
        let manager = songbird::get(ctx)
            .await
            .ok_or_else(|| "gerenciador de voz indisponível".to_owned())?
            .clone();
        *self.voice_manager.write().await = Some(manager.clone());
        manager
            .join(guild_id, channel_id)
            .await
            .map_err(|error| format!("não foi possível entrar no canal de voz: {error}"))?;
        Ok("entrei no seu canal de voz".into())
    }

    async fn leave(&self, ctx: &Context, guild_id: Option<GuildId>) -> Result<String, String> {
        let id = guild_id.ok_or_else(|| "este comando só funciona em um servidor".to_owned())?;
        self.stop_active(id).await;
        let session = self
            .service
            .sessions
            .get_or_create(id.get(), &self.service.config)
            .await;
        session.lock().await.player.stop();
        let manager = songbird::get(ctx)
            .await
            .ok_or_else(|| "gerenciador de voz indisponível".to_owned())?
            .clone();
        manager
            .leave(id)
            .await
            .map_err(|error| format!("não foi possível sair do canal de voz: {error}"))?;
        Ok("saí do canal de voz".into())
    }

    async fn skip(&self, guild_id: Option<GuildId>) -> Result<String, String> {
        let id = guild_id.ok_or_else(|| "este comando só funciona em um servidor".to_owned())?;
        self.stop_active(id).await;
        let next = {
            let session = self
                .service
                .sessions
                .get_or_create(id.get(), &self.service.config)
                .await;
            let mut session = session.lock().await;
            session.player.skip()
        };
        if let Some(track) = next {
            self.start_track(id, &track).await?;
            Ok(format!("tocando a próxima: `{}`", track.title))
        } else {
            Ok("faixa pulada; a fila está vazia".into())
        }
    }

    async fn reply(&self, message: &Message, content: impl Into<String>, ctx: &Context) {
        let mut content = content.into();
        if content.chars().count() > 1900 {
            content = content.chars().take(1880).collect::<String>();
            content.push('…');
        }
        if let Err(error) = message.channel_id.say(&ctx.http, content).await {
            warn!(?error, "falha ao enviar resposta Discord");
        }
    }

    async fn dispatch(&self, ctx: &Context, message: &Message, command: Command) {
        let guild_id = message.guild_id;
        let result: Result<String, String> = match command {
            Command::Play(query) => self.play(ctx, message, query).await,
            Command::Join => self.join(ctx, message).await,
            Command::Leave => self.leave(ctx, guild_id).await,
            Command::Pause => self.control_active(guild_id, |track| track.pause(), "pausado").await,
            Command::Resume => self.control_active(guild_id, |track| track.play(), "retomado").await,
                        Command::Stop => {
                if let Some(id) = guild_id {
                    self.stop_active(id).await;
                    let _ = self.mutate_session(Some(id), |session| { session.player.stop(); Ok(String::new()) }).await;
                }
                Ok("parado".into())
            }
                        Command::Skip => self.skip(guild_id).await,
            Command::Volume(value) => self.control_active(guild_id, |track| track.set_volume(value as f32 / 100.0), &format!("volume ajustado para {value}%")).await,
            Command::Mute => self.control_active(guild_id, |track| track.set_volume(0.0), "mutado").await,
            Command::Queue => self.queue(guild_id).await,
            Command::NowPlaying => self.now_playing(guild_id).await,
            Command::Shuffle => self.mutate_session(guild_id, |session| { session.player.queue.shuffle(); Ok("fila embaralhada".into()) }).await,
            Command::Repeat(mode) => self.mutate_session(guild_id, move |session| { session.player.set_repeat(mode); Ok(format!("repeat definido para {mode:?}")) }).await,
            Command::Remove(index) => self.mutate_session(guild_id, move |session| session.player.queue.remove(index).map(|track| format!("removido `{}`", track.title)).map_err(|error| error.to_string())).await,
            Command::Clear => self.mutate_session(guild_id, |session| { session.player.queue.clear(); Ok("fila limpa".into()) }).await,
            Command::Previous => self.mutate_session(guild_id, |session| { session.player.queue.previous().map(|track| { let _ = session.player.queue.push_front(track.clone()); format!("faixa anterior recuperada: `{}`", track.title) }).ok_or_else(|| "não há histórico".into()) }).await,
            Command::Help => Ok("comandos: `!join`, `!leave`, `!play <busca/url>`, `!pause`, `!resume`, `!stop`, `!skip`, `!queue`, `!nowplaying`, `!shuffle`, `!volume <0-100>`, `!mute`, `!repeat off|track|queue`, `!remove <índice>`, `!clear`".into()),
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
    async fn ready(&self, ctx: Context, ready: Ready) {
        if let Some(manager) = songbird::get(&ctx).await {
            *self.runtime.voice_manager.write().await = Some(manager);
        }
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

struct EndHandler {
    runtime: BotRuntime,
    guild_id: GuildId,
    track_id: String,
}

#[async_trait]
impl SongbirdEventHandler for EndHandler {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        let is_current_track = matches!(
            ctx,
            EventContext::Track(tracks)
                if tracks.iter().any(|(_, handle)| handle.uuid().to_string() == self.track_id)
        );
        if !is_current_track {
            return None;
        }
        let runtime = self.runtime.clone();
        let guild_id = self.guild_id;
        tokio::spawn(async move {
            if let Err(error) = runtime.play_next(guild_id).await {
                tracing::error!(?error, ?guild_id, "falha ao iniciar próxima faixa");
            }
        });
        Some(Event::Cancel)
    }
}

struct TrackErrorHandler {
    guild_id: GuildId,
    track_id: String,
}

#[async_trait]
impl SongbirdEventHandler for TrackErrorHandler {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        let is_current_track = matches!(
            ctx,
            EventContext::Track(tracks)
                if tracks.iter().any(|(_, handle)| handle.uuid().to_string() == self.track_id)
        );
        if is_current_track {
            tracing::error!(
                guild_id = ?self.guild_id,
                track_id = %self.track_id,
                "Songbird reportou erro ao preparar ou decodificar a faixa"
            );
            Some(Event::Cancel)
        } else {
            None
        }
    }
}
