# AGENTS.md

## Critical Rules (Non-Negotiable)

**NEVER commit unless user explicitly says "commit this" or "create a commit".**

| Action       | Required User Input                         |
| ------------ | ------------------------------------------- |
| `git add`    | "commit this" or "stage these files"        |
| `git commit` | "commit", "create commit", or "commit this" |
| `git push`   | "push to remote" or "push this"             |
| `git reset`  | "revert" or "reset commits"                 |
| Delete files | "delete", "remove", or "clean up"           |

---

## Commands

### Development (Docker)

```bash
docker compose up -d              # Start all services
docker compose build              # Build rust-exporter image
docker compose logs -f rust-exporter  # Follow logs
docker compose down               # Stop all services
```

### Development (Rust - Local)

```bash
cd rust-exporter
cargo run                         # Run locally on :8080 (requires Rust toolchain)
cargo test                        # Run unit tests
cargo clippy                      # Lint check
cargo build --release             # Build optimized binary
```

### Monitoring & Debugging

```bash
docker compose ps                 # Check container status
docker stats                      # Real-time resource usage
curl http://127.0.0.1:8080/health      # Health check
curl http://127.0.0.1:8080/api/metrics # Raw metrics (JSON)
curl http://127.0.0.1:8080/            # Dashboard HTML
```

### Before Committing

```bash
docker compose build && docker compose up -d  # Verify build starts correctly
curl http://127.0.0.1:8080/health             # Verify exporter works
```

Or for Rust changes:

```bash
cd rust-exporter && cargo test && cargo clippy
```

---

## Architecture

Single-container monitoring dashboard with embedded frontend.

### Service Stack

```
Caddy (host) → rust-exporter:8080 (dashboard + API)
```

### Data Flow

```
Host /proc, /sys (read-only)
        ↓
rust-exporter (collects every 2s)
        ↓
Browser polls /api/metrics (every 2s)
        ↓
Chart.js renders live charts
```

### Key Files

| File | Purpose |
|------|---------|
| `rust-exporter/src/main.rs` | Entry point, spawns collector task |
| `rust-exporter/src/collector.rs` | Background metrics collection loop |
| `rust-exporter/src/state.rs` | AppState, LiveMetrics, History structs |
| `rust-exporter/src/handlers.rs` | HTTP handlers (/, /api/metrics, /health) |
| `rust-exporter/static/index.html` | Embedded frontend (Tailwind + Chart.js) |
| `rust-exporter/Cargo.toml` | Rust dependencies (axum, sysinfo, serde) |
| `rust-exporter/Dockerfile` | Multi-stage musl build |
| `docker-compose.yml` | Single service with network_mode: host |

---

## Gotchas

1. **Network mode must be `host`** - Required to read actual host network I/O stats (not Docker's virtual bridge)
2. **Port 8080 exposed on all interfaces** - Use firewall/Caddy to restrict external access
3. **No persistent storage** - All metrics are in-memory, reset on container restart
4. **CPU temperature may be None** - Not all VPS platforms expose thermal sensors
5. **First 2 seconds show 0 MB/s** - Network speed requires delta calculation from previous tick
6. **Read-only mounts required** - `/proc` and `/sys` must be mounted for sysinfo to work

---

## Git Workflow

### Commit Message Format

```
type(scope): description
```

Types: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`, `perf`, `ci`, `build`, `revert`

Examples:
```
feat(exporter): add SWAP and CPU temperature metrics
fix(network): use host network mode for accurate I/O stats
docs(readme): update architecture diagram
chore(deps): update sysinfo crate
```

### Branch Naming

```
feature/add-disk-io-metrics
bugfix/fix-network-speed-calculation
chore/update-base-image
hotfix/security-patch
```

---

## When Adding New Files

| New File                     | Must Update                      |
|------------------------------|----------------------------------|
| `docker-compose.yml` services | `documentation/ARCHITECTURE.md` |
| `rust-exporter/src/*.rs`     | `documentation/ARCHITECTURE.md` |
| `rust-exporter/static/*`     | `documentation/ARCHITECTURE.md` |

---

## Security Rules

1. **Port 8080 on host network** - Use Caddy or firewall to restrict access
2. **Never commit secrets** - Use environment variables for passwords
3. **Read-only mounts for /proc and /sys** - Container must not write to system dirs
4. **Caddy handles authentication** - rust-exporter has no auth, relies on reverse proxy

---

## Resource Limits

Target resource usage (idle):

| Service | RAM Target |
|---------|------------|
| rust-exporter | <25MB |
| Caddy (host) | <50MB |
| **Total** | <75MB |

If RAM exceeds targets:
- Check for memory leaks in rust-exporter
- Reduce history length in `state.rs` (HISTORY_LENGTH)

---

## Adding New Features

### Pre-Implementation Checklist

Before adding any new feature, update `documentation/TODO.md` with:

1. **Feature description** - What it adds/changes
2. **Affected components** - Which files change
3. **Security impact** - New endpoints, authentication, data exposure
4. **Resource impact** - RAM/CPU/storage estimates

### Implementation Rules

| Rule | Reason |
|------|--------|
| Test locally first | `cargo test && cargo clippy` before docker build |
| Document in ARCHITECTURE.md | Required per "When Adding New Files" |
| Keep RAM under 25MB | Memory-constrained VPS environment |
| Use host network mode | Required for accurate network stats |

### Validation Steps

After implementing a new feature:

1. **Build test**: `docker compose build` must succeed
2. **Runtime test**: `docker compose up -d` must start container
3. **Health check**: `curl -f http://127.0.0.1:8080/health` returns 200
4. **API test**: `curl http://127.0.0.1:8080/api/metrics | jq` returns valid JSON
5. **Resource check**: `docker stats` - RAM < 30MB
6. **Network check**: `docker inspect rust-exporter --format='{{.HostConfig.NetworkMode}}'` returns "host"

### Common Feature Types

#### Adding a New Metric

1. Edit `rust-exporter/src/state.rs` - add field to `LiveMetrics` struct
2. Edit `rust-exporter/src/collector.rs` - collect the metric in `run_metrics_loop()`
3. Edit `rust-exporter/static/index.html` - add display element
4. Update `documentation/ARCHITECTURE.md` → "Metrics Collected" table
5. Run `cd rust-exporter && cargo test && cargo clippy`
6. Rebuild: `docker compose build`
7. Restart: `docker compose up -d`
8. Verify: `curl http://127.0.0.1:8080/api/metrics | jq '.current.<new_metric>'`

#### Adding a New Chart

1. Edit `rust-exporter/static/index.html`
2. Add `<canvas id="new-chart">` element
3. Add Chart.js initialization in `<script>` section
4. Add data series to `updateCharts()` function
5. Rebuild: `docker compose build`
6. Test: Visit dashboard and verify chart renders

#### Changing Polling Interval

1. Edit `rust-exporter/src/collector.rs`
2. Change `tokio::time::sleep(std::time::Duration::from_secs(2)).await`
3. Update frontend: `setInterval(fetchMetrics, X)` in index.html
4. Rebuild and restart

---

## Troubleshooting

### Container Won't Start

```bash
docker compose logs rust-exporter
```

### Network Traffic Shows Zero

```bash
# Verify host network mode
docker inspect rust-exporter --format='{{.HostConfig.NetworkMode}}'
# Should return "host"
```

### Dashboard Shows "Connection error"

1. Verify container: `docker compose ps`
2. Test health: `curl -f http://127.0.0.1:8080/health`
3. Check browser console for JavaScript errors

### Metrics All Zero

1. Check /proc and /sys mounts in docker-compose.yml are `:ro`
2. Verify container can read: `docker compose exec rust-exporter ls /proc`

### CPU Temperature Shows "--"

- Not all systems expose CPU temperature sensors
- VPS platforms often don't expose thermal data
- The app gracefully handles missing sensors