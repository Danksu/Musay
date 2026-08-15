# Musay v0.1.0 — release local otimizada

Esta release foi gerada para execução local como bot Discord. O executável solicita o token de forma oculta, procura `yt-dlp` ao lado do binário ou no `PATH`, conecta Serenity/Songbird, responde aos comandos prefixados e encerra com `Ctrl+C`.

## Artefato gerado

O build executado foi `cargo build --release` com o perfil otimizado do projeto: `opt-level = 3`, `lto = "thin"`, `codegen-units = 1`, `panic = "abort"`, símbolos removidos, debug desativado e compilação incremental desativada. O executável Linux x86_64 gerado tem aproximadamente 15 MiB e foi confirmado como ELF stripped. O pacote Linux desta sessão inclui também o binário standalone oficial do yt-dlp versão 2026.07.04; FFmpeg continua recomendado para compatibilidade ampliada de formatos.

## Validações

Antes do release foram executados `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all-targets`, `cargo check --all-features`, `cargo run -- --self-check` e `git diff --check`. A suíte terminou com **8 testes aprovados e 0 falhos**. O executável release também passou em `--self-check`.

## Uso

Execute `./musay` no Linux, `musay.exe` no Windows ou o binário correspondente no macOS. Digite o token quando solicitado. Para tocar música, distribua `yt-dlp` junto do executável ou instale-o no `PATH`; FFmpeg é recomendado para ampliar a compatibilidade de formatos. Configure previamente o bot no Discord Developer Portal, habilite Message Content Intent, convide-o ao servidor e conceda permissões de mensagem e voz.

## Limitações conhecidas

A conexão real contra uma conta Discord e uma fonte de áudio externa não foi exercitada neste ambiente por depender de token e rede do usuário. O avanço automático de fila está implementado via evento de término Songbird, mas testes end-to-end de voz ainda são uma etapa futura. Dashboard, comandos slash, SQLite, métricas e reconexão com backoff ainda não fazem parte desta release.

O `cargo-audit` v0.22.2 encontrou 10 vulnerabilidades transitivas e 4 avisos de manutenção, principalmente na cadeia de voz Songbird/OpenMLS/libcrux e em versões antigas de rustls-webpki. Esses advisories estão documentados em `AUDIT_REPORT.md` e não foram mascarados. A release é funcional, mas não deve ser anunciada como livre de vulnerabilidades até a atualização compatível do upstream.
