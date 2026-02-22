param(
  [switch]$User,
  [switch]$System,
  [switch]$Help
)

$ErrorActionPreference = 'Stop'

if ($Help) {
  Write-Host "Usage: .\\windows.ps1 [-User|-System]"
  Write-Host "  default  : install BREOM_HOME into $HOME\\.breom"
  Write-Host "  -System  : install BREOM_HOME into C:\\Program Files\\Breom"
  Write-Host "  BREOM_HOME_PATH env var overrides both defaults"
  exit 0
}

if ($User -and $System) {
  Write-Error "Use either -User or -System, not both."
}

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = Resolve-Path (Join-Path $ScriptDir "..")
$DefaultBreomHomePath = if ($System) { "C:\Program Files\Breom" } else { Join-Path $HOME ".breom" }
$BreomHomePath = if ($env:BREOM_HOME_PATH) { $env:BREOM_HOME_PATH } else { $DefaultBreomHomePath }

$CargoTomlPath = Join-Path $RepoRoot "Cargo.toml"
$CargoTomlContent = Get-Content -Path $CargoTomlPath -Raw
$VersionMatch = [regex]::Match($CargoTomlContent, '(?m)^version\s*=\s*"([^"]+)"')
if (-not $VersionMatch.Success) {
  Write-Error "failed to read version from $CargoTomlPath"
}
$BreomVersion = $VersionMatch.Groups[1].Value
$VersionedRoot = Join-Path $BreomHomePath $BreomVersion

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
  Write-Error "cargo is not installed. Install Rust from https://rustup.rs first."
}

Write-Host "[1/4] Installing breom CLI via cargo..."
cargo install --path "$RepoRoot" --locked --force --root "$VersionedRoot"

Write-Host "[2/4] Preparing BREOM_HOME at $BreomHomePath"
$stdInstallPath = Join-Path $VersionedRoot "src"
New-Item -ItemType Directory -Path $stdInstallPath -Force | Out-Null

$stdPath = Join-Path $RepoRoot "std"
if (Test-Path $stdPath) {
  if (Test-Path $stdInstallPath) {
    Remove-Item -Recurse -Force $stdInstallPath
  }
  New-Item -ItemType Directory -Path $stdInstallPath -Force | Out-Null
  Copy-Item -Path (Join-Path $stdPath "*") -Destination $stdInstallPath -Recurse -Force
}

Write-Host "[3/4] Configuring BREOM_HOME=$BreomHomePath"
[Environment]::SetEnvironmentVariable('BREOM_HOME', $BreomHomePath, 'User')
$env:BREOM_HOME = $BreomHomePath

$breomBin = Join-Path $VersionedRoot "bin"
$userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
if ([string]::IsNullOrWhiteSpace($userPath)) {
  $userPath = ""
}

if (-not ($userPath -split ';' | Where-Object { $_ -eq $breomBin })) {
  $newPath = if ([string]::IsNullOrWhiteSpace($userPath)) { $breomBin } else { "$userPath;$breomBin" }
  [Environment]::SetEnvironmentVariable('Path', $newPath, 'User')
  Write-Host "Added breom bin to user PATH: $breomBin"
}

$env:Path = "$breomBin;$env:Path"

Write-Host "[4/4] Done."
Write-Host ""
Write-Host "BREOM_HOME is now set to: $env:BREOM_HOME"
Write-Host "std installed to: $stdInstallPath"
Write-Host "breom binary path: $breomBin"
Write-Host "Open a new terminal to pick up environment changes."
Write-Host "Verify with: breom --help"
