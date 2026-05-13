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
                    ┌───────────┴───────────┐
                    │                       │
                    ▼                       ▼
        ┌───────────────────┐    ┌──────────────────┐
        │   Grafana:3000     │    │   (Reserved)     │
        │ system.estv.fr     │    │                  │
        └───────────────────┘    └──────────────────┘
                    │
                    ▼ internal queries
        ┌───────────────────────────────────────────┐
        │           monitoring_net (Docker)          │
        │  ┌─────────────┐  ┌────────────────────┐ │
        │  │ Grafana     │  │ VictoriaMetrics    │ │
        │  │ :3000       │──│ :8428              │ │
        │  └─────────────┘  └────────────────────┘ │
        │                          │               │
        │                          ▼ scrapes       │
        │                   ┌──────────────┐      │
        │                   │rust-exporter │      │
        │                   │:8080/metrics │      │
        │                   └──────────────┘      │
        └───────────────────────────────────────────┘
```

## 2. Component Details

### 2.1 rust-exporter (Custom Metrics Collector)

**Purpose**: Lightweight system metrics exporter

**Technology Stack**:
- Language: Rust (nightly for optimizations)
- Web Framework: axum (minimal overhead)
- System Info: sysinfo crate
- Memory Target: <15MB RSS

**Exposed Metrics** (Prometheus format on `/metrics`):
```
# TYPE node_cpu_usage_percent gauge
node_cpu_usage_percent{host="vps"} 12.5

# TYPE node_memory_used_bytes gauge
node_memory_used_bytes{host="vps"} 2147483648

# TYPE node_memory_total_bytes gauge
node_memory_total_bytes{host="vps"} 4294967296

# TYPE node_disk_used_bytes gauge
node_disk_used_bytes{host="vps",mount="/"} 107374182400

# TYPE node_disk_total_bytes gauge
node_disk_total_bytes{host="vps",mount="/"} 214748364800
```

**NEW: Exposed Metrics** (Prometheus format on `/metrics`):
```
# CPU (gauge)
sys_cpu_usage_percent{host="vps"} 12.5

# Memory (gauge)
sys_mem_used_bytes{host="vps"} 2147483648
sys_mem_total_bytes{host="vps"} 4294967296

# Processes (gauge)
sys_processes_total{host="vps"} 156

# Disk (gauge - all mountpoints)
sys_disk_free_bytes{host="vps",mount="/"} 107374182400

# Network I/O (counter - cumulative bytes)
sys_network_rx_bytes_total{host="vps",interface="eth0"} 1548593845
sys_network_tx_bytes_total{host="vps",interface="eth0"} 20485739

# Uptime (gauge)
sys_uptime_seconds{host="vps"} 86400
```

**Endpoint**: `GET /metrics` → Prometheus text format

**Internal Port**: 8080 (Docker network only, never exposed to host)

### 2.2 VictoriaMetrics (Time-Series Database)

**Purpose**: Prometheus-compatible metrics storage

**Advantages over Prometheus**:
- ~7x less RAM usage
- Single binary, no dependencies
- Drop-in replacement for Prometheus
- Built-in data deduplication and compression

**Configuration**:
- Scrape interval: 10 seconds
- Retention: 1 year (configurable)
- Storage: Docker named volume `victoriametrics_data`

**Key Endpoints** (internal only):
- `GET /metrics/vm` → Metrics about VictoriaMetrics itself
- `GET /api/v1/query` → PromQL queries
- `GET /api/v1/query_range` → Range queries for Grafana

**Startup Flags**:
- `-promscrape.config=/etc/vmagent/scrape.yml`
- `-retentionPeriod=12`
- `-storageDataPath=/victoria-metrics-data`

### 2.3 Grafana (Visualization Layer)

**Purpose**: Dashboard rendering and querying

**Configuration**:
- Pre-configured datasource: VictoriaMetrics at `http://victoriametrics:8428`
- Pre-configured dashboards: System overview (CPU/RAM/Disk)
- Anonymous access: Disabled
- Admin password: Auto-generated on first run (check logs)

**Internal Port**: 3000 (proxied via Caddy)

**Storage**: Docker named volume `grafana_data`

### 2.4 Caddy (Ingress Controller)

**Purpose**: TLS termination, reverse proxy, authentication

**Features**:
- Automatic HTTPS via Let's Encrypt
- HTTP→HTTPS redirect
- BasicAuth enforcement for protected routes
- Zero-downtime config reload

**Routes**:
| Domain | Target | Auth |
|--------|--------|------|
| system.estv.fr | grafana:3000 | BasicAuth required |

**Security Headers**: Strict-Transport-Security, X-Frame-Options, X-Content-Type-Options

## 3. Network Architecture

### 3.1 Docker Networks

| Network Name | Driver | Scope | Purpose |
|--------------|--------|-------|---------|
| monitoring_net | bridge | local | Internal service communication |
| caddy_net | bridge | local | Caddy ↔ backend services |

### 3.2 Port Mapping

| Service | Host Port | Container Port | Exposed |
|---------|-----------|----------------|---------|
| Caddy | 80, 443 | 80, 443 | ✓ (public ingress) |
| rust-exporter | - | 8080 | ✗ (internal only) |
| VictoriaMetrics | - | 8428 | ✗ (internal only) |
| Grafana | - | 3000 | ✗ (internal only) |

### 3.3 Service Discovery

All services communicate via Docker DNS:
- `rust-exporter` → `http://rust-exporter:8080/metrics`
- `victoriametrics` → `http://victoriametrics:8428`
- `grafana` → `http://grafana:3000`

## 4. Data Flow

### 4.1 Metrics Collection Flow

```
1. rust-exporter
   └─> Polls sysinfo every 1s (internal cache)
   └─> Exposes latest metrics on GET /metrics

2. VictoriaMetrics
   └─> Scrapes http://rust-exporter:8080/metrics every 10s
   └─> Stores time-series data in /victoria-metrics-data
   └─> Compresses and deduplicates data automatically

3. Grafana
   └─> User queries dashboard
   └─> Grafana sends PromQL query to VictoriaMetrics
   └─> VictoriaMetrics returns time-series data
   └─> Grafana renders visualization
```

### 4.2 User Request Flow

```
User → https://system.estv.fr
     → Caddy validates BasicAuth
     → Caddy proxies to grafana:3000
     → Grafana serves dashboard
     → Dashboard queries VictoriaMetrics for data
```

## 5. Security Model

### 5.1 Attack Surface

| Vector | Mitigation |
|--------|------------|
| Public ports | Only 80/443 exposed via Caddy |
| Unauth'd access | BasicAuth at Caddy layer |
| Container escape | Non-root containers, minimal capabilities |
| Data exfiltration | No volumes mounted to host paths |
| Lateral movement | Internal network isolation |

### 5.2 Authentication Flow

```
GET https://system.estv.fr
    │
    ├─> Caddy receives request
    ├─> Caddy checks BasicAuth header
    │   ├── Valid → Proxy to Grafana
    │   └── Invalid/Missing → 401 Unauthorized
    └─> Grafana never sees unauthenticated requests
```

### 5.3 Container Security

| Service | User | Capabilities | Read-Only Root |
|---------|------|--------------|----------------|
| rust-exporter | nonroot:65532 | none | ✓ |
| VictoriaMetrics | victoriametrics | none | ✗ (data dir) |
| Grafana | grafana | none | ✗ (data dir) |
| Caddy | caddy | none | ✓ |

## 6. Resource Estimates

| Service | CPU | RAM (Idle) | RAM (Peak) | Disk I/O |
|---------|-----|------------|------------|----------|
| rust-exporter | 1m | 8MB | 15MB | Negligible |
| VictoriaMetrics | 10m | 50MB | 100MB | Write-heavy on scrape |
| Grafana | 10m | 40MB | 80MB | Read on dashboard load |
| Caddy | 5m | 20MB | 50MB | Negligible |
| **Total** | ~26m | ~120MB | ~245MB | — |

## 7. Failure Modes

| Failure | Impact | Recovery |
|---------|--------|----------|
| rust-exporter crash | No new metrics, dashboards show stale data | Auto-restart via Docker restart policy |
| VictoriaMetrics crash | No metric storage, queries fail | Auto-restart, data persists in volume |
| Grafana crash | Dashboard unavailable | Auto-restart, config persists in volume |
| Caddy crash | All external access blocked | Auto-restart, no data loss |
| Host reboot | All services down | Docker Compose restart policy brings all up |

## 8. Backup Strategy

| Data | Method | Frequency | Retention |
|------|--------|-----------|-----------|
| VictoriaMetrics TS | Volume snapshot or vmbackup | Daily | 7 days |
| Grafana config | Volume snapshot | Weekly | 4 weeks |

**Recommended**: Use `victoriametrics/vmbackup` sidecar container or host-level volume backups.

## 9. Monitoring the Monitor

### 9.1 Health Checks

| Service | Health Endpoint | Docker Healthcheck |
|---------|-----------------|-------------------|
| rust-exporter | GET /health → 200 OK | `curl -f http://localhost:8080/health` |
| VictoriaMetrics | GET /health → 200 OK | `curl -f http://localhost:8428/health` |
| Grafana | GET /api/health → 200 OK | `curl -f http://localhost:3000/api/health` |
| Caddy | GET /health → 200 OK | `caddy validate --config /etc/caddy/Caddyfile` |

### 9.2 Self-Monitoring Metrics

VictoriaMetrics exposes its own metrics at `/metrics/vm`. Consider adding a dashboard for:
- `vm_rows` - Total stored rows
- `vm_data_size_bytes` - Storage size
- `vm_request_duration_seconds` - Query latency

## 10. Extension Points

### 10.1 Adding New Exporters

```yaml
# docker-compose.yml addition
services:
  custom-exporter:
    build: ./custom-exporter
    networks:
      - monitoring_net

# vmagent.yml addition
scrape_configs:
  - job_name: 'custom-exporter'
    static_configs:
      - targets: ['custom-exporter:8080']
```

### 10.2 Adding Alerting

Add Alertmanager (minimal overhead):
```yaml
services:
  alertmanager:
    image: prom/alertmanager:latest
    networks:
      - monitoring_net
```

Configure VictoriaMetrics with `-alerts=...` flag.

### 10.3 Adding Authentication

For multi-user Grafana setup, configure OAuth/GitHub login via Grafana's `auth.github` section in `grafana.ini`.