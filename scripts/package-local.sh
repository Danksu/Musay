#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
TARGET=${1:-"$ROOT/dist/musay-local"}
mkdir -p "$TARGET"
cd "$ROOT"

cargo build --release
BIN="$ROOT/target/release/musay"
cp "$BIN" "$TARGET/musay"
cp README.md "$TARGET/README.md"
cp .env.example "$TARGET/.env.example"

if command -v yt-dlp >/dev/null 2>&1; then cp "$(command -v yt-dlp)" "$TARGET/yt-dlp"; chmod +x "$TARGET/yt-dlp"; fi
if command -v ffmpeg >/dev/null 2>&1; then cp "$(command -v ffmpeg)" "$TARGET/ffmpeg"; chmod +x "$TARGET/ffmpeg"; fi

cat > "$TARGET/COMO-EXECUTAR.txt" <<'EOF'
1. Instale o pacote do bot no servidor Discord e habilite Message Content Intent.
2. Coloque yt-dlp e, se necessário, ffmpeg nesta mesma pasta ou instale-os no PATH.
3. Execute ./musay.
4. Digite o token quando solicitado; a entrada é oculta e não é salva.
5. Pressione Ctrl+C para encerrar.
EOF

echo "Pacote criado em: $TARGET"
