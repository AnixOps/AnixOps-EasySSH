# Multi-stage build for EasySSH Standard with Web UI
FROM node:22-bookworm AS web-builder

WORKDIR /app/web
COPY apps/web/package*.json ./
RUN npm ci
COPY apps/web/ ./
RUN npm run build

FROM rust:1.75-bookworm AS rust-builder

WORKDIR /app

# Install system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libgtk-4-dev \
    libadwaita-1-dev \
    && rm -rf /var/lib/apt/lists/*

# Build Rust components
COPY Cargo.toml Cargo.lock ./
COPY core/ ./core/
COPY platforms/linux/easyssh-gtk4/ ./platforms/linux/easyssh-gtk4/
RUN cargo build --release -p easyssh-core

FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libgtk-4-1 \
    libadwaita-1-0 \
    openssh-client \
    && rm -rf /var/lib/apt/lists/*

COPY --from=rust-builder /app/target/release/libeasyssh_core.so* /usr/local/lib/
COPY --from=web-builder /app/web/dist/ /usr/share/easyssh/web/

ENV RUST_LOG=info
ENV WEB_ASSETS=/usr/share/easyssh/web

EXPOSE 8080 3000

ENTRYPOINT ["/usr/local/bin/easyssh-standard"]
