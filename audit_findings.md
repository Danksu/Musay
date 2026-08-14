# Auditoria técnica do baseline

## Evidência do baseline

| Verificação | Resultado | Classificação |
|---|---|---|
| `cargo test --all-targets` | 4 testes aprovados | Confirmado |
| `cargo check --all-targets` | Compila, com 19 warnings de código não utilizado | Confirmado |
| `cargo clippy` | Não executado porque o componente `clippy` não estava instalado no toolchain | Confirmado |
| `cargo audit` | Não disponível no ambiente | Confirmado; não equivale a ausência de vulnerabilidades |
| Gateway Discord real | Não implementado no `main.rs` | Confirmado |
| yt-dlp/FFmpeg real | Não executado pelo resolver atual | Confirmado |
| Frontend/API/banco relacional | Não existem neste repositório | Confirmado; categorias não aplicáveis ao código atual |

## P0 — crítico

Nenhum P0 confirmado no núcleo avaliado. Não foi encontrada credencial versionada, execução de shell, autenticação de usuário ou endpoint HTTP no código atual.

## P1 — alto

| ID | Achado confirmado | Impacto | Correção planejada |
|---|---|---|---|
| P1-01 | `CommandService::play` lê uma sessão, altera uma cópia e depois substitui o valor no registry. Duas chamadas simultâneas para a mesma guild podem perder uma faixa ou atualização. | Race lógica e perda de trabalho sob concorrência. | Executar mutações dentro de um lock por guild ou usar `Arc<Mutex<GuildSession>>` no registry. |
| P1-02 | `Player::start_next` não implementa o comportamento de `RepeatMode::Queue`; `RepeatMode::Track` e `skip` também têm semântica incorreta após zerar `current`. | Reprodução incorreta, fila que termina antes do esperado e histórico inconsistente. | Modelar finalização/skip separadamente e reencaminhar a faixa conforme o modo. |
| P1-03 | O container executa como root e o Dockerfile não habilita o transporte Discord real. | Hardening insuficiente e artefato de deploy que aparenta estar pronto sem estar funcional. | Usuário não-root, imagem mínima, documentação explícita e profile de build coerente. |

## P2 — médio

| ID | Achado confirmado | Impacto | Correção planejada |
|---|---|---|---|
| P2-01 | `BasicResolver` aceita qualquer URL sintaticamente válida e não limita comprimento de consulta. | Superfície de SSRF/abuso quando um downloader real for acoplado; consumo excessivo de recursos. | Limite de entrada, allowlist de esquemas e hosts conhecidos; rejeitar credenciais, IPs privados e esquemas não HTTP(S). |
| P2-02 | `Config` aceita valores sem validação semântica: fila zero, prefixo vazio, caminho inseguro e duração zero. | Configuração silenciosamente inválida. | Erros específicos, limites e validação fail-closed. |
| P2-03 | `JsonStore` não valida tamanho do arquivo antes de desserializar e não protege explicitamente contra symlink/path inesperado. | Exaustão de memória e escrita em local não pretendido em instalações inseguras. | Limite de bytes, rejeição de symlink quando aplicável e diretório configurável validado. |
| P2-04 | Erros de configuração em `main` apenas imprimem mensagem e retornam código de sucesso. | Orquestrador pode considerar o serviço saudável apesar de falha de inicialização. | `exit(2)` e logs estruturados sem token. |
| P2-05 | O registry tem um lock global e clona sessões inteiras; a abordagem atual é correta apenas para baixa concorrência e ainda perde atualizações. | Contenção e inconsistência. | Lock por guild com API de mutação serializada. |
| P2-06 | O manifesto contém dependências opcionais Discord/Songbird não usadas por nenhum módulo. | Falsa percepção de integração e custo de supply chain/build quando feature é ativada. | Manter explicitamente como plano de integração ou implementar adapter; não afirmar que o bot está conectado. |

## P3 — baixo

| ID | Achado confirmado | Correção planejada |
|---|---|---|
| P3-01 | Cobertura concentra-se no happy path e não testa limite de fila, entradas inválidas, repeat, concorrência, persistência corrompida e shutdown. | Adicionar testes comportamentais e de invariantes. |
| P3-02 | Warnings de dead code ocultam qualidade e tornam regressões menos visíveis. | Expor biblioteca/core e usar testes de integração, ou aplicar atributos somente onde o adapter é planejado. |
| P3-03 | Não há CI para format, check, test e auditoria de dependências. | Adicionar workflow sem alegar que auditoria de advisories é concluída localmente. |

## Áreas não aplicáveis ou não confirmadas

Não há frontend, API HTTP, sessão web, cookies, banco SQL, migrations, filas externas, uploads, templates ou autenticação própria neste baseline. Também não foi possível afirmar ausência de vulnerabilidades de dependências porque `cargo-audit` não estava instalado; a análise do lockfile foi estrutural, não uma consulta de advisories.
