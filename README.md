# system-estv

Lightweight self-hosted monitoring stack for a single VPS. Exposes CPU, RAM, and disk metrics via a custom Rust exporter, stores time-series data in VictoriaMetrics, and visualizes through Grafana—all behind Caddy with automatic HTTPS and BasicAuth.

## Architecture

```
Internet → Caddy (443) → BasicAuth → Grafana → VictoriaMetrics → rust-exporter
```

| Component | Purpose | RAM |
|-----------|---------|-----|
| rust-exporter | Custom metrics collector (Rust + axum) | <15MB |
| VictoriaMetrics | Time-series database (Prometheus-compatible) | <100MB |
| Grafana | Dashboard visualization | <80MB |
| Caddy | Reverse proxy, TLS, authentication | <50MB |

**Total**: ~250MB idle

## Quick Start

### Prerequisites

- Docker + Docker Compose
- Domain pointing to VPS (`system.estv.fr`)
- Ports 80/443 available

### Deploy

```bash
# 1. Clone repository
git clone https://github.com/yourorg/system-estv.git
cd system-estv

# 2. Generate BasicAuth password hash
docker run --rm caddy:latest caddy hash-password --plaintext 'YOUR_PASSWORD'
# Copy the output hash

# 3. Update caddy/Caddyfile - replace BCRYPT_HASH with generated hash

# 4. Build and start
docker compose build
docker compose up -d

# 5. Verify services
docker compose ps
docker compose exec rust-exporter curl http://localhost:8080/health

# 6. Access Grafana
# Visit https://system.estv.fr
# Login with BasicAuth credentials
```

## Configuration

### BasicAuth

Edit `caddy/Caddyfile`:

```caddy
basicauth * {
    admin $2a$14$YOUR_BCRYPT_HASH_HERE
}
```

Generate hash:
```bash
docker exec caddy caddy hash-password --plaintext 'YOUR_PASSWORD'
```

### Grafana Datasource

Grafana auto-connects to VictoriaMetrics at `http://victoriametrics:8428`. No manual configuration needed if using the provided `docker-compose.yml`.

### Scrape Interval

Edit `config/vmagent.yml`:

```yaml
global:
  scrape_interval: 10s  # Adjust as needed
```

### Metrics Retention

Edit `docker-compose.yml` VictoriaMetrics command:

```yaml
command:
  - -retentionPeriod=12  # Months (default: 12)
```

## Exposed Metrics

rust-exporter provides Prometheus-compatible metrics:

| Metric | Type | Description |
|--------|------|-------------|
| `sys_cpu_usage_percent` | gauge | CPU utilization (0-100) |
| `sys_mem_used_bytes` | gauge | RAM used |
| `sys_mem_total_bytes` | gauge | Total RAM |
| `sys_processes_total` | gauge | Total number of processes |
| `sys_disk_free_bytes` | gauge | Free disk space (per mount) |
| `sys_network_rx_bytes_total` | counter | Network bytes received (cumulative) |
| `sys_network_tx_bytes_total` | counter | Network bytes transmitted (cumulative) |
| `sys_uptime_seconds` | gauge | System uptime in seconds |

Query examples:
```promql
# RAM usage percentage
100 * sys_mem_used_bytes / sys_mem_total_bytes

# Network download speed (bytes/sec)
rate(sys_network_rx_bytes_total[1m])

# Network upload speed (bytes/sec)
rate(sys_network_tx_bytes_total[1m])
```

## Development

### Local Rust Development

```bash
cd rust-exporter
cargo run              # Run locally on :8080
cargo test             # Run tests
cargo build --release  # Optimized build
```

### Rebuild Container

```bash
docker compose build rust-exporter
docker compose up -d rust-exporter
```

### View Logs

```bash
docker compose logs -f rust-exporter
docker compose logs -f victoriametrics
docker compose logs -f grafana
docker compose logs -f caddy
```

## Security

- **No exposed ports**: rust-exporter, VictoriaMetrics, and Grafana run on internal Docker network only
- **BasicAuth enforcement**: Caddy validates credentials before proxying to Grafana
- **Read-only mounts**: `/proc` and `/sys` mounted read-only to rust-exporter
- **Automatic HTTPS**: Caddy manages Let's Encrypt certificates

## Maintenance

### Backup

```bash
# Backup volumes
docker run --rm -v victoriametrics_data:/data -v $(pwd)/backup:/backup alpine tar czf /backup/victoriametrics.tar.gz /data
docker run --rm -v grafana_data:/data -v $(pwd)/backup:/backup alpine tar czf /backup/grafana.tar.gz /data
```

### Restore

```bash
docker compose down
docker run --rm -v victoriametrics_data:/data -v $(pwd)/backup:/backup alpine tar xzf /backup/victoriametrics.tar.gz -C /
docker compose up -d
```

### Resource Monitoring

```bash
docker stats  # Real-time container resource usage
```

## Troubleshooting

### Metrics Not Appearing

```bash
# Check exporter
docker compose exec rust-exporter curl http://localhost:8080/metrics

# Check VictoriaMetrics targets
docker compose exec victoriametrics curl http://localhost:8428/api/v1/targets
```

### Grafana Can't Connect

```bash
# Test internal connectivity
docker compose exec grafana curl http://victoriametrics:8428/health
```

### Certificate Issues

```bash
docker compose logs caddy
```

## Documentation

- [CONTEXT.md](documentation/CONTEXT.md) - Project vision and design principles
- [ARCHITECTURE.md](documentation/ARCHITECTURE.md) - Detailed system architecture
- [TODO.md](documentation/TODO.md) - Deployment checklist

## License

MIT