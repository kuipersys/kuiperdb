#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Run kuiperdb container with Podman
.DESCRIPTION
    Runs kuiperdb container with persistent volumes for data and logs, health checks, and auto-restart
.PARAMETER Port
    Host port to expose (default: 8080)
.PARAMETER DataVolume
    Named volume for persistent data (default: kuiperdb-data)
.PARAMETER LogsVolume
    Named volume for logs (default: kuiperdb-logs)
.PARAMETER ContainerName
    Name for the container (default: kuiperdb)
.PARAMETER Tag
    Image tag to use (default: kuiperdb:latest)
.PARAMETER Detach
    Run in detached mode (default: true)
.EXAMPLE
    .\podman-run.ps1
.EXAMPLE
    .\podman-run.ps1 -Port 9090 -DataVolume my-kuiperdb-data
#>

param(
    [int]$Port = 8080,
    [string]$DataVolume = "kuiperdb-data",
    [string]$LogsVolume = "kuiperdb-logs",
    [string]$ContainerName = "kuiperdb",
    [string]$Tag = "kuiperdb:latest",
    [switch]$Detach = $true
)

$ErrorActionPreference = "Stop"

Write-Host "=== Starting kuiperdb container ===" -ForegroundColor Cyan

# Check if container already exists
$existing = podman ps -a --filter "name=^${ContainerName}$" --format "{{.Names}}"
if ($existing -eq $ContainerName) {
    Write-Host "Container '$ContainerName' already exists. Removing..." -ForegroundColor Yellow
    podman rm -f $ContainerName | Out-Null
}

# Create volumes if they don't exist
Write-Host "Ensuring volumes exist..." -ForegroundColor Green
foreach ($vol in @($DataVolume, $LogsVolume)) {
    podman volume inspect $vol 2>$null | Out-Null
    if ($LASTEXITCODE -ne 0) {
        Write-Host "Creating volume '$vol'..." -ForegroundColor Yellow
        podman volume create $vol | Out-Null
    }
}

# Run container
$runArgs = @(
    "run",
    "--name", $ContainerName,
    "-p", "${Port}:8080",
    "-v", "${DataVolume}:/app/data:Z",
    "-v", "${LogsVolume}:/app/logs:Z",
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

Write-Host "Starting container..." -ForegroundColor Green
Write-Host "  Port: $Port" -ForegroundColor Yellow
Write-Host "  Data Volume: $DataVolume" -ForegroundColor Yellow
Write-Host "  Logs Volume: $LogsVolume" -ForegroundColor Yellow
Write-Host "  Image: $Tag" -ForegroundColor Yellow

& podman $runArgs

if ($LASTEXITCODE -ne 0) {
    throw "Failed to start container"
}

Start-Sleep -Seconds 2

# Show container status
Write-Host "`n=== Container started ===" -ForegroundColor Green
podman ps --filter "name=^${ContainerName}$"

Write-Host "`nContainer logs (last 20 lines):" -ForegroundColor Cyan
podman logs --tail 20 $ContainerName

Write-Host "`nAPI endpoint: http://localhost:${Port}" -ForegroundColor Green
Write-Host "View logs: podman logs -f $ContainerName" -ForegroundColor Yellow
Write-Host "Stop container: podman stop $ContainerName" -ForegroundColor Yellow
Write-Host "Access log files: podman exec $ContainerName ls -la /app/logs" -ForegroundColor Yellow
