#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Inspect Podman volume contents
.DESCRIPTION
    Browse and inspect files inside a Podman volume
.PARAMETER VolumeName
    Name of the volume to inspect (default: kuiperdb-data)
.PARAMETER Action
    Action to perform: browse, list, copy, or shell
.PARAMETER OutputPath
    Path to copy volume contents to (used with -Action copy)
.EXAMPLE
    .\podman-inspect-volume.ps1 -VolumeName kuiperdb-data
.EXAMPLE
    .\podman-inspect-volume.ps1 -VolumeName kuiperdb-logs -Action list
.EXAMPLE
    .\podman-inspect-volume.ps1 -Action copy -OutputPath ./volume-contents
#>

param(
    [string]$VolumeName = "kuiperdb-data",
    [ValidateSet("browse", "list", "copy", "shell")]
    [string]$Action = "browse",
    [string]$OutputPath = "./volume-export"
)

$ErrorActionPreference = "Stop"

Write-Host "=== Podman Volume Inspector ===" -ForegroundColor Cyan
Write-Host "Volume: $VolumeName" -ForegroundColor Yellow
Write-Host "Action: $Action" -ForegroundColor Yellow
Write-Host ""

# Verify volume exists
Write-Host "Checking if volume exists..." -ForegroundColor Green
podman volume inspect $VolumeName 2>$null | Out-Null
if ($LASTEXITCODE -ne 0) {
    Write-Host "ERROR: Volume '$VolumeName' not found" -ForegroundColor Red
    Write-Host ""
    Write-Host "Available volumes:" -ForegroundColor Yellow
    podman volume ls
    exit 1
}

Write-Host "✓ Volume found" -ForegroundColor Green
Write-Host ""

switch ($Action) {
    "browse" {
        Write-Host "=== Volume Contents ===" -ForegroundColor Cyan
        podman run --rm -v "${VolumeName}:/data:ro" mcr.microsoft.com/azurelinux/base/core:3.0 sh -c "ls -lah /data"
        
        Write-Host ""
        Write-Host "=== File Tree ===" -ForegroundColor Cyan
        podman run --rm -v "${VolumeName}:/data:ro" mcr.microsoft.com/azurelinux/base/core:3.0 sh -c "find /data -type f -ls 2>/dev/null || find /data"
    }
    
    "list" {
        Write-Host "=== File Listing ===" -ForegroundColor Cyan
        podman run --rm -v "${VolumeName}:/data:ro" mcr.microsoft.com/azurelinux/base/core:3.0 sh -c "find /data -type f"
    }
    
    "copy" {
        Write-Host "Copying volume contents to: $OutputPath" -ForegroundColor Green
        
        # Create output directory
        New-Item -ItemType Directory -Path $OutputPath -Force | Out-Null
        
        # Copy files from volume
        podman run --rm `
            -v "${VolumeName}:/data:ro" `
            -v "${OutputPath}:/export:Z" `
            mcr.microsoft.com/azurelinux/base/core:3.0 `
            sh -c "cp -r /data/* /export/ 2>/dev/null || cp -r /data/. /export/"
        
        if ($LASTEXITCODE -eq 0) {
            Write-Host ""
            Write-Host "✓ Files copied successfully" -ForegroundColor Green
            Write-Host ""
            Write-Host "Contents:" -ForegroundColor Yellow
            Get-ChildItem -Path $OutputPath -Recurse | Select-Object FullName, Length | Format-Table -AutoSize
        } else {
            Write-Host "✗ Copy failed" -ForegroundColor Red
        }
    }
    
    "shell" {
        Write-Host "Opening interactive shell..." -ForegroundColor Green
        Write-Host "Volume mounted at: /data" -ForegroundColor Yellow
        Write-Host "Type 'exit' to quit" -ForegroundColor Gray
        Write-Host ""
        
        podman run --rm -it -v "${VolumeName}:/data" mcr.microsoft.com/azurelinux/base/core:3.0 sh
    }
}

Write-Host ""
Write-Host "=== Additional Commands ===" -ForegroundColor Cyan
Write-Host "  List files:    .\podman-inspect-volume.ps1 -VolumeName $VolumeName -Action list" -ForegroundColor Gray
Write-Host "  Copy files:    .\podman-inspect-volume.ps1 -VolumeName $VolumeName -Action copy" -ForegroundColor Gray
Write-Host "  Open shell:    .\podman-inspect-volume.ps1 -VolumeName $VolumeName -Action shell" -ForegroundColor Gray
Write-Host "  View metadata: podman volume inspect $VolumeName" -ForegroundColor Gray
