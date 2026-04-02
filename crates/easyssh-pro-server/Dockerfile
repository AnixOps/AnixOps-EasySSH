# Build stage
FROM rust:1.75 as builder

WORKDIR /app

# Copy Cargo files
COPY Cargo.toml Cargo.lock ./
COPY pro-server/Cargo.toml ./pro-server/
COPY core/Cargo.toml ./core/
COPY tui/Cargo.toml ./tui/

# Copy source code
COPY pro-server/src ./pro-server/src
COPY core/src ./core/src
COPY tui/src ./tui/src

# Build release binary
RUN cargo build --release -p easyssh-pro-server

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    libssl3 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 easyssh

# Copy binary from builder
COPY --from=builder /app/target/release/pro-server /usr/local/bin/pro-server

# Set ownership
RUN chown easyssh:easyssh /usr/local/bin/pro-server

USER easyssh

WORKDIR /home/easyssh

# Expose port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:8080/api/v1/health || exit 1

# Run the server
CMD ["pro-server"]