Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "   Starting Cloudflare Tunnel Central (Rust + Next.js)     " -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

$backendDir = Join-Path $PSScriptRoot "backend"
$frontendDir = Join-Path $PSScriptRoot "frontend"

Write-Host "[1/2] Starting Rust Backend on http://127.0.0.1:8080..." -ForegroundColor Green
Start-Process -FilePath "cargo" -ArgumentList "run" -WorkingDirectory $backendDir

Start-Sleep -Seconds 2

Write-Host "[2/2] Starting Next.js Frontend on http://localhost:3000..." -ForegroundColor Green
Start-Process -FilePath "npm" -ArgumentList "run", "dev" -WorkingDirectory $frontendDir

Write-Host "`nReady! Open http://localhost:3000 in your browser." -ForegroundColor Yellow

