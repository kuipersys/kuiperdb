# OpenTelemetry Logging Configuration

## Overview

KuiperDb uses OpenTelemetry for structured logging with the following features:
- **JSON formatted logs** written to `./logs/KuiperDb.log`
- **Daily log rotation** for automatic log management
- **Console output** for development with human-readable format
- **Distributed tracing** with span tracking across operations
- **Structured fields** for easy log parsing and analysis

## Log Locations

- **File logs**: `./logs/KuiperDb.log` (rotates daily with timestamp)
- **Console**: Standard output (stdout)

## Log Levels

Set log levels via the `RUST_LOG` environment variable:

```bash
# Default (debug for KuiperDb, info for dependencies)
RUST_LOG=KuiperDb=debug,KuiperDb_core=debug,actix_web=info

# Verbose (trace everything)
RUST_LOG=trace

# Production (info level)
RUST_LOG=info

# Specific module debugging
RUST_LOG=KuiperDb::api=trace,KuiperDb_core::store=debug
```

## Log Format

### File Naming
```
logs/KuiperDb.2026-02-04_0.log  (first 10MB of the day)
logs/KuiperDb.2026-02-04_1.log  (next 10MB)
logs/KuiperDb.2026-02-04_2.log  (and so on...)
```

### Rotation Policy
- **Size limit**: 10MB per file
- **Daily rotation**: New sequence starts each day
- **File numbering**: `_0`, `_1`, `_2`, etc.
- **Maximum files**: Up to 10 files per day (0-9)

### JSON File Format
```json
{
  "timestamp": "2026-02-03T23:58:00.123456Z",
  "level": "INFO",
  "target": "KuiperDb::api",
  "fields": {
    "message": "Document stored successfully",
    "db": "mydb",
    "table": "docs",
    "doc_id": "abc-123"
  },
  "span": {
    "name": "store_document",
    "db": "mydb",
    "table": "docs"
  }
}
```

### Console Format
```
2026-02-03T23:58:00.123Z  INFO KuiperDb::api: Document stored successfully db=mydb table=docs
```

## Trace Spans

The following API endpoints are instrumented with spans:

- `store_document` - Document storage operations
- `get_document` - Document retrieval
- `delete_document` - Document deletion
- `search` - Search operations

Each span includes relevant context fields (database, table, doc_id, etc.)

## Docker/Podman Integration

Logs are written to `/app/logs` inside the container. To persist logs:

```powershell
# Mount logs volume
podman run -v kuiperdb-logs:/app/logs:Z KuiperDb:latest
```

Or access logs directly:

```powershell
# View live logs
podman logs -f KuiperDb

# Copy log files from container
podman cp KuiperDb:/app/logs ./container-logs
```

## Log Rotation

- **Rotation triggers**: 10MB file size OR midnight (whichever comes first)
- **File naming**: `KuiperDb.YYYY-MM-DD_N.log` where N is the file number (0-9)
- **Maximum files**: Up to 10 log files per day
- **Old files**: Automatically cleaned up based on date

### Example Log Files
```
logs/KuiperDb.2026-02-03_0.log  (yesterday, 10MB)
logs/KuiperDb.2026-02-03_1.log  (yesterday, 5MB)
logs/KuiperDb.2026-02-04_0.log  (today, 10MB)
logs/KuiperDb.2026-02-04_1.log  (today, current)
```

## Performance

- **Non-blocking I/O**: Logs are written asynchronously
- **Buffered writes**: Reduces disk I/O overhead
- **Minimal overhead**: Tracing uses zero-cost abstractions

## Querying Logs

Use `jq` for JSON log analysis:

```bash
# Filter by level (all files for today)
cat logs/KuiperDb.2026-02-04_*.log | jq 'select(.level == "ERROR")'

# Extract specific fields
cat logs/KuiperDb.2026-02-04_*.log | jq '{time: .timestamp, msg: .fields.message, db: .span.db}'

# Count by target
cat logs/KuiperDb.2026-02-04_*.log | jq -r '.target' | sort | uniq -c

# Search for specific operations
cat logs/KuiperDb.2026-02-04_*.log | jq 'select(.span.name == "store_document")'
```

PowerShell examples:

```powershell
# Read all log files for today
$today = Get-Date -Format "yyyy-MM-dd"
$logFiles = Get-ChildItem -Path logs -Filter "KuiperDb.$today_*.log"

# Parse and filter
$logFiles | ForEach-Object {
    Get-Content $_.FullName | ConvertFrom-Json | Where-Object { $_.level -eq "ERROR" }
}

# Get total log size for today
($logFiles | Measure-Object -Property Length -Sum).Sum / 1MB
```

## Troubleshooting

### No logs appearing

1. Check `RUST_LOG` environment variable
2. Verify logs directory exists and is writable
3. Check file permissions

### Logs not rotating

- Size rotation happens when file reaches 10MB
- Daily rotation happens at midnight
- Check file sizes: `Get-ChildItem logs | Select-Object Name, Length`

### High disk usage

- Each day can have up to 10 files of 10MB (100MB max per day)
- Old log files are not automatically deleted
- Implement retention policy: `Get-ChildItem logs -Filter "KuiperDb.*.log" | Where-Object { $_.LastWriteTime -lt (Get-Date).AddDays(-30) } | Remove-Item`

## Future Enhancements

To send logs to an OpenTelemetry collector:

1. Update `telemetry.rs` to use OTLP exporter
2. Configure collector endpoint via environment variable
3. Enable metrics collection alongside logs

Example configuration:
```rust
opentelemetry_otlp::new_exporter()
    .tonic()
    .with_endpoint("http://otel-collector:4317")
```

## See Also

- [tracing crate](https://docs.rs/tracing/)
- [OpenTelemetry Rust](https://opentelemetry.io/docs/instrumentation/rust/)
- [tracing-subscriber](https://docs.rs/tracing-subscriber/)
