# Relatório de auditoria técnica e hardening

## Escopo e método

A auditoria foi conduzida sobre o repositório Rust atual do Musay, seguindo os fluxos executáveis e os artefatos de build, sem inferir capacidades externas não verificadas. Após a auditoria inicial, foi implementado um runtime real de gateway Discord/voz, que foi compilado e testado estaticamente; a conexão contra uma conta Discord real não foi executada neste ambiente. Foram executados `cargo test --all-targets`, `cargo check --all-targets`, `cargo fmt --all` e inspeções estáticas dos módulos, manifesto, lockfile, Dockerfile e configuração. O componente Clippy não estava instalado no ambiente inicial; por isso, a ausência de resultado local de Clippy não é tratada como evidência de qualidade nem de segurança.

## Status verificado

| Área | Evidência | Status |
|---|---|---|
| Compilação | `cargo check --all-targets` executado após o primeiro conjunto de correções | Confirmado |
| Testes | 8 testes comportamentais passaram no conjunto corrigido | Confirmado |
| Formatação | `cargo fmt --all -- --check` executado | Confirmado |
| Clippy | `cargo clippy --all-targets -- -D warnings` executado sem erros | Confirmado |
| Advisories | `cargo audit` v0.22.2 executado; 10 vulnerabilidades transitivas e 4 avisos de manutenção encontrados | Falha upstream documentada; não mascarada |
| Credenciais versionadas | Nenhuma encontrada na inspeção do repositório | Confirmado para o snapshot auditado |
| Runtime Discord | `cargo check`, `cargo test --all-targets` e `cargo build --release` executados após a integração Serenity/Songbird | Confirmado em compilação; conexão real depende de token, intents e ambiente externo |

## Correções aplicadas no primeiro conjunto

A implementação agora serializa mutações por guild com `Arc<Mutex<GuildSession>>`, evitando a perda de atualizações causada pelo padrão anterior de clonar e substituir sessões. O player distingue início, término e skip e implementa repetição de faixa e de fila de modo observável nos testes. O resolver rejeita entradas vazias, longas ou com caracteres de controle, esquemas que não sejam HTTP(S), credenciais embutidas e endereços IP locais/privados conhecidos.

A configuração passou a validar prefixo, volume, tamanho máximo da fila, timeout de saída e caminho relativo sem traversal. Falhas de configuração agora terminam com código de erro, inclusive fora do modo self-check. A persistência JSON impõe limite de 16 MiB, rejeita symlinks na leitura e usa escrita temporária seguida de rename. Dependências não utilizadas foram removidas do manifesto.

## Riscos residuais

A proteção contra SSRF é uma mitigação de entrada, não uma garantia completa contra DNS rebinding ou redirecionamentos perigosos; o resolver de produção deve manter política de rede no cliente HTTP, limitar redirects e executar em sandbox quando necessário. A persistência JSON ainda depende de permissões corretas do diretório e não oferece lock entre processos. O runtime Discord/Songbird está compilado, mas a conexão real, permissões do servidor, existência de `yt-dlp`/FFmpeg e reprodução contra uma fonte externa não foram exercitadas neste ambiente.

Não foi possível concluir uma auditoria de advisories sem `cargo-audit`. O workflow adiciona formatação, check, testes e Clippy, mas a verificação de vulnerabilidades de dependências deve ser executada separadamente em CI quando o time escolher e fixar a ferramenta de advisories.

## Distribuição e operação

O fluxo final de operação é local: o executável pede o token interativamente, procura `yt-dlp` ao lado do binário ou no `PATH`, avisa sobre FFmpeg ausente e conecta ao Discord. O Dockerfile foi removido do projeto para não sugerir container como requisito do usuário final. Os scripts `scripts/package-local.sh` e `scripts/package-local.ps1` geram pastas distribuíveis por plataforma.

## Próximas ações prioritárias

A próxima grande entrega deve adicionar testes de integração com fakes de voice, reconexão com backoff limitado, sandbox de FFmpeg e observabilidade de falhas. Depois disso, devem ser adicionados testes de restart/persistência, comandos slash e uma estratégia upstream para reduzir os advisories transitivos do stack Songbird/OpenMLS.

## Auditoria de advisories atualizada

O `cargo-audit` v0.22.2 foi instalado e executado contra o lockfile atual. O resultado encontrou **10 vulnerabilidades transitivas** e **4 avisos de manutenção**. A maioria entra pelo stack opcional de voz do Songbird 0.6.0, especialmente `davey`, `openmls_rust_crypto`, `hpke-rs`, `libcrux-*`, `derivative` e `instant`. O lockfile também contém `rustls-webpki` 0.102.8 em uma cadeia antiga, enquanto outra cadeia já usa 0.103.14.

Esses advisories não foram ocultados nem marcados como resolvidos artificialmente. A correção estrutural exige atualizar o Songbird/Serenity ou substituir componentes transitivos do stack de voz; forçar versões incompatíveis com `cargo update --precise` poderia quebrar a compatibilidade e gerar uma falsa sensação de segurança. A release deve carregar essa limitação no relatório até que a cadeia upstream seja atualizada e o `cargo audit` passe.

| Categoria | Evidência | Ação nesta entrega |
|---|---|---|
| `libcrux-secrets`, `libcrux-sha3`, `libcrux-aesgcm` | Introduzidos por `davey -> openmls_rust_crypto -> hpke-rs` dentro do Songbird | Registrados como risco transitivo; não usar `--ignore` para mascarar |
| `rustls-webpki` 0.102.8 | Cadeia antiga de TLS em dependência transitiva | Manter a versão exigida pelo upstream até atualização compatível |
| `derivative`, `instant`, `proc-macro-error2` | Dependências transitivas não mantidas | Monitorar atualização do Songbird/Serenity |
| Ferramenta de advisories | `cargo audit` executado localmente | CI deve executar o mesmo comando e falhar em novas vulnerabilidades não revisadas |

A auditoria de advisories é, portanto, **concluída com falhas upstream documentadas**, não aprovada. O binário não deve ser anunciado como livre de vulnerabilidades enquanto essa cadeia permanecer no lockfile.
