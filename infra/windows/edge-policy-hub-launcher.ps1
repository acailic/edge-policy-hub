$ErrorActionPreference = "Stop"

function Write-Log {
  param(
    [string]$Message,
    [string]$Level = "INFO"
  )
  $timestamp = (Get-Date).ToString("s")
  $entry = "[$timestamp] [$Level] $Message"
  $entry | Tee-Object -FilePath $script:LauncherLog -Append
}

$BaseDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$BinDir = Join-Path $BaseDir "bin"
$DataDir = Join-Path $BaseDir "data"
$AuditDir = Join-Path $DataDir "audit"
$QuotaDir = Join-Path $DataDir "quota"
$ConfigDir = Join-Path $BaseDir "config"
$TenantsDir = Join-Path $ConfigDir "tenants.d"
$LogsDir = Join-Path $BaseDir "logs"
$PidFile = Join-Path $BaseDir "edge-policy-hub.pid"
$HmacSecret = Join-Path $ConfigDir "hmac-secret.txt"
$LauncherLog = Join-Path $LogsDir "launcher.log"

$services = @(
  @{ Name = "edge-policy-enforcer"; Executable = "edge-policy-enforcer.exe" },
  @{ Name = "edge-policy-audit-store"; Executable = "edge-policy-audit-store.exe" },
  @{ Name = "edge-policy-quota-tracker"; Executable = "edge-policy-quota-tracker.exe" },
  @{ Name = "edge-policy-proxy-http"; Executable = "edge-policy-proxy-http.exe" },
  @{ Name = "edge-policy-bridge-mqtt"; Executable = "edge-policy-bridge-mqtt.exe" }
)

New-Item -ItemType Directory -Path $BinDir -Force | Out-Null
New-Item -ItemType Directory -Path $AuditDir -Force | Out-Null
New-Item -ItemType Directory -Path $QuotaDir -Force | Out-Null
New-Item -ItemType Directory -Path $TenantsDir -Force | Out-Null
New-Item -ItemType Directory -Path $LogsDir -Force | Out-Null

if (-not (Test-Path $LauncherLog)) {
  New-Item -ItemType File -Path $LauncherLog -Force | Out-Null
}

if (-not (Test-Path $HmacSecret)) {
  Write-Log "Generating HMAC secret."
  $rng = [System.Security.Cryptography.RandomNumberGenerator]::Create()
  $bytes = New-Object byte[] 32
  $rng.GetBytes($bytes)
  $secret = [System.Convert]::ToBase64String($bytes)
  $secret | Out-File -FilePath $HmacSecret -Encoding utf8 -Force
}

$processTable = @{}
$restartTracker = @{}
$shutdownRequested = $false

$cancelHandler = {
  param($sender, $eventArgs)
  $script:shutdownRequested = $true
  $eventArgs.Cancel = $true
  Write-Log "Shutdown requested. Stopping services..." "WARN"
}
[Console]::CancelKeyPress += $cancelHandler

function Start-ServiceProcess {
  param(
    [string]$Name,
    [string]$Executable
  )

  $exePath = Join-Path $BinDir $Executable
  if (-not (Test-Path $exePath)) {
    throw "Executable not found: $exePath"
  }

  $stdout = Join-Path $LogsDir "$Name.out.log"
  $stderr = Join-Path $LogsDir "$Name.err.log"

  Write-Log "Starting $Name from $exePath"
  $process = Start-Process -FilePath $exePath `
    -WorkingDirectory $BaseDir `
    -NoNewWindow `
    -RedirectStandardOutput $stdout `
    -RedirectStandardError $stderr `
    -PassThru

  return $process
}

function Write-PidFile {
  $pids = $processTable.GetEnumerator() | ForEach-Object { "$($_.Key)=$($_.Value.Id)" }
  $pids -join [Environment]::NewLine | Out-File -FilePath $PidFile -Encoding utf8 -Force
}

foreach ($svc in $services) {
  $proc = Start-ServiceProcess -Name $svc.Name -Executable $svc.Executable
  $processTable[$svc.Name] = $proc
  $restartTracker[$svc.Name] = 0
}
Write-PidFile

Write-Log "Edge Policy Hub services started. Monitoring..."

try {
  while (-not $shutdownRequested) {
    Start-Sleep -Seconds 5
    foreach ($svc in $services) {
      $proc = $processTable[$svc.Name]
      if ($null -eq $proc) {
        continue
      }
      if ($proc.HasExited) {
        $restartTracker[$svc.Name]++
        $attempt = $restartTracker[$svc.Name]
        $delay = [math]::Min([math]::Pow(2, $attempt), 30)
        Write-Log "$($svc.Name) exited with code $($proc.ExitCode). Restart attempt $attempt in $delay seconds." "WARN"
        Start-Sleep -Seconds $delay
        try {
          $processTable[$svc.Name] = Start-ServiceProcess -Name $svc.Name -Executable $svc.Executable
          Write-PidFile
        } catch {
          Write-Log "Failed to restart $($svc.Name): $_" "ERROR"
        }
      }
    }
  }
} finally {
  foreach ($svc in $services) {
    $proc = $processTable[$svc.Name]
    if ($proc -and -not $proc.HasExited) {
      Write-Log "Stopping $($svc.Name) (PID $($proc.Id))."
      try {
        $proc.CloseMainWindow() | Out-Null
        if (-not $proc.WaitForExit(15000)) {
          Write-Log "$($svc.Name) did not exit gracefully. Killing process." "WARN"
          $proc.Kill()
        }
      } catch {
        Write-Log "Error stopping $($svc.Name): $_" "ERROR"
      }
    }
  }
  Remove-Item -Path $PidFile -ErrorAction SilentlyContinue
  [Console]::CancelKeyPress -= $cancelHandler
  Write-Log "Edge Policy Hub services stopped."
}
