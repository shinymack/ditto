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
$downloadSuccess = $false
try {
    Write-Host "Fetching latest release version..." -ForegroundColor Cyan
    $releaseUrl = "https://api.github.com/repos/$repo/releases/latest"
    $latestRelease = Invoke-RestMethod -Uri $releaseUrl -Headers @{ "User-Agent" = "DittoInstaller" }
    $tag = $latestRelease.tag_name

    $assetCandidates = @("ditto_windows_x64.exe", "ditto_windows_x86_64.exe", "ditto.exe")
    foreach ($asset in $assetCandidates) {
        $downloadUrl = "https://github.com/$repo/releases/download/$tag/$asset"
        Write-Host "Trying download ($asset)..." -ForegroundColor Cyan
        try {
            Invoke-WebRequest -Uri $downloadUrl -OutFile $tempPath -UseBasicParsing -ErrorAction Stop
            if ((Test-Path $tempPath) -and ((Get-Item $tempPath).Length -gt 1000)) {
                Move-Item -Path $tempPath -Destination $targetPath -Force
                $downloadSuccess = $true
                Write-Host "Successfully downloaded and installed Ditto $tag!" -ForegroundColor Green
                break
            }
        } catch {
            if (Test-Path $tempPath) { Remove-Item $tempPath -Force }
        }
    }
} catch {
    Write-Host "Unable to contact GitHub Releases API." -ForegroundColor Yellow
}

if (-not $downloadSuccess) {
    Write-Host "Downloading release binary failed. Checking for local cargo build..." -ForegroundColor Yellow
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
