# KuiperDb Build & Development Guide

## Quick Start

### Development Build
```bash
# Build entire workspace in debug mode
cargo build --workspace

# Build only the server binary
cargo build --bin kuiperdb-server

# Build with optimizations
cargo build --release --bin kuiperdb-server
```

### Running the Server
```bash
# Development mode
cargo run

# Release mode
cargo run --release

# Or run the binary directly
./target/release/kuiperdb-server
```

## Build Scripts

### Linux/macOS
```bash
./scripts/build.sh
```
Output: `target/release/kuiperdb-server`

### Windows
```powershell
.\scripts\build.ps1
```
Output: `target\release\kuiperdb-server.exe`

## Development Commands

### Testing
```bash
# Run all tests in workspace
cargo test --workspace

# Run tests for specific crate
cargo test -p kuiperdb-core
cargo test -p kuiperdb-rs
cargo test -p kuiperdb-server
```

### Code Quality
```bash
# Format code
cargo fmt --all

# Check formatting
cargo fmt --all -- --check

# Run clippy (linter)
cargo clippy --workspace

# Fix clippy warnings
cargo clippy --workspace --fix
```

### Building Individual Crates
```bash
# Build core library
cargo build -p kuiperdb-core

# Build client library
cargo build -p kuiperdb-client

# Build main binary
cargo build -p kuiperdb
```

## Cross-Compilation

### Linux to Windows
```bash
# Install target
rustup target add x86_64-pc-windows-gnu

# Build
cargo build --release --target x86_64-pc-windows-gnu --bin kuiperdb-server
```

### Windows to Linux
```powershell
# Install target
rustup target add x86_64-unknown-linux-gnu

# Build (requires cross-compilation toolchain)
cargo build --release --target x86_64-unknown-linux-gnu --bin kuiperdb-server
```

## Release Build

### Optimized Release
```bash
cargo build --release --bin kuiperdb-server
strip target/release/kuiperdb-server  # Linux/macOS only
```

### Profile-Guided Optimization (PGO)
```bash
# Step 1: Build instrumented binary
RUSTFLAGS="-Cprofile-generate=/tmp/pgo-data" cargo build --release

# Step 2: Run workload to generate profile data
./target/release/KuiperDb
# ... use the application ...

# Step 3: Build with PGO
RUSTFLAGS="-Cprofile-use=/tmp/pgo-data" cargo build --release
```

## CI/CD

The project uses GitHub Actions for continuous integration:
- **On PR/Push**: Build and test all platforms
- **On Tag**: Create release with binaries for Windows and Linux

Workflow file: `.github/workflows/rust.yml`

## Dependencies

### Update Dependencies
```bash
# Check for outdated dependencies
cargo outdated

# Update all dependencies
cargo update

# Update specific dependency
cargo update -p <package-name>
```

### Audit Dependencies
```bash
# Install cargo-audit
cargo install cargo-audit

# Check for security vulnerabilities
cargo audit
```

## Workspace Structure

```
KuiperDb/
├── kuiperdb-core/       # Core library (storage, indexing, search)
├── kuiperdb-client/     # HTTP client library
├── KuiperDb/            # Main binary & server
└── target/           # Build artifacts
```

## Troubleshooting

### Clean Build
```bash
# Remove all build artifacts
cargo clean

# Rebuild everything
cargo build --workspace --release
```

### Dependency Issues
```bash
# Update Cargo.lock
cargo update

# Force rebuild of all dependencies
cargo clean && cargo build --workspace
```

### Windows-Specific Issues

If you encounter linker errors on Windows, ensure you have:
1. Visual Studio Build Tools installed
2. Rust MSVC toolchain: `rustup default stable-msvc`

## Performance

### Build Time Optimization
```bash
# Use mold linker (Linux)
sudo apt install mold
export RUSTFLAGS="-C link-arg=-fuse-ld=mold"

# Use lld linker
export RUSTFLAGS="-C link-arg=-fuse-ld=lld"

# Parallel compilation (adjust based on CPU cores)
export CARGO_BUILD_JOBS=8
```

### Binary Size Optimization

Add to `Cargo.toml`:
```toml
[profile.release]
opt-level = "z"      # Optimize for size
lto = true           # Enable Link Time Optimization
codegen-units = 1    # Better optimization, slower compile
strip = true         # Strip symbols
```

## Documentation

### Generate Documentation
```bash
# Build documentation for entire workspace
cargo doc --workspace --no-deps

# Open in browser
cargo doc --workspace --no-deps --open
```

## Benchmarking

```bash
# Run benchmarks
cargo bench

# Specific benchmark
cargo bench --bench <benchmark-name>
```
