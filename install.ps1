$ErrorActionPreference = "Stop"

Write-Host "=== Installing Ditto Clipboard Manager ===" -ForegroundColor Cyan

$repo = "shinymack/ditto"
$installDir = "$HOME\AppData\Local\Microsoft\WindowsApps"
if (-not (Test-Path $installDir)) {
    New-Item -ItemType Directory -Force -Path $installDir | Out-Null
}

$targetPath = "$installDir\ditto.exe"
$tempPath = "$installDir\ditto.exe.tmp"

# Try downloading precompiled binary from GitHub Releases
try {
    Write-Host "Fetching latest release version..." -ForegroundColor Cyan
    $releaseUrl = "https://api.github.com/repos/$repo/releases/latest"
    $latestRelease = Invoke-RestMethod -Uri $releaseUrl -Headers @{ "User-Agent" = "DittoInstaller" }
    $tag = $latestRelease.tag_name

    Write-Host "Downloading Ditto $tag for Windows..." -ForegroundColor Cyan
    $downloadUrl = "https://github.com/$repo/releases/download/$tag/ditto_windows_x86_64.exe"
    Invoke-WebRequest -Uri $downloadUrl -OutFile $tempPath -UseBasicParsing

    if (Test-Path $tempPath) {
        Move-Item -Path $tempPath -Destination $targetPath -Force
        Write-Host "Successfully downloaded and installed Ditto $tag!" -ForegroundColor Green
    }
} catch {
    Write-Host "Downloading precompiled binary failed. Checking for local cargo build..." -ForegroundColor Yellow
    if ((Get-Command "cargo" -ErrorAction SilentlyContinue) -and (Get-Command "bun" -ErrorAction SilentlyContinue)) {
        Write-Host "Building release binary locally..." -ForegroundColor Cyan
        bun run build
        Copy-Item "target\release\ditto.exe" $targetPath -Force
    } else {
        Write-Error "Could not download release binary or build locally. Please check your internet connection or install Rust/Cargo."
        exit 1
    }
}

Write-Host "Verifying binary execution..." -ForegroundColor Cyan
try {
    & "$targetPath" -v
} catch {}

Write-Host "`nDitto installed successfully!" -ForegroundColor Green
Write-Host "To start Ditto daemon:  ditto start" -ForegroundColor Yellow
Write-Host "To toggle visibility:   ditto toggle" -ForegroundColor Yellow
