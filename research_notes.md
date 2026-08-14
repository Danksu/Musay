# Notas de engenharia reversa — fase inicial

## ZerioDev/Music-bot

Observado no repositório e README: projeto JavaScript/Node.js com diretórios `commands`, `events`, `buttons`, além de `config.js`, `loader.js`, `main.js` e `process_tools.js`. A configuração usa `discord-player`/extrator do YouTube, FFmpeg, ponte Spotify, volume máximo e padrão, loop, mensagens extras, papel DJ opcional, saída por ausência de usuários e saída ao terminar. O README informa suporte multilíngue e controles por botões como voltar, pular, pausar/retomar, salvar faixa, volume e loop. Licença indicada: GPL-3.0. A página web também confirma a organização e as opções de configuração.

## ItzSudhan/Discord-MusicBot

Observado no repositório e README: projeto JavaScript/TypeScript, com `commands`, `events`, `lib`, `util`, `api` e `dashboard`; usa arquitetura baseada em Lavalink e inclui suporte a Spotify, SoundCloud e YouTube, shuffling, controle de volume, slash commands e dashboard web. O repositório está marcado como arquivado/read-only. A árvore contém módulos explícitos para `EpicPlayer`, logger, slash commands, banco/configuração de guild, carregamento de comandos e obtenção de Lavalink. A página lista JavaScript como linguagem dominante, com HTML e TypeScript. Um histórico de alteração documenta correções de reconexão do bot/Lavalink, reentrada no canal quando expulso e tratamento de falhas.

## Diretrizes de análise

As capacidades efetivamente observadas devem ser separadas das inferidas e das propostas para Rust. Os quatro repositórios serão examinados por código, manifests, configurações, licenças, comandos, eventos, filas, fontes e ciclo de vida de sessões por guild.

## Fontes consultadas

1. https://github.com/ZerioDev/Music-bot
2. https://github.com/ItzSudhan/Discord-MusicBot

## Just-Some-Bots/MusicBot

Observado no repositório e README: projeto Python 3.8+ baseado em `discord.py`, com diretório `musicbot` contendo bot, configuração, downloader, entry, player, playlist, permissões, cache de arquivos, Spotify, JSON e utilitários. O fluxo documentado é `play <url>` → download/processamento → reprodução em voice; suporta múltiplos servidores, permissões, mídia ao vivo experimental e uma lista/autoplaylist quando a fila está vazia. O código e histórico indicam busca, URLs diretas, YouTube e outros serviços via yt-dlp, playlists múltiplas, mídia local, seek, velocidade, shuffle, repeat, round-robin, fila persistente, cache com limites, blocklists por usuário/faixa, prefixo por guild, timers de inatividade, reconexão/rejoin, tratamento de StageChannel, logs, reinício e shutdown gracioso. Há `Dockerfile`, `docker-compose.example.yml`, scripts de instalação/execução e exemplos de configuração/permissões. Licença deve ser conferida diretamente no arquivo do repositório.

## Fontes consultadas

3. https://github.com/Just-Some-Bots/MusicBot
