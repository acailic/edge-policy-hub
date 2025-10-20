param(
  [switch]$KeepData,
  [switch]$RemoveAll,
  [switch]$WhatIf,
  [switch]$Backup
)

$ErrorActionPreference = "Stop"

function Assert-Administrator {
  if (-not ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw "This uninstaller must be run from an elevated PowerShell session."
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

$InstallDir = Join-Path $env:ProgramFiles "Edge Policy Hub"
$BinDir = Join-Path $InstallDir "bin"
$LogsDir = Join-Path $InstallDir "logs"
$DataDir = Join-Path $InstallDir "data"
$ConfigDir = Join-Path $InstallDir "config"
$LogFile = Join-Path $env:TEMP "edge-policy-hub-uninstall.log"
$ServiceExe = Join-Path $BinDir "EdgePolicyHubService.exe"
$ServiceName = "EdgePolicyHub"

if (-not (Test-Path $LogFile)) {
  New-Item -ItemType File -Path $LogFile -Force | Out-Null
}

Write-Log "Starting Edge Policy Hub uninstallation."

if ($KeepData -and $RemoveAll) {
  throw "Parameters -KeepData and -RemoveAll cannot be used together."
}

$preserveData = $KeepData
if (-not $KeepData -and -not $RemoveAll) {
  $answer = Read-Host "Keep data directories? (Y/n)"
  if ([string]::IsNullOrWhiteSpace($answer) -or $answer -match "^[Yy]") {
    $preserveData = $true
  }
}
if ($RemoveAll) {
  $preserveData = $false
}

if (-not $preserveData -and -not $RemoveAll) {
  $confirm = Read-Host "This will permanently delete all data. Continue? (y/N)"
  if ($confirm -notmatch "^[Yy]") {
    $preserveData = $true
  }
}

if (Get-Service -Name $ServiceName -ErrorAction SilentlyContinue) {
  Write-Log "Stopping Windows service '$ServiceName'."
  Stop-Service -Name $ServiceName -Force -ErrorAction SilentlyContinue
}

if (Test-Path $ServiceExe) {
  Write-Log "Unregistering Windows service '$ServiceName'."
  if ($WhatIf) {
    Write-Log "[WhatIf] Would execute: $ServiceExe uninstall"
  } else {
    & $ServiceExe uninstall | Out-Null
  }
}

Write-Log "Removing firewall rules."
if ($WhatIf) {
  Write-Log "[WhatIf] Would remove firewall rule 'Edge Policy HTTP Proxy'"
  Write-Log "[WhatIf] Would remove firewall rule 'Edge Policy MQTT Bridge'"
} else {
  Remove-NetFirewallRule -DisplayName "Edge Policy HTTP Proxy" -ErrorAction SilentlyContinue
  Remove-NetFirewallRule -DisplayName "Edge Policy MQTT Bridge" -ErrorAction SilentlyContinue
}

if (-not $preserveData -and $Backup) {
  $timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
  $backupPath = Join-Path $env:TEMP "edge-policy-hub-backup-$timestamp.zip"
  Write-Log "Creating backup at $backupPath"
  if (-not $WhatIf) {
    Compress-Archive -Path $DataDir, $ConfigDir, $LogsDir -DestinationPath $backupPath -ErrorAction SilentlyContinue
  }
}

if ($WhatIf) {
  Write-Log "[WhatIf] Would remove binaries in $BinDir"
} else {
  if (Test-Path $BinDir) {
    Remove-Item -Path $BinDir -Recurse -Force -ErrorAction SilentlyContinue
  }
}

if (-not $preserveData) {
  if ($WhatIf) {
    Write-Log "[WhatIf] Would remove $InstallDir"
  } else {
    if (Test-Path $InstallDir) {
      Remove-Item -Path $InstallDir -Recurse -Force -ErrorAction SilentlyContinue
    }
  }
} else {
  Write-Log "Preserving data directories."
  if (-not $WhatIf) {
    if (Test-Path $BinDir) {
      Remove-Item -Path $BinDir -Recurse -Force -ErrorAction SilentlyContinue
    }
  }
}

$removeUser = Read-Host "Remove Edge Policy Hub desktop shortcuts? (y/N)"
if ($removeUser -match "^[Yy]") {
  $desktopShortcut = Join-Path ([Environment]::GetFolderPath("CommonDesktopDirectory")) "Edge Policy Hub.lnk"
  if ($WhatIf) {
    Write-Log "[WhatIf] Would remove $desktopShortcut"
  } else {
    Remove-Item -Path $desktopShortcut -ErrorAction SilentlyContinue
  }
}

Write-Log "Uninstallation completed."
if ($preserveData) {
  Write-Log "Data preserved at $InstallDir"
} else {
  Write-Log "Installation directory removed."
}
Write-Log "Log file located at $LogFile"
