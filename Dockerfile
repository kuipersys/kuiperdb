# Multi-stage build for kuiperdb using Azure Linux
# Optimized for resilience and minimal footprint

# Build stage
FROM mcr.microsoft.com/azurelinux/base/core:3.0 AS builder

# Install build dependencies
# Azure Linux uses tdnf package manager
RUN tdnf install -y \
    gcc \
    glibc-devel \
    kernel-headers \
    binutils \
    make \
    cmake \
    curl \
    ca-certificates \
    sqlite-devel \
    pkg-config \
    libgcc \
    nodejs \
    npm \
    && tdnf clean all

# Install Rust
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile minimal \
    && chmod -R a+w $RUSTUP_HOME $CARGO_HOME

# Set working directory
WORKDIR /build

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY src/kuiperdb-core ./src/kuiperdb-core
COPY src/kuiperdb-rs ./src/kuiperdb-rs
COPY src/kuiperdb-server ./src/kuiperdb-server

# Build release binary
RUN cargo build --release --bin kuiperdb-server

# Build TypeScript client first (needed by React package)
COPY src/kuiperdb-ts ./src/kuiperdb-ts
WORKDIR /build/src/kuiperdb-ts
RUN npm ci && npm run build

# Build React package (needed by UI)
WORKDIR /build
COPY src/kuiperdb-react ./src/kuiperdb-react
WORKDIR /build/src/kuiperdb-react
RUN npm ci && npm run build

# Build UI
WORKDIR /build
COPY src/kuiperdb-ui ./src/kuiperdb-ui
WORKDIR /build/src/kuiperdb-ui
RUN npm ci && npm run build
WORKDIR /build

# Runtime stage
FROM mcr.microsoft.com/azurelinux/base/core:3.0

# Install runtime dependencies
RUN tdnf install -y \
    sqlite-libs \
    ca-certificates \
    shadow-utils \
    && tdnf clean all

# Create non-root user for security
RUN groupadd -r kuiperdb && useradd -r -g kuiperdb kuiperdb

# Create directories for data and config
RUN mkdir -p /app/data /app/config /app/logs && \
    chown -R kuiperdb:kuiperdb /app

WORKDIR /app

# Copy binary from builder
COPY --from=builder /build/target/release/kuiperdb-server /app/kuiperdb-server

# Copy UI static files
COPY --from=builder /build/src/kuiperdb-ui/dist /app/static

# Copy config to working directory (not subdirectory)
COPY config.json /app/config.json
COPY schema.sql /app/schema.sql

# Set ownership
RUN chown -R kuiperdb:kuiperdb /app

# Switch to non-root user
USER kuiperdb

# Expose default port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD ["/bin/sh", "-c", "pidof kuiperdb-server || exit 1"]

# Volume for persistent data
VOLUME ["/app/data", "/app/logs"]

# Set environment variables
ENV RUST_LOG=info \
    DATA_DIR=/app/data \
    LOG_DIR=/app/logs

# Run the application
CMD ["/app/kuiperdb-server"]
