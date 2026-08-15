# Pesquisa de diagnóstico de reprodução — 2026-08-15

## Songbird oficial

Fonte: https://github.com/serenity-rs/songbird

A documentação oficial descreve o Songbird como um driver de voz que inclui eventos, filas, seeking e conversão de fontes para Opus. O repositório mostra exemplos e código da série 0.6.0. Um ponto relevante é que a criação de uma fonte YoutubeDl é lazy: a resolução e a abertura do stream acontecem quando o track é promovido para reprodução. Portanto, entrar no canal não comprova que `yt-dlp` conseguiu resolver a URL nem que o stream foi decodificado.

O código local do Songbird confirma que `YoutubeDl::query` executa o programa `yt-dlp`, acrescenta `-j`, seleciona `ba[abr>0][vcodec=none]/best` e usa `--no-playlist`. Erros de ausência do executável, status de saída diferente de zero e JSON inválido são retornados como `AudioStreamError`. O `TrackHandle::make_playable_async` existe para forçar a inicialização lazy e expor o erro antes de considerar a faixa tocando. O runtime anterior apenas chamava `play_input` e não aguardava esse callback, deixando falhas de resolução silenciosas.

O Songbird também possui `TrackEvent::Error`; o runtime anterior só registrava `TrackEvent::End`. Isso explica um cenário em que o bot entra na chamada, o processo da fonte falha e o usuário não recebe diagnóstico.

## yt-dlp oficial

Fonte: https://github.com/yt-dlp/yt-dlp

A documentação confirma que o binário standalone é uma forma oficial de instalação. `--no-playlist` limita uma URL que aponta para vídeo e playlist ao vídeo individual. A seleção de áudio feita pelo Songbird é `ba[abr>0][vcodec=none]/best`, isto é, prioriza uma faixa somente de áudio e usa o melhor fallback disponível.

Causas que precisam ser verificadas no runtime: `yt-dlp` ausente ou não encontrado no PATH do processo, bloqueio/alteração da plataforma de origem, falha de extração com status não zero, saída JSON vazia ou inválida, URL de pesquisa não resolvida, stream com protocolo incompatível, FFmpeg ausente quando a fonte exige conversão e erro Opus/voice sem telemetria.

## Hipóteses prioritárias

1. O runtime não chama `make_playable_async`, então o comando responde e o bot entra no canal antes de a resolução realmente ser validada.
2. O runtime não registra `TrackEvent::Error`, portanto a falha aparece como silêncio.
3. O fluxo não envia stderr/status do yt-dlp para o usuário e não distingue erro de resolução de erro de transporte.
4. O preflight valida apenas `yt-dlp --version`, não a capacidade de resolver uma fonte de áudio real.
5. A ausência de FFmpeg é apenas aviso, embora algumas fontes possam depender dele.
6. O pacote Windows foi compilado para GNU/MinGW e deve ser executado em Windows para validar DLLs, rede, firewall, Opus e console; a compilação PE por si só não valida reprodução.

## Reprodução controlada

Com o pacote anterior, `yt-dlp 2026.07.04` falhou em busca e URL do YouTube com `No supported JavaScript runtime could be found`, além de HTTP 429/anti-bot. Isso confirmou que o pacote Windows/Linux anterior estava incompleto para o yt-dlp atual: incluía `yt-dlp`, mas não Deno.

O Deno oficial 2.9.5 foi colocado no PATH temporário e o aviso de runtime JavaScript desapareceu. As tentativas de YouTube restantes retornaram `Video unavailable` ou `Sign in to confirm you're not a bot`, falhas externas da plataforma e não ausência de Deno. A mesma configuração conseguiu resolver uma fonte direta MP3 pública (`https://www.soundhelix.com/examples/mp3/SoundHelix-Song-1.mp3`) em JSON com formato `audio only` e protocolo HTTPS.

A correção aplicada ao runtime exige Deno no preflight, passa `--js-runtimes deno` ao yt-dlp, inclui Deno nos pacotes e chama `TrackHandle::make_playable_async` antes de confirmar que a faixa está tocando. Também foi registrado `TrackEvent::Error` para que erros de parsing/decoding posteriores não fiquem silenciosos.
