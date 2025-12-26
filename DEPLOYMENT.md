# 🚀 Deployment Guide

## Step-by-Step Instructions for Linux Lab Environment

This guide walks you through deploying both the webapp server and the Grafana stack from scratch.

---

## 📑 Table of Contents

1. [Prerequisites](#1-prerequisites)
2. [VM Preparation](#2-vm-preparation)
3. [Deploy Webapp Server](#3-deploy-webapp-server)
4. [Deploy Grafana Stack](#4-deploy-grafana-stack)
5. [Configure Prometheus Scraping](#5-configure-prometheus-scraping)
6. [Verify Deployment](#6-verify-deployment)
7. [Access Services](#7-access-services)
8. [Troubleshooting](#8-troubleshooting)

---

## 1. Prerequisites

### 1.1 Hardware Requirements

| Component | Webapp VM | Grafana VM |
|-----------|-----------|------------|
| CPU | 2+ cores | 2+ cores |
| RAM | 4GB minimum, 8GB recommended | 4GB minimum |
| Disk | 20GB | 50GB (for metrics storage) |
| Network | Static IP recommended | Static IP recommended |

### 1.2 Software Requirements

- **OS**: Ubuntu 22.04 LTS or 24.04 LTS (recommended)
- **Docker**: 24.0+ with Compose V2
- **Git**: For cloning the repository

### 1.3 Network Requirements

Ensure these ports can communicate between VMs:

```
Webapp VM → Grafana VM:
  • TCP 3100 (Loki - log shipping)
  • TCP 4317 (Tempo - traces, optional)

Grafana VM → Webapp VM:
  • TCP 8001-8005 (microservice metrics)
  • TCP 9100 (Node Exporter)
  • TCP 8081 (cAdvisor)
  • TCP 9187 (Postgres Exporter)
  • TCP 9121 (Redis Exporter)
  • TCP 8080 (Traefik metrics)
```

---

## 2. VM Preparation

### 2.1 Update System (Both VMs)

```bash
# Update package list and upgrade existing packages
sudo apt update && sudo apt upgrade -y

# Install essential tools
sudo apt install -y \
    curl \
    wget \
    git \
    vim \
    htop \
    net-tools \
    jq
```

### 2.2 Install Docker (Both VMs)

```bash
# Remove old Docker versions (if any)
sudo apt remove -y docker docker-engine docker.io containerd runc 2>/dev/null

# Install prerequisites
sudo apt install -y \
    ca-certificates \
    curl \
    gnupg \
    lsb-release

# Add Docker's official GPG key
sudo mkdir -p /etc/apt/keyrings
curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo gpg --dearmor -o /etc/apt/keyrings/docker.gpg

# Add Docker repository
echo \
  "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/ubuntu \
  $(lsb_release -cs) stable" | sudo tee /etc/apt/sources.list.d/docker.list > /dev/null

# Install Docker
sudo apt update
sudo apt install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin

# Add your user to docker group (logout/login required)
sudo usermod -aG docker $USER

# Verify installation
docker --version
docker compose version
```

### 2.3 Configure Firewall (Both VMs)

```bash
# If using ufw (Ubuntu's default firewall)
# Webapp VM
sudo ufw allow 22/tcp     # SSH
sudo ufw allow 80/tcp     # HTTP (Traefik)
sudo ufw allow 3000/tcp   # Frontend
sudo ufw allow 8080/tcp   # Traefik Dashboard
sudo ufw allow 8001:8005/tcp  # Microservices (for Prometheus)
sudo ufw allow 9100/tcp   # Node Exporter
sudo ufw allow 8081/tcp   # cAdvisor
sudo ufw allow 9187/tcp   # Postgres Exporter
sudo ufw allow 9121/tcp   # Redis Exporter

# Grafana VM
sudo ufw allow 22/tcp     # SSH
sudo ufw allow 3000/tcp   # Grafana
sudo ufw allow 9090/tcp   # Prometheus
sudo ufw allow 3100/tcp   # Loki
sudo ufw allow 9093/tcp   # Alertmanager

# Enable firewall
sudo ufw enable
sudo ufw status
```

---

## 3. Deploy Webapp Server

### 3.1 Clone/Copy Project

```bash
# Create project directory
mkdir -p ~/projects
cd ~/projects

# Option A: If you have git access
git clone <your-repo-url> grafana-lab-webapp

# Option B: Copy files manually
# Upload the project folder to ~/projects/grafana-lab-webapp
```

### 3.2 Configure Environment

```bash
cd ~/projects/grafana-lab-webapp

# Copy environment template
cp .env.example .env

# Edit environment variables
vim .env
```

**Edit `.env` file:**

```bash
# .env file contents

# ===========================================
# Database Configuration
# ===========================================
POSTGRES_USER=webapp
POSTGRES_PASSWORD=your_secure_password_here
POSTGRES_DB=orderdb

# ===========================================
# Redis Configuration
# ===========================================
REDIS_PASSWORD=your_redis_password_here

# ===========================================
# RabbitMQ Configuration
# ===========================================
RABBITMQ_USER=webapp
RABBITMQ_PASSWORD=your_rabbitmq_password_here

# ===========================================
# JWT Configuration
# ===========================================
JWT_SECRET=your_super_secret_jwt_key_minimum_32_chars

# ===========================================
# Grafana Stack Configuration
# Replace with your Grafana VM IP address
# ===========================================
LOKI_URL=http://192.168.1.20:3100
TEMPO_URL=http://192.168.1.20:4317

# ===========================================
# Application Settings
# ===========================================
NODE_ENV=production
LOG_LEVEL=info
```

### 3.3 Build and Start Services

```bash
# Build all Docker images (this may take 10-15 minutes first time)
docker compose build

# Start all services in detached mode
docker compose up -d

# Watch logs (optional, Ctrl+C to exit)
docker compose logs -f

# Check all services are running
docker compose ps
```

### 3.4 Verify Services

```bash
# Check each service health endpoint
curl http://localhost:8001/health  # Order Service
curl http://localhost:8002/health  # Inventory Service (Rust)
curl http://localhost:8003/health  # Payment Service
curl http://localhost:8004/health  # User Service
curl http://localhost:8005/health  # Notification Service

# Check metrics endpoints
curl http://localhost:8001/metrics | head -20
curl http://localhost:8002/metrics | head -20

# Check frontend
curl http://localhost:3000 | head -20
```

---

## 4. Deploy Grafana Stack

### 4.1 Create Grafana Stack Directory

```bash
# On Grafana VM
mkdir -p ~/grafana-stack/{prometheus,loki,tempo,grafana,alertmanager}
cd ~/grafana-stack
```

### 4.2 Create Docker Compose for Grafana Stack

```bash
cat > docker-compose.yml << 'EOF'
# =============================================================================
# GRAFANA STACK - docker-compose.yml
# =============================================================================
# This file deploys the complete Grafana observability stack:
# - Prometheus: Metrics collection and storage
# - Loki: Log aggregation
# - Tempo: Distributed tracing
# - Grafana: Visualization
# - Alertmanager: Alert routing
# =============================================================================

version: '3.8'

# =============================================================================
# NETWORKS
# =============================================================================
networks:
  grafana-net:
    driver: bridge

# =============================================================================
# VOLUMES
# =============================================================================
volumes:
  prometheus-data:
  loki-data:
  tempo-data:
  grafana-data:

# =============================================================================
# SERVICES
# =============================================================================
services:

  # ---------------------------------------------------------------------------
  # PROMETHEUS - Metrics Collection
  # ---------------------------------------------------------------------------
  prometheus:
    image: prom/prometheus:v2.48.0
    container_name: prometheus
    restart: unless-stopped
    ports:
      - "9090:9090"
    volumes:
      - ./prometheus/prometheus.yml:/etc/prometheus/prometheus.yml:ro
      - ./prometheus/alerts.yml:/etc/prometheus/alerts.yml:ro
      - prometheus-data:/prometheus
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.path=/prometheus'
      - '--storage.tsdb.retention.time=15d'
      - '--web.enable-lifecycle'
      - '--web.enable-admin-api'
    networks:
      - grafana-net

  # ---------------------------------------------------------------------------
  # LOKI - Log Aggregation
  # ---------------------------------------------------------------------------
  loki:
    image: grafana/loki:2.9.2
    container_name: loki
    restart: unless-stopped
    ports:
      - "3100:3100"
    volumes:
      - ./loki/loki-config.yml:/etc/loki/loki-config.yml:ro
      - loki-data:/loki
    command: -config.file=/etc/loki/loki-config.yml
    networks:
      - grafana-net

  # ---------------------------------------------------------------------------
  # TEMPO - Distributed Tracing
  # ---------------------------------------------------------------------------
  tempo:
    image: grafana/tempo:2.3.1
    container_name: tempo
    restart: unless-stopped
    ports:
      - "3200:3200"   # Tempo API
      - "4317:4317"   # OTLP gRPC
      - "4318:4318"   # OTLP HTTP
    volumes:
      - ./tempo/tempo-config.yml:/etc/tempo/tempo-config.yml:ro
      - tempo-data:/var/tempo
    command: -config.file=/etc/tempo/tempo-config.yml
    networks:
      - grafana-net

  # ---------------------------------------------------------------------------
  # ALERTMANAGER - Alert Routing
  # ---------------------------------------------------------------------------
  alertmanager:
    image: prom/alertmanager:v0.26.0
    container_name: alertmanager
    restart: unless-stopped
    ports:
      - "9093:9093"
    volumes:
      - ./alertmanager/alertmanager.yml:/etc/alertmanager/alertmanager.yml:ro
    command:
      - '--config.file=/etc/alertmanager/alertmanager.yml'
      - '--storage.path=/alertmanager'
    networks:
      - grafana-net

  # ---------------------------------------------------------------------------
  # GRAFANA - Visualization
  # ---------------------------------------------------------------------------
  grafana:
    image: grafana/grafana:10.2.2
    container_name: grafana
    restart: unless-stopped
    ports:
      - "3000:3000"
    environment:
      - GF_SECURITY_ADMIN_USER=admin
      - GF_SECURITY_ADMIN_PASSWORD=admin123
      - GF_USERS_ALLOW_SIGN_UP=false
    volumes:
      - grafana-data:/var/lib/grafana
      - ./grafana/provisioning:/etc/grafana/provisioning:ro
    depends_on:
      - prometheus
      - loki
      - tempo
    networks:
      - grafana-net
EOF
```

### 4.3 Create Prometheus Configuration

```bash
# Create Prometheus config
# IMPORTANT: Replace 192.168.1.10 with your Webapp VM IP address

cat > prometheus/prometheus.yml << 'EOF'
# =============================================================================
# PROMETHEUS CONFIGURATION
# =============================================================================
# Global settings and scrape configurations for all metrics sources
# =============================================================================

global:
  # How frequently to scrape targets
  scrape_interval: 15s
  
  # How frequently to evaluate alerting rules
  evaluation_interval: 15s
  
  # Attach these labels to any time series or alerts
  external_labels:
    environment: 'lab'
    region: 'local'

# =============================================================================
# ALERTMANAGER CONFIGURATION
# =============================================================================
alerting:
  alertmanagers:
    - static_configs:
        - targets:
          - alertmanager:9093

# =============================================================================
# RULE FILES
# =============================================================================
rule_files:
  - /etc/prometheus/alerts.yml

# =============================================================================
# SCRAPE CONFIGURATIONS
# =============================================================================
scrape_configs:

  # ---------------------------------------------------------------------------
  # Prometheus Self-Monitoring
  # ---------------------------------------------------------------------------
  - job_name: 'prometheus'
    static_configs:
      - targets: ['localhost:9090']
        labels:
          service: 'prometheus'
          tier: 'monitoring'

  # ---------------------------------------------------------------------------
  # WEBAPP VM - Microservices
  # IMPORTANT: Replace 192.168.1.10 with your Webapp VM IP
  # ---------------------------------------------------------------------------
  
  # Order Service (Go)
  - job_name: 'order-service'
    static_configs:
      - targets: ['192.168.1.10:8001']
        labels:
          service: 'order-service'
          language: 'go'
          tier: 'application'
    metrics_path: /metrics

  # Inventory Service (Rust)
  - job_name: 'inventory-service'
    static_configs:
      - targets: ['192.168.1.10:8002']
        labels:
          service: 'inventory-service'
          language: 'rust'
          tier: 'application'
    metrics_path: /metrics

  # Payment Service (Python)
  - job_name: 'payment-service'
    static_configs:
      - targets: ['192.168.1.10:8003']
        labels:
          service: 'payment-service'
          language: 'python'
          tier: 'application'
    metrics_path: /metrics

  # User Service (Java/Spring Boot)
  - job_name: 'user-service'
    static_configs:
      - targets: ['192.168.1.10:8004']
        labels:
          service: 'user-service'
          language: 'java'
          tier: 'application'
    metrics_path: /actuator/prometheus

  # Notification Service (Node.js)
  - job_name: 'notification-service'
    static_configs:
      - targets: ['192.168.1.10:8005']
        labels:
          service: 'notification-service'
          language: 'nodejs'
          tier: 'application'
    metrics_path: /metrics

  # ---------------------------------------------------------------------------
  # WEBAPP VM - Infrastructure Components
  # ---------------------------------------------------------------------------
  
  # Traefik (API Gateway)
  - job_name: 'traefik'
    static_configs:
      - targets: ['192.168.1.10:8080']
        labels:
          service: 'traefik'
          tier: 'infrastructure'
    metrics_path: /metrics

  # ---------------------------------------------------------------------------
  # WEBAPP VM - OS and Container Metrics
  # ---------------------------------------------------------------------------
  
  # Node Exporter (Host OS Metrics)
  - job_name: 'node-exporter'
    static_configs:
      - targets: ['192.168.1.10:9100']
        labels:
          instance: 'webapp-vm'
          tier: 'infrastructure'

  # cAdvisor (Container Metrics)
  - job_name: 'cadvisor'
    static_configs:
      - targets: ['192.168.1.10:8081']
        labels:
          instance: 'webapp-vm'
          tier: 'infrastructure'

  # ---------------------------------------------------------------------------
  # WEBAPP VM - Database Exporters
  # ---------------------------------------------------------------------------
  
  # PostgreSQL Exporter
  - job_name: 'postgres-exporter'
    static_configs:
      - targets: ['192.168.1.10:9187']
        labels:
          service: 'postgresql'
          tier: 'database'

  # Redis Exporter
  - job_name: 'redis-exporter'
    static_configs:
      - targets: ['192.168.1.10:9121']
        labels:
          service: 'redis'
          tier: 'database'
EOF
```

### 4.4 Create Alert Rules

```bash
cat > prometheus/alerts.yml << 'EOF'
# =============================================================================
# PROMETHEUS ALERT RULES
# =============================================================================

groups:
  # ---------------------------------------------------------------------------
  # Service Health Alerts
  # ---------------------------------------------------------------------------
  - name: service_health
    rules:
      # Alert when a service is down
      - alert: ServiceDown
        expr: up == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "Service {{ $labels.job }} is down"
          description: "{{ $labels.job }} has been down for more than 1 minute."

      # Alert when error rate is high
      - alert: HighErrorRate
        expr: |
          sum(rate(http_requests_total{status=~"5.."}[5m])) by (service)
          /
          sum(rate(http_requests_total[5m])) by (service)
          > 0.05
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High error rate on {{ $labels.service }}"
          description: "Error rate is {{ $value | humanizePercentage }} on {{ $labels.service }}"

  # ---------------------------------------------------------------------------
  # Infrastructure Alerts
  # ---------------------------------------------------------------------------
  - name: infrastructure
    rules:
      # High CPU usage
      - alert: HighCPUUsage
        expr: 100 - (avg by(instance) (rate(node_cpu_seconds_total{mode="idle"}[5m])) * 100) > 80
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High CPU usage on {{ $labels.instance }}"
          description: "CPU usage is above 80% for 5 minutes"

      # High Memory usage
      - alert: HighMemoryUsage
        expr: (1 - (node_memory_MemAvailable_bytes / node_memory_MemTotal_bytes)) * 100 > 80
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High memory usage on {{ $labels.instance }}"
          description: "Memory usage is above 80%"

      # Disk space low
      - alert: DiskSpaceLow
        expr: (node_filesystem_avail_bytes{mountpoint="/"} / node_filesystem_size_bytes{mountpoint="/"}) * 100 < 20
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Low disk space on {{ $labels.instance }}"
          description: "Disk space is below 20%"

  # ---------------------------------------------------------------------------
  # Application Alerts
  # ---------------------------------------------------------------------------
  - name: application
    rules:
      # High latency
      - alert: HighLatency
        expr: |
          histogram_quantile(0.95, 
            sum(rate(http_request_duration_seconds_bucket[5m])) by (le, service)
          ) > 1
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High latency on {{ $labels.service }}"
          description: "95th percentile latency is above 1 second"

      # Low inventory alert
      - alert: LowInventory
        expr: inventory_stock_level < 10
        for: 1m
        labels:
          severity: warning
        annotations:
          summary: "Low stock for SKU {{ $labels.sku }}"
          description: "Stock level is {{ $value }} units"
EOF
```

### 4.5 Create Loki Configuration

```bash
cat > loki/loki-config.yml << 'EOF'
# =============================================================================
# LOKI CONFIGURATION
# =============================================================================

auth_enabled: false

server:
  http_listen_port: 3100
  grpc_listen_port: 9096

common:
  instance_addr: 127.0.0.1
  path_prefix: /loki
  storage:
    filesystem:
      chunks_directory: /loki/chunks
      rules_directory: /loki/rules
  replication_factor: 1
  ring:
    kvstore:
      store: inmemory

query_range:
  results_cache:
    cache:
      embedded_cache:
        enabled: true
        max_size_mb: 100

schema_config:
  configs:
    - from: 2020-10-24
      store: boltdb-shipper
      object_store: filesystem
      schema: v11
      index:
        prefix: index_
        period: 24h

ruler:
  alertmanager_url: http://alertmanager:9093

analytics:
  reporting_enabled: false
EOF
```

### 4.6 Create Tempo Configuration

```bash
cat > tempo/tempo-config.yml << 'EOF'
# =============================================================================
# TEMPO CONFIGURATION
# =============================================================================

server:
  http_listen_port: 3200

distributor:
  receivers:
    otlp:
      protocols:
        grpc:
          endpoint: 0.0.0.0:4317
        http:
          endpoint: 0.0.0.0:4318

storage:
  trace:
    backend: local
    local:
      path: /var/tempo/traces
    wal:
      path: /var/tempo/wal

metrics_generator:
  registry:
    external_labels:
      source: tempo
  storage:
    path: /var/tempo/generator/wal
EOF
```

### 4.7 Create Alertmanager Configuration

```bash
cat > alertmanager/alertmanager.yml << 'EOF'
# =============================================================================
# ALERTMANAGER CONFIGURATION
# =============================================================================

global:
  # SMTP configuration for email alerts (configure if needed)
  # smtp_smarthost: 'smtp.gmail.com:587'
  # smtp_from: 'alertmanager@example.com'
  # smtp_auth_username: 'your-email@gmail.com'
  # smtp_auth_password: 'your-app-password'

route:
  group_by: ['alertname', 'service']
  group_wait: 10s
  group_interval: 10s
  repeat_interval: 1h
  receiver: 'default'

  routes:
    - match:
        severity: critical
      receiver: 'critical'
    - match:
        severity: warning
      receiver: 'warning'

receivers:
  - name: 'default'
    # Webhook receiver (for testing)
    webhook_configs:
      - url: 'http://localhost:5001/webhook'
        send_resolved: true

  - name: 'critical'
    webhook_configs:
      - url: 'http://localhost:5001/webhook'
        send_resolved: true

  - name: 'warning'
    webhook_configs:
      - url: 'http://localhost:5001/webhook'
        send_resolved: true

inhibit_rules:
  - source_match:
      severity: 'critical'
    target_match:
      severity: 'warning'
    equal: ['alertname', 'service']
EOF
```

### 4.8 Create Grafana Provisioning

```bash
# Create datasources provisioning
mkdir -p grafana/provisioning/datasources
cat > grafana/provisioning/datasources/datasources.yml << 'EOF'
# =============================================================================
# GRAFANA DATASOURCES PROVISIONING
# =============================================================================

apiVersion: 1

datasources:
  # Prometheus - Metrics
  - name: Prometheus
    type: prometheus
    access: proxy
    url: http://prometheus:9090
    isDefault: true
    editable: false

  # Loki - Logs
  - name: Loki
    type: loki
    access: proxy
    url: http://loki:3100
    editable: false
    jsonData:
      derivedFields:
        - name: TraceID
          matcherRegex: "trace_id=(\\w+)"
          url: "$${__value.raw}"
          datasourceUid: tempo

  # Tempo - Traces
  - name: Tempo
    type: tempo
    access: proxy
    url: http://tempo:3200
    uid: tempo
    editable: false
EOF
```

### 4.9 Start Grafana Stack

```bash
cd ~/grafana-stack

# Start all services
docker compose up -d

# Check status
docker compose ps

# View logs
docker compose logs -f
```

---

## 5. Configure Prometheus Scraping

### 5.1 Update Prometheus Targets

Edit `~/grafana-stack/prometheus/prometheus.yml` and replace `192.168.1.10` with your actual Webapp VM IP address.

```bash
# Get Webapp VM IP
hostname -I

# Edit Prometheus config
vim ~/grafana-stack/prometheus/prometheus.yml

# After editing, reload Prometheus
curl -X POST http://localhost:9090/-/reload
```

### 5.2 Verify Scraping

```bash
# Check Prometheus targets
curl http://localhost:9090/api/v1/targets | jq '.data.activeTargets[] | {job: .labels.job, health: .health}'

# Or open in browser
# http://<grafana-vm-ip>:9090/targets
```

---

## 6. Verify Deployment

### 6.1 Health Check Script

Create and run this script on both VMs:

```bash
cat > ~/check-health.sh << 'EOF'
#!/bin/bash
# =============================================================================
# Health Check Script
# =============================================================================

echo "=========================================="
echo "Docker Status"
echo "=========================================="
docker ps --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}"

echo ""
echo "=========================================="
echo "Container Resource Usage"
echo "=========================================="
docker stats --no-stream --format "table {{.Name}}\t{{.CPUPerc}}\t{{.MemUsage}}"

echo ""
echo "=========================================="
echo "Disk Usage"
echo "=========================================="
df -h / | tail -1

echo ""
echo "=========================================="
echo "Memory Usage"
echo "=========================================="
free -h

echo ""
echo "=========================================="
echo "Service Health Checks"
echo "=========================================="
services=(
    "http://localhost:8001/health:Order-Service"
    "http://localhost:8002/health:Inventory-Service"
    "http://localhost:8003/health:Payment-Service"
    "http://localhost:8004/health:User-Service"
    "http://localhost:8005/health:Notification-Service"
    "http://localhost:9090/-/healthy:Prometheus"
    "http://localhost:3100/ready:Loki"
    "http://localhost:3000/api/health:Grafana"
)

for service in "${services[@]}"; do
    url="${service%%:*}"
    name="${service##*:}"
    response=$(curl -s -o /dev/null -w "%{http_code}" "$url" 2>/dev/null || echo "000")
    if [ "$response" = "200" ]; then
        echo "✅ $name: OK"
    else
        echo "❌ $name: FAILED (HTTP $response)"
    fi
done
EOF

chmod +x ~/check-health.sh
~/check-health.sh
```

---

## 7. Access Services

### 7.1 Webapp Server URLs

| Service | URL | Credentials |
|---------|-----|-------------|
| Frontend UI | http://webapp-vm:3000 | - |
| Traefik Dashboard | http://webapp-vm:8080 | - |
| RabbitMQ Management | http://webapp-vm:15672 | guest/guest |

### 7.2 Grafana Stack URLs

| Service | URL | Credentials |
|---------|-----|-------------|
| Grafana | http://grafana-vm:3000 | admin/admin123 |
| Prometheus | http://grafana-vm:9090 | - |
| Alertmanager | http://grafana-vm:9093 | - |

---

## 8. Troubleshooting

### 8.1 Common Issues

#### Services not starting

```bash
# Check logs
docker compose logs <service-name>

# Check if ports are in use
sudo lsof -i :<port>
sudo netstat -tlnp | grep <port>
```

#### Prometheus can't scrape targets

```bash
# Test connectivity from Grafana VM to Webapp VM
curl http://<webapp-vm-ip>:8001/metrics

# Check firewall
sudo ufw status
sudo iptables -L -n
```

#### Container out of memory

```bash
# Check container limits
docker inspect <container-name> | grep -A 5 Memory

# Increase Docker memory limits in docker-compose.yml
```

#### Disk space issues

```bash
# Clean up Docker
docker system prune -a
docker volume prune
```

### 8.2 Useful Commands

```bash
# Restart all services
docker compose restart

# Rebuild and restart specific service
docker compose up -d --build <service-name>

# View real-time logs
docker compose logs -f --tail=100 <service-name>

# Enter container shell
docker compose exec <service-name> sh

# Check network connectivity
docker compose exec <service-name> ping <other-service>
```

---

## 📚 Next Steps

1. **Explore Grafana**: Create your first dashboard
2. **Set up Alerts**: Configure email/Slack notifications
3. **Generate Load**: Use the load generator script
4. **Break Things**: Kill containers, observe alerts

See [docs/DASHBOARDS.md](./docs/DASHBOARDS.md) for dashboard creation guide.

---

**Happy Monitoring! 🎯**
