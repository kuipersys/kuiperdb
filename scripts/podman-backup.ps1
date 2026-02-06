#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Backup kuiperdb persistent volume
.DESCRIPTION
    Creates a backup of the kuiperdb data volume to a tar.gz file
.PARAMETER VolumeName
    Volume to backup (default: kuiperdb-data)
.PARAMETER BackupPath
    Path to save backup file (default: .\backups\kuiperdb-backup-{timestamp}.tar.gz)
.EXAMPLE
    .\podman-backup.ps1
.EXAMPLE
    .\podman-backup.ps1 -VolumeName kuiperdb-data -BackupPath "C:\backups\kuiperdb.tar.gz"
#>

param(
    [string]$VolumeName = "kuiperdb-data",
    [string]$BackupPath = ""
)

$ErrorActionPreference = "Stop"

Write-Host "=== Backing up kuiperdb volume ===" -ForegroundColor Cyan

# Create backups directory if not specified
if ([string]::IsNullOrEmpty($BackupPath)) {
    $scriptPath = Split-Path -Parent $MyInvocation.MyCommand.Path
    $repoRoot = Split-Path -Parent $scriptPath
    $backupDir = Join-Path $repoRoot "backups"
    
    if (-not (Test-Path $backupDir)) {
        New-Item -ItemType Directory -Path $backupDir | Out-Null
    }
    
    $timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
    $BackupPath = Join-Path $backupDir "kuiperdb-backup-${timestamp}.tar.gz"
}

# Verify volume exists
Write-Host "Checking volume '$VolumeName'..." -ForegroundColor Green
podman volume inspect $VolumeName 2>$null | Out-Null
if ($LASTEXITCODE -ne 0) {
    throw "Volume '$VolumeName' not found"
}

# Create backup using temporary container
Write-Host "Creating backup to: $BackupPath" -ForegroundColor Yellow

$backupDir = Split-Path -Parent $BackupPath
$backupFile = Split-Path -Leaf $BackupPath

# Ensure backup directory exists
if (-not (Test-Path $backupDir)) {
    New-Item -ItemType Directory -Path $backupDir -Force | Out-Null
}

# Run backup container
podman run --rm `
    -v "${VolumeName}:/data:ro" `
    -v "${backupDir}:/backup:Z" `
    mcr.microsoft.com/azurelinux/base/core:3.0 `
    tar czf "/backup/${backupFile}" -C /data .

if ($LASTEXITCODE -ne 0) {
    throw "Backup failed"
}

$fileInfo = Get-Item $BackupPath
Write-Host "`n=== Backup complete ===" -ForegroundColor Green
Write-Host "File: $BackupPath" -ForegroundColor Yellow
Write-Host "Size: $([math]::Round($fileInfo.Length / 1MB, 2)) MB" -ForegroundColor Yellow
