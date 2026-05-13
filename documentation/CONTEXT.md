# Project Context (CONTEXT.md)

## 1. Project Vision

A self-hosted monitoring stack optimized for low memory usage on a single Hetzner VPS (4GB RAM). The system provides real-time hardware metrics visualization without the overhead of traditional monitoring solutions like Prometheus.

## 2. Infrastructure

- **Host**: Hetzner VPS, Ubuntu, 4GB RAM
- **Container Runtime**: Docker + Docker Compose
- **Reverse Proxy**: Caddy (automated HTTPS via Let's Encrypt)
- **Domain**: system.estv.fr

## 3. Design Principles

1. **Minimal RAM**: Custom Rust exporter (<15MB), VictoriaMetrics (<100MB), no heavy agents
2. **Security First**: Internal Docker network, no exposed ports except through Caddy, BasicAuth enforcement
3. **Observability**: CPU, RAM, and Disk metrics exposed in Prometheus format

## 4. Technology Stack

| Layer | Technology | Purpose |
|-------|------------|---------|
| Exporter | Rust + axum + sysinfo | Collects system metrics, exposes /metrics |
| Storage | VictoriaMetrics | Time-series database (Prometheus-compatible) |
| Visualization | Grafana | Dashboard rendering |
| Ingress | Caddy | Reverse proxy, TLS termination, authentication |

## 5. Network Topology

```
Internet → Caddy (80/443) → BasicAuth → Grafana (3000)
                                ↓
                        VictoriaMetrics (8428)
                                ↓
                        rust-exporter (8080)
```

All internal communication happens via Docker's `monitoring_net` bridge network. No service ports are exposed to the host.

## 6. Security Model

- **rust-exporter**: No external access, metrics endpoint only
- **VictoriaMetrics**: No external access, scrape-only mode
- **Grafana**: Accessible only via Caddy with BasicAuth
- **Caddy**: Single ingress point, enforces authentication before proxying

## 7. Data Persistence

Docker named volumes:
- `victoriametrics_data`: Time-series storage
- `grafana_data`: Dashboards, settings, user preferences
