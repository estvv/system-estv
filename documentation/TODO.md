# Deployment Checklist (TODO.md)

## Phase 1: Prerequisites

- [ ] Verify Docker and Docker Compose are installed on VPS
- [ ] Verify ports 80 and 443 are not in use by other services
- [ ] Verify DNS records point to VPS:
  - [ ] system.estv.fr → VPS IP address
- [ ] Create `.env` file with configuration (copy from `.env.example`)

## Phase 2: Configuration

- [ ] Generate BasicAuth password hash for Caddy
  ```bash
  docker run --rm caddy:latest caddy hash-password --plaintext 'YOUR_PASSWORD'
  ```
- [ ] Update `caddy/Caddyfile` with hashed password (replace `$2a$...` placeholder)
- [ ] Review `config/vmagent.yml` scrape interval (default: 10s)
- [ ] (Optional) Adjust retention period in VictoriaMetrics (default: 12 months)

## Phase 3: Build and Deploy

- [ ] Clone/pull repository to VPS
- [ ] Build Docker images:
  ```bash
  docker compose build
  ```
- [ ] Start all services:
  ```bash
  docker compose up -d
  ```
- [ ] Verify all containers are running:
  ```bash
  docker compose ps
  ```

## Phase 4: Validation

- [ ] Check rust-exporter health:
  ```bash
  docker compose exec rust-exporter curl -f http://localhost:8080/health
  ```
- [ ] Check rust-exporter metrics:
  ```bash
  docker compose exec rust-exporter curl http://localhost:8080/metrics
  ```
- [ ] Verify VictoriaMetrics is scraping:
  ```bash
  docker compose exec victoriametrics curl http://localhost:8428/api/v1/query?query=up
  ```
- [ ] Access Grafana at https://system.estv.fr
- [ ] Login with BasicAuth credentials (Caddy layer)
- [ ] Login to Grafana admin (password in logs or custom)
- [ ] Add VictoriaMetrics as datasource (URL: http://victoriametrics:8428)
- [ ] Import or create dashboard for system metrics

## Phase 5: Post-Deployment

- [ ] Configure Grafana dashboard to use VictoriaMetrics datasource
- [ ] Create dashboard panels for:
  - [ ] CPU usage (node_cpu_usage_percent)
  - [ ] Memory usage (node_memory_used_bytes / node_memory_total_bytes)
  - [ ] Disk usage (node_disk_used_bytes / node_disk_total_bytes)
- [ ] (Optional) Configure Grafana alerts for high CPU/memory usage
- [ ] (Optional) Set up volume backups for victoriametrics_data and grafana_data
- [ ] (Optional) Configure log rotation if needed

## Phase 6: Security Hardening

- [ ] Verify no ports are exposed directly to host (except 80/443 via Caddy)
  ```bash
  docker compose ps  # Should show no host ports for rust-exporter, victoriametrics, grafana
  ```
- [ ] Test BasicAuth enforcement:
  ```bash
  curl -I https://system.estv.fr  # Should return 401
  curl -I -u user:password https://system.estv.fr  # Should return 200
  ```
- [ ] Review UFW/firewall rules (only 22, 80, 443 should be open)
- [ ] (Optional) Configure fail2ban for additional protection

## Phase 7: Monitoring

- [ ] Verify Grafana dashboard displays real-time data
- [ ] Check VictoriaMetrics storage size growth over 24h
- [ ] Monitor container resource usage:
  ```bash
  docker stats
  ```
- [ ] Expected total RAM usage: <250MB

## Troubleshooting

### Container Won't Start

```bash
docker compose logs <service-name>
```

### Caddy Certificate Issues

```bash
docker compose logs caddy
```

### Metrics Not Appearing

```bash
# Check rust-exporter logs
docker compose logs rust-exporter

# Check VictoriaMetrics scrape status
docker compose exec victoriametrics curl http://localhost:8428/api/v1/targets
```

### Grafana Can't Connect to VictoriaMetrics

```bash
# Verify network connectivity
docker compose exec grafana ping victoriametrics
docker compose exec grafana curl http://victoriametrics:8428/health
```

### BasicAuth Not Working

```bash
# Verify Caddyfile syntax
docker compose exec caddy caddy validate --config /etc/caddy/Caddyfile

# Re-generate password hash
docker compose exec caddy caddy hash-password --plaintext 'YOUR_PASSWORD'
```

## Rollback

```bash
# Stop all services
docker compose down

# Revert to previous version
git checkout <previous-commit>

# Rebuild and restart
docker compose build
docker compose up -d
```