# Multi-stage build for EasySSH Lite
# Stage 1: Builder
FROM rust:1.75-bookworm AS builder

WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Install cargo-chef for dependency caching
RUN cargo install cargo-chef

# Prepare recipe
COPY Cargo.toml Cargo.lock ./
COPY core/Cargo.toml ./core/
COPY tui/Cargo.toml ./tui/
RUN cargo chef prepare --recipe-path recipe.json

# Build dependencies (cached layer)
RUN cargo chef cook --release --recipe-path recipe.json

# Copy source and build
COPY . .
RUN cargo build --release -p easyssh-core -p easyssh-tui

# Stage 2: Runtime
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    openssh-client \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 easyssh

# Copy binaries
COPY --from=builder /app/target/release/easyssh /usr/local/bin/
COPY --from=builder /app/target/release/libeasyssh_core.so* /usr/local/lib/ || true

USER easyssh
WORKDIR /home/easyssh

ENV RUST_LOG=info
EXPOSE 8080

ENTRYPOINT ["easyssh"]
CMD ["--help"]
