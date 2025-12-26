# 🔧 Troubleshooting Guide

## Common Issues and Solutions

---

## Container Issues

### Service won't start

```bash
# Check container logs
docker compose logs <service-name>

# Check if port is in use
sudo lsof -i :<port>

# Restart specific service
docker compose restart <service-name>
```

### Out of memory

```bash
# Check container stats
docker stats

# Increase Docker memory limit
# Edit /etc/docker/daemon.json
```

### Build fails

```bash
# Clean build cache
docker builder prune

# Rebuild without cache
docker compose build --no-cache <service-name>
```

---

## Database Issues

### PostgreSQL connection refused

1. Check if container is running: `docker compose ps postgres`
2. Check logs: `docker compose logs postgres`
3. Verify credentials in `.env` file
4. Test connection: `docker compose exec postgres psql -U webapp -d orderdb`

### Migration fails

```bash
# Connect to database and check tables
docker compose exec postgres psql -U webapp -d orderdb -c "\dt"

# Reset database (WARNING: loses data)
docker compose down -v postgres
docker compose up -d postgres
```

---

## Network Issues

### Services can't communicate

```bash
# Check if services are on same network
docker network inspect grafana-lab-webapp_webapp-network

# Test connectivity from one container
docker compose exec order-service ping inventory-service
```

### Prometheus can't scrape targets

1. Check if metrics endpoint is accessible:
   ```bash
   curl http://localhost:8001/metrics
   ```
2. Verify Prometheus config has correct IP
3. Check firewall rules

---

## Metrics Issues

### No data in Grafana

1. Check Prometheus targets: http://localhost:9090/targets
2. Verify scrape config in prometheus.yml
3. Check service is exposing /metrics

### Wrong metrics values

- Counter resets on service restart (normal)
- Check time range in Grafana
- Verify metric labels match query

---

## Log Issues

### Logs not appearing in Loki

1. Check Promtail is running: `docker compose ps promtail`
2. Verify Loki URL in promtail config
3. Check Promtail logs: `docker compose logs promtail`

---

## Quick Fixes

```bash
# Restart everything
docker compose down && docker compose up -d

# Clean restart
docker compose down -v && docker compose up -d --build

# Check all service health
./scripts/health-check.sh
```
