# system-estv

Ultra-lightweight, self-hosted monitoring dashboard for a VPS. Single Rust binary with embedded frontend provides real-time CPU, RAM, SWAP, disk, network, and process metrics. Optimized for low-memory environments running local LLMs.

## Architecture

```
Host /proc & /sys (read-only)
        ↓
rust-exporter (collects every 2s)
        ↓ in-memory history (60 points)
/api/metrics (JSON)
        ↓
Browser polls every 2s
        ↓
Tailwind + Chart.js dashboard
```

| Component | Purpose | RAM |
|-----------|---------|-----|
| rust-exporter | All-in-one: metrics collector + web server + embedded frontend | <25MB |
| Caddy (host) | Reverse proxy with HTTPS and BasicAuth | <50MB |

**Total**: ~75MB idle (saves ~175MB vs VictoriaMetrics + Grafana stack)

## Features

- **Real-time Metrics**: CPU %, RAM, SWAP, Disk, Network I/O, Processes, Uptime
- **CPU Temperature**: Thermal sensor reading (when available)
- **Top 5 Processes**: Ranked by CPU usage with RAM consumption
- **Live Charts**: System activity, network traffic, memory usage
- **Zero Dependencies**: Tailwind CSS and Chart.js via CDN
- **Embedded Frontend**: Single binary, no static file serving needed

## Quick Start

### Prerequisites

- Docker + Docker Compose
- Caddy installed on host (for HTTPS + BasicAuth)
- Domain pointing to VPS (`system.estv.fr`)

### Deploy

```bash
# 1. Clone repository
git clone https://github.com/yourorg/system-estv.git
cd system-estv

# 2. Build and start
docker compose build
docker compose up -d

# 3. Verify
docker compose ps
curl http://127.0.0.1:8080/health

# 4. Configure Caddy (example)
# system.estv.fr {
#     basicauth * {
#         admin $2a$14$YOUR_BCRYPT_HASH
#     }
#     reverse_proxy 127.0.0.1:8080
# }

# 5. Access dashboard
# Visit https://system.estv.fr
```

## Configuration

### Caddy BasicAuth

Generate hash on host:
```bash
caddy hash-password --plaintext 'YOUR_PASSWORD'
```

Add to Caddyfile:
```caddy
system.estv.fr {
    basicauth * {
        admin $2a$14$YOUR_HASH_HERE
    }
    reverse_proxy 127.0.0.1:8080
}
```

### Polling Interval

Edit `rust-exporter/src/collector.rs`:
```rust
tokio::time::sleep(std::time::Duration::from_secs(2)).await;
```

Update frontend `index.html`:
```javascript
setInterval(fetchMetrics, 2000);
```

### History Length

Edit `rust-exporter/src/state.rs`:
```rust
pub const HISTORY_LENGTH: usize = 60;
```

## Exposed Metrics

JSON API at `/api/metrics`:

```json
{
  "current": {
    "cpu_percent": 23.5,
    "ram_used_gb": 1.2,
    "ram_total_gb": 3.8,
    "ram_percent": 31.6,
    "swap_used_gb": 0.5,
    "swap_total_gb": 2.0,
    "swap_percent": 25.0,
    "disk_used_gb": 100.0,
    "disk_total_gb": 200.0,
    "disk_free_gb": 100.0,
    "disk_percent": 50.0,
    "processes": 89,
    "uptime_secs": 86400,
    "net_rx_mbps": 1.45,
    "net_tx_mbps": 0.23,
    "cpu_temp_celsius": 45.0,
    "top_processes": [
      {"name": "process1", "cpu_percent": 5.2, "ram_mb": 150}
    ]
  },
  "history": {
    "timestamps": [2, 4, 6, ...],
    "cpu": [22.1, 23.5, ...],
    "ram": [1.1, 1.2, ...],
    "swap": [0.4, 0.5, ...],
    "net_rx": [1.2, 1.45, ...],
    "net_tx": [0.2, 0.23, ...],
    "cpu_temp": [44.0, 45.0, ...]
  }
}
```

## Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/` | GET | Dashboard (embedded HTML) |
| `/api/metrics` | GET | Current metrics + history (JSON) |
| `/health` | GET | Health check (200 OK) |

## Development

### Local Rust Development

```bash
cd rust-exporter
cargo run              # Run locally on :8080
cargo test             # Run tests
cargo clippy           # Lint
cargo build --release  # Optimized build
```

### Rebuild Container

```bash
docker compose build
docker compose up -d
```

### View Logs

```bash
docker compose logs -f rust-exporter
```

## Security

- **network_mode: host**: Required for accurate network I/O stats
- **No persistent storage**: All metrics are in-memory, no data at rest
- **Caddy handles auth**: rust-exporter has no authentication
- **Read-only mounts**: `/proc` and `/sys` mounted read-only

## Troubleshooting

### Network Traffic Shows Zero

```bash
# Verify host network mode is active
docker inspect rust-exporter --format='{{.HostConfig.NetworkMode}}'
# Should return: host
```

### Container Won't Start

```bash
docker compose logs rust-exporter
```

### Metrics All Zero

Verify read-only mounts exist:
```yaml
volumes:
  - /proc:/proc:ro
  - /sys:/sys:ro
```

### CPU Temperature Not Showing

- VPS platforms often don't expose thermal sensors
- The app gracefully handles missing sensors (`cpu_temp_celsius: null`)

## Resource Monitoring

```bash
docker stats rust-exporter
```

Expected: ~17-25MB RAM, <1% CPU idle

## Documentation

- [CONTEXT.md](documentation/CONTEXT.md) - Project vision and design principles
- [ARCHITECTURE.md](documentation/ARCHITECTURE.md) - Detailed system architecture
- [TODO.md](documentation/TODO.md) - Deployment checklist

## License

MIT