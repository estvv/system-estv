# Deployment Checklist (TODO.md) - v0.3.0

## Phase 1: Prerequisites

- [x] Verify Docker and Docker Compose are installed on VPS
- [x] Verify Caddy is installed on host OS (not containerized)
- [x] Verify ports 80 and 443 are not in use by other services
- [x] Verify DNS records point to VPS:
  - [x] system.estv.fr → VPS IP address
- [x] Configure Caddy to proxy system.estv.fr → 127.0.0.1:8080 with BasicAuth

## Phase 2: Configuration

- [ ] Verify Caddyfile includes BasicAuth for system.estv.fr
  ```text
  system.estv.fr {
      basicauth * {
          <username> <bcrypt_hash>
      }
      reverse_proxy 127.0.0.1:3001
  }
  ```
- [ ] Generate BasicAuth password hash if not set:
  ```bash
  caddy hash-password --plaintext 'YOUR_PASSWORD'
  ```

## Phase 3: Build and Deploy

- [ ] Pull latest code to VPS
- [ ] Build Docker image:
  ```bash
  docker compose build
  ```
- [ ] Start container:
  ```bash
  docker compose up -d
  ```
- [ ] Verify container is running:
  ```bash
  docker compose ps
  ```

## Phase 4: Validation

- [ ] Check container logs:
  ```bash
  docker compose logs rust-exporter
  ```
  Expected: `rust-exporter listening on 0.0.0.0:8080`

- [ ] Test health endpoint:
  ```bash
  curl -f http://127.0.0.1:8080/health
  ```
  Expected: Empty response, status 200

- [ ] Test API endpoint:
  ```bash
  curl http://127.0.0.1:8080/api/metrics | jq
  ```
  Expected: JSON with `current` and `history` objects containing:
  - `cpu_percent`, `cpu_temp_celsius`
  - `ram_used_gb`, `ram_total_gb`, `ram_percent`
  - `swap_used_gb`, `swap_total_gb`, `swap_percent`
  - `disk_used_gb`, `disk_total_gb`, `disk_free_gb`, `disk_percent`
  - `processes`, `top_processes` (array of top 5)
  - `net_rx_mbps`, `net_tx_mbps`
  - `uptime_secs`

- [ ] Test dashboard HTML:
  ```bash
  curl http://127.0.0.1:8080/
  ```
  Expected: HTML content with Tailwind/Chart.js references

- [ ] Verify network_mode is host:
  ```bash
  docker inspect rust-exporter --format='{{.HostConfig.NetworkMode}}'
  ```
  Expected: `host`

- [ ] Access via Caddy:
  ```bash
  curl -I https://system.estv.fr
  ```
  Expected: 401 Unauthorized (no auth header)

- [ ] Access with BasicAuth:
  ```bash
  curl -I -u username:password https://system.estv.fr
  ```
  Expected: 200 OK with HTML content

## Phase 5: Browser Testing

- [ ] Open https://system.estv.fr in browser
- [ ] Enter BasicAuth credentials
- [ ] Verify gauges display non-zero values after ~4 seconds:
  - [ ] CPU Usage % (with temperature if available)
  - [ ] RAM Used / Total GB + percentage
  - [ ] SWAP Used / Total GB + percentage
  - [ ] Disk Used / Total GB + percentage + free space
  - [ ] Processes count
  - [ ] Uptime
  - [ ] Network RX/TX MB/s (should show values with `network_mode: host`)
- [ ] Verify Top 5 Processes table shows process names, CPU%, and RAM
- [ ] Verify charts populate with data points over time:
  - [ ] System Activity (CPU %)
  - [ ] Network Traffic (RX/TX)
  - [ ] Memory & SWAP (RAM + SWAP)
- [ ] Verify "Last update" timestamp changes every 2 seconds

## Phase 6: Performance Check

- [ ] Monitor container resource usage:
  ```bash
  docker stats rust-exporter
  ```
  Expected: RAM < 30MB, CPU < 5%

- [ ] Check host memory savings:
  ```bash
  free -h
  ```
  Compare to previous VictoriaMetrics + Grafana setup (~150MB freed)

## Phase 7: Security Hardening

- [ ] Verify no unexpected host ports:
  ```bash
  docker compose ps
  ```
  Should only show: `127.0.0.1:3001->3000/tcp`

- [ ] Test that direct access to 3001 requires localhost:
  ```bash
  # From another machine (should fail):
  curl http://<vps-ip>:3001/health
  ```

- [ ] Verify UFW/firewall rules (only 22, 80, 443 open):
  ```bash
  sudo ufw status
  ```

## Troubleshooting

### Container Won't Start

```bash
docker compose logs rust-exporter
```

Common issues:
- Missing `/proc` or `/sys` mounts → verify docker-compose.yml volumes
- Port 3001 in use → `lsof -i :3001` to find conflicting process

### Dashboard Shows "Connection error"

1. Verify container: `docker compose ps`
2. Test health: `curl -f http://127.0.0.1:3001/health`
3. Check browser console (F12) for JavaScript errors
4. Verify Caddy proxy: `caddy validate --config /etc/caddy/Caddyfile` (on host)

### All Metrics Show Zero

1. Check `/proc` mount:
   ```bash
   docker compose exec rust-exporter ls /proc
   ```
   Should show directories like `cpuinfo`, `meminfo`, etc.

2. Check `/sys` mount:
   ```bash
   docker compose exec rust-exporter ls /sys
   ```

### Charts Not Updating

1. Open browser DevTools → Network tab
2. Check `/api/metrics` responses:
   - Status: 200 OK
   - Response: JSON with non-empty `history` arrays
3. If history arrays are empty, wait 10 seconds for data to accumulate

### BasicAuth Not Working

```bash
# Verify Caddyfile syntax (on host)
caddy validate --config /path/to/Caddyfile

# Reload Caddy
sudo systemctl reload caddy
```

## Rollback

```bash
# Stop container
docker compose down

# Revert to previous version
git checkout <previous-commit>

# Rebuild and restart
docker compose build && docker compose up -d
```

## Notes

- **No persistent data**: All metrics reset on container restart
- **Network speed accuracy**: First 2 seconds after start will show 0 MB/s (need previous tick for delta calculation)
- **History depth**: 60 data points at 2s intervals = 2 minutes of chart history
- **Browser compatibility**: Requires modern browser with JavaScript enabled
- **Network mode**: Uses `network_mode: host` to read actual host network stats (critical for VPS)
- **CPU temperature**: May not be available on all VPS platforms (displays "--" if unavailable)