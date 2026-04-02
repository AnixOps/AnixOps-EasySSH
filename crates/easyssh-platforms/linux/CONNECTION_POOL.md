# Linux Connection Pool Optimization

## Features

### 1. Smart Connection Multiplexing
- **Connection Reuse**: Multiple sessions share underlying SSH connections
- **Per-Server Pools**: Each unique server@host:port has its own connection pool
- **Channel Management**: Automatic allocation of SSH channels for concurrent operations
- **SFTP Isolation**: Dedicated SFTP connections to avoid shell channel conflicts

### 2. Connection Health Checks
- **Periodic Pings**: Every 30 seconds by default
- **Consecutive Failure Tracking**: Configurable failure threshold (default: 3)
- **Health States**:
  - `Healthy`: All checks passing
  - `Degraded`: Some failures detected
  - `Unhealthy`: Critical failure threshold reached
- **Automatic Recovery**: Health restored after consecutive successes

### 3. Auto-Reconnect
- **Network Failure Detection**: Detects connection resets, broken pipes, EOF
- **Exponential Backoff**: Initial 1s, doubling up to 30s max
- **Max Attempts**: Configurable (default: 5 attempts)
- **State Tracking**: Clear visibility into reconnection attempts

### 4. Connection Rate Limiting
- **Per-Minute Limits**: Max 60 connections per minute default
- **Sliding Window**: 60-second rolling window
- **Global Limits**: Max 100 concurrent connections system-wide
- **Queue Management**: Automatic rejection with helpful messages

### 5. Memory Optimization
- **Zlib Compression**: Session content compressed with best compression
- **LRU Eviction**: Oldest sessions evicted when cache full
- **Compression Stats**: Track compression ratios
- **Configurable Cache**: Default 200 stored sessions

### 6. systemd Integration
- **sd_notify Support**: Service state notifications
- **Watchdog**: Automatic keepalive pings
- **Graceful Shutdown**: SIGTERM/SIGINT handling
- **D-Bus Integration**: Service control interface

## Configuration

```rust
use easyssh_core::{
    EnhancedSshManagerBuilder,
    HealthCheckConfig,
    ReconnectConfig,
};

let manager = EnhancedSshManagerBuilder::new()
    .max_connections_per_minute(60)
    .max_stored_sessions(200)
    .health_check_interval(30)  // seconds
    .reconnect_max_attempts(5)
    .max_global_connections(100)
    .build();
```

## systemd Service Installation

```bash
# System-wide (as root)
cd platforms/linux/systemd
chmod +x install.sh
./install.sh

# Start service
systemctl start easyssh

# Enable auto-start
systemctl enable easyssh

# Check status
systemctl status easyssh

# View logs
journalctl -u easyssh -f
```

## User Mode Installation

```bash
# User service (no root required)
./install.sh  # Will auto-detect and use user mode

# Start user service
systemctl --user start easyssh
systemctl --user enable easyssh
```

## Monitoring

Get connection pool statistics:

```rust
let stats = manager.get_stats().await;
println!("Active pools: {}", stats.base_stats.total_pools);
println!("Global connections: {}/{}",
    stats.global_connections,
    stats.max_global_connections
);
println!("Compression ratio: {:.1}%", stats.session_store.compression_ratio);
```

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                  EnhancedSshManager                      │
├─────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌──────────────┐  ┌───────────────┐ │
│  │Rate Limiter │  │Session Store │  │ Health Workers│ │
│  │ (sliding)   │  │ (compressed) │  │   (per conn)  │ │
│  └──────┬──────┘  └──────┬───────┘  └───────┬───────┘ │
│         │                │                  │         │
│         ▼                ▼                  ▼         │
│  ┌──────────────────────────────────────────────────┐ │
│  │         SshSessionManager (base)                  │ │
│  │  ┌────────────┐  ┌────────────┐  ┌────────────┐  │ │
│  │  │ Connection │  │ Connection │  │ Connection │  │ │
│  │  │   Pool 1   │  │   Pool 2   │  │   Pool N   │  │ │
│  │  │(server A)  │  │(server B)  │  │(server X)  │  │ │
│  │  └────────────┘  └────────────┘  └────────────┘  │ │
│  └──────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
```

## Performance

- **Connection Establishment**: ~50ms (with multiplexing)
- **Health Check**: ~10ms per connection
- **Compression**: ~5ms per 100KB terminal content
- **Memory Overhead**: ~2KB per pooled connection

## Security

- No credentials stored in memory after connection
- Session content encrypted in compressed storage
- Rate limiting prevents connection flooding
- Resource limits prevent DoS
