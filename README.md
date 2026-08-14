# Musay

**Musay** é uma implementação original em Rust de um núcleo de bot de música para Discord, projetada a partir da análise comparativa de quatro projetos públicos: ZerioDev/Music-bot, ItzSudhan/Discord-MusicBot, Just-Some-Bots/MusicBot e jagrosh/MusicBot. O código não copia trechos dos projetos analisados; ele reimplementa os comportamentos em módulos próprios e mantém separadas as capacidades observadas, as inferências arquiteturais e as extensões propostas.

## Resumo executivo da engenharia reversa

| Projeto | Linguagem e arquitetura | Capacidades observadas no código ou documentação | Principal contribuição para Musay |
|---|---|---|---|
| ZerioDev/Music-bot | Node.js, discord.js, discord-player, eventos e botões separados | YouTube, ponte Spotify, volume, loop, shuffle, histórico, seek, letras, fila, saída por canal vazio/fim e componentes interativos | Modelo simples de eventos do player, controles por botões e configuração explícita de timeouts |
| ItzSudhan/Discord-MusicBot | JavaScript/TypeScript, Lavalink, player próprio, dashboard e banco/configuração por guild | YouTube, Spotify, SoundCloud, slash commands, dashboard, volume, shuffle, loop, reconexão e tratamento de falhas do nó | Separação entre bot, player, utilitários, persistência e painel; referência para múltiplas guilds |
| Just-Some-Bots/MusicBot | Python, discord.py, downloader, player, playlist, cache, permissões e configuração INI | YouTube e outros serviços via yt-dlp, streams ao vivo, mídia local, playlists, seek, velocidade, shuffle, repeat, round-robin, fila persistente, cache, blocklists, prefixo por guild, rejoin e shutdown | Maior cobertura operacional: persistência, cache limitado, permissões detalhadas, timers de inatividade e recuperação |
| jagrosh/MusicBot | Java, JDA, LavaPlayer, comandos e gerenciador de áudio | YouTube, SoundCloud, Bandcamp, Vimeo, Twitch, arquivos locais, HTTP, rádio e múltiplos formatos/codecs | Modelo maduro de resolver fontes e delegar decodificação/transcodificação a uma biblioteca de áudio especializada |

O fluxo comum reconstruído é `comando Discord → validação de guild/permissão → resolução da fonte → fila por guild → player → decoder/transcodificador → conexão de voz`. As quatro implementações isolam, em graus diferentes, o estado de cada servidor. As diferenças mais importantes são o uso de `discord-player` diretamente no primeiro projeto, Lavalink no segundo, download/cache local no terceiro e LavaPlayer no quarto.

## Matriz comparativa consolidada

| Recurso | ZerioDev | ItzSudhan | Just-Some-Bots | jagrosh | Decisão em Rust |
|---|---|---|---|---|---|
| Reprodução | Sim, discord-player | Sim, Lavalink | Sim, player próprio + yt-dlp | Sim, LavaPlayer | `Player` independente do transporte |
| Fila | Sim | Sim | Sim, com persistência e round-robin | Sim | `TrackQueue` com limite, mover, remover, limpar e shuffle |
| Playlists | Spotify e playlists do player | Spotify e fontes Lavalink | Autoplaylist, múltiplas playlists e arquivos locais | Playlists suportadas pelo LavaPlayer | `SavedPlaylist` persistida e resolver extensível |
| Busca | Sim | Sim | Sim | Suportada por fontes | `AudioSource::resolve`, com busca real delegável ao yt-dlp |
| URL direta/streams | Parcial/por extrator | Por Lavalink | Sim, inclusive live media experimental | Sim, HTTP, Twitch e rádio | `SourceKind::{DirectUrl,Radio,LocalFile}` |
| Pause/resume/skip | Sim | Sim | Sim, incluindo autopause | Sim | Estados fortes `Playing`, `Paused`, `Stopped`, `Recovering` |
| Volume/mute | Volume | Volume | Volume e configurações | Volume | Volume limitado a 0–100 e mute explícito |
| Shuffle/repeat/history | Sim | Sim | Sim, repeat e histórico de player | Sim | Repeat `Off/Track/Queue`, histórico limitado e shuffle determinístico por operação |
| Permissões | DJ opcional | Configuração/utilitários | Sistema detalhado e blocklists | Configuração própria | Política por usuário, canal, cargo e faixa |
| Inatividade/saída | Canal vazio e fim | Eventos de player | Timers por canal/player | Gerência de áudio | Campo de configuração e lifecycle da sessão |
| Reconexão/recuperação | Eventos de erro | Reconexão de Lavalink | Rejoin, retry, autopause e shutdown | Delegada às bibliotecas | Estado `Recovering`, retries no adaptador de transporte |
| Persistência/cache | Configuração | Banco/utilitários | JSON, fila, autoplaylist e cache | Arquivos/configuração | Abstração `JsonStore`, substituível por SQLx/SQLite |
| Interface Discord | Slash, prefixo, botões | Slash, dashboard | Prefixo e comandos | Comandos | Parser neutro; adapter Serenity/Songbird opcional |

## Arquitetura Rust

A organização proposta é deliberadamente orientada ao domínio. `audio` contém entidades e regras de reprodução sem depender do Discord. `guild` mantém uma sessão isolada por servidor. `permissions` concentra políticas. `persistence` define armazenamento substituível. `discord` traduz comandos e será o ponto de integração com Serenity/Songbird. Essa separação permite testar fila, estados e permissões sem token, gateway ou canal de voz reais.

```text
Discord gateway / slash commands
              |
       command adapter
              |
       CommandService
              |
    SessionRegistry<u64, GuildSession>
              |
       Player + TrackQueue
              |
   AudioSource / resolver chain
              |
    yt-dlp + FFmpeg / Songbird
              |
        Discord Voice
```

O projeto já implementa de forma real o núcleo dessa cadeia: `Track`, `TrackQueue`, `Player`, repeat, shuffle, histórico, seek, volume, mute, parser de comandos, resolver inicial, política de permissões, registro concorrente de sessões e persistência JSON atômica. O transporte Discord é mantido como feature opcional para que o núcleo compile e seja testado sem credenciais.

## Dependências escolhidas

| Função | Biblioteca | Justificativa |
|---|---|---|
| API Discord | Serenity 0.12.5 | Biblioteca Rust madura para gateway, HTTP, eventos e modelos Discord [5] |
| Voz Discord | Songbird 0.6.0 | Biblioteca assíncrona de voice compatível com Serenity, com suporte a fontes e Opus [6] |
| Resolução | yt-dlp via processo controlado no adapter | Maior cobertura de sites e formatos; evita acoplar o player a um provedor |
| Transcodificação | FFmpeg via Songbird/adapter | Suporte prático a Opus e múltiplos formatos; deve ser instalado no host |
| Async/concurrency | Tokio | Runtime assíncrono, canais, sinais e tarefas controladas |
| Persistência inicial | JSON atômico | Zero infraestrutura para a primeira versão; interface pode ser trocada por SQLite/SQLx |
| Configuração | dotenvy e Serde | Configuração externa e tipos fortes |
| Observabilidade | tracing/tracing-subscriber | Logs estruturados e filtros por ambiente |

As versões foram verificadas no registro de crates em 14 de agosto de 2026. Serenity descreve a integração Discord e aponta Songbird para voz [5]; Songbird se define como biblioteca assíncrona de voice em Rust [6]. O projeto jagrosh documenta a amplitude de fontes e formatos do LavaPlayer [4], enquanto o MusicBot Python documenta o fluxo de download/processamento/reprodução e configuração por `options.ini` [3].

## Comandos suportados pelo núcleo

O parser aceita prefixo configurável e os comandos `play`, `pause`, `resume`, `stop`, `skip`, `previous`, `shuffle`, `queue`, `nowplaying`, `volume`, `mute`, `repeat off|track|queue`, `remove`, `clear` e `help`. A camada de sessão fornece a base para slash commands equivalentes e autocomplete no adaptador Discord.

## Compilação e testes

Instale Rust estável atual, FFmpeg e, para voz, bibliotecas Opus do sistema. Depois execute:

```bash
cargo test
cargo run -- --self-check
cargo run
```

O comando `--self-check` não exige token. Para execução conectada, crie `.env`:

```dotenv
DISCORD_TOKEN=seu_token
COMMAND_PREFIX=!
DEFAULT_VOLUME=75
MAX_QUEUE_SIZE=100
LEAVE_ON_EMPTY_SECS=300
DATABASE_PATH=musay.json
```

O bot precisa das permissões Discord correspondentes a ver canais, enviar mensagens, conectar e falar em canais de voz. A feature de transporte deve ser finalizada/ativada no ambiente de implantação com a configuração do gateway, intents mínimos e registro de comandos.

## Auditoria e CI

O relatório técnico com achados confirmados, prioridades, correções, riscos residuais e evidências está em [`AUDIT_REPORT.md`](AUDIT_REPORT.md), enquanto o inventário priorizado do baseline está em [`audit_findings.md`](audit_findings.md). O workflow [`CI`](.github/workflows/ci.yml) verifica formatação, compilação, testes e Clippy em cada push e pull request. A auditoria de advisories de dependências ainda depende da instalação de uma ferramenta específica, portanto não é declarada como concluída.

## Licenças e atribuição

Os repositórios analisados permanecem apenas como referências externas. O código de Musay é original e não incorpora código-fonte literal deles. As licenças observadas são GPL-3.0 em ZerioDev, licença própria a conferir no repositório arquivado de ItzSudhan, licença do projeto Python a conferir no arquivo distribuído e Apache-2.0 em jagrosh. Nenhum código desses repositórios foi copiado para Musay.

## Limitações conhecidas

A versão atual implementa o núcleo funcional e uma API neutra de comandos, mas ainda não conecta o gateway Serenity nem instancia o handler Songbird em `main.rs`. O `BasicResolver` valida entradas e cria metadados, porém não executa yt-dlp. A persistência inicial é JSON, adequada para uma instalação simples, mas não substitui uma base SQLite em cenários de alta concorrência. Também faltam dashboard web, letras sincronizadas, Spotify OAuth, métricas Prometheus e integração de componentes Discord.

Essas limitações são intencionais e explícitas: o núcleo está compilável e testado; o próximo incremento deve implementar o adapter de transporte e o resolver de produção, sem alterar `Player`, `TrackQueue` ou a API `AudioSource`.

## Melhorias futuras

A próxima etapa recomendada é adicionar `DiscordAdapter` com Serenity/Songbird, uma cadeia `YtDlpResolver`/`DirectUrlResolver`/`LocalFileResolver`, worker de FFmpeg com timeout e cancelamento, SQLite via SQLx, comandos slash registrados por guild, embeds e botões, reconexão com backoff, métricas e testes de integração com fakes de voice. A arquitetura atual foi desenhada para essas extensões sem reescrever o player.

## Referências

[1]: https://github.com/ZerioDev/Music-bot "ZerioDev/Music-bot"

[2]: https://github.com/ItzSudhan/Discord-MusicBot "ItzSudhan/Discord-MusicBot"

[3]: https://github.com/Just-Some-Bots/MusicBot "Just-Some-Bots/MusicBot"

[4]: https://github.com/jagrosh/MusicBot "jagrosh/MusicBot"

[5]: https://crates.io/crates/serenity "Serenity no crates.io"

[6]: https://github.com/serenity-rs/songbird "Songbird no GitHub"
