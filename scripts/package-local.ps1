$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$Target = if ($args.Count -gt 0) { $args[0] } else { Join-Path $Root "dist\musay-local" }
New-Item -ItemType Directory -Force -Path $Target | Out-Null
Set-Location $Root

cargo build --release
Copy-Item "$Root\target\release\musay.exe" "$Target\musay.exe" -Force
Copy-Item "$Root\README.md" "$Target\README.md" -Force
Copy-Item "$Root\.env.example" "$Target\.env.example" -Force

$YtDlp = Get-Command yt-dlp.exe -ErrorAction SilentlyContinue
if ($YtDlp) { Copy-Item $YtDlp.Source "$Target\yt-dlp.exe" -Force }
$Deno = Get-Command deno.exe -ErrorAction SilentlyContinue
if ($Deno) { Copy-Item $Deno.Source "$Target\deno.exe" -Force }
$Ffmpeg = Get-Command ffmpeg.exe -ErrorAction SilentlyContinue
if ($Ffmpeg) { Copy-Item $Ffmpeg.Source "$Target\ffmpeg.exe" -Force }

@"
1. Configure o bot no Discord Developer Portal e habilite Message Content Intent.
2. Coloque yt-dlp.exe e deno.exe nesta mesma pasta ou no PATH; FFmpeg é recomendado.
3. Execute musay.exe.
4. Digite o token quando solicitado; a entrada é oculta e não é salva.
5. Pressione Ctrl+C para encerrar.
"@ | Set-Content (Join-Path $Target "COMO-EXECUTAR.txt")
Write-Host "Pacote criado em: $Target"
