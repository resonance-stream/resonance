# Resonance Production Deployment Guide

This guide covers deploying Resonance in a production environment with proper security, reliability, and maintenance procedures.

---

## Table of Contents

- [Prerequisites](#prerequisites)
- [Quick Production Deployment](#quick-production-deployment)
- [Environment Configuration](#environment-configuration)
- [Docker Compose Production Setup](#docker-compose-production-setup)
- [HTTPS & Reverse Proxy](#https--reverse-proxy)
- [Security Hardening](#security-hardening)
- [Backup Procedures](#backup-procedures)
- [Monitoring & Logging](#monitoring--logging)
- [Upgrade Procedures](#upgrade-procedures)
- [Disaster Recovery](#disaster-recovery)
- [Troubleshooting](#troubleshooting)

---

## Prerequisites

### System Requirements

| Resource | Minimum | Recommended |
|----------|---------|-------------|
| CPU | 2 cores | 4+ cores |
| RAM | 4 GB | 8+ GB |
| Storage | 50 GB | 100+ GB (depends on library size) |
| Network | 10 Mbps | 100+ Mbps |

### Software Requirements

- **Docker** v24.0+ with Docker Compose v2.20+
- **Linux server** (Ubuntu 22.04 LTS recommended)
- A **domain name** with DNS configured
- **SSL certificate** (Let's Encrypt recommended)
- **Lidarr** instance with your music library configured

### Recommended Infrastructure

- Dedicated server or VPS (not shared hosting)
- SSD storage for database and cache
- Separate backup storage location
- Reverse proxy (Traefik, Caddy, or nginx)

---

## Quick Production Deployment

For experienced users, here's the quick setup:

```bash
# 1. Clone and configure
git clone https://github.com/resonance-stream/resonance.git
cd resonance
cp .env.example .env.production

# 2. Generate secure secrets
DB_PASSWORD=$(openssl rand -base64 24)
JWT_SECRET=$(openssl rand -base64 32)
MEILISEARCH_KEY=$(openssl rand -base64 32)

# 3. Update .env.production with generated secrets and your configuration
vim .env.production

# 4. Deploy with production compose file
docker compose -f docker-compose.prod.yml --env-file .env.production up -d

# 5. Verify all services are healthy
docker compose -f docker-compose.prod.yml ps
```

---

## Environment Configuration

### Production Environment File

Create a dedicated `.env.production` file (never commit this to version control):

```bash
# =============================================================================
# PRODUCTION ENVIRONMENT CONFIGURATION
# =============================================================================

# -----------------------------------------------------------------------------
# Database
# -----------------------------------------------------------------------------
# Generate with: openssl rand -base64 24
DB_PASSWORD=<GENERATE_SECURE_PASSWORD>
POSTGRES_USER=resonance
POSTGRES_DB=resonance
POSTGRES_PASSWORD=${DB_PASSWORD}
DATABASE_URL=postgres://${POSTGRES_USER}:${DB_PASSWORD}@postgres:5432/${POSTGRES_DB}

# -----------------------------------------------------------------------------
# Authentication
# -----------------------------------------------------------------------------
# Generate with: openssl rand -base64 32 (MUST be at least 32 characters)
JWT_SECRET=<GENERATE_SECURE_SECRET>
JWT_ACCESS_EXPIRY=15m
JWT_REFRESH_EXPIRY=7d

# -----------------------------------------------------------------------------
# Services
# -----------------------------------------------------------------------------
REDIS_URL=redis://redis:6379
MEILISEARCH_URL=http://meilisearch:7700
MEILISEARCH_KEY=<GENERATE_SECURE_KEY>
MEILI_ENV=production

# -----------------------------------------------------------------------------
# Ollama AI
# -----------------------------------------------------------------------------
OLLAMA_URL=http://ollama:11434
OLLAMA_MODEL=mistral
EMBEDDING_MODEL=nomic-embed-text

# -----------------------------------------------------------------------------
# Lidarr Integration
# -----------------------------------------------------------------------------
LIDARR_URL=http://your-lidarr-host:8686
LIDARR_API_KEY=<YOUR_LIDARR_API_KEY>
MUSIC_LIBRARY_PATH=/path/to/your/music

# -----------------------------------------------------------------------------
# Server Configuration
# -----------------------------------------------------------------------------
PORT=4440
API_PORT=4441
RUST_LOG=info,sqlx=warn
ENVIRONMENT=production

# -----------------------------------------------------------------------------
# CORS - REQUIRED in production
# -----------------------------------------------------------------------------
# Set to your actual domain(s)
CORS_ORIGINS=https://music.yourdomain.com

# -----------------------------------------------------------------------------
# Rate Limiting
# -----------------------------------------------------------------------------
AUTH_RATE_LIMIT=10
API_RATE_LIMIT=100
STREAM_RATE_LIMIT=60
```

### Secret Generation

Generate all required secrets before deployment:

```bash
# Database password
echo "DB_PASSWORD=$(openssl rand -base64 24)"

# JWT secret (must be at least 32 characters)
echo "JWT_SECRET=$(openssl rand -base64 32)"

# Meilisearch master key
echo "MEILISEARCH_KEY=$(openssl rand -base64 32)"
```

> ⚠️ **Security Warning**: Store these secrets securely. Consider using a secrets manager for production deployments.

---

## Docker Compose Production Setup

Use the production Docker Compose file `docker-compose.prod.yml` which includes:

- Resource limits for all containers
- Proper logging configuration
- Health check conditions
- Security configurations
- Restart policies

### Starting Production Services

```bash
# Start all services
docker compose -f docker-compose.prod.yml --env-file .env.production up -d

# View service status
docker compose -f docker-compose.prod.yml ps

# View logs
docker compose -f docker-compose.prod.yml logs -f

# View logs for specific service
docker compose -f docker-compose.prod.yml logs -f resonance
```

### Service Health Verification

After starting, verify all services are healthy:

```bash
# Check container health status
docker compose -f docker-compose.prod.yml ps

# Test API health endpoint
curl -f http://localhost:4441/health

# Test Meilisearch
curl -f http://localhost:7700/health

# Test Redis
docker compose -f docker-compose.prod.yml exec redis redis-cli ping

# Test PostgreSQL
docker compose -f docker-compose.prod.yml exec postgres pg_isready -U resonance
```

---

## HTTPS & Reverse Proxy

Production deployments **must** use HTTPS. Choose one of the following approaches:

### Option 1: Caddy (Recommended for Simplicity)

Caddy automatically handles SSL certificate management with Let's Encrypt.

Create `Caddyfile`:

```caddyfile
music.yourdomain.com {
    # Web frontend
    reverse_proxy /api/* resonance:8080
    reverse_proxy /graphql resonance:8080
    reverse_proxy /ws resonance:8080
    reverse_proxy /* web:80

    # Security headers
    header {
        X-Frame-Options "DENY"
        X-Content-Type-Options "nosniff"
        X-XSS-Protection "1; mode=block"
        Referrer-Policy "strict-origin-when-cross-origin"
        Strict-Transport-Security "max-age=31536000; includeSubDomains"
    }

    # Enable compression
    encode gzip zstd
}
```

Add to your Docker Compose:

```yaml
services:
  caddy:
    image: caddy:2-alpine
    container_name: resonance-caddy
    restart: unless-stopped
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./Caddyfile:/etc/caddy/Caddyfile:ro
      - caddy-data:/data
      - caddy-config:/config
    networks:
      - resonance-network

volumes:
  caddy-data:
  caddy-config:
```

### Option 2: Traefik

For more complex setups with multiple services:

```yaml
services:
  traefik:
    image: traefik:v3.0
    container_name: resonance-traefik
    restart: unless-stopped
    command:
      - "--api.insecure=false"
      - "--providers.docker=true"
      - "--providers.docker.exposedbydefault=false"
      - "--entrypoints.web.address=:80"
      - "--entrypoints.websecure.address=:443"
      - "--certificatesresolvers.letsencrypt.acme.tlschallenge=true"
      - "--certificatesresolvers.letsencrypt.acme.email=your-email@example.com"
      - "--certificatesresolvers.letsencrypt.acme.storage=/letsencrypt/acme.json"
      - "--entrypoints.web.http.redirections.entrypoint.to=websecure"
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock:ro
      - traefik-letsencrypt:/letsencrypt
    networks:
      - resonance-network

  web:
    labels:
      - "traefik.enable=true"
      - "traefik.http.routers.resonance.rule=Host(`music.yourdomain.com`)"
      - "traefik.http.routers.resonance.entrypoints=websecure"
      - "traefik.http.routers.resonance.tls.certresolver=letsencrypt"
```

### Option 3: nginx with Let's Encrypt

For traditional nginx setups:

```nginx
# /etc/nginx/sites-available/resonance
server {
    listen 80;
    server_name music.yourdomain.com;
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name music.yourdomain.com;

    ssl_certificate /etc/letsencrypt/live/music.yourdomain.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/music.yourdomain.com/privkey.pem;

    # SSL configuration
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256;
    ssl_prefer_server_ciphers off;
    ssl_session_timeout 1d;
    ssl_session_cache shared:SSL:50m;

    # Security headers
    add_header X-Frame-Options "DENY" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header X-XSS-Protection "1; mode=block" always;
    add_header Referrer-Policy "strict-origin-when-cross-origin" always;
    add_header Strict-Transport-Security "max-age=31536000; includeSubDomains" always;

    # Gzip compression
    gzip on;
    gzip_vary on;
    gzip_min_length 1024;
    gzip_types text/plain text/css application/json application/javascript text/xml;

    # API proxy
    location /api {
        proxy_pass http://localhost:4441;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    # GraphQL proxy
    location /graphql {
        proxy_pass http://localhost:4441;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    # WebSocket proxy
    location /ws {
        proxy_pass http://localhost:4441;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_read_timeout 86400;
    }

    # Web frontend
    location / {
        proxy_pass http://localhost:4440;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

Set up Let's Encrypt with certbot:

```bash
sudo apt install certbot python3-certbot-nginx
sudo certbot --nginx -d music.yourdomain.com
```

---

## Security Hardening

### 1. Docker Security

```bash
# Create dedicated network (done automatically by compose)
docker network create resonance-network

# Ensure containers run as non-root (configured in Dockerfiles)
docker compose -f docker-compose.prod.yml exec resonance id
# Should output: uid=1000(resonance) gid=1000(resonance)
```

### 2. Firewall Configuration

```bash
# UFW (Ubuntu)
sudo ufw default deny incoming
sudo ufw default allow outgoing
sudo ufw allow ssh
sudo ufw allow 80/tcp
sudo ufw allow 443/tcp
sudo ufw enable

# Verify
sudo ufw status
```

### 3. PostgreSQL Security

The database is not exposed externally by default. For additional security:

```yaml
# In docker-compose.prod.yml, PostgreSQL has no port mappings
# Only accessible within the Docker network
```

### 4. Redis Security

Redis is also internal-only. If you need external access:

```yaml
# Add password authentication
redis:
  command: >
    redis-server
    --requirepass ${REDIS_PASSWORD}
    --appendonly yes
```

### 5. Environment Variable Security

```bash
# Set restrictive permissions on environment file
chmod 600 .env.production
chown root:root .env.production
```

---

## Backup Procedures

### Automated Backup Script

Create `/opt/resonance/backup.sh`:

```bash
#!/bin/bash
# Resonance Backup Script
# Run via cron: 0 3 * * * /opt/resonance/backup.sh

set -euo pipefail

# Configuration
BACKUP_DIR="/opt/resonance/backups"
RETENTION_DAYS=30
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
COMPOSE_FILE="/path/to/resonance/docker-compose.prod.yml"
MEILISEARCH_HOST="http://localhost:7700"
MEILISEARCH_KEY="${MEILISEARCH_KEY:-}"  # Set in environment or .env.production

# Create backup directory
mkdir -p "${BACKUP_DIR}"

echo "[$(date)] Starting Resonance backup..."

# 1. Database backup
echo "Backing up PostgreSQL..."
docker compose -f "${COMPOSE_FILE}" exec -T postgres \
    pg_dump -U resonance -d resonance -Fc \
    > "${BACKUP_DIR}/postgres_${TIMESTAMP}.dump"

# 2. Redis backup with LASTSAVE polling (ensures save completes)
echo "Backing up Redis..."
# Get the last save timestamp before triggering new save
LAST_SAVE_BEFORE=$(docker compose -f "${COMPOSE_FILE}" exec -T redis redis-cli LASTSAVE | tr -d '[:space:]')
docker compose -f "${COMPOSE_FILE}" exec -T redis redis-cli BGSAVE

# Poll LASTSAVE until it changes (max 60 seconds)
echo "Waiting for Redis BGSAVE to complete..."
TIMEOUT=60
ELAPSED=0
while [ $ELAPSED -lt $TIMEOUT ]; do
    LAST_SAVE_AFTER=$(docker compose -f "${COMPOSE_FILE}" exec -T redis redis-cli LASTSAVE | tr -d '[:space:]')
    if [ "$LAST_SAVE_AFTER" != "$LAST_SAVE_BEFORE" ]; then
        echo "Redis BGSAVE completed (took ${ELAPSED}s)"
        break
    fi
    sleep 1
    ELAPSED=$((ELAPSED + 1))
done

if [ $ELAPSED -ge $TIMEOUT ]; then
    echo "WARNING: Redis BGSAVE timed out after ${TIMEOUT}s, continuing with potentially stale data"
fi

docker compose -f "${COMPOSE_FILE}" cp redis:/data/dump.rdb \
    "${BACKUP_DIR}/redis_${TIMESTAMP}.rdb"

# 3. Meilisearch backup using snapshots API (consistent point-in-time backup)
echo "Backing up Meilisearch..."
# Create a snapshot via the API (creates a consistent backup without locking)
SNAPSHOT_RESPONSE=$(curl -s -X POST "${MEILISEARCH_HOST}/snapshots" \
    -H "Authorization: Bearer ${MEILISEARCH_KEY}" \
    -H "Content-Type: application/json")

# Extract task UID and wait for completion
TASK_UID=$(echo "${SNAPSHOT_RESPONSE}" | grep -o '"taskUid":[0-9]*' | grep -o '[0-9]*')
if [ -n "$TASK_UID" ]; then
    echo "Waiting for Meilisearch snapshot task ${TASK_UID} to complete..."
    TIMEOUT=300
    ELAPSED=0
    while [ $ELAPSED -lt $TIMEOUT ]; do
        TASK_STATUS=$(curl -s "${MEILISEARCH_HOST}/tasks/${TASK_UID}" \
            -H "Authorization: Bearer ${MEILISEARCH_KEY}" | grep -o '"status":"[^"]*"' | cut -d'"' -f4)
        if [ "$TASK_STATUS" = "succeeded" ]; then
            echo "Meilisearch snapshot completed (took ${ELAPSED}s)"
            break
        elif [ "$TASK_STATUS" = "failed" ]; then
            echo "ERROR: Meilisearch snapshot failed"
            break
        fi
        sleep 2
        ELAPSED=$((ELAPSED + 2))
    done

    if [ $ELAPSED -ge $TIMEOUT ]; then
        echo "WARNING: Meilisearch snapshot timed out after ${TIMEOUT}s"
    fi
fi

# Copy the snapshot file from the container's snapshot directory
# Snapshots are stored in /meili_data/snapshots/ with timestamp-based names
docker compose -f "${COMPOSE_FILE}" exec -T meilisearch \
    sh -c 'cd /meili_data/snapshots && ls -t | head -1 | xargs cat' \
    > "${BACKUP_DIR}/meilisearch_${TIMESTAMP}.snapshot" 2>/dev/null || {
    echo "WARNING: Could not copy snapshot file, falling back to data directory backup"
    docker compose -f "${COMPOSE_FILE}" exec -T meilisearch \
        tar czf - /meili_data \
        > "${BACKUP_DIR}/meilisearch_${TIMESTAMP}.tar.gz"
}

# 4. Configuration backup
echo "Backing up configuration..."
tar czf "${BACKUP_DIR}/config_${TIMESTAMP}.tar.gz" \
    --exclude='.env*' \
    docker-compose.prod.yml \
    docker/

# 5. Compress database backup
gzip "${BACKUP_DIR}/postgres_${TIMESTAMP}.dump"

# 6. Clean up old backups
echo "Cleaning up old backups..."
find "${BACKUP_DIR}" -type f -mtime +${RETENTION_DAYS} -delete

# 7. Calculate backup size
BACKUP_SIZE=$(du -sh "${BACKUP_DIR}" | cut -f1)
echo "[$(date)] Backup completed. Total size: ${BACKUP_SIZE}"

# Optional: Upload to remote storage
# aws s3 sync "${BACKUP_DIR}/" s3://your-bucket/resonance-backups/
# rclone sync "${BACKUP_DIR}/" remote:resonance-backups/
```

### Restore Procedures

```bash
# 1. Stop services (keep database running)
docker compose -f docker-compose.prod.yml stop resonance resonance-worker web

# 2. Restore PostgreSQL
gunzip -k backups/postgres_TIMESTAMP.dump.gz
docker compose -f docker-compose.prod.yml exec -T postgres \
    pg_restore -U resonance -d resonance --clean --if-exists \
    < backups/postgres_TIMESTAMP.dump

# 3. Restore Redis
docker compose -f docker-compose.prod.yml stop redis
docker compose -f docker-compose.prod.yml cp \
    backups/redis_TIMESTAMP.rdb redis:/data/dump.rdb
docker compose -f docker-compose.prod.yml start redis

# 4. Restore Meilisearch from snapshot
docker compose -f docker-compose.prod.yml stop meilisearch
# Copy snapshot to container's import directory
docker compose -f docker-compose.prod.yml cp \
    backups/meilisearch_TIMESTAMP.snapshot meilisearch:/meili_data/snapshots/
# Start Meilisearch with snapshot import flag
docker compose -f docker-compose.prod.yml run --rm meilisearch \
    meilisearch --import-snapshot /meili_data/snapshots/meilisearch_TIMESTAMP.snapshot
docker compose -f docker-compose.prod.yml start meilisearch

# Alternative: If using legacy tar.gz backup format
# docker compose -f docker-compose.prod.yml run --rm -v $(pwd)/backups:/backups meilisearch \
#     tar xzf /backups/meilisearch_TIMESTAMP.tar.gz -C /
# docker compose -f docker-compose.prod.yml start meilisearch

# 5. Restart all services
docker compose -f docker-compose.prod.yml up -d
```

### Backup Verification

```bash
# Test database backup integrity
pg_restore --list backups/postgres_TIMESTAMP.dump

# Test Redis backup
redis-check-rdb backups/redis_TIMESTAMP.rdb

# Test archive integrity
gzip -t backups/postgres_TIMESTAMP.dump.gz

# Test Meilisearch snapshot (snapshots are self-contained binary files)
# Check file exists and has reasonable size
ls -lh backups/meilisearch_TIMESTAMP.snapshot

# Alternative: If using legacy tar.gz backup format
# tar tzf backups/meilisearch_TIMESTAMP.tar.gz
```

---

## Monitoring & Logging

### Log Management

View logs in real-time:

```bash
# All services
docker compose -f docker-compose.prod.yml logs -f

# Specific service
docker compose -f docker-compose.prod.yml logs -f resonance

# Last 100 lines
docker compose -f docker-compose.prod.yml logs --tail=100 resonance
```

### Log Rotation

Docker's json-file driver is configured in `docker-compose.prod.yml` with automatic rotation.

For system-level log aggregation, consider:

- **Loki + Grafana** - Docker log driver integration
- **ELK Stack** - Elasticsearch, Logstash, Kibana
- **Datadog** - Commercial monitoring solution

### Health Monitoring Script

Create `/opt/resonance/healthcheck.sh`:

```bash
#!/bin/bash
# Health check script for monitoring integration

set -euo pipefail

API_URL="http://localhost:4441/health"
WEB_URL="http://localhost:4440"

check_service() {
    local name=$1
    local url=$2
    if curl -sf "${url}" > /dev/null 2>&1; then
        echo "OK: ${name}"
        return 0
    else
        echo "FAIL: ${name}"
        return 1
    fi
}

# Check all services
FAILED=0

check_service "API" "${API_URL}" || FAILED=1
check_service "Web" "${WEB_URL}" || FAILED=1

# Check Docker containers
for service in resonance resonance-worker postgres redis meilisearch; do
    if docker compose -f docker-compose.prod.yml ps "${service}" | grep -q "healthy\|running"; then
        echo "OK: Container ${service}"
    else
        echo "FAIL: Container ${service}"
        FAILED=1
    fi
done

exit ${FAILED}
```

### Prometheus Metrics (Optional)

Enable metrics endpoint in `.env.production`:

```bash
METRICS_ENABLED=true
```

Prometheus scrape config:

```yaml
scrape_configs:
  - job_name: 'resonance'
    static_configs:
      - targets: ['resonance:8080']
    metrics_path: '/metrics'
```

---

## Upgrade Procedures

### Standard Upgrade

```bash
# 1. Backup first (always!)
/opt/resonance/backup.sh

# 2. Pull latest changes
cd /path/to/resonance
git fetch origin
git checkout main
git pull

# 3. Review changes
git log --oneline HEAD@{1}..HEAD

# 4. Pull new images
docker compose -f docker-compose.prod.yml pull

# 5. Rebuild custom images
docker compose -f docker-compose.prod.yml build --no-cache

# 6. Apply database migrations (if any)
# Migrations run automatically on API startup

# 7. Rolling restart
docker compose -f docker-compose.prod.yml up -d

# 8. Verify health
docker compose -f docker-compose.prod.yml ps
curl -f http://localhost:4441/health
```

### Major Version Upgrade

For major version upgrades with breaking changes:

```bash
# 1. Create full backup
/opt/resonance/backup.sh

# 2. Read release notes and migration guide
# Check: https://github.com/resonance-stream/resonance/releases

# 3. Stop all services
docker compose -f docker-compose.prod.yml down

# 4. Update source code
git fetch origin
git checkout v2.0.0  # or appropriate version tag

# 5. Review and update environment variables
diff .env.example .env.production

# 6. Rebuild and start
docker compose -f docker-compose.prod.yml build --no-cache
docker compose -f docker-compose.prod.yml up -d

# 7. Run any manual migration steps from release notes

# 8. Verify functionality
```

### Rollback Procedure

If an upgrade fails:

```bash
# 1. Stop services
docker compose -f docker-compose.prod.yml down

# 2. Checkout previous version
git checkout v1.5.0  # previous working version

# 3. Restore database if needed
/opt/resonance/restore.sh postgres_TIMESTAMP.dump.gz

# 4. Rebuild and start
docker compose -f docker-compose.prod.yml build
docker compose -f docker-compose.prod.yml up -d
```

---

## Disaster Recovery

### Complete System Recovery

If you need to recover on a new server:

```bash
# 1. Install prerequisites
apt update && apt install -y docker.io docker-compose-v2 git

# 2. Clone repository
git clone https://github.com/resonance-stream/resonance.git
cd resonance

# 3. Restore configuration
# Copy .env.production from backup or recreate

# 4. Start infrastructure services first
docker compose -f docker-compose.prod.yml up -d postgres redis meilisearch

# 5. Wait for services to be healthy
sleep 30

# 6. Restore PostgreSQL
gunzip -c backup/postgres_latest.dump.gz | \
    docker compose -f docker-compose.prod.yml exec -T postgres \
    pg_restore -U resonance -d resonance --clean

# 7. Restore Redis
docker compose -f docker-compose.prod.yml cp \
    backup/redis_latest.rdb redis:/data/dump.rdb
docker compose -f docker-compose.prod.yml restart redis

# 8. Restore Meilisearch (or let it rebuild from database)
# Meilisearch can rebuild indexes from the database automatically

# 9. Start remaining services
docker compose -f docker-compose.prod.yml up -d

# 10. Verify functionality
curl -f http://localhost:4441/health
```

### Data Recovery Priority

1. **PostgreSQL** - Contains all user data, playlists, settings
2. **Redis** - Session data (users will need to re-login if lost)
3. **Meilisearch** - Search indexes (can be rebuilt from database)
4. **Ollama** - AI models (can be re-downloaded)

---

## Troubleshooting

### Common Issues

#### Services Won't Start

```bash
# Check logs for errors
docker compose -f docker-compose.prod.yml logs

# Check for port conflicts
sudo netstat -tlnp | grep -E '4440|4441|5432|6379|7700'

# Verify environment file
docker compose -f docker-compose.prod.yml config
```

#### Database Connection Issues

```bash
# Test database connection
docker compose -f docker-compose.prod.yml exec postgres \
    psql -U resonance -d resonance -c "SELECT 1"

# Check database logs
docker compose -f docker-compose.prod.yml logs postgres

# Verify DATABASE_URL format
echo $DATABASE_URL
```

#### High Memory Usage

```bash
# Check container resource usage
docker stats

# Adjust limits in docker-compose.prod.yml
# Reduce Meilisearch memory if needed
```

#### Search Not Working

```bash
# Check Meilisearch health
curl http://localhost:7700/health

# Rebuild search indexes (via worker)
docker compose -f docker-compose.prod.yml restart resonance-worker
```

#### WebSocket Connection Failures

```bash
# Verify WebSocket endpoint is accessible
curl -i -N \
    -H "Connection: Upgrade" \
    -H "Upgrade: websocket" \
    -H "Sec-WebSocket-Key: test" \
    -H "Sec-WebSocket-Version: 13" \
    http://localhost:4441/ws

# Check reverse proxy WebSocket configuration
```

### Getting Help

- **GitHub Issues**: [resonance-stream/resonance/issues](https://github.com/resonance-stream/resonance/issues)
- **Documentation**: Check the `docs/` directory
- **Community**: Join our Discord server

### Debug Mode

For debugging production issues:

```bash
# Enable verbose logging (temporarily)
docker compose -f docker-compose.prod.yml exec resonance \
    env RUST_LOG=debug /app/resonance-api

# Or update .env.production temporarily
RUST_LOG=debug,sqlx=info
```

---

## Additional Resources

- [Docker Compose Reference](https://docs.docker.com/compose/)
- [PostgreSQL Administration](https://www.postgresql.org/docs/current/admin.html)
- [Redis Administration](https://redis.io/docs/management/)
- [Meilisearch Documentation](https://www.meilisearch.com/docs)

---

*Last updated: 2025*
