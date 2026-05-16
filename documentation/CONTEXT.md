# Project Context (CONTEXT.md)

## 1. Project Vision

A self-hosted, ultra-lightweight monitoring dashboard optimized for minimal RAM usage on a single Hetzner VPS (4GB RAM). The system provides real-time hardware metrics visualization through a custom-built all-in-one Rust application, eliminating the need for heavy monitoring stacks like VictoriaMetrics and Grafana. Designed specifically for LLM workloads with SWAP monitoring to track memory overflow.

## 2. Infrastructure

- **Host**: Hetzner VPS, Ubuntu, 4GB RAM
- **Container Runtime**: Docker + Docker Compose
- **Reverse Proxy**: Caddy (host OS, not containerized)
- **Domain**: system.estv.fr

## 3. Design Principles

1. **Minimal RAM**: Single Rust binary (<25MB total) vs traditional stack (~195MB)
2. **Security First**: Caddy handles TLS/auth, container binds to localhost only
3. **All-in-One**: Backend + frontend in single binary, no external dependencies
4. **Zero Persistence**: All metrics in-memory, no database overhead
5. **Network Accuracy**: Uses `network_mode: host` to read actual host network stats

## 4. Technology Stack

| Layer | Technology | Purpose |
|-------|------------|---------|
| Backend | Rust + axum + sysinfo | Metrics collection, web server, JSON API |
| Frontend | HTML + Tailwind CSS + Chart.js | Dashboard (served from CDN) |
| Ingress | Caddy (host) | Reverse proxy, TLS termination, authentication |

## 5. Architecture Comparison

### Previous Stack (Removed)
```
rust-exporter (15MB) → VictoriaMetrics (100MB) → Grafana (80MB) = ~195MB
```

### Current Stack
```
Caddy (host, 20MB) → rust-exporter (25MB) = ~45MB total
```

**RAM Savings: ~150MB** freed for LLM workloads.

## 6. Network Topology

```
Internet → Caddy (host, 80/443) → BasicAuth → 127.0.0.1:3001
                                              ↓
                                       rust-exporter:3000
                                              │
                                       ┌──────┴──────┐
                                       │ Background  │
                                       │ Collector   │
                                       │ (2s cycle)  │
                                       └──────┬──────┘
                                              │
                                       ┌──────┴──────┐
                                       │ In-Memory   │
                                       │ Storage     │
                                       │ (60 points) │
                                       └──────┬──────┘
                                              │
                                       ┌──────┴──────┐
                                       │ Axum Server │
                                       │ / (HTML)    │
                                       │ /api/metrics│
                                       │ /health     │
                                       └─────────────┘
```

## 7. Security Model

- **rust-exporter**: Exposed only on localhost (127.0.0.1:3001), no authentication
- **Caddy**: Single ingress point, handles BasicAuth and TLS
- **Container**: Non-root user (65534), read-only mounts for /proc and /sys
- **No data persistence**: All metrics reset on restart

## 8. Metrics Collected

| Metric | Source | Calculation |
|--------|--------|-------------|
| CPU Usage % | sysinfo | Global CPU average |
| CPU Temperature | sysinfo::Components | Thermal sensor (if available) |
| RAM Used/Total GB | sysinfo | Used/Total memory |
| RAM % Used | sysinfo | (Used/Total) × 100 |
| SWAP Used/Total GB | sysinfo | Used/Total swap |
| SWAP % Used | sysinfo | (Used/Total) × 100 |
| Disk Used/Total/Free GB | sysinfo::Disks | Space on `/` mount |
| Disk % Used | sysinfo | (Used/Total) × 100 |
| Processes | sysinfo | Count of running processes |
| Top 5 Processes | sysinfo | Sorted by CPU usage |
| Uptime | sysinfo | System uptime seconds |
| Network RX/TX MB/s | sysinfo::Networks | Delta bytes / elapsed time |

**Network Speed Calculation** (critical for accuracy):
```
speed_mbps = ((current_bytes - previous_bytes) / elapsed_seconds) / 1_000_000
```

All network interfaces are aggregated into single RX/TX values.

## 9. Frontend

- **Framework**: Vanilla JavaScript (no build step)
- **Styling**: Tailwind CSS via CDN (glassmorphism dark theme)
- **Charts**: Chart.js via CDN (line charts for system/network)
- **Polling**: `setInterval(fetchMetrics, 2000)` — matches backend collection interval
- **Responsive**: Works on mobile/tablet/desktop

## 10. Deployment

### Build

```bash
docker compose build
```

### Run

```bash
docker compose up -d
```

### Health Check

```bash
curl -f http://127.0.0.1:3001/health
```

### Access

Navigate to `https://system.estv.fr` (Caddy proxies to `127.0.0.1:3001`)