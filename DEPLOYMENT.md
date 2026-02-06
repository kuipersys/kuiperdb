# KuiperDb Resilient Deployment Summary

## What Was Implemented

### 1. Docker Setup with Azure Linux 3.0 ✅

**Files Created:**
- `Dockerfile` - Multi-stage build using `mcr.microsoft.com/azurelinux/base/core:3.0`
- `.dockerignore` - Optimized build context

**Security Features:**
- Non-root user (`KuiperDb:KuiperDb`)
- Minimal base image (Azure Linux 3.0)
- Multi-stage build for smaller final image
- Health checks built into container

**Volumes:**
- `/app/data` - Persistent database storage
- `/app/logs` - Log files with rotation

### 2. OpenTelemetry Logging ✅

**Files Created/Modified:**
- `KuiperDb/src/telemetry.rs` - Telemetry initialization module
- `TELEMETRY.md` - Logging documentation

**Features:**
- **Dual output**: JSON logs to file + human-readable console
- **Daily rotation**: Automatic log rotation at midnight
- **Structured logging**: All logs in JSON format for easy parsing
- **Span instrumentation**: API endpoints tracked with spans
- **Non-blocking I/O**: Async log writes for performance

**Log Locations:**
- File: `./logs/KuiperDb.log` (rotates to `KuiperDb.log.YYYY-MM-DD`)
- Console: stdout with colored output

**Instrumented Endpoints:**
- `store_document` - Document storage
- `get_document` - Document retrieval
- `delete_document` - Document deletion  
- `search` - Search operations

### 3. PowerShell Management Scripts ✅

**Files Created:**
- `scripts/podman-build.ps1` - Build container images
- `scripts/podman-run.ps1` - Run with persistent volumes
- `scripts/podman-stop.ps1` - Stop/remove containers
- `scripts/podman-health.ps1` - Health monitoring
- `scripts/podman-backup.ps1` - Backup data volumes
- `scripts/podman-restore.ps1` - Restore from backups
- `scripts/podman-systemd.ps1` - Generate systemd service
- `scripts/README.md` - Complete script documentation

## Resilience Features

### Data Persistence
- ✅ Named volumes (not bind mounts) for portability
- ✅ Separate volumes for data and logs
- ✅ Backup/restore scripts with compression
- ✅ SELinux labels for proper permissions

### Container Resilience
- ✅ `--restart=always` for auto-restart on failure
- ✅ Health checks every 30 seconds
- ✅ Process monitoring via `pidof KuiperDb`
- ✅ Non-root execution for security

### System Resilience
- ✅ systemd service file generation (Linux)
- ✅ Resource limits configurable
- ✅ Graceful shutdown handling
- ✅ Log guard for flush on exit

### Monitoring & Observability
- ✅ Structured JSON logs for analysis
- ✅ Health check script with resource monitoring
- ✅ Real-time log tailing
- ✅ Trace spans for request tracking

## Quick Start

### Build the Image
```powershell
.\scripts\podman-build.ps1
```

### Run the Container
```powershell
.\scripts\podman-run.ps1
```

### Monitor Health
```powershell
.\scripts\podman-health.ps1 -Watch
```

### Backup Data
```powershell
.\scripts\podman-backup.ps1
```

## Configuration

### Log Levels
Set via `RUST_LOG` environment variable:
```powershell
# Production (info level)
podman run -e RUST_LOG=info KuiperDb:latest

# Debug mode
podman run -e RUST_LOG=debug KuiperDb:latest

# Specific modules
podman run -e RUST_LOG=KuiperDb::api=trace,KuiperDb_core=debug KuiperDb:latest
```

### Volume Management
```powershell
# List volumes
podman volume ls

# Inspect volume
podman volume inspect kuiperdb-data
podman volume inspect kuiperdb-logs

# Backup volumes
.\scripts\podman-backup.ps1 -VolumeName kuiperdb-data
```

## Production Deployment

### Linux with systemd

1. Generate service file:
```powershell
.\scripts\podman-systemd.ps1
```

2. Install on Linux:
```bash
sudo cp KuiperDb.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable KuiperDb
sudo systemctl start KuiperDb
```

3. Monitor:
```bash
sudo systemctl status KuiperDb
sudo journalctl -u KuiperDb -f
```

### Backup Strategy

1. **Daily backups**:
```powershell
# Schedule with Task Scheduler (Windows) or cron (Linux)
.\scripts\podman-backup.ps1
```

2. **Keep multiple generations**:
- Backups include timestamp: `kuiperdb-backup-YYYYMMDD-HHMMSS.tar.gz`
- Implement retention policy (e.g., keep last 30 days)

3. **Test restores regularly**:
```powershell
.\scripts\podman-restore.ps1 -BackupPath ".\backups\kuiperdb-backup-20260203-235800.tar.gz"
```

## Log Analysis

### Query JSON Logs with jq

```bash
# Filter by level
cat logs/KuiperDb.log | jq 'select(.level == "ERROR")'

# Extract specific fields
cat logs/KuiperDb.log | jq '{time: .timestamp, msg: .fields.message}'

# Search for operations
cat logs/KuiperDb.log | jq 'select(.span.name == "store_document")'

# Count errors
cat logs/KuiperDb.log | jq 'select(.level == "ERROR")' | wc -l
```

### Access Logs in Container

```powershell
# View live logs
podman logs -f KuiperDb

# Copy logs from container
podman cp KuiperDb:/app/logs ./container-logs

# Exec into container
podman exec -it KuiperDb sh
cd /app/logs
ls -la
```

## Architecture Decisions

### Why Azure Linux 3.0?
- Latest official Azure Linux image
- Replaces deprecated CBL-Mariner
- Minimal attack surface
- Microsoft-supported

### Why Named Volumes?
- Portable across hosts
- Managed by Podman/Docker
- Better performance than bind mounts
- Easier backup/restore

### Why Tracing + File Logs?
- Structured data for analysis
- Non-blocking async writes
- Automatic rotation
- Zero-cost abstractions in Rust
- Easy integration with log aggregators

### Why PowerShell Scripts?
- Cross-platform (Windows, Linux, macOS)
- Rich error handling
- Built into Windows
- Human-readable syntax

## Future Enhancements

### OpenTelemetry Collector Integration
To send telemetry to a collector, update `telemetry.rs`:

```rust
use opentelemetry_otlp::WithExportConfig;

let otlp_exporter = opentelemetry_otlp::new_exporter()
    .tonic()
    .with_endpoint("http://otel-collector:4317");
    
// Add OTLP layer to subscriber
```

### Metrics Collection
Add metrics instrumentation:
```rust
use opentelemetry::metrics::Counter;

let request_counter = meter
    .u64_counter("requests_total")
    .with_description("Total number of requests")
    .init();
```

### Distributed Tracing
Enable trace propagation across services:
```rust
use opentelemetry::propagation::TextMapPropagator;
```

## Documentation

- **Main README**: `README.md` - Project overview
- **Telemetry**: `TELEMETRY.md` - Logging guide
- **Scripts**: `scripts/README.md` - Script documentation
- **Build**: `BUILD.md` - Build instructions
- **Docker**: `Dockerfile` - Container definition

## Testing Checklist

- [x] Docker image builds successfully
- [x] Container starts with health checks
- [x] Logs written to file in JSON format
- [x] Console logs display correctly
- [x] Volumes persist data across restarts
- [x] Backup/restore works
- [ ] Health monitoring script runs
- [ ] systemd service starts (Linux only)
- [ ] Auto-restart works on failure

## Support

For issues or questions:
1. Check logs: `podman logs KuiperDb` or `./logs/KuiperDb.log`
2. Run health check: `.\scripts\podman-health.ps1`
3. Review telemetry docs: `TELEMETRY.md`
4. Check script help: `Get-Help .\scripts\podman-run.ps1 -Full`

---

**Built with:**
- Rust 2021 Edition
- Azure Linux 3.0
- OpenTelemetry / tracing
- Podman
- PowerShell 7+
