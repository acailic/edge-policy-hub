# Docker Deployment Guide

## Quick Start

### Prerequisites

- Docker 24.0+ with Compose V2
- 4-16 GB RAM available
- Ports 8080 (HTTP proxy) and 1883 (MQTT) available

### Deploy

```bash
# Clone repository
git clone https://github.com/acailic/edge-policy-hub.git
cd edge-policy-hub/infra/docker

# Copy and customize environment file
cp .env.example .env
vim .env  # Set UPSTREAM_URL and other settings

# Generate HMAC secret
openssl rand -base64 32 > hmac-secret.txt

# Start all services
docker-compose up -d

# Check status
docker-compose ps
docker-compose logs -f
```

## Architecture

The Docker Compose deployment includes five services:

1. **enforcer** (port 8181): OPA policy enforcement engine
2. **audit-store** (port 8182): Audit logging with SQLite
3. **quota-tracker** (port 8183): Quota management
4. **proxy-http** (port 8080): HTTP egress proxy
5. **bridge-mqtt** (ports 1883, 8883): MQTT broker/bridge

All services communicate via the internal `edge-policy-net` network. Only proxy-http and bridge-mqtt expose ports to the host.

## Configuration

### Environment Variables

See `.env.example` for all available options. Key settings:

- `UPSTREAM_URL`: Backend service URL for HTTP proxy
- `LOG_LEVEL`: Logging verbosity (`debug`, `info`, `warn`, `error`)
- `ENABLE_DEFERRED_UPLOAD`: Enable cloud sync for audit logs
- `UPLOAD_ENDPOINT`: Cloud endpoint for audit log uploads

### Secrets

The HMAC secret for audit signing is loaded from a Docker secret:

```bash
# Create secret
echo "your-secret-key" | docker secret create hmac-secret -

# Or use file
docker secret create hmac-secret hmac-secret.txt
```

### Volumes

Persistent data is stored in named volumes:

- `audit-data`: Audit log SQLite databases (per-tenant)
- `quota-data`: Quota tracker SQLite database
- `policy-bundles`: Rego policy bundles (shared with enforcer)

Backup volumes regularly:

```bash
docker run --rm -v audit-data:/data -v $(pwd):/backup alpine tar czf /backup/audit-backup.tar.gz /data
```

## Service Dependencies

Startup order:

1. enforcer, audit-store, quota-tracker (parallel)
2. proxy-http, bridge-mqtt (after core services)

Docker Compose handles dependencies via `depends_on` with health checks.

## Development Mode

Use `docker-compose.dev.yml` for development with hot-reload:

```bash
docker-compose -f docker-compose.yml -f docker-compose.dev.yml up
```

This mounts source code and uses `cargo-watch` for automatic rebuilds.

## Production Deployment

### Pre-built Images

Use pre-built images from GitHub Container Registry:

```yaml
services:
  enforcer:
    image: ghcr.io/acailic/edge-policy-enforcer:v1.0.0
    # Remove build section
```

### Resource Limits

Add resource constraints for production:

```yaml
services:
  enforcer:
    deploy:
      resources:
        limits:
          cpus: '1.0'
          memory: 512M
        reservations:
          cpus: '0.5'
          memory: 256M
```

### TLS Configuration

Mount TLS certificates as volumes:

```yaml
volumes:
  - ./certs:/var/lib/edge-policy-hub/certs:ro
environment:
  ENABLE_TLS: "true"
  TLS_CERT_PATH: "/var/lib/edge-policy-hub/certs/server.crt"
  TLS_KEY_PATH: "/var/lib/edge-policy-hub/certs/server.key"
```

## Monitoring

### View Logs

```bash
# All services
docker-compose logs -f

# Specific service
docker-compose logs -f enforcer

# Last 100 lines
docker-compose logs --tail=100 proxy-http
```

### Check Health

```bash
# Service status
docker-compose ps

# Health check status
docker inspect edge-policy-enforcer | jq '.[0].State.Health'
```

### Resource Usage

```bash
docker stats
```

## Troubleshooting

### Services Not Starting

- Check logs: `docker-compose logs`
- Verify ports are available: `netstat -tuln | grep -E '8080|1883|8181'`
- Check resource limits: `docker stats`

### Enforcer Unreachable

- Verify network: `docker network inspect edge-policy-net`
- Check enforcer health: `curl http://localhost:8181/health`
- Review enforcer logs: `docker-compose logs enforcer`

### Data Persistence Issues

- List volumes: `docker volume ls`
- Inspect volume: `docker volume inspect audit-data`
- Check permissions: `docker exec edge-policy-audit-store ls -la /var/lib/edge-policy-hub/data/audit`

## Upgrading

### Pull Latest Images

```bash
docker-compose pull
docker-compose up -d
```

### Rebuild from Source

```bash
docker-compose build --no-cache
docker-compose up -d
```

### Zero-Downtime Upgrade

```bash
# Scale up new version
docker-compose up -d --scale enforcer=2

# Wait for health checks
sleep 10

# Scale down old version
docker-compose up -d --scale enforcer=1
```

## Backup and Restore

### Backup

```bash
# Stop services
docker-compose stop

# Backup volumes
docker run --rm -v audit-data:/data -v $(pwd):/backup alpine tar czf /backup/audit-backup.tar.gz /data
docker run --rm -v quota-data:/data -v $(pwd):/backup alpine tar czf /backup/quota-backup.tar.gz /data
docker run --rm -v policy-bundles:/data -v $(pwd):/backup alpine tar czf /backup/bundles-backup.tar.gz /data

# Restart services
docker-compose start
```

### Restore

```bash
# Stop services
docker-compose stop

# Restore volumes
docker run --rm -v audit-data:/data -v $(pwd):/backup alpine tar xzf /backup/audit-backup.tar.gz -C /

# Restart services
docker-compose start
```

## Uninstall

### Remove Everything

```bash
# Stop and remove containers
docker-compose down

# Remove volumes (WARNING: deletes all data)
docker-compose down -v

# Remove images
docker rmi $(docker images 'ghcr.io/acailic/edge-policy-*' -q)
```

### Keep Data

```bash
# Stop and remove containers only
docker-compose down

# Volumes are preserved and can be reused on next deployment
```

Refer to the main project README for the overall architecture and service descriptions.
