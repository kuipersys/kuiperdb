#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Manage kuiperdb log files via API
.DESCRIPTION
    View, analyze, and clean up kuiperdb rotating log files using the REST API
.PARAMETER Action
    Action to perform: list, view, analyze, or cleanup
.PARAMETER Date
    Date to filter logs (format: yyyy-MM-dd, default: today)
.PARAMETER DaysToKeep
    Number of days to keep when cleaning up (default: 30)
.PARAMETER BaseUrl
    kuiperdb API base URL (default: http://localhost:8080)
.EXAMPLE
    .\manage-logs.ps1 -Action list
.EXAMPLE
    .\manage-logs.ps1 -Action view -Date 2026-02-04
.EXAMPLE
    .\manage-logs.ps1 -Action cleanup -DaysToKeep 7
.EXAMPLE
    .\manage-logs.ps1 -Action analyze -BaseUrl http://remote-server:8080
#>

param(
    [ValidateSet("list", "view", "analyze", "cleanup")]
    [string]$Action = "list",
    [string]$Date = (Get-Date -Format "yyyy-MM-dd"),
    [int]$DaysToKeep = 30,
    [string]$BaseUrl = "http://localhost:8080"
)

$ErrorActionPreference = "Stop"

Write-Host "=== kuiperdb Log File Manager (API Client) ===" -ForegroundColor Cyan
Write-Host "Server: $BaseUrl" -ForegroundColor Gray
Write-Host ""

# Test API connectivity
try {
    $null = Invoke-RestMethod -Uri "$BaseUrl/health" -Method GET -TimeoutSec 5 -ErrorAction Stop
} catch {
    Write-Host "ERROR: Cannot connect to kuiperdb API at $BaseUrl" -ForegroundColor Red
    Write-Host "Make sure the server is running." -ForegroundColor Yellow
    Write-Host ""
    Write-Host "Start server with: cargo run --release" -ForegroundColor Cyan
    Write-Host "Or container with: .\scripts\podman-run-dev.ps1" -ForegroundColor Cyan
    exit 1
}

switch ($Action) {
    "list" {
        Write-Host "Fetching log files..." -ForegroundColor Yellow
        
        try {
            $response = Invoke-RestMethod -Uri "$BaseUrl/logs" -Method GET
            
            if ($response.total_count -eq 0) {
                Write-Host "  No log files found" -ForegroundColor Gray
            } else {
                Write-Host ""
                $byDate = $response.files | Group-Object date | Sort-Object Name
                
                foreach ($group in $byDate) {
                    $dateStr = $group.Name
                    $files = $group.Group
                    $daySize = ($files | Measure-Object -Property size -Sum).Sum
                    
                    Write-Host "  $dateStr  ($([math]::Round($daySize / 1MB, 2)) MB total)" -ForegroundColor Cyan
                    foreach ($file in $files | Sort-Object number) {
                        $sizeMB = [math]::Round($file.size / 1MB, 2)
                        Write-Host "    • $($file.name) - $sizeMB MB" -ForegroundColor Gray
                    }
                    Write-Host ""
                }
                
                Write-Host "Total: $([math]::Round($response.total_size / 1MB, 2)) MB across $($response.total_count) files" -ForegroundColor Green
            }
        } catch {
            Write-Host "ERROR: Failed to fetch log files" -ForegroundColor Red
            Write-Host $_.Exception.Message -ForegroundColor Gray
        }
    }
    
    "view" {
        Write-Host "Viewing logs for: $Date" -ForegroundColor Yellow
        Write-Host ""
        
        try {
            # Get list of files for this date
            $allFiles = Invoke-RestMethod -Uri "$BaseUrl/logs" -Method GET
            $dateFiles = $allFiles.files | Where-Object { $_.date -eq $Date } | Sort-Object number
            
            if ($dateFiles.Count -eq 0) {
                Write-Host "No log files found for $Date" -ForegroundColor Red
                Write-Host ""
                Write-Host "Available dates:" -ForegroundColor Yellow
                $allFiles.files | Select-Object -ExpandProperty date -Unique | ForEach-Object {
                    Write-Host "  $_" -ForegroundColor Gray
                }
            } else {
                Write-Host "Found $($dateFiles.Count) file(s):" -ForegroundColor Green
                foreach ($file in $dateFiles) {
                    Write-Host "  • $($file.name)" -ForegroundColor Cyan
                }
                Write-Host ""
                
                # Fetch and display last 20 entries from all files
                Write-Host "Last 20 log entries:" -ForegroundColor Yellow
                $allEntries = @()
                
                foreach ($file in $dateFiles) {
                    try {
                        $entries = Invoke-RestMethod -Uri "$BaseUrl/logs/$($file.name)" -Method GET
                        $allEntries += $entries
                    } catch {
                        Write-Host "  Warning: Could not read $($file.name)" -ForegroundColor Yellow
                    }
                }
                
                $allEntries | Select-Object -Last 20 | ForEach-Object {
                    $level = $_.level
                    $color = switch ($level) {
                        "ERROR" { "Red" }
                        "WARN" { "Yellow" }
                        "INFO" { "Green" }
                        "DEBUG" { "Cyan" }
                        default { "White" }
                    }
                    Write-Host "[$($_.timestamp)] [$level] $($_.fields.message)" -ForegroundColor $color
                }
            }
        } catch {
            Write-Host "ERROR: Failed to view logs" -ForegroundColor Red
            Write-Host $_.Exception.Message -ForegroundColor Gray
        }
    }
    
    "analyze" {
        Write-Host "Analyzing logs for: $Date" -ForegroundColor Yellow
        Write-Host ""
        
        try {
            $analysis = Invoke-RestMethod -Uri "$BaseUrl/logs/analyze/$Date" -Method GET
            
            Write-Host "📊 Log Statistics" -ForegroundColor Cyan
            Write-Host ""
            
            Write-Host "By Level:" -ForegroundColor Yellow
            $analysis.by_level.PSObject.Properties | Sort-Object Value -Descending | ForEach-Object {
                Write-Host "  $($_.Name): $($_.Value)" -ForegroundColor White
            }
            Write-Host ""
            
            Write-Host "Top 10 Targets:" -ForegroundColor Yellow
            $analysis.by_target | ForEach-Object {
                Write-Host "  $($_[0]): $($_[1])" -ForegroundColor White
            }
            Write-Host ""
            
            if ($analysis.api_operations.Count -gt 0) {
                Write-Host "API Operations:" -ForegroundColor Yellow
                $analysis.api_operations | ForEach-Object {
                    Write-Host "  $($_[0]): $($_[1])" -ForegroundColor White
                }
                Write-Host ""
            }
            
            if ($analysis.errors.Count -gt 0) {
                Write-Host "❌ Errors Found: $($analysis.errors.Count)" -ForegroundColor Red
                $analysis.errors | Select-Object -First 5 | ForEach-Object {
                    Write-Host "  $_" -ForegroundColor Red
                }
                Write-Host ""
            }
            
            Write-Host "Total entries: $($analysis.total_entries)" -ForegroundColor Green
            
        } catch {
            Write-Host "ERROR: Failed to analyze logs" -ForegroundColor Red
            Write-Host $_.Exception.Message -ForegroundColor Gray
        }
    }
    
    "cleanup" {
        Write-Host "Cleaning up log files older than $DaysToKeep days..." -ForegroundColor Yellow
        Write-Host ""
        
        try {
            $body = @{ days_to_keep = $DaysToKeep } | ConvertTo-Json
            $result = Invoke-RestMethod -Uri "$BaseUrl/logs/cleanup" -Method POST -Body $body -ContentType "application/json"
            
            if ($result.deleted_count -eq 0) {
                Write-Host "✓ No old log files to clean up" -ForegroundColor Green
            } else {
                Write-Host "Deleted $($result.deleted_count) file(s) totaling $([math]::Round($result.deleted_size / 1MB, 2)) MB:" -ForegroundColor Yellow
                $result.deleted_files | ForEach-Object {
                    Write-Host "  • $_" -ForegroundColor Gray
                }
                Write-Host ""
                Write-Host "✓ Cleanup complete" -ForegroundColor Green
            }
        } catch {
            Write-Host "ERROR: Failed to cleanup logs" -ForegroundColor Red
            Write-Host $_.Exception.Message -ForegroundColor Gray
        }
    }
}

Write-Host ""
Write-Host "=== Commands ===" -ForegroundColor Cyan
Write-Host "  List all:      .\manage-logs.ps1 -Action list" -ForegroundColor Gray
Write-Host "  View today:    .\manage-logs.ps1 -Action view" -ForegroundColor Gray
Write-Host "  View date:     .\manage-logs.ps1 -Action view -Date 2026-02-03" -ForegroundColor Gray
Write-Host "  Analyze:       .\manage-logs.ps1 -Action analyze" -ForegroundColor Gray
Write-Host "  Cleanup:       .\manage-logs.ps1 -Action cleanup -DaysToKeep 30" -ForegroundColor Gray
Write-Host "  Remote server: .\manage-logs.ps1 -Action list -BaseUrl http://server:8080" -ForegroundColor Gray
