# EasySSH Lite - Lightweight Alpine-based Docker Image
# Minimal size for embedded and containerized deployments

# ============================================================================
# Build Stage
# ============================================================================
FROM rust:1.75-alpine as builder

# Install build dependencies
RUN apk add --no-cache \
    musl-dev \
    openssl-dev \
    openssl-libs-static \
    pkgconfig \
    libssh2-dev \
    zlib-dev \
    clang \
    llvm

# Set environment for static linking
ENV RUSTFLAGS="-C target-feature=+crt-static"
ENV OPENSSL_STATIC=1
ENV OPENSSL_LIB_DIR=/usr/lib
ENV OPENSSL_INCLUDE_DIR=/usr/include

# Create app directory
WORKDIR /usr/src/easyssh

# Copy manifests
COPY Cargo.toml Cargo.lock ./
COPY crates/easyssh-core/Cargo.toml crates/easyssh-core/
COPY crates/easyssh-tui/Cargo.toml crates/easyssh-tui/

# Copy source
COPY crates/easyssh-core/src crates/easyssh-core/src
COPY crates/easyssh-tui/src crates/easyssh-tui/src

# Build release binary (CLI only, no UI)
WORKDIR /usr/src/easyssh/crates/easyssh-tui
RUN cargo build --release --features lite --no-default-features

# Strip binary
RUN strip target/release/easyssh-tui

# ============================================================================
# Runtime Stage
# ============================================================================
FROM alpine:3.19 as runtime

# Install runtime dependencies
RUN apk add --no-cache \
    ca-certificates \
    openssh-client

# Create non-root user
RUN addgroup -g 1000 easyssh && adduser -u 1000 -G easyssh -s /bin/sh -D easyssh

# Create config directory
RUN mkdir -p /home/easyssh/.easyssh && chown easyssh:easyssh /home/easyssh/.easyssh

# Copy binary
COPY --from=builder /usr/src/easyssh/crates/easyssh-tui/target/release/easyssh-tui /usr/local/bin/easyssh-lite

# Set permissions
RUN chmod +x /usr/local/bin/easyssh-lite && chown easyssh:easyssh /usr/local/bin/easyssh-lite

# Switch to non-root user
USER easyssh

# Set working directory
WORKDIR /home/easyssh

# Health check
HEALTHCHECK --interval=60s --timeout=10s --start-period=5s --retries=3 \
    CMD easyssh-lite --version || exit 1

# Run TUI
ENTRYPOINT ["easyssh-lite"]
