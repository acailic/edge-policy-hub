$ErrorActionPreference = "Stop"

function Assert-Administrator {
  if (-not ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw "This installer must be run from an elevated PowerShell session."
  }
}

function Write-Log {
  param(
    [string]$Message,
    [string]$Level = "INFO"
  )
  $timestamp = (Get-Date).ToString("s")
  $entry = "[$timestamp] [$Level] $Message"
  $entry | Tee-Object -FilePath $script:LogFile -Append
}

Assert-Administrator

$SourceDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$InstallDir = Join-Path $env:ProgramFiles "Edge Policy Hub"
$BinDir = Join-Path $InstallDir "bin"
$ConfigDir = Join-Path $InstallDir "config"
$TenantsDir = Join-Path $ConfigDir "tenants.d"
$DataDir = Join-Path $InstallDir "data"
$AuditDir = Join-Path $DataDir "audit"
$QuotaDir = Join-Path $DataDir "quota"
$LogsDir = Join-Path $InstallDir "logs"
$LogFile = Join-Path $LogsDir "install.log"
$LauncherScript = Join-Path $BinDir "edge-policy-hub-launcher.ps1"
$LauncherExe = Join-Path $BinDir "edge-policy-hub-launcher.exe"
$ServiceExe = Join-Path $BinDir "EdgePolicyHubService.exe"
$ServiceConfig = Join-Path $BinDir "edge-policy-hub-service.xml"
$ServiceName = "EdgePolicyHub"
$HmacSecret = Join-Path $ConfigDir "hmac-secret.txt"

New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
New-Item -ItemType Directory -Path $BinDir -Force | Out-Null
New-Item -ItemType Directory -Path $ConfigDir -Force | Out-Null
New-Item -ItemType Directory -Path $TenantsDir -Force | Out-Null
New-Item -ItemType Directory -Path $AuditDir -Force | Out-Null
New-Item -ItemType Directory -Path $QuotaDir -Force | Out-Null
New-Item -ItemType Directory -Path $LogsDir -Force | Out-Null

if (-not (Test-Path $LogFile)) {
  New-Item -ItemType File -Path $LogFile -Force | Out-Null
}

Write-Log "Starting Edge Policy Hub installation."

$binaryNames = @(
  "edge-policy-enforcer.exe",
  "edge-policy-audit-store.exe",
  "edge-policy-quota-tracker.exe",
  "edge-policy-proxy-http.exe",
  "edge-policy-bridge-mqtt.exe"
)

foreach ($binary in $binaryNames) {
  $source = Join-Path $SourceDir $binary
  if (-not (Test-Path $source)) {
    throw "Missing binary: $source"
  }
  $destination = Join-Path $BinDir $binary
  Write-Log "Copying $binary to $destination"
  Copy-Item -Path $source -Destination $destination -Force
}

$winswSource = Join-Path $SourceDir "WinSW.exe"
if (-not (Test-Path $winswSource)) {
  throw "Missing WinSW executable: $winswSource"
}
Copy-Item -Path $winswSource -Destination $ServiceExe -Force

$launcherSource = Join-Path $SourceDir "edge-policy-hub-launcher.ps1"
Copy-Item -Path $launcherSource -Destination $LauncherScript -Force

$launcherExeSource = Join-Path $SourceDir "edge-policy-hub-launcher.exe"
if (Test-Path $launcherExeSource) {
  Copy-Item -Path $launcherExeSource -Destination $LauncherExe -Force
}

$serviceConfigSource = Join-Path $SourceDir "edge-policy-hub-service.xml"
Copy-Item -Path $serviceConfigSource -Destination $ServiceConfig -Force

if (-not (Test-Path $LauncherExe)) {
  Write-Log "Launcher executable not found; configuring WinSW to run PowerShell script."
  [xml]$serviceXml = Get-Content -Path $ServiceConfig
  $powershellPath = Join-Path $env:SystemRoot "System32\WindowsPowerShell\v1.0\powershell.exe"
  $serviceXml.service.executable = $powershellPath
  $serviceXml.service.arguments = '-NoLogo -NoProfile -ExecutionPolicy Bypass -File "%BASE%\edge-policy-hub-launcher.ps1"'
  $serviceXml.Save($ServiceConfig)
}

if (-not (Test-Path $HmacSecret)) {
  Write-Log "Generating HMAC secret."
  $rng = [System.Security.Cryptography.RandomNumberGenerator]::Create()
  $bytes = New-Object byte[] 32
  $rng.GetBytes($bytes)
  $secret = [System.Convert]::ToBase64String($bytes)
  $secret | Out-File -FilePath $HmacSecret -Encoding utf8 -Force
}

Write-Log "Installing Windows service '$ServiceName'."
& $ServiceExe install | Out-Null
Set-Service -Name $ServiceName -StartupType Automatic

Write-Log "Starting Windows service '$ServiceName'."
Start-Service -Name $ServiceName

Write-Log "Verifying service status."
$serviceStatus = Get-Service -Name $ServiceName
Write-Log "Service status: $($serviceStatus.Status)"

Write-Log "Creating firewall rules."
New-NetFirewallRule -DisplayName "Edge Policy HTTP Proxy" -Direction Inbound -Protocol TCP -LocalPort 8080 -Action Allow -Profile Any -ErrorAction SilentlyContinue | Out-Null
New-NetFirewallRule -DisplayName "Edge Policy MQTT Bridge" -Direction Inbound -Protocol TCP -LocalPort 1883 -Action Allow -Profile Any -ErrorAction SilentlyContinue | Out-Null

Write-Log "Installation complete."
Write-Host ""
Write-Host "Edge Policy Hub installed successfully at $InstallDir"
Write-Host "Service name: $ServiceName"
Write-Host "Next steps:"
Write-Host "  1. Launch the desktop UI to configure tenants."
Write-Host "  2. Review logs at $LogsDir"
Write-Host "  3. Manage the service with 'Get-Service $ServiceName'"
