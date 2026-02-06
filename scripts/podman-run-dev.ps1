#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Run kuiperdb container with local filesystem bind mounts
.DESCRIPTION
    Runs kuiperdb container using bind mounts to local directories instead of named volumes.
    Perfect for development with VS Code - directly access files in your workspace.
.PARAMETER Port
    Host port to expose (default: 8080)
.PARAMETER DataDir
    Local directory for data (default: ./data)
.PARAMETER LogsDir
    Local directory for logs (default: ./logs)
.PARAMETER ContainerName
    Name for the container (default: kuiperdb-dev)
.PARAMETER Tag
    Image tag to use (default: kuiperdb:latest)
.PARAMETER Detach
    Run in detached mode (default: true)
.EXAMPLE
    .\podman-run-dev.ps1
.EXAMPLE
    .\podman-run-dev.ps1 -Port 9090 -DataDir ./my-data -LogsDir ./my-logs
#>

param(
    [int]$Port = 8080,
    [string]$DataDir = "./data",
    [string]$LogsDir = "./logs",
    [string]$ContainerName = "kuiperdb-dev",
    [string]$Tag = "kuiperdb:latest",
    [switch]$Detach = $true
)

$ErrorActionPreference = "Stop"

Write-Host "=== Starting kuiperdb container (Development Mode) ===" -ForegroundColor Cyan
Write-Host ""
Write-Host "📁 Using local filesystem bind mounts" -ForegroundColor Yellow
Write-Host "   Files will be directly accessible in VS Code!" -ForegroundColor Green

# Resolve to absolute paths
$DataDirAbs = (Resolve-Path -Path $DataDir -ErrorAction SilentlyContinue) ?? (New-Item -ItemType Directory -Path $DataDir -Force | Select-Object -ExpandProperty FullName)
$LogsDirAbs = (Resolve-Path -Path $LogsDir -ErrorAction SilentlyContinue) ?? (New-Item -ItemType Directory -Path $LogsDir -Force | Select-Object -ExpandProperty FullName)

Write-Host ""
Write-Host "Directories:" -ForegroundColor Yellow
Write-Host "  Data: $DataDirAbs" -ForegroundColor Gray
Write-Host "  Logs: $LogsDirAbs" -ForegroundColor Gray

# Check if container already exists
$existing = podman ps -a --filter "name=^${ContainerName}$" --format "{{.Names}}"
if ($existing -eq $ContainerName) {
    Write-Host ""
    Write-Host "Container '$ContainerName' already exists. Removing..." -ForegroundColor Yellow
    podman rm -f $ContainerName | Out-Null
}

# Ensure directories exist
New-Item -ItemType Directory -Path $DataDirAbs -Force | Out-Null
New-Item -ItemType Directory -Path $LogsDirAbs -Force | Out-Null

# Run container with bind mounts
$runArgs = @(
    "run",
    "--name", $ContainerName,
    "-p", "${Port}:8080",
    "-v", "${DataDirAbs}:/app/data:Z",
    "-v", "${LogsDirAbs}:/app/logs:Z",
    "-e", "RUST_LOG=info",
    "--restart", "always",
    "--health-cmd", "pidof kuiperdb || exit 1",
    "--health-interval", "30s",
    "--health-timeout", "10s",
    "--health-retries", "3"
)

if ($Detach) {
    $runArgs += "-d"
}

$runArgs += $Tag

Write-Host ""
Write-Host "Starting container..." -ForegroundColor Green
Write-Host "  Port: $Port" -ForegroundColor Yellow
Write-Host "  Image: $Tag" -ForegroundColor Yellow

& podman $runArgs

if ($LASTEXITCODE -ne 0) {
    throw "Failed to start container"
}

Start-Sleep -Seconds 2

# Show container status
Write-Host ""
Write-Host "=== Container started ===" -ForegroundColor Green
podman ps --filter "name=^${ContainerName}$"

Write-Host ""
Write-Host "Container logs (last 10 lines):" -ForegroundColor Cyan
podman logs --tail 10 $ContainerName

Write-Host ""
Write-Host "✅ Development Setup Ready!" -ForegroundColor Green
Write-Host ""
Write-Host "📂 Access files directly in VS Code:" -ForegroundColor Yellow
Write-Host "   • Database files: $DataDirAbs" -ForegroundColor White
Write-Host "   • Log files:      $LogsDirAbs" -ForegroundColor White
Write-Host ""
Write-Host "🌐 API endpoint: http://localhost:${Port}" -ForegroundColor Cyan
Write-Host ""
Write-Host "Commands:" -ForegroundColor Yellow
Write-Host "  View logs:      podman logs -f $ContainerName" -ForegroundColor White
Write-Host "  Stop container: podman stop $ContainerName" -ForegroundColor White
Write-Host "  Restart:        podman restart $ContainerName" -ForegroundColor White
Write-Host ""
Write-Host "💡 Tip: Open this workspace in VS Code to edit files directly!" -ForegroundColor Cyan
Write-Host "    code $DataDirAbs" -ForegroundColor Gray
Write-Host "    code $LogsDirAbs" -ForegroundColor Gray
