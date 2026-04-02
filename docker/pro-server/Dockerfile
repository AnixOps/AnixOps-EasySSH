# EasySSH Pro Server Dockerfile
# Multi-stage build for optimized production image

# ============================================================================
# Build Stage
# ============================================================================
FROM rust:1.75-slim-bookworm as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    cmake \
    clang \
    llvm \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /usr/src/easyssh

# Copy manifests first for caching
COPY Cargo.toml Cargo.lock ./
COPY crates/easyssh-core/Cargo.toml crates/easyssh-core/
COPY crates/easyssh-pro-server/Cargo.toml crates/easyssh-pro-server/

# Copy source code
COPY crates/easyssh-core/src crates/easyssh-core/src
COPY crates/easyssh-pro-server/src crates/easyssh-pro-server/src

# Build release binary
WORKDIR /usr/src/easyssh/crates/easyssh-pro-server
RUN cargo build --release --features pro

# Strip binary to reduce size
RUN strip target/release/easyssh-pro-server

# ============================================================================
# Runtime Stage
# ============================================================================
FROM debian:bookworm-slim as runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

# Create non-root user
RUN groupadd -r easyssh && useradd -r -g easyssh easyssh

# Create data directory
RUN mkdir -p /data/easyssh && chown easyssh:easyssh /data/easyssh

# Copy binary from builder
COPY --from=builder /usr/src/easyssh/crates/easyssh-pro-server/target/release/easyssh-pro-server /usr/local/bin/

# Set permissions
RUN chmod +x /usr/local/bin/easyssh-pro-server && chown easyssh:easyssh /usr/local/bin/easyssh-pro-server

# Switch to non-root user
USER easyssh

# Expose ports
EXPOSE 8080
EXPOSE 8443

# Volume for persistent data
VOLUME ["/data/easyssh"]

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:8080/health || exit 1

# Set environment
ENV RUST_LOG=info
ENV EASYSSH_DATA_DIR=/data/easyssh

# Run
ENTRYPOINT ["easyssh-pro-server"]
