# 📊 Grafana Observability Stack

This folder contains the complete Grafana observability stack to deploy on your **Grafana VM** (separate from the webapp VM).

## Components

| Component | Port | Purpose |
|-----------|------|---------|
| **Prometheus** | 9090 | Metrics collection & storage |
| **Alertmanager** | 9093 | Alert routing & notifications |
| **Grafana** | 3000 | Visualization & dashboards |
| **Loki** | 3100 | Log aggregation |
| **Tempo** | 3200, 4317, 4318 | Distributed tracing |

## What's Monitored

| Category | Source | Metrics |
|----------|--------|---------|
| **OS Metrics** | Node Exporter | CPU, Memory, Disk, Network |
| **Container Metrics** | cAdvisor | Per-container CPU/Memory |
| **Service/API Metrics** | Each service /metrics | Request rate, latency, errors |
| **Database Metrics** | postgres_exporter | Connections, queries, cache |
| **Cache Metrics** | redis_exporter | Memory, connections, commands |
| **Logs** | Promtail → Loki | All container logs (JSON parsed) |
| **Traces** | Tempo | Distributed request tracing |

## Pre-configured Alerts

The stack includes **30+ alert rules** for:

- 🔴 **Critical**: Service down, high CPU/memory (>95%), payment failures
- 🟡 **Warning**: High latency, error rate >5%, disk space <20%
- 🟢 **Info**: Container restarts, low inventory stock

## Quick Start

### 1. Copy to Grafana VM

```bash
# On your local machine
scp -r grafana-stack/ user@grafana-vm:/home/user/
```

### 2. Configure

```bash
# On Grafana VM
cd grafana-stack
cp .env.example .env

# Edit .env and set your webapp VM IP
nano .env
```

**Important:** Update `WEBAPP_VM_IP` in `.env` with your webapp VM's actual IP address.

### 3. Update Prometheus Config

Edit `prometheus/prometheus.yml` and replace all `${WEBAPP_VM_IP}` with your actual IP:

```bash
sed -i 's/\${WEBAPP_VM_IP}/192.168.1.100/g' prometheus/prometheus.yml
```

### 4. Start the Stack

```bash
docker compose up -d
```

### 5. Access Services

- **Grafana**: http://grafana-vm:3000 (admin/admin)
- **Prometheus**: http://grafana-vm:9090
- **Loki**: http://grafana-vm:3100

## Firewall Configuration

Ensure these ports are open on the Grafana VM:

```bash
# Allow Grafana
sudo ufw allow 3000/tcp

# Allow Prometheus (for debugging)
sudo ufw allow 9090/tcp

# Allow Loki (for Promtail from webapp VM)
sudo ufw allow 3100/tcp

# Allow Tempo OTLP (for traces from webapp VM)
sudo ufw allow 4317/tcp
sudo ufw allow 4318/tcp
```

## Verify Prometheus Targets

1. Open http://grafana-vm:9090/targets
2. All targets should show "UP" status
3. If targets are "DOWN", check:
   - Webapp VM firewall allows connections from Grafana VM
   - IP address is correct in prometheus.yml
   - Services are running on webapp VM

## Pre-configured Datasources

Grafana is auto-configured with:
- ✅ Prometheus (default)
- ✅ Loki (with trace correlation)
- ✅ Tempo (with log/metrics correlation)

## Directory Structure

```
grafana-stack/
├── docker-compose.yml          # Main orchestration
├── .env.example                 # Environment template
├── prometheus/
│   └── prometheus.yml          # Scrape configuration
├── grafana/
│   └── provisioning/
│       └── datasources/
│           └── datasources.yml # Auto-configured datasources
├── loki/
│   └── loki-config.yml         # Loki configuration
└── tempo/
    └── tempo-config.yml        # Tempo configuration
```

## Next Steps

1. Import dashboards in Grafana
2. Create alerts for critical metrics
3. Set up log-based alerts in Loki
4. Explore traces in Tempo
