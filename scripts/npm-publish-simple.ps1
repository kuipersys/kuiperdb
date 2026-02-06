#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Publish KuiperDb npm packages at their current versions

.DESCRIPTION
    This script builds and publishes the KuiperDb npm packages to npm at their current versions.
    It does NOT bump versions - use this when you've already set the versions manually.
    
    Handles packages in dependency order:
    1. @kuiperdb/client (kuiperdb-ts)
    2. @kuiperdb/react (kuiperdb-react)

.PARAMETER SkipBuild
    If set, skips the build step (useful if you've already built)

.PARAMETER Tag
    The npm dist-tag to publish under (e.g., 'latest', 'beta', 'next')
    Default: latest

.PARAMETER Otp
    One-time password from your authenticator app (for TOTP-based 2FA).
    Not needed if using security key - npm will prompt automatically.

.PARAMETER DryRun
    If set, performs all steps except the actual npm publish

.EXAMPLE
    .\scripts\npm-publish-simple.ps1
    
.EXAMPLE
    .\scripts\npm-publish-simple.ps1 -Tag beta

.EXAMPLE
    .\scripts\npm-publish-simple.ps1 -SkipBuild

.EXAMPLE
    .\scripts\npm-publish-simple.ps1 -Otp 123456
#>

param(
    [Parameter()]
    [switch]$SkipBuild,
    
    [Parameter()]
    [string]$Tag = 'latest',
    
    [Parameter()]
    [switch]$DryRun,
    
    [Parameter()]
    [string]$Otp
)

$ErrorActionPreference = 'Stop'

# Get the workspace root
$WorkspaceRoot = Split-Path -Parent $PSScriptRoot

# Define package directories in dependency order
$Packages = @(
    @{
        Name = "@kuiperdb/client"
        Path = Join-Path $WorkspaceRoot "src\kuiperdb-ts"
        DisplayName = "KuiperDb TypeScript Client"
    },
    @{
        Name = "@kuiperdb/react"
        Path = Join-Path $WorkspaceRoot "src\kuiperdb-react"
        DisplayName = "KuiperDb React Components"
    }
)

function Write-Step {
    param([string]$Message)
    Write-Host "`n==> $Message" -ForegroundColor Cyan
}

function Write-Success {
    param([string]$Message)
    Write-Host "✓ $Message" -ForegroundColor Green
}

function Write-Warning {
    param([string]$Message)
    Write-Host "⚠ $Message" -ForegroundColor Yellow
}

function Write-Error {
    param([string]$Message)
    Write-Host "✗ $Message" -ForegroundColor Red
}

function Get-PackageVersion {
    param([string]$PackagePath)
    
    $packageJsonPath = Join-Path $PackagePath "package.json"
    $packageJson = Get-Content $packageJsonPath | ConvertFrom-Json
    return $packageJson.version
}

function Update-CrossPackageDependencies {
    param(
        [string]$PackagePath,
        [hashtable]$Versions
    )
    
    $packageJsonPath = Join-Path $PackagePath "package.json"
    $packageJson = Get-Content $packageJsonPath -Raw | ConvertFrom-Json
    
    $updated = $false
    $originalDeps = @{}
    
    # Update dependencies
    if ($packageJson.dependencies) {
        foreach ($dep in $packageJson.dependencies.PSObject.Properties) {
            if ($dep.Value -match '^file:') {
                # Store original value
                $originalDeps[$dep.Name] = $dep.Value
                
                # Check if this is one of our published packages
                $versionToUse = $null
                foreach ($pkgName in $Versions.Keys) {
                    if ($dep.Name -eq $pkgName) {
                        $versionToUse = "^$($Versions[$pkgName])"
                        break
                    }
                }
                
                if ($versionToUse) {
                    Write-Host "    Updating $($dep.Name): $($dep.Value) → $versionToUse" -ForegroundColor Yellow
                    $packageJson.dependencies.$($dep.Name) = $versionToUse
                    $updated = $true
                }
            }
        }
    }
    
    # Update devDependencies
    if ($packageJson.devDependencies) {
        foreach ($dep in $packageJson.devDependencies.PSObject.Properties) {
            if ($dep.Value -match '^file:') {
                # Store original value
                $originalDeps[$dep.Name] = $dep.Value
                
                # Check if this is one of our published packages
                $versionToUse = $null
                foreach ($pkgName in $Versions.Keys) {
                    if ($dep.Name -eq $pkgName) {
                        $versionToUse = "^$($Versions[$pkgName])"
                        break
                    }
                }
                
                if ($versionToUse) {
                    Write-Host "    Updating $($dep.Name): $($dep.Value) → $versionToUse" -ForegroundColor Yellow
                    $packageJson.devDependencies.$($dep.Name) = $versionToUse
                    $updated = $true
                }
            }
        }
    }
    
    if ($updated) {
        $packageJson | ConvertTo-Json -Depth 100 | Set-Content $packageJsonPath -Encoding UTF8
    }
    
    return $originalDeps
}

function Restore-CrossPackageDependencies {
    param(
        [string]$PackagePath,
        [hashtable]$OriginalDeps
    )
    
    if ($OriginalDeps.Count -eq 0) {
        return
    }
    
    $packageJsonPath = Join-Path $PackagePath "package.json"
    $packageJson = Get-Content $packageJsonPath -Raw | ConvertFrom-Json
    
    foreach ($depName in $OriginalDeps.Keys) {
        $originalValue = $OriginalDeps[$depName]
        
        if ($packageJson.dependencies -and $packageJson.dependencies.$depName) {
            Write-Host "    Restoring $depName → $originalValue" -ForegroundColor Cyan
            $packageJson.dependencies.$depName = $originalValue
        }
        
        if ($packageJson.devDependencies -and $packageJson.devDependencies.$depName) {
            Write-Host "    Restoring $depName → $originalValue" -ForegroundColor Cyan
            $packageJson.devDependencies.$depName = $originalValue
        }
    }
    
    $packageJson | ConvertTo-Json -Depth 100 | Set-Content $packageJsonPath -Encoding UTF8
}

function Build-Package {
    param([string]$PackagePath)
    
    Push-Location $PackagePath
    try {
        Write-Host "  Building package..." -NoNewline
        
        # Clean dist directory if it exists
        $distPath = Join-Path $PackagePath "dist"
        if (Test-Path $distPath) {
            Remove-Item -Recurse -Force $distPath
        }
        
        npm run build 2>&1 | Out-Null
        if ($LASTEXITCODE -ne 0) {
            throw "Build failed"
        }
        Write-Host " Done" -ForegroundColor Green
    } finally {
        Pop-Location
    }
}

function Publish-Package {
    param(
        [string]$PackagePath,
        [string]$Tag,
        [bool]$DryRun,
        [string]$Otp
    )
    
    Push-Location $PackagePath
    try {
        if ($DryRun) {
            Write-Host "  [DRY RUN] Would run: npm publish --access public --tag $Tag" -ForegroundColor Yellow
        } else {
            Write-Host "  Publishing to npm (tag: $Tag)..." -NoNewline
            
            if ($Otp) {
                npm publish --access public --tag $Tag --otp $Otp
            } else {
                npm publish --access public --tag $Tag
            }
            
            if ($LASTEXITCODE -ne 0) {
                throw "Publish failed"
            }
            Write-Host " Done" -ForegroundColor Green
        }
    } finally {
        Pop-Location
    }
}

# Main script execution
try {
    Write-Host @"

╔══════════════════════════════════════════════════════════╗
║         KuiperDb NPM Package Publisher (Simple)         ║
╚══════════════════════════════════════════════════════════╝

"@ -ForegroundColor Cyan

    # Check if we're in the right directory
    if (-not (Test-Path (Join-Path $WorkspaceRoot "Cargo.toml"))) {
        throw "This script must be run from the KuiperDb workspace root or scripts directory"
    }

    # Verify all packages exist
    Write-Step "Verifying package directories..."
    foreach ($pkg in $Packages) {
        if (-not (Test-Path $pkg.Path)) {
            throw "Package directory not found: $($pkg.Path)"
        }
        Write-Success "$($pkg.DisplayName) found"
    }

    # Check npm authentication (skip in dry run)
    if (-not $DryRun) {
        Write-Step "Checking npm authentication..."
        $whoami = npm whoami 2>&1
        if ($LASTEXITCODE -ne 0) {
            throw "Not authenticated with npm. Please run: npm login"
        }
        Write-Success "Authenticated as: $whoami"
    } else {
        Write-Warning "Dry run mode - skipping authentication check"
    }

    # Display current versions
    Write-Step "Package versions to publish:"
    $versions = @{}
    foreach ($pkg in $Packages) {
        $version = Get-PackageVersion -PackagePath $pkg.Path
        $versions[$pkg.Name] = $version
        Write-Host "  $($pkg.Name): $version" -ForegroundColor White
    }

    # Confirm with user
    if (-not $DryRun) {
        Write-Host "`nTag: $Tag" -ForegroundColor Yellow
        $confirm = Read-Host "Publish these versions? (yes/no)"
        if ($confirm -ne 'yes') {
            Write-Warning "Aborted by user"
            exit 0
        }
    }

    # Update cross-package dependencies (file: → version)
    Write-Step "Updating cross-package dependencies..."
    $originalDepsMap = @{}
    foreach ($pkg in $Packages) {
        Write-Host "`n$($pkg.DisplayName):" -ForegroundColor White
        $originalDeps = Update-CrossPackageDependencies -PackagePath $pkg.Path -Versions $versions
        $originalDepsMap[$pkg.Path] = $originalDeps
    }

    # Build packages
    if (-not $SkipBuild) {
        Write-Step "Building packages..."
        foreach ($pkg in $Packages) {
            Write-Host "`n$($pkg.DisplayName):" -ForegroundColor White
            Build-Package -PackagePath $pkg.Path
        }
    } else {
        Write-Warning "Skipping build step"
    }

    # Publish packages
    Write-Step "Publishing packages..."
    foreach ($pkg in $Packages) {
        Write-Host "`n$($pkg.DisplayName):" -ForegroundColor White
        Publish-Package -PackagePath $pkg.Path -Tag $Tag -DryRun $DryRun -Otp $Otp
    }

    # Restore file: references for local development
    Write-Step "Restoring local file references..."
    foreach ($pkg in $Packages) {
        if ($originalDepsMap[$pkg.Path].Count -gt 0) {
            Write-Host "`n$($pkg.DisplayName):" -ForegroundColor White
            Restore-CrossPackageDependencies -PackagePath $pkg.Path -OriginalDeps $originalDepsMap[$pkg.Path]
        }
    }

    # Summary
    Write-Host "`n" -NoNewline
    Write-Host "═══════════════════════════════════════════════════════════" -ForegroundColor Green
    if ($DryRun) {
        Write-Host "DRY RUN COMPLETE" -ForegroundColor Yellow
    } else {
        Write-Host "PUBLISH COMPLETE" -ForegroundColor Green
    }
    Write-Host "═══════════════════════════════════════════════════════════" -ForegroundColor Green
    
    Write-Host "`nPublished versions:" -ForegroundColor Cyan
    foreach ($pkg in $Packages) {
        Write-Host "  $($pkg.Name)@$($versions[$pkg.Name])" -ForegroundColor White
    }
    
    if (-not $DryRun) {
        Write-Host "`nPackages are now available on npm!" -ForegroundColor Green
        Write-Host "`nInstall with:" -ForegroundColor Cyan
        Write-Host "  npm install $($Packages[0].Name)@$($versions[$Packages[0].Name])" -ForegroundColor White
        Write-Host "  npm install $($Packages[1].Name)@$($versions[$Packages[1].Name])" -ForegroundColor White
    }

} catch {
    Write-Host "`n" -NoNewline
    Write-Error "Error: $_"
    Write-Host $_.ScriptStackTrace -ForegroundColor Red
    exit 1
}
