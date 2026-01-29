# Delete paa_data.db to reset DB. Run before tauri dev to apply 001_init.sql on fresh DB.
# Ensure the app is not running (close Tauri app / stop tauri dev) before running.
$ErrorActionPreference = "Stop"
$dir = Join-Path $env:APPDATA "jp.github.hina0118.paa"
$db = Join-Path $dir "paa_data.db"
if (-not (Test-Path $db)) {
    Write-Host "Not found (already reset or never created): $db"
    exit 0
}
try {
    Remove-Item $db -Force
    Write-Host "Deleted: $db"
} catch {
    Write-Host "Failed to delete (close the app first): $db"
    Write-Host $_.Exception.Message
    exit 1
}
