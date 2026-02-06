#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Restore kuiperdb persistent volume from backup
.DESCRIPTION
    Restores kuiperdb data volume from a tar.gz backup file
.PARAMETER BackupPath
    Path to backup file to restore
.PARAMETER VolumeName
    Volume to restore to (default: kuiperdb-data)
.PARAMETER Force
    Force restore without confirmation
.EXAMPLE
    .\podman-restore.ps1 -BackupPath ".\backups\kuiperdb-backup-20260203-120000.tar.gz"
.EXAMPLE
    .\podman-restore.ps1 -BackupPath ".\backups\kuiperdb.tar.gz" -Force
#>

param(
    [Parameter(Mandatory=$true)]
    [string]$BackupPath,
    [string]$VolumeName = "kuiperdb-data",
    [switch]$Force
)

$ErrorActionPreference = "Stop"

Write-Host "=== Restoring kuiperdb volume ===" -ForegroundColor Cyan

# Verify backup file exists
if (-not (Test-Path $BackupPath)) {
    throw "Backup file not found: $BackupPath"
}

$fileInfo = Get-Item $BackupPath
Write-Host "Backup file: $BackupPath" -ForegroundColor Yellow
Write-Host "Size: $([math]::Round($fileInfo.Length / 1MB, 2)) MB" -ForegroundColor Yellow

# Check if volume exists
$volumeExists = $false
podman volume inspect $VolumeName 2>$null | Out-Null
if ($LASTEXITCODE -eq 0) {
    $volumeExists = $true
    Write-Host "WARNING: Volume '$VolumeName' already exists and will be overwritten!" -ForegroundColor Red
}

# Confirm unless Force is specified
if (-not $Force) {
    $response = Read-Host "Continue with restore? (yes/no)"
    if ($response -ne "yes") {
        Write-Host "Restore cancelled." -ForegroundColor Yellow
        exit 0
    }
}

# Create volume if it doesn't exist
if (-not $volumeExists) {
    Write-Host "Creating volume '$VolumeName'..." -ForegroundColor Green
    podman volume create $VolumeName | Out-Null
}

# Restore from backup
Write-Host "Restoring data..." -ForegroundColor Green

$backupDir = Split-Path -Parent (Resolve-Path $BackupPath)
$backupFile = Split-Path -Leaf $BackupPath

podman run --rm `
    -v "${VolumeName}:/data:Z" `
    -v "${backupDir}:/backup:ro" `
    mcr.microsoft.com/azurelinux/base/core:3.0 `
    sh -c "rm -rf /data/* && tar xzf /backup/${backupFile} -C /data"

if ($LASTEXITCODE -ne 0) {
    throw "Restore failed"
}

Write-Host "`n=== Restore complete ===" -ForegroundColor Green
Write-Host "Volume '$VolumeName' has been restored from backup." -ForegroundColor Yellow
