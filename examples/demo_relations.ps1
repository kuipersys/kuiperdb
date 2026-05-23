#!/usr/bin/env pwsh
# Demonstration of Document Relations in KuiperDb

Write-Host "`n╔══════════════════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║          KuiperDb DOCUMENT RELATIONS DEMONSTRATION           ║" -ForegroundColor Cyan
Write-Host "╚══════════════════════════════════════════════════════════════╝" -ForegroundColor Cyan
Write-Host ""

$baseUrl = "http://localhost:8080"

# Get the D&D characters
Write-Host "Step 1: Getting D&D characters..." -ForegroundColor Yellow
$docs = (Invoke-RestMethod -Uri "$baseUrl/db/dnd_campaign/characters/documents").documents

if ($docs.Count -lt 3) {
    Write-Host "❌ Not enough documents. Please run add_characters.ps1 first." -ForegroundColor Red
    exit 1
}

$theron = $docs | Where-Object { $_.content -like "*Theron*" } | Select-Object -First 1
$zephyr = $docs | Where-Object { $_.content -like "*Zephyr*" } | Select-Object -First 1
$gimble = $docs | Where-Object { $_.content -like "*Gimble*" } | Select-Object -First 1

Write-Host "  ✓ Theron (Paladin):   $($theron.id.Substring(0,12))..." -ForegroundColor Green
Write-Host "  ✓ Zephyr (Ranger):    $($zephyr.id.Substring(0,12))..." -ForegroundColor Green
Write-Host "  ✓ Gimble (Artificer): $($gimble.id.Substring(0,12))..." -ForegroundColor Green

# Create relations between characters
Write-Host "`nStep 2: Creating relations between party members..." -ForegroundColor Yellow

# Theron → Zephyr: Party Member
Write-Host "  Creating: Theron → Zephyr (PARTY_MEMBER)..." -ForegroundColor White
$body1 = @"
{
  "source_id": "$($theron.id)",
  "target_id": "$($zephyr.id)",
  "relation_type": "PARTY_MEMBER",
  "metadata": {
    "relationship": "Long-time adventuring companion",
    "trust_level": "high"
  }
}
"@

try {
    $rel1Result = Invoke-RestMethod -Uri "$baseUrl/db/dnd_campaign/relations" `
        -Method Post `
        -Body $body1 `
        -ContentType "application/json"
    Write-Host "    ✓ Relation ID: $($rel1Result.id.Substring(0,12))..." -ForegroundColor Green
} catch {
    if ($_.Exception.Response.StatusCode -eq 500 -and $_ -match "UNIQUE constraint") {
        Write-Host "    ⚠ Relation already exists" -ForegroundColor Yellow
    } else {
        throw
    }
}

# Theron → Gimble: Party Member
Write-Host "  Creating: Theron → Gimble (PARTY_MEMBER)..." -ForegroundColor White
$body2 = @"
{
  "source_id": "$($theron.id)",
  "target_id": "$($gimble.id)",
  "relation_type": "PARTY_MEMBER",
  "metadata": {
    "relationship": "Joined the party recently",
    "trust_level": "medium"
  }
}
"@

try {
    $rel2Result = Invoke-RestMethod -Uri "$baseUrl/db/dnd_campaign/relations" `
        -Method Post `
        -Body $body2 `
        -ContentType "application/json"
    Write-Host "    ✓ Relation ID: $($rel2Result.id.Substring(0,12))..." -ForegroundColor Green
} catch {
    if ($_.Exception.Response.StatusCode -eq 500 -and $_ -match "UNIQUE constraint") {
        Write-Host "    ⚠ Relation already exists" -ForegroundColor Yellow
    } else {
        throw
    }
}

# Zephyr → Gimble: Party Member
Write-Host "  Creating: Zephyr → Gimble (PARTY_MEMBER)..." -ForegroundColor White
$body3 = @"
{
  "source_id": "$($zephyr.id)",
  "target_id": "$($gimble.id)",
  "relation_type": "PARTY_MEMBER",
  "metadata": {
    "relationship": "Fellow adventurer",
    "trust_level": "medium"
  }
}
"@

try {
    $rel3Result = Invoke-RestMethod -Uri "$baseUrl/db/dnd_campaign/relations" `
        -Method Post `
        -Body $body3 `
        -ContentType "application/json"
    Write-Host "    ✓ Relation ID: $($rel3Result.id.Substring(0,12))..." -ForegroundColor Green
} catch {
    if ($_.Exception.Response.StatusCode -eq 500 -and $_ -match "UNIQUE constraint") {
        Write-Host "    ⚠ Relation already exists" -ForegroundColor Yellow
    } else {
        throw
    }
}

# Zephyr → Theron: Mentor relationship
Write-Host "  Creating: Zephyr → Theron (MENTORED_BY)..." -ForegroundColor White
$body4 = @"
{
  "source_id": "$($zephyr.id)",
  "target_id": "$($theron.id)",
  "relation_type": "MENTORED_BY",
  "metadata": {
    "relationship": "Theron taught Zephyr about honor and justice",
    "duration_years": 5
  }
}
"@

try {
    $rel4Result = Invoke-RestMethod -Uri "$baseUrl/db/dnd_campaign/relations" `
        -Method Post `
        -Body $body4 `
        -ContentType "application/json"
    Write-Host "    ✓ Relation ID: $($rel4Result.id.Substring(0,12))..." -ForegroundColor Green
} catch {
    if ($_.Exception.Response.StatusCode -eq 500 -and $_ -match "UNIQUE constraint") {
        Write-Host "    ⚠ Relation already exists" -ForegroundColor Yellow
    } else {
        throw
    }
}

# Get relations for Theron
Write-Host "`nStep 3: Querying Theron's relations..." -ForegroundColor Yellow
$theronRelations = Invoke-RestMethod -Uri "$baseUrl/db/dnd_campaign/documents/$($theron.id)/relations"

Write-Host "  Theron has $($theronRelations.Count) relations:" -ForegroundColor White
foreach ($rel in $theronRelations) {
    $direction = if ($rel.source_id -eq $theron.id) { "→" } else { "←" }
    $otherId = if ($rel.source_id -eq $theron.id) { $rel.target_id } else { $rel.source_id }
    $otherDoc = $docs | Where-Object { $_.id -eq $otherId }
    $otherName = if ($otherDoc.content -match "# (.+)") { $matches[1] } else { "Unknown" }
    Write-Host "    $direction $($rel.relation_type) $direction $otherName" -ForegroundColor Cyan
}

# Get relations for Zephyr
Write-Host "`nStep 4: Querying Zephyr's relations..." -ForegroundColor Yellow
$zephyrRelations = Invoke-RestMethod -Uri "$baseUrl/db/dnd_campaign/documents/$($zephyr.id)/relations"

Write-Host "  Zephyr has $($zephyrRelations.Count) relations:" -ForegroundColor White
foreach ($rel in $zephyrRelations) {
    $direction = if ($rel.source_id -eq $zephyr.id) { "→" } else { "←" }
    $otherId = if ($rel.source_id -eq $zephyr.id) { $rel.target_id } else { $rel.source_id }
    $otherDoc = $docs | Where-Object { $_.id -eq $otherId }
    $otherName = if ($otherDoc.content -match "# (.+)") { $matches[1] } else { "Unknown" }
    Write-Host "    $direction $($rel.relation_type) $direction $otherName" -ForegroundColor Cyan
}

# Graph traversal
Write-Host "`nStep 5: Graph traversal from Theron (depth 2)..." -ForegroundColor Yellow
$traversalRequest = @{
    start_id = $theron.id
    max_depth = 2
    relation_types = @()
} | ConvertTo-Json

$traversalResult = Invoke-RestMethod -Uri "$baseUrl/db/dnd_campaign/graph/traverse" `
    -Method Post `
    -Body $traversalRequest `
    -ContentType "application/json"

Write-Host "  Found $($traversalResult.document_ids.Count) documents and $($traversalResult.relations.Count) relations:" -ForegroundColor White
foreach ($docId in $traversalResult.document_ids) {
    $doc = $docs | Where-Object { $_.id -eq $docId }
    $name = if ($doc -and $doc.content -match "# (.+)") { $matches[1].Trim() } else { "Unknown" }
    $depth = $traversalResult.depth_map.$docId
    Write-Host "    • $name (depth: $depth)" -ForegroundColor Cyan
}
Write-Host "  Relations found:" -ForegroundColor White
foreach ($rel in $traversalResult.relations) {
    $sourceDoc = $docs | Where-Object { $_.id -eq $rel.source_id }
    $targetDoc = $docs | Where-Object { $_.id -eq $rel.target_id }
    $sourceName = if ($sourceDoc -and $sourceDoc.content -match "# (.+)") { $matches[1].Trim() } else { "Unknown" }
    $targetName = if ($targetDoc -and $targetDoc.content -match "# (.+)") { $matches[1].Trim() } else { "Unknown" }
    Write-Host "    • $sourceName ──$($rel.relation_type)──> $targetName" -ForegroundColor Cyan
}

# Graph statistics
Write-Host "`nStep 6: Graph statistics for dnd_campaign..." -ForegroundColor Yellow
$stats = Invoke-RestMethod -Uri "$baseUrl/db/dnd_campaign/graph/stats"

Write-Host "  Total Nodes:    $($stats.node_count)" -ForegroundColor White
Write-Host "  Total Edges:    $($stats.edge_count)" -ForegroundColor White
Write-Host "  Has Cycles:     $($stats.has_cycles)" -ForegroundColor White
Write-Host "  In Degrees:" -ForegroundColor White
foreach ($degree in $stats.in_degrees.PSObject.Properties) {
    $doc = $docs | Where-Object { $_.id -eq $degree.Name }
    $name = if ($doc -and $doc.content -match "# (.+)") { $matches[1].Trim() } else { "Unknown" }
    Write-Host "    • $name : $($degree.Value)" -ForegroundColor Cyan
}
Write-Host "  Out Degrees:" -ForegroundColor White
foreach ($degree in $stats.out_degrees.PSObject.Properties) {
    $doc = $docs | Where-Object { $_.id -eq $degree.Name }
    $name = if ($doc -and $doc.content -match "# (.+)") { $matches[1].Trim() } else { "Unknown" }
    Write-Host "    • $name : $($degree.Value)" -ForegroundColor Cyan
}

Write-Host "`n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Gray
Write-Host "✓ Document Relations Demonstration Complete!" -ForegroundColor Green
Write-Host ""
Write-Host "Party Structure:" -ForegroundColor Yellow
Write-Host "  Theron (Leader) ──→ Zephyr (Ranger)" -ForegroundColor White
Write-Host "                  └──→ Gimble (Artificer)" -ForegroundColor White
Write-Host "  Zephyr ──→ Gimble" -ForegroundColor White
Write-Host "  Zephyr ──MENTORED_BY──→ Theron" -ForegroundColor White
Write-Host ""
Write-Host "API Endpoints Used:" -ForegroundColor Yellow
Write-Host "  POST   /db/{db}/relations" -ForegroundColor Cyan
Write-Host "  GET    /db/{db}/documents/{doc_id}/relations" -ForegroundColor Cyan
Write-Host "  POST   /db/{db}/graph/traverse" -ForegroundColor Cyan
Write-Host "  GET    /db/{db}/graph/stats" -ForegroundColor Cyan
Write-Host ""
