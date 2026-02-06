#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Generate systemd service file for kuiperdb (Linux only)
.DESCRIPTION
    Creates a systemd service unit file for running kuiperdb as a system service
    This provides the highest level of resilience with automatic restart on failure
.PARAMETER ContainerName
    Name of the container (default: kuiperdb)
.PARAMETER OutputPath
    Path to save the service file (default: ./kuiperdb.service)
.EXAMPLE
    .\podman-systemd.ps1
.NOTES
    After generating, copy to /etc/systemd/system/ and run:
    sudo systemctl daemon-reload
    sudo systemctl enable kuiperdb
    sudo systemctl start kuiperdb
#>

param(
    [string]$ContainerName = "kuiperdb",
    [string]$OutputPath = "kuiperdb.service"
)

$ErrorActionPreference = "Stop"

Write-Host "=== Generating systemd service file ===" -ForegroundColor Cyan

# Check if running on Linux
if ($IsWindows) {
    Write-Host "WARNING: This script generates systemd files for Linux." -ForegroundColor Yellow
    Write-Host "The file will be created but cannot be used on Windows." -ForegroundColor Yellow
    Write-Host "Consider using Windows Services or Task Scheduler for Windows hosts." -ForegroundColor Yellow
}

# Generate service file content
$serviceContent = @"
[Unit]
Description=kuiperdb - Lightweight Vector Database
After=network-online.target
Wants=network-online.target

[Service]
Type=forking
Restart=always
RestartSec=10
StartLimitInterval=200
StartLimitBurst=5

# Run as podman container
ExecStartPre=-/usr/bin/podman stop ${ContainerName}
ExecStartPre=-/usr/bin/podman rm ${ContainerName}
ExecStart=/usr/bin/podman run \
    --name ${ContainerName} \
    -d \
    -p 8080:8080 \
    -v kuiperdb-data:/app/data:Z \
    -v kuiperdb-logs:/app/logs:Z \
    -e RUST_LOG=info \
    --health-cmd "pidof kuiperdb || exit 1" \
    --health-interval 30s \
    --health-timeout 10s \
    --health-retries 3 \
    kuiperdb:latest

ExecStop=/usr/bin/podman stop -t 10 ${ContainerName}
ExecStopPost=/usr/bin/podman rm ${ContainerName}

# Security settings
NoNewPrivileges=yes
PrivateTmp=yes

# Resource limits (optional - adjust as needed)
MemoryLimit=2G
CPUQuota=200%

[Install]
WantedBy=multi-user.target
"@

# Write service file
$serviceContent | Out-File -FilePath $OutputPath -Encoding utf8 -NoNewline

Write-Host "`n=== Service file generated ===" -ForegroundColor Green
Write-Host "File: $OutputPath" -ForegroundColor Yellow

Write-Host "`nTo install on Linux:" -ForegroundColor Cyan
Write-Host "  sudo cp $OutputPath /etc/systemd/system/" -ForegroundColor White
Write-Host "  sudo systemctl daemon-reload" -ForegroundColor White
Write-Host "  sudo systemctl enable kuiperdb" -ForegroundColor White
Write-Host "  sudo systemctl start kuiperdb" -ForegroundColor White

Write-Host "`nTo check status:" -ForegroundColor Cyan
Write-Host "  sudo systemctl status kuiperdb" -ForegroundColor White
Write-Host "  sudo journalctl -u kuiperdb -f" -ForegroundColor White
