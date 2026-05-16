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
│                    Container Port: 8080                          │
│                    Network Mode: host                             │
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
    "ram_percent": 31.5,
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
      {"name": "process1", "cpu_percent": 5.2, "ram_mb": 150},
      {"name": "process2", "cpu_percent": 3.1, "ram_mb": 80}
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

**Internal Architecture**:
- `AppState`: Shared state with `RwLock` for concurrent read/write
- `collector.rs`: Background tokio task collecting metrics every 2s
- `handlers.rs`: HTTP route handlers for `/`, `/api/metrics`, `/health`
- `state.rs`: Data structures for `LiveMetrics`, `HistoryPoint`, `History`

### 2.2 Caddy (Ingress Controller)

**Purpose**: TLS termination, reverse proxy, authentication

**Configuration**:
- Caddy is installed on host OS (not containerized)
- Proxies `https://system.estv.fr` → `http://127.0.0.1:8080`
- Enforces BasicAuth before forwarding requests

**Routes**:
| Domain | Target | Auth |
|--------|--------|------|
| system.estv.fr | 127.0.0.1:8080 | BasicAuth required |

**Security Headers**: Strict-Transport-Security, X-Frame-Options, X-Content-Type-Options

## 3. Network Architecture

### 3.1 Port Mapping

| Service | Host Binding | Container Port | Notes |
|---------|--------------|----------------|-------|
| rust-exporter | host network | 8080 | Uses `network_mode: host` |

**Important**: With `network_mode: host`, the container shares the host's network namespace. This is required for accurate network I/O statistics. The container listens on `0.0.0.0:8080` inside the host's network.

### 3.2 Host Volume Mounts

| Host Path | Container Path | Purpose |
|-----------|----------------|---------|
| `/proc` | `/proc` | Process/system metrics (read-only) |
| `/sys` | `/sys` | System metrics (read-only) |

## 4. Data Flow

### 4.1 Metrics Collection Flow

```
1. Background Task (every 2s)
   ├─> Refreshes System, Components, Disks, Networks, Processes
   ├─> CPU: Global CPU usage percentage
   ├─> CPU Temp: From Components (thermal sensors)
   ├─> RAM: Used/Total memory from sysinfo
   ├─> SWAP: Used/Total swap from sysinfo
   ├─> Disk: Used/Total/Free on "/" mount
   ├─> Processes: Count + Top 5 by CPU usage
   ├─> Uptime: System uptime seconds
   ├─> Network: Aggregates ALL interfaces
   │      delta_rx = current_rx - previous_rx
   │      delta_tx = current_tx - previous_tx
   │      speed_mbps = (delta / elapsed_secs) / 1_000_000
   └─> Updates AppState:
          - Arc<RwLock<LiveMetrics>> (current snapshot)
          - Arc<RwLock<VecDeque<HistoryPoint>>> (rolling history 60 pts)

2. Frontend Polling (every 2s)
   ├─> GET /api/metrics → JSON response
   ├─> Updates gauges: CPU, RAM, SWAP, Disk, Processes, Uptime, Temp, Network
   └─> Updates Chart.js line charts with history arrays
```

### 4.2 User Request Flow

```
User → https://system.estv.fr
     → Caddy validates BasicAuth
     → Caddy proxies to 127.0.0.1:8080
     → rust-exporter serves embedded index.html
     → Dashboard polls /api/metrics every 2s
```

## 5. Security Model

### 5.1 Attack Surface

| Vector | Mitigation |
|--------|------------|
| Public access | Caddy on host enforces BasicAuth before proxying to localhost:8080 |
| Container network | Read-only mounts for /proc and /sys |
| Data exposure | No persistent data (in-memory only) |

### 5.2 Network Mode Considerations

**`network_mode: host`**: The container uses the host's network stack directly.

**Why this is needed**:
- Network statistics (`/proc/net/dev`) must reflect the **host's** interfaces, not Docker's virtual bridge
- Without host network, the container would only see its own isolated traffic (near zero)
- This is critical for accurate network I/O monitoring on a VPS

**Security implications**:
- The container has full network visibility (same as any process on host)
- Port 8080 is accessible on all host interfaces
- Caddy should proxy from localhost only, or firewall rules should restrict external access to 8080

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
curl -f http://127.0.0.1:8080/health
# Returns: 200 OK (empty body)
```

### 9.2 Test Dashboard

```bash
curl http://127.0.0.1:8080/
# Returns: HTML content

curl http://127.0.0.1:8080/api/metrics | jq
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
2. Check health: `curl -f http://127.0.0.1:8080/health`
3. Verify Caddy is proxying: `curl -I https://system.estv.fr`
4. Check browser console for JavaScript errors

### Charts Not Updating

1. Open browser dev tools → Network tab
2. Check `/api/metrics` responses are successful (200 OK)
3. Verify JSON contains non-empty `history` arrays

### Network Traffic Shows Zero

1. **Verify `network_mode: host` is set** in docker-compose.yml
2. Check container network: `docker inspect rust-exporter --format='{{.HostConfig.NetworkMode}}'`
   - Should return "host"
3. Without host network mode, Docker's virtual bridge shows no traffic from the VPS

### CPU Temperature Shows `null`

- Not all systems expose CPU temperature sensors
- The app gracefully handles missing sensors and displays "--" in the UI
- On VPS environments, thermal sensors may not be available

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