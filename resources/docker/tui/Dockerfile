# Minimal TUI-only Docker image
FROM rust:1.75-alpine AS builder

RUN apk add --no-cache musl-dev openssl-dev pkgconfig

WORKDIR /app

# Build with musl for static linking
COPY Cargo.toml Cargo.lock ./
COPY core/ ./core/
COPY tui/ ./tui/

RUN cargo build --release --target x86_64-unknown-linux-musl -p easyssh-tui

# Scratch image with minimal dependencies
FROM alpine:latest AS runtime

RUN apk add --no-cache ca-certificates openssh-client

COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/easyssh /usr/local/bin/

# Create non-root user
RUN adduser -D -u 1000 easyssh
USER easyssh
WORKDIR /home/easyssh

ENV TERM=xterm-256color
ENV RUST_LOG=info

ENTRYPOINT ["easyssh"]
