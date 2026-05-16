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
docker compose logs -f <service> # Follow logs (rust-exporter, victoriametrics, grafana, caddy)
docker compose down               # Stop all services
docker compose down -v            # Stop and remove volumes (data loss!)
```

### Development (Rust - Local)

```bash
cd rust-exporter
cargo run                         # Run locally (requires Rust toolchain)
cargo test                        # Run unit tests
cargo build --release             # Build optimized binary
```

### Monitoring & Debugging

```bash
docker compose ps                 # Check container status
docker stats                      # Real-time resource usage
docker compose exec rust-exporter curl http://localhost:8080/health      # Health check
docker compose exec rust-exporter curl http://localhost:8080/metrics     # Raw metrics
docker compose exec victoriametrics curl http://localhost:8428/health     # VM health
docker compose exec victoriametrics curl 'http://localhost:8428/api/v1/query?query=up'  # Query metrics
docker compose exec grafana curl http://localhost:3000/api/health         # Grafana health
docker compose exec caddy caddy validate --config /etc/caddy/Caddyfile   # Validate Caddy config
```

### Before Committing

```bash
docker compose build && docker compose up -d  # Verify build starts correctly
docker compose exec rust-exporter curl http://localhost:8080/health  # Verify exporter works
```

Or for Rust changes:

```bash
cd rust-exporter && cargo test && cargo clippy
```

---

## Architecture

Containerized monitoring stack with internal network isolation.

### Service Stack

```
Caddy (ingress) → Grafana (ui) → VictoriaMetrics (tsdb) → rust-exporter (metrics)
```

### Data Flow

```
rust-exporter:8080/metrics
        ↓ (scraped every 10s)
VictoriaMetrics:8428 (storage)
        ↓ (queried via PromQL)
Grafana:3000 (dashboards)
        ↓ (proxied)
Caddy:443 → system.estv.fr (public, BasicAuth required)
```

### Key Files

| File | Purpose |
|------|---------|
| `rust-exporter/src/main.rs` | Metrics collection logic |
| `rust-exporter/Cargo.toml` | Rust dependencies (axum, sysinfo) |
| `rust-exporter/Dockerfile` | Multi-stage build (Rust → scratch) |
| `config/vmagent.yml` | VictoriaMetrics scrape config |
| `docker-compose.yml` | Service orchestration |
| `caddy/Caddyfile` | Reverse proxy + BasicAuth |

---300

## Gotchas

1. **runt-exporter must be built before `docker compose up`** - no pre-built image
2. **VictoriaMetrics scrape target uses Docker DNS**: `rust-exporter:8080` (not localhost)
3. **Grafana datasource URL**: `http://victoriametrics:8428` (internal Docker network)
4. **BasicAuth must be set in Caddyfile** before first deployment
5. **No ports exposed to host except Caddy (80/443)** - all internal communication via Docker network
6. **Volumes persist data** - `docker compose down` does NOT delete data, use `docker compose down -v`
7. **rust-exporter reads host metrics**: Docker container needs `/proc` and `/sys` mounted read-only

---

## Git Workflow

### Commit Message Format

```
type(scope): description
```

Types: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`, `perf`, `ci`, `build`, `revert`

Examples:
```
feat(exporter): add disk I/O metrics
fix(caddy): correct BasicAuth hash format
docs(readme): update deployment instructions
chore(deps): update VictoriaMetrics to v1.100
```

### Branch Naming

```
feature/add-temperature-metrics
bugfix/fix-cpu-percentage-calculation
chore/update-base-image
hotfix/security-patch
```

---

## When Adding New Files

| New File                     | Must Update                      |
|------------------------------|----------------------------------|
| `docker-compose.yml` services | `documentation/ARCHITECTURE.md` |
| `rust-exporter/src/*.rs`     | `documentation/ARCHITECTURE.md` |
| `caddy/*.conf`               | `documentation/ARCHITECTURE.md` |
| `config/*.yml`               | `documentation/ARCHITECTURE.md` |

---

## Security Rules

1. **Never expose rust-exporter, VictoriaMetrics, or Grafana ports to host** - only Caddy
2. **Never commit BasicAuth passwords** - use environment variables or Docker secrets
3. **Always use read-only mounts for system dirs** (`/proc`, `/sys`) in rust-exporter
4. **Validate Caddyfile before restart** - syntax errors block all traffic
5. **Keep VictoriaMetrics retention reasonable** - storage grows linearly with metrics volume

---

## Resource Limits

Target resource usage (idle):

| Service | RAM Target |
|---------|------------|
| rust-exporter | <15MB |
| VictoriaMetrics | <100MB |
| Grafana | <80MB |
| Caddy | <50MB |
| **Total** | <250MB |

If RAM exceeds targets:
- Check for memory leaks in rust-exporter
- Reduce VictoriaMetrics retention period
- Limit Grafana dashboard refresh intervals

---

## Adding New Features

### Pre-Implementation Checklist

Before adding any new feature, update `documentation/TODO.md` with:

1. **Feature description** - What it adds/changes
2. **Affected components** - Which services/files change
3. **Security impact** - New ports, authentication, data exposure
4. **Resource impact** - RAM/CPU/storage estimates

### Implementation Rules

| Rule | Reason |
|------|--------|
| No new host ports | Only Caddy exposes to public (Security Rule #1) |
| Use existing Docker network | New services must join `monitoring_net` |
| Document in ARCHITECTURE.md | Required per "When Adding New Files" |
| Test locally before compose | `cargo test` or `docker compose build && docker compose up -d` |
| Update AGENTS.md if needed | New commands, gotchas, or security rules |

### Validation Steps

After implementing a new feature:

1. **Build test**: `docker compose build` must succeed
2. **Runtime test**: `docker compose up -d` must start all containers
3. **Health check**: All health endpoints must return 200
   ```bash
   docker compose exec rust-exporter curl -f http://localhost:8080/health
   docker compose exec victoriametrics curl -f http://localhost:8428/health
   docker compose exec grafana curl -f http://localhost:3000/api/health
   ```
4. **Port audit**: `docker compose ps` - no unexpected host port bindings
5. **Resource check**: `docker stats` - total RAM < 250MB target
6. **Security review**: Verify no secrets committed, no new public endpoints

### Common Feature Types

#### Adding a New Metric to rust-exporter

1. Edit `rust-exporter/src/main.rs`
2. Add metric to `get_metrics()` function
3. Update `documentation/ARCHITECTURE.md` → "Exposed Metrics" table
4. Run `cargo test && cargo clippy`
5. Rebuild: `docker compose build rust-exporter`
6. Restart: `docker compose up -d rust-exporter`
7. Verify: `docker compose exec rust-exporter curl http://localhost:8080/metrics | grep <new_metric>`

#### Adding a New Service

1. Add service to `docker-compose.yml` with `networks: [monitoring_net]`
2. Ensure no `ports:` mapping to host (internal only)
3. Update `documentation/ARCHITECTURE.md`:
   - Service stack diagram
   - Component details table
   - Data flow (if applicable)
4. Update `documentation/TODO.md` with health check commands
5. Test: `docker compose up -d && docker compose ps`

#### Adding a New Endpoint to Caddy

1. Edit `caddy/Caddyfile`
2. Validate syntax: `docker compose exec caddy caddy validate --config /etc/caddy/Caddyfile`
3. Reload: `docker compose restart caddy`
4. Verify: `curl -I https://<new-endpoint>`
5. Update `documentation/ARCHITECTURE.md` → Caddy routes table

#### Changing Scrape Configuration

1. Edit `config/vmagent.yml`
2. Restart VictoriaMetrics: `docker compose restart victoriametrics`
3. Verify targets: `docker compose exec victoriametrics curl http://localhost:8428/api/v1/targets`
4. Update `documentation/ARCHITECTURE.md` if interval changes