# Deployment Checklist (TODO.md)

## Phase 1: Prerequisites

- [x] Verify Docker and Docker Compose are installed on VPS
- [x] Verify Caddy is installed on host OS (not containerized)
- [x] Verify ports 80 and 443 are not in use by other services
- [x] Verify DNS records point to VPS:
  - [x] system.estv.fr → VPS IP address
- [x] Configure Caddy to proxy system.estv.fr → 127.0.0.1:3001 with BasicAuth

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
  Expected: `rust-exporter listening on 0.0.0.0:3000`

- [ ] Test health endpoint:
  ```bash
  curl -f http://127.0.0.1:3001/health
  ```
  Expected: Empty response, status 200

- [ ] Test API endpoint:
  ```bash
  curl http://127.0.0.1:3001/api/metrics | jq
  ```
  Expected: JSON with `current` and `history` objects

- [ ] Test dashboard HTML:
  ```bash
  curl http://127.0.0.1:3001/
  ```
  Expected: HTML content with Tailwind/Chart.js references

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
  - [ ] CPU Usage %
  - [ ] RAM Used / Total GB
  - [ ] Disk Free GB
  - [ ] Processes count
  - [ ] Uptime
  - [ ] Network RX/TX MB/s
- [ ] Verify charts populate with data points over time
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