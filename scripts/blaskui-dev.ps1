#Requires -Version 5.1
<#
.SYNOPSIS
  Riavvio pulito e SINGOLO di BlaskUI (tauri dev).
  Evita il problema delle istanze sovrapposte: uccide wrapper/blaskui/backend
  orfani, ne lancia UNO solo e aspetta la finestra leggendo STDERR
  (il launcher Rust scrive con eprintln!, quindi i marcatori sono in .err).
#>
param(
    [string]$Root = 'C:\Users\blasc\Desktop\OPENBLA\open-webui-main\open-webui-main',
    [int]$TimeoutSec = 300,
    [switch]$Visible
)
$ErrorActionPreference = 'SilentlyContinue'
$out = "$env:TEMP\blaskui-dev.out"
$err = "$env:TEMP\blaskui-dev.err"

# 1. Pulizia istanze precedenti (verificata, non presunta)
Get-CimInstance Win32_Process -Filter "Name='powershell.exe'" |
    Where-Object { $_.CommandLine -match 'tauri dev' -and $_.CommandLine -notmatch 'blaskui-dev\.ps1' } |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force }
Get-Process blaskui | Stop-Process -Force
Get-CimInstance Win32_Process -Filter "Name='python.exe'" |
    Where-Object { $_.CommandLine -match 'uvicorn open_webui' } |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force }
Start-Sleep 5
if (@(Get-Process blaskui).Count -gt 0) {
    Write-Output 'ATTENZIONE: blaskui.exe ancora vivo (file bloccato, es. antivirus). Riprova tra poco.'
    exit 3
}

# 2. Ollama su se spento
try {
    Invoke-WebRequest http://localhost:11434/api/tags -UseBasicParsing -TimeoutSec 5 | Out-Null
    Write-Output 'Ollama: attivo'
}
catch {
    Start-Process -FilePath 'C:\Users\blasc\AppData\Local\Programs\Ollama\ollama.exe' `
        -ArgumentList 'serve' -NoNewWindow `
        -RedirectStandardOutput "$env:TEMP\ollama-serve.out" `
        -RedirectStandardError "$env:TEMP\ollama-serve.err"
    Start-Sleep 6
    Write-Output 'Ollama: riacceso'
}

# 3. Lancio singolo, log separati (finestra visibile con -Visible: niente prompt nascosti)
Remove-Item $out -Force
Remove-Item $err -Force
$tauriArgs = @('-NoProfile', '-Command', 'tauri dev')
if ($Visible) {
    Start-Process -FilePath powershell.exe -ArgumentList $tauriArgs `
        -WorkingDirectory $Root `
        -RedirectStandardOutput $out -RedirectStandardError $err | Out-Null
}
else {
    Start-Process -FilePath powershell.exe -ArgumentList $tauriArgs `
        -WorkingDirectory $Root -NoNewWindow `
        -RedirectStandardOutput $out -RedirectStandardError $err | Out-Null
}
Write-Output 'tauri dev lanciato, attendo la finestra...'

# 4. Attesa marcatore (poll brevi: se ci fermano, il lanciato resta vivo e si riprende dal log)
$deadline = (Get-Date).AddSeconds($TimeoutSec)
while ((Get-Date) -lt $deadline) {
    Start-Sleep 2
    $m = Select-String -Path $err -Pattern 'finestra navigata su (\S+)' | Select-Object -Last 1
    if ($m) {
        $url = $m.Matches[0].Groups[1].Value.TrimEnd('/')
        Write-Output "APP: $url"
        exit 0
    }
    $e = Get-Content $err | Select-String -Pattern 'error\[|FAILED|panicked|Accesso negato|failed to remove' | Select-Object -First 1
    if ($e) {
        Write-Output "ERRORE AVVIO: $e"
        exit 1
    }
}
Write-Output 'TIMEOUT: finestra non rilevata, vedi $env:TEMP\blaskui-dev.err (il lancio continua in background)'
exit 2
