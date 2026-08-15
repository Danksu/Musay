# Distribuição local plug and play

O Musay é distribuído como um executável local. **Docker não faz parte do fluxo do usuário final**. O usuário recebe uma pasta, executa `musay` no Linux/macOS ou `musay.exe` no Windows, informa o token quando solicitado e o bot conecta ao Discord.

## O que precisa ser configurado uma única vez

No [Discord Developer Portal](https://discord.com/developers/applications), crie a aplicação, crie o bot, copie o token, habilite **Message Content Intent** e gere um convite OAuth2 com os escopos `bot` e, se desejado, `applications.commands`. As permissões mínimas para os comandos atuais são visualizar canais, ler histórico, enviar mensagens, conectar e falar em canais de voz. O bot também precisa estar presente no servidor em que será usado.

O token não deve ser colocado no pacote, no `.env`, em scripts ou em repositórios. O programa solicita o token com entrada oculta em cada execução. Se o token estiver errado, expirado ou revogado, o Discord recusará a conexão e o programa exibirá o erro sem registrar o segredo.

## Conteúdo recomendado do pacote

| Arquivo | Obrigatório | Função |
|---|---:|---|
| `musay` ou `musay.exe` | Sim | Executável do bot |
| `yt-dlp` ou `yt-dlp.exe` | Sim para `!play` | Busca e resolução de fontes |
| `deno` ou `deno.exe` | Sim para YouTube atual | Runtime JavaScript usado pelo yt-dlp/EJS |
| `ffmpeg` ou `ffmpeg.exe` | Recomendado | Compatibilidade com formatos que exigem transcodificação |
| `README.md` | Recomendado | Manual geral |
| `.env.example` | Opcional | Ajustes operacionais sem token |
| `COMO-EXECUTAR.txt` | Recomendado | Instruções rápidas |

O runtime procura `yt-dlp`, `deno` e `ffmpeg` primeiro no diretório do executável e também aceita ferramentas disponíveis no `PATH`. A ausência de `yt-dlp` ou Deno impede a inicialização normal com uma mensagem clara. A ausência de FFmpeg gera um aviso e pode limitar alguns formatos.

## Empacotamento

No Linux ou macOS, com Rust instalado no computador de build:

```bash
chmod +x scripts/package-local.sh
./scripts/package-local.sh
```

O resultado fica em `dist/musay-local`. O script compila em modo release e copia o executável, o README, o exemplo de configuração e os binários auxiliares encontrados no `PATH`.

No Windows PowerShell:

```powershell
.\scripts\package-local.ps1
```

O empacotador deve ser executado em uma máquina compatível com o sistema operacional alvo. Um binário Linux não é executável nativamente no Windows, e o mesmo vale para Windows e macOS. Para distribuir para os três sistemas, gere um pacote em cada plataforma ou use builds cross-platform validados separadamente.

## Execução pelo usuário final

No Windows, dê duplo clique em `musay.exe` ou abra o PowerShell na pasta e execute `./musay.exe`. No Linux/macOS, execute `./musay`. O terminal solicitará:

```text
Token do bot Discord (entrada oculta):
```

Após a autenticação, a janela permanece aberta enquanto o bot estiver conectado. O usuário pode usar `!help`, `!play <busca ou URL>`, `!pause`, `!resume`, `!stop`, `!skip`, `!queue`, `!nowplaying`, `!volume <0-100>`, `!mute`, `!repeat off|track|queue`, `!shuffle`, `!remove <índice>` e `!clear`. Para finalizar, pressione `Ctrl+C` no terminal. O runtime encerra os shards Discord de forma graciosa.

O executável não consegue corrigir uma configuração inválida no Developer Portal, uma permissão ausente, um token incorreto ou um bot que não foi convidado ao servidor. Nesses casos, a mensagem de erro indica a etapa que precisa ser corrigida.

## Segurança operacional

Nunca envie o token por chat, não o coloque em screenshots e não o salve em arquivos compartilhados. Se ele vazar, revogue-o e gere outro no Developer Portal. O pacote não precisa de acesso administrativo ao computador e não deve ser executado como administrador/root.
