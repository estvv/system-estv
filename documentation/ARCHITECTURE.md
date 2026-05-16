# Architecture Overview (ARCHITECTURE.md)

## 1. System Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         Internet                                 │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Caddy (Reverse Proxy)                         │
│                    Ports: 80, 443                                │
│                    - TLS termination (Let's Encrypt)             │
│                    - BasicAuth for system.estv.fr                │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                    rust-exporter (All-in-One)                   │
│                    Container Port: 3000                          │
│                    Host Binding: 127.0.0.1:3001                   │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  Background Collector (2s interval)                      │  │
│  │  - CPU Usage %                                            │  │
│  │  - RAM Used/Total GB                                      │  │
│  │  - Disk Free GB (/ mount)                                 │  │
│  │  - Process Count                                          │  │
│  │  - System Uptime                                          │  │
│  │  - Network Speed (RX/TX MB/s)                             │  │
│  └──────────────────────────────────────────────────────────┘  │
│                              │                                   │
│                              ▼                                   │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  In-Memory Storage (Arc<RwLock>)                         │  │
│  │  - Live metrics (current snapshot)                        │  │
│  │  - Rolling history (60 data points)                       │  │
│  └──────────────────────────────────────────────────────────┘  │
│                              │                                   │
│                              ▼                                   │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  Axum Web Server                                          │  │
│  │  - GET /           → Embedded HTML dashboard              │  │
│  │  - GET /api/metrics→ JSON (current + history)             │  │
│  │  - GET /health     → Health check (200 OK)                │  │
│  └──────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

## 2. Component Details

### 2.1 rust-exporter (All-in-One Dashboard)

**Purpose**: Single-binary monitoring dashboard with embedded frontend

**Technology Stack**:
- Language: Rust (stable, musl target for scratch image)
- Web Framework: axum (minimal overhead, async)
- System Metrics: sysinfo crate
- Memory Target: <25MB RSS

**Key Features**:
- **Background Collection Loop**: Spawns on startup, runs every 2 seconds
- **Network Speed Calculation**: Computes delta bytes / elapsed time in MB/s
- **Rolling History**: Stores last 60 data points (2 minutes at 2s intervals)
- **Zero External Dependencies**: All frontend assets (Tailwind, Chart.js) via CDN
- **Embedded HTML**: Compiled into binary via `include_str!()`

**Endpoints**:

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/` | GET | Serves embedded index.html (Tailwind + Chart.js dashboard) |
| `/api/metrics` | GET | Returns JSON with current metrics + history arrays |
| `/health` | GET | Health check endpoint (returns 200 OK) |

**JSON Response Format (`GET /api/metrics`)**:
```json
{
  "current": {
    "cpu_percent": 23.5,
    "ram_used_gb": 1.2,
    "ram_total_gb": 3.8,
    "disk_free_gb": 45.2,
    "processes": 89,
    "uptime_secs": 86400,
    "net_rx_mbps": 1.45,
    "net_tx_mbps": 0.23
  },
  "history": {
    "timestamps": [0, 2, 4, 6, ...],
    "cpu": [22.1, 23.5, 21.8, ...],
    "ram": [1.1, 1.2, 1.15, ...],
    "net_rx": [1.2, 1.45, 1.3, ...],
    "net_tx": [0.2, 0.23, 0.18, ...]
  }
}
```

**Internal Architecture**:
- `AppState`: Shared state with `RwLock` for concurrent read/write
- `collector.rs`: Background tokio task collecting metrics every 2s
- `handlers.rs`: HTTP route handlers for `/`, `/api/metrics`, `/health`
- `state.rs`: Data structures for `LiveMetrics`, `HistoryPoint`, `History`

### 2.2 Caddy (Ingress Controller)

**Purpose**: TLS termination, reverse proxy, authentication

**Configuration**:
- Caddy is installed on host OS (not containerized)
- Proxies `https://system.estv.fr` → `http://127.0.0.1:3001`
- Enforces BasicAuth before forwarding requests

**Routes**:
| Domain | Target | Auth |
|--------|--------|------|
| system.estv.fr | 127.0.0.1:3001 | BasicAuth required |

**Security Headers**: Strict-Transport-Security, X-Frame-Options, X-Content-Type-Options

## 3. Network Architecture

### 3.1 Port Mapping

| Service | Host Port | Container Port | Exposed |
|---------|-----------|----------------|---------|
| rust-exporter | 127.0.0.1:3001 | 3000 | localhost only |

### 3.2 Host Volume Mounts

| Host Path | Container Path | Purpose |
|-----------|----------------|---------|
| `/proc` | `/proc` | Process/system metrics (read-only) |
| `/sys` | `/sys` | System metrics (read-only) |

## 4. Data Flow

### 4.1 Metrics Collection Flow

```
1. Background Task (rust-exporter, every 2s)
   ├─> Refreshes System struct via sysinfo
   ├─> Calculates CPU%, RAM, Disk, Processes, Uptime
   ├─> Calculates network speed:
   │      curr_rx_bytes, curr_tx_bytes (from /proc/net)
   │      delta = current - previous
   │      speed_mbps = (delta / elapsed_secs) / 1_000_000
   ├─> Aggregates ALL network interfaces
   └─> Updates AppState:
          - Arc<RwLock<LiveMetrics>> (current snapshot)
          - Arc<RwLock<VecDeque<HistoryPoint>>> (rolling history)

2. Frontend Polling (index.html, every 2s)
   ├─> GET /api/metrics → JSON response
   ├─> Updates gauges: CPU, RAM, Disk, Processes, Uptime, Network Speed
   └─> Updates Chart.js line charts with history arrays
```

### 4.2 User Request Flow

```
User → https://system.estv.fr
     → Caddy validates BasicAuth
     → Caddy proxies to 127.0.0.1:3001
     → rust-exporter serves embedded index.html
     → Dashboard polls /api/metrics every 2s
```

## 5. Security Model

### 5.1 Attack Surface

| Vector | Mitigation |
|--------|------------|
| Public ports | Only 80/443 via Caddy |
| Unauth'd access | BasicAuth at Caddy layer |
| Container escape | Non-root container (user 65534) |
| Host access | Read-only mounts for /proc, /sys |
| Data exposure | No persistent data (in-memory only) |

### 5.2 Authentication Flow

```
GET https://system.estv.fr
     │
     ├─> Caddy receives request
     ├─> Caddy checks BasicAuth header
     │   ├── Valid → Proxy to 127.0.0.1:3001
     │   └── Invalid/Missing → 401 Unauthorized
     └─> rust-exporter serves dashboard
```

### 5.3 Container Security

| Service | User | Capabilities | Read-Only Root |
|---------|------|--------------|----------------|
| rust-exporter | 65534:65534 | none | ✓ (scratch image) |

## 6. Resource Estimates

| Service | CPU | RAM (Idle) | RAM (Peak) | Disk I/O |
|---------|-----|------------|------------|----------|
| rust-exporter | 1m | 15MB | 25MB | Negligible |
| Caddy (host) | 1m | <10MB | 20MB | Negligible |
| **Total** | ~2m | ~25MB | ~45MB | — |

**Comparison to Previous Stack**:
- Before: rust-exporter (15MB) + VictoriaMetrics (100MB) + Grafana (80MB) = **~195MB**
- After: rust-exporter (25MB) + Caddy (20MB) = **~45MB**
- **Savings: ~150MB RAM**

## 7. Failure Modes

| Failure | Impact | Recovery |
|---------|--------|----------|
| rust-exporter crash | Dashboard unavailable | Auto-restart via Docker restart policy |
| Caddy crash | All external access blocked | Manual restart (host service) |
| Host reboot | Container down | Docker restart policy brings up container |

## 8. Storage

**No persistent volumes required.**

All metrics are stored in-memory:
- Current snapshot: One struct instance
- History: Fixed-size `VecDeque` capped at 60 elements
- Data resets on container restart

## 9. Monitoring

### 9.1 Health Check

```bash
curl -f http://127.0.0.1:3001/health
# Returns: 200 OK (empty body)
```

### 9.2 Test Dashboard

```bash
curl http://127.0.0.1:3001/
# Returns: HTML content

curl http://127.0.0.1:3001/api/metrics | jq
# Returns: JSON with current metrics and history
```

## 10. Extension Points

### 10.1 Adding New Metrics

1. Add new fields to `LiveMetrics` struct in `src/state.rs`
2. Update `collector.rs` to gather new metric
3. Update `index.html` frontend to display new metric
4. Rebuild: `docker compose build`

### 10.2 Changing Polling Interval

Edit `collector.rs`:
```rust
tokio::time::sleep(std::time::Duration::from_secs(2)).await;
```

Change `2` to desired interval in seconds.

### 10.3 Changing History Length

Edit `src/state.rs`:
```rust
pub const HISTORY_LENGTH: usize = 60;
```

Change `60` to desired number of data points.

## 11. Troubleshooting

### Container Won't Start

```bash
docker compose logs rust-exporter
```

### Dashboard Shows "Connection error"

1. Verify container is running: `docker compose ps`
2. Check health: `curl -f http://127.0.0.1:3001/health`
3. Verify Caddy is proxying: `curl -I https://system.estv.fr`
4. Check browser console for JavaScript errors

### Charts Not Updating

1. Open browser dev tools → Network tab
2. Check `/api/metrics` responses are successful (200 OK)
3. Verify JSON contains non-empty `history` arrays

### Metrics All Zero

1. Check /proc and /sys mounts:
   ```bash
   docker compose exec rust-exporter ls /proc
   docker compose exec rust-exporter ls /sys
   ```
2. Both should show directories/files if mounted correctly

## 12. Rollback

```bash
# Stop container
docker compose down

# Revert to previous version (if using git)
git checkout <previous-commit>

# Rebuild and restart
docker compose build && docker compose up -d
```