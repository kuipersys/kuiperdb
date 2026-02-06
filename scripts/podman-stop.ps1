#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Stop kuiperdb container
.DESCRIPTION
    Stops and optionally removes the kuiperdb container
.PARAMETER ContainerName
    Name of the container to stop (default: kuiperdb)
.PARAMETER Remove
    Remove the container after stopping
.EXAMPLE
    .\podman-stop.ps1
.EXAMPLE
    .\podman-stop.ps1 -Remove
#>

param(
    [string]$ContainerName = "kuiperdb",
    [switch]$Remove
)

$ErrorActionPreference = "Stop"

Write-Host "=== Stopping kuiperdb container ===" -ForegroundColor Cyan

# Check if container exists
$existing = podman ps -a --filter "name=^${ContainerName}$" --format "{{.Names}}"
if ($existing -ne $ContainerName) {
    Write-Host "Container '$ContainerName' not found." -ForegroundColor Yellow
    exit 0
}

# Stop container
Write-Host "Stopping container '$ContainerName'..." -ForegroundColor Green
podman stop $ContainerName

if ($Remove) {
    Write-Host "Removing container '$ContainerName'..." -ForegroundColor Yellow
    podman rm $ContainerName
    Write-Host "Container removed." -ForegroundColor Green
} else {
    Write-Host "Container stopped. Use 'podman start $ContainerName' to restart." -ForegroundColor Yellow
}
