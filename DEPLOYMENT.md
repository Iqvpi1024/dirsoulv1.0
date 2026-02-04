# DirSoul Docker Deployment Guide

Complete guide for deploying DirSoul using Docker and Docker Compose.

## Prerequisites

- Docker 20.10+
- Docker Compose 2.0+
- 8GB RAM minimum (16GB recommended for Ollama models)
- 20GB disk space minimum

## Quick Start

### 1. Clone Repository

```bash
git clone https://github.com/iqvpi1024/dirsoulv1.0.git
cd dirsoulv1.0
```

### 2. Configure Environment

```bash
# Copy example environment file
cp .env.example .env

# Edit configuration
nano .env
```

### 3. Start Services

```bash
# Start all services
docker-compose up -d

# Check status
docker-compose ps
```

### 4. Initialize Database

```bash
# Run migrations
docker-compose exec app diesel migration run

# Or use the initialization script
docker-compose exec app bash scripts/init_db.sh
```

### 5. Pull Ollama Models

```bash
# Pull embedding model (nomic-embed-text-v1.5)
docker-compose exec ollama ollama pull nomic-embed-text:v1.5

# Pull inference model (phi4-mini)
docker-compose exec ollama ollama pull phi4-mini
```

### 6. Access Services

- **API**: http://localhost:8080
- **Streamlit UI**: http://localhost:8501
- **MinIO Console**: http://localhost:9001
- **Health Check**: http://localhost:8080/health

## Services

| Service | Port | Description |
|---------|------|-------------|
| app | 8080, 8501 | DirSoul API + Streamlit UI |
| db | 5432 | PostgreSQL database |
| ollama | 11434 | Local LLM service |
| minio | 9000, 9001 | Object storage (S3-compatible) |
| telegram-bot | - | Telegram bot (optional) |

## Configuration

### Environment Variables

Create a `.env` file:

```bash
# Database
POSTGRES_USER=dirsoul
POSTGRES_PASSWORD=your_secure_password
POSTGRES_DB=dirsoul_db

# DirSoul API
DIRSOUL_API_HOST=0.0.0.0
DIRSOUL_API_PORT=8080

# Ollama
OLLAMA_HOST=http://ollama:11434

# MinIO
MINIO_ENDPOINT=minio:9000
MINIO_ACCESS_KEY=your_access_key
MINIO_SECRET_KEY=your_secret_key
MINIO_BUCKET=dirsoul-cold-data

# Telegram Bot (optional)
TELEGRAM_BOT_TOKEN=your_bot_token
```

### Volume Mounts

- `./data:/app/data` - Application data
- `./config:/app/config` - Configuration files
- `./prompts:/app/prompts` - Prompt templates
- `./logs:/app/logs` - Log files

## Common Commands

### Start Services

```bash
# Start all services
docker-compose up -d

# Start specific service
docker-compose up -d app

# Start with Telegram bot
docker-compose --profile telegram up -d
```

### Stop Services

```bash
# Stop all services
docker-compose down

# Stop and remove volumes
docker-compose down -v
```

### View Logs

```bash
# View all logs
docker-compose logs -f

# View specific service logs
docker-compose logs -f app
docker-compose logs -f db

# View last 100 lines
docker-compose logs --tail=100 app
```

### Execute Commands

```bash
# Enter app container
docker-compose exec app bash

# Run Rust tests
docker-compose exec app cargo test

# Run Python tests
docker-compose exec app python -m pytest

# Access PostgreSQL
docker-compose exec db psql -U dirsoul -d dirsoul_db
```

### Database Management

```bash
# Run migrations
docker-compose exec app diesel migration run

# Rollback migration
docker-compose exec app diesel migration revert

# Reset database
docker-compose exec db psql -U dirsoul -c "DROP DATABASE IF EXISTS dirsoul_db; CREATE DATABASE dirsoul_db;"
```

### Ollama Management

```bash
# List models
docker-compose exec ollama ollama list

# Pull model
docker-compose exec ollama ollama pull phi4-mini

# Run model
docker-compose exec ollama ollama run phi4-mini "Hello"

# Show model info
docker-compose exec ollama ollama show phi4-mini
```

## Production Deployment

### Resource Limits

Edit `docker-compose.yml` to add resource limits:

```yaml
services:
  app:
    deploy:
      resources:
        limits:
          cpus: '2'
          memory: 4G
        reservations:
          cpus: '1'
          memory: 2G
```

### Reverse Proxy (Nginx)

```nginx
server {
    listen 80;
    server_name your-domain.com;

    location /api {
        proxy_pass http://localhost:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }

    location / {
        proxy_pass http://localhost:8501;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
    }
}
```

### SSL/HTTPS

Use Certbot for free SSL:

```bash
sudo apt-get install certbot python3-certbot-nginx
sudo certbot --nginx -d your-domain.com
```

### Backup Strategy

```bash
# Backup database
docker-compose exec db pg_dump -U dirsoul dirsoul_db > backup.sql

# Backup volumes
docker run --rm -v dirsoul_postgres_data:/data -v $(pwd):/backup alpine tar czf /backup/postgres_backup.tar.gz /data

# Automated backup script
./scripts/backup.sh
```

### Monitoring

```bash
# Container stats
docker stats

# Service health
curl http://localhost:8080/health

# Database connections
docker-compose exec db psql -U dirsoul -c "SELECT count(*) FROM pg_stat_activity;"
```

## Troubleshooting

### Container Won't Start

```bash
# Check logs
docker-compose logs app

# Check resource usage
docker stats

# Restart service
docker-compose restart app
```

### Database Connection Issues

```bash
# Check if database is running
docker-compose ps db

# Test connection
docker-compose exec db pg_isready -U dirsoul

# Check logs
docker-compose logs db
```

### Ollama Not Responding

```bash
# Check Ollama status
docker-compose exec ollama ollama list

# Restart Ollama
docker-compose restart ollama

# Check model availability
curl http://localhost:11434/api/tags
```

### Out of Memory

```bash
# Check memory usage
docker stats

# Stop unused services
docker-compose stop telegram-bot

# Increase swap space
sudo fallocate -l 4G /swapfile
sudo chmod 600 /swapfile
sudo mkswap /swapfile
sudo swapon /swapfile
```

## Upgrading

### Update Application

```bash
# Pull latest changes
git pull

# Rebuild containers
docker-compose build

# Restart services
docker-compose up -d
```

### Database Migration

```bash
# Run new migrations
docker-compose exec app diesel migration run

# Verify migration status
docker-compose exec app diesel migration list
```

## Development

### Build Without Cache

```bash
docker-compose build --no-cache
```

### Mount Source Code

```yaml
# In docker-compose.yml
services:
  app:
    volumes:
      - ./src/rust:/app/src/rust:ro
      - ./src/python:/app/src/python:ro
```

### Hot Reload

```bash
# Watch for changes and rebuild
docker-compose watch
```

## Security

### Change Default Passwords

```bash
# Generate strong passwords
openssl rand -base64 32

# Update .env file
nano .env
```

### Network Isolation

```yaml
# Create separate network for sensitive services
networks:
  dirsoul-network:
    internal: true
  external-network:
    external: true
```

### Secrets Management

```bash
# Use Docker secrets (Swarm mode)
echo "your_secret" | docker secret create db_password -
```

## Performance Tuning

### PostgreSQL Optimization

```sql
-- Increase shared_buffers
ALTER SYSTEM SET shared_buffers = '256MB';

-- Increase effective_cache_size
ALTER SYSTEM SET effective_cache_size = '1GB';

-- Apply changes
SELECT pg_reload_conf();
```

### Ollama Optimization

```bash
# Use quantized models
docker-compose exec ollama ollama pull phi4-mini:q4_0

# Set number of threads
OLLAMA_NUM_THREADS=4 docker-compose up -d ollama
```

## Support

- **Documentation**: See `docs/` directory
- **Issues**: https://github.com/iqvpi1024/dirsoulv1.0/issues
- **Chat**: https://github.com/iqvpi1024/dirsoulv1.0/discussions
