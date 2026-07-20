Write-Host "=== Installing Ditto Clipboard Manager ===" -ForegroundColor Blue

# Check dependencies
foreach ($cmd in "cargo", "bun") {
    if (-not (Get-Command $cmd -ErrorAction SilentlyContinue)) {
        Write-Error "Error: $cmd is not installed. Please install it first."
        exit 1
    }
}

Write-Host "Building release binary..." -ForegroundColor Blue
bun run build

# Target directory
$installDir = "$HOME\AppData\Local\Microsoft\WindowsApps"
if (-not (Test-Path $installDir)) {
    New-Item -ItemType Directory -Force -Path $installDir | Out-Null
}

Write-Host "Installing binary to $installDir\ditto.exe..." -ForegroundColor Blue
Copy-Item "target\release\ditto.exe" "$installDir\ditto.exe" -Force

Write-Host "Running initial configuration..." -ForegroundColor Blue
& "$installDir\ditto.exe" --help | Out-Null

Write-Host "Ditto installed successfully!" -ForegroundColor Green
Write-Host "To start Ditto daemon:  ditto start" -ForegroundColor Yellow
Write-Host "To toggle visibility:  ditto toggle" -ForegroundColor Yellow
