#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Build kuiperdb container image with Podman
.DESCRIPTION
    Builds the kuiperdb Docker image using Podman with Azure Linux base
.PARAMETER Tag
    Tag for the image (default: kuiperdb:latest)
.PARAMETER NoCache
    Build without cache
.EXAMPLE
    .\podman-build.ps1
.EXAMPLE
    .\podman-build.ps1 -Tag kuiperdb:v0.1.1 -NoCache
#>

param(
    [string]$Tag = "kuiperdb:latest",
    [switch]$NoCache
)

$ErrorActionPreference = "Stop"

Write-Host "=== Building kuiperdb container image ===" -ForegroundColor Cyan
Write-Host "Tag: $Tag" -ForegroundColor Yellow

# Navigate to repository root
$scriptPath = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = Split-Path -Parent $scriptPath
Push-Location $repoRoot

try {
    # Build arguments
    $buildArgs = @(
        "build",
        "-t", $Tag,
        "-f", "Dockerfile",
        "."
    )

    if ($NoCache) {
        $buildArgs += "--no-cache"
    }

    # Build the image
    Write-Host "Building image..." -ForegroundColor Green
    & podman $buildArgs

    if ($LASTEXITCODE -ne 0) {
        throw "Build failed with exit code $LASTEXITCODE"
    }

    Write-Host "`n=== Build successful ===" -ForegroundColor Green
    Write-Host "Image: $Tag" -ForegroundColor Yellow

    # Show image info
    Write-Host "`nImage details:" -ForegroundColor Cyan
    podman images $Tag

} finally {
    Pop-Location
}
