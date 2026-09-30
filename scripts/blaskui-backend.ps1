#Requires -Version 5.1
<#
.SYNOPSIS
  Avvio pulito e SINGOLO del backend BlaskUI in detached. Esce subito con esito.
  Evita istanze sovrapposte e i timeout del tool: kill orfani -> avvio detached
  (Start-Process senza -NoNewWindow) -> poll /health con timeout breve -> output
  pulito e uscita immediata.
.PARAMETER Root
  Radice del repo (default: cartella di questo script).
.PARAMETER Port
  Porta locale (default: BLASKUI_PORT oppure free, stub 60710).
.PARAMETER TimeoutSec
  Timeout attesa /health (default 45).
#>
param(
    [string]$Root = 'C:\Users\blasc\Desktop\OPENBLA\open-webui-main\open-webui-main',
    [int]$Port = 0,
    [int]$TimeoutSec = 45
)
$ErrorActionPreference = 'SilentlyContinue'

$backendDir = Join-Path $Root 'backend'
$venvPython = Join-Path $Root '.venv-blaskui\Scripts\python.exe'
if (-not (Test-Path $venvPython)) { $venvPython = Join-Path $Root '.venv-blaskui\bin\python' }
$dataDir = 'C:\Users\blasc\Desktop\OPENBLA\open-webui-main\open-webui-main\backend\data-blaskui'
if ($env:BLASKUI_DATA_DIR) { $dataDir = $env:BLASKUI_DATA_DIR }

$out = Join-Path $env:TEMP 'blaskui-backend.out'
$err = Join-Path $env:TEMP 'blaskui-backend.err'

if ($Port -eq 0) { $Port = if ($env:BLASKUI_PORT) { [int]$env:BLASKUI_PORT } else { 60710 } }

# 1. Kill orfani su questa porta
Get-CimInstance Win32_Process -Filter "Name='python.exe'" |
    Where-Object { $_.CommandLine -match 'uvicorn open_webui' -and $_.CommandLine -match "port $Port" } |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force }
Start-Sleep 2

# 2. Secret key (come il launcher Rust)
$keyFile = Join-Path $backendDir '.webui_secret_key'
$key = if (Test-Path $keyFile) { (Get-Content $keyFile -Raw).Trim() } else { $null }
if (-not $key) {
    $bytes = New-Object byte[] 24
    (New-Object Random).NextBytes($bytes)
    $key = [Convert]::ToBase64String($bytes)
    Set-Content -Path $keyFile -Value $key
}

# 3. Avvio detached (env sette PRIMA dello spawn: il figlio le eredita)
Remove-Item $out, $err -Force -ErrorAction SilentlyContinue
$env:WEBUI_SECRET_KEY = $key
$env:PYTHONIOENCODING = 'utf-8'
$env:DATA_DIR = $dataDir
$env:CORS_ALLOW_ORIGIN = 'http://localhost:5173;http://localhost:8080'

# cmd /c "python.exe -m uvicorn ... > out 2> err" — lo spawner cmd apra i file da sé:
# il figlio non condivide gli handle della console, quindi il tool torna subito.
$inner = '""{0}" -m uvicorn open_webui.main:app --host 127.0.0.1 --port {1} --workers 1 > "{2}" 2> "{3}""' -f $venvPython, $Port, $out, $err
$p = Start-Process -FilePath 'cmd.exe' -ArgumentList @('/c', $inner) `
    -WorkingDirectory $backendDir -WindowStyle Hidden -PassThru

"Backend PID $($p.Id) avviato, attendo /health su :$Port ..."

# 4. Poll /health col timeout
$deadline = (Get-Date).AddSeconds($TimeoutSec)
while ((Get-Date) -lt $deadline) {
    Start-Sleep 2
    try {
        $r = Invoke-WebRequest "http://127.0.0.1:$Port/health" -UseBasicParsing -TimeoutSec 3
        if ($r.StatusCode -eq 200) {
            Write-Output "BACKEND OK http://127.0.0.1:$Port (pid $($p.Id))"
            exit 0
        }
    } catch {}
}
Write-Output "BACKEND TIMEOUT: /health non risponde su :$Port (vedi $err)"
Get-Content $err -Tail 20
exit 2