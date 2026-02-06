#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Check health status of kuiperdb container
.DESCRIPTION
    Monitors container health, resource usage, and connectivity
.PARAMETER ContainerName
    Name of the container (default: kuiperdb)
.PARAMETER Watch
    Continuously monitor health (refresh every 5 seconds)
.EXAMPLE
    .\podman-health.ps1
.EXAMPLE
    .\podman-health.ps1 -Watch
#>

param(
    [string]$ContainerName = "kuiperdb",
    [switch]$Watch
)

$ErrorActionPreference = "Stop"

function Show-Health {
    Clear-Host
    Write-Host "=== kuiperdb Container Health ===" -ForegroundColor Cyan
    Write-Host "Time: $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')" -ForegroundColor Gray
    Write-Host ""

    # Check if container exists
    $containerInfo = podman ps -a --filter "name=^${ContainerName}$" --format json | ConvertFrom-Json
    
    if (-not $containerInfo) {
        Write-Host "Container '$ContainerName' not found!" -ForegroundColor Red
        return $false
    }

    # Container status
    Write-Host "Container Status:" -ForegroundColor Yellow
    Write-Host "  State: $($containerInfo.State)" -ForegroundColor $(if ($containerInfo.State -eq "running") { "Green" } else { "Red" })
    Write-Host "  Status: $($containerInfo.Status)" -ForegroundColor Gray
    
    if ($containerInfo.State -ne "running") {
        Write-Host "`nContainer is not running!" -ForegroundColor Red
        return $false
    }

    # Health check
    $health = podman healthcheck run $ContainerName 2>&1
    $healthStatus = if ($LASTEXITCODE -eq 0) { "Healthy" } else { "Unhealthy" }
    $healthColor = if ($LASTEXITCODE -eq 0) { "Green" } else { "Red" }
    
    Write-Host "  Health: $healthStatus" -ForegroundColor $healthColor

    # Resource usage
    Write-Host "`nResource Usage:" -ForegroundColor Yellow
    $stats = podman stats --no-stream --format json $ContainerName | ConvertFrom-Json
    Write-Host "  CPU: $($stats.CPUPerc)" -ForegroundColor Gray
    Write-Host "  Memory: $($stats.MemUsage)" -ForegroundColor Gray
    Write-Host "  Network I/O: $($stats.NetIO)" -ForegroundColor Gray
    Write-Host "  Block I/O: $($stats.BlockIO)" -ForegroundColor Gray

    # Port mappings
    Write-Host "`nPort Mappings:" -ForegroundColor Yellow
    $inspect = podman inspect $ContainerName | ConvertFrom-Json
    $ports = $inspect.NetworkSettings.Ports
    foreach ($port in $ports.PSObject.Properties) {
        $hostBindings = $port.Value
        if ($hostBindings) {
            foreach ($binding in $hostBindings) {
                Write-Host "  $($port.Name) -> $($binding.HostIp):$($binding.HostPort)" -ForegroundColor Gray
            }
        }
    }

    # Volume info
    Write-Host "`nVolume Mounts:" -ForegroundColor Yellow
    foreach ($mount in $inspect.Mounts) {
        Write-Host "  $($mount.Name) -> $($mount.Destination)" -ForegroundColor Gray
    }

    # Recent logs
    Write-Host "`nRecent Logs (last 5 lines):" -ForegroundColor Yellow
    $logs = podman logs --tail 5 $ContainerName 2>&1
    $logs | ForEach-Object { Write-Host "  $_" -ForegroundColor Gray }

    # API connectivity test
    Write-Host "`nAPI Connectivity:" -ForegroundColor Yellow
    try {
        $port = $inspect.NetworkSettings.Ports.'8080/tcp'[0].HostPort
        $response = Invoke-WebRequest -Uri "http://localhost:$port" -Method GET -TimeoutSec 5 -ErrorAction Stop
        Write-Host "  Status: Connected (HTTP $($response.StatusCode))" -ForegroundColor Green
    } catch {
        Write-Host "  Status: Unable to connect - $($_.Exception.Message)" -ForegroundColor Red
    }

    return $true
}

# Main loop
do {
    $continue = Show-Health
    
    if ($Watch -and $continue) {
        Write-Host "`nRefreshing in 5 seconds... (Ctrl+C to exit)" -ForegroundColor Gray
        Start-Sleep -Seconds 5
    }
} while ($Watch -and $continue)

if (-not $continue) {
    exit 1
}
