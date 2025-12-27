# Enterprise Order Management System

## Grafana Observability Lab - Complete Microservices Project

This project is a **production-grade, enterprise-style Order Management System** designed specifically for learning **Grafana Stack observability**. It demonstrates real-world monitoring scenarios across multiple technologies.

---

## Table of Contents

1. [Project Purpose](#-project-purpose)
2. [Architecture Overview](#-architecture-overview)
3. [Technology Stack](#-technology-stack)
4. [Monitoring Scenarios Covered](#-monitoring-scenarios-covered)
5. [Quick Start](#-quick-start)
6. [Project Structure](#-project-structure)
7. [Learning Path](#-learning-path)
8. [Documentation](#-documentation)

---

## Project Purpose

This webapp serves as a **comprehensive observability laboratory** where you can:

- **Learn Grafana Stack**: Prometheus, Loki, Tempo, Grafana dashboards
- **Practice Polyglot Monitoring**: Each service uses a different language
- **Understand Enterprise Patterns**: Real-world microservices architecture
- **Experiment Safely**: Break things, create alerts, simulate failures

### Why Order Management System?

An e-commerce order system naturally requires:
- Multiple interconnected services (microservices pattern)
- Database transactions (PostgreSQL metrics)
- Caching layer (Redis metrics)
- Message queues (async processing)
- API gateway (traffic metrics)
- Background workers (job metrics)

This creates diverse, realistic monitoring scenarios.

---

##  Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           WEBAPP SERVER                                  │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  ┌────────────────────────────────────────────────────────────────┐    │
│  │              FRONTEND (Next.js + Shadcn/ui)                     │    │
│  │              Enterprise Dashboard UI - Port 3000                │    │
│  └────────────────────────────┬───────────────────────────────────┘    │
│                               │                                         │
│  ┌────────────────────────────┴───────────────────────────────────┐    │
│  │              API GATEWAY (Traefik)                              │    │
│  │              Routing, Load Balancing, Metrics - Port 80/8080    │    │
│  └──────┬──────────┬──────────┬──────────┬──────────┬─────────────┘    │
│         │          │          │          │          │                   │
│  ┌──────┴───┐ ┌────┴────┐ ┌───┴────┐ ┌───┴────┐ ┌───┴─────┐           │
│  │  Order   │ │Inventory│ │Payment │ │  User  │ │  Notif  │           │
│  │   Go     │ │  Rust │ │ Python │ │  Java  │ │ Node.js │           │
│  │  :8001   │ │  :8002  │ │ :8003  │ │ :8004  │ │  :8005  │           │
│  └────┬─────┘ └────┬────┘ └───┬────┘ └───┬────┘ └────┬────┘           │
│       │            │          │          │           │                  │
│  ┌────┴────────────┴──────────┴──────────┴───────────┴────┐            │
│  │                    DATA LAYER                          │            │
│  │  PostgreSQL :5432 │ Redis :6379 │ RabbitMQ :5672       │            │
│  └────────────────────────────────────────────────────────┘            │
│                                                                         │
│  ┌────────────────────────────────────────────────────────┐            │
│  │              OBSERVABILITY AGENTS                       │            │
│  │  • Node Exporter (OS metrics)     - Port 9100          │            │
│  │  • cAdvisor (Container metrics)   - Port 8081          │            │
│  │  • Promtail (Log shipping)                             │            │
│  └────────────────────────────────────────────────────────┘            │
│                               │                                         │
└───────────────────────────────┼─────────────────────────────────────────┘
                                │ Metrics / Logs / Traces
                                ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                      GRAFANA STACK VM (Separate)                         │
├─────────────────────────────────────────────────────────────────────────┤
│  • Prometheus  :9090  - Metrics collection & storage                    │
│  • Loki        :3100  - Log aggregation                                 │
│  • Tempo       :3200  - Distributed tracing                             │
│  • Grafana     :3000  - Visualization & dashboards                      │
│  • Alertmanager:9093  - Alert routing                                   │
└─────────────────────────────────────────────────────────────────────────┘
```

For detailed architecture explanation, see [ARCHITECTURE.md](./ARCHITECTURE.md)

---

##  Technology Stack

### Microservices (Polyglot)

| Service | Language | Framework | Port | Purpose |
|---------|----------|-----------|------|---------|
| Order | Go | Gin | 8001 | Order CRUD, workflow |
| Inventory | Rust | Axum | 8002 | Stock management |
| Payment | Python | FastAPI | 8003 | Payment processing |
| User | Java | Spring Boot | 8004 | Auth & user management |
| Notification | Node.js | Express | 8005 | Email/SMS worker |
| Frontend | TypeScript | Next.js | 3000 | Enterprise UI |

### Infrastructure

| Component | Purpose | Port |
|-----------|---------|------|
| Traefik | API Gateway, reverse proxy | 80, 8080 |
| PostgreSQL | Primary database | 5432 |
| Redis | Caching, sessions | 6379 |
| RabbitMQ | Message queue | 5672, 15672 |

### Observability (on Grafana Stack VM)

| Component | Purpose | Port |
|-----------|---------|------|
| Prometheus | Metrics collection | 9090 |
| Loki | Log aggregation | 3100 |
| Tempo | Distributed tracing | 3200 |
| Grafana | Dashboards | 3000 |
| Alertmanager | Alerting | 9093 |

---

## Monitoring Scenarios Covered

| Scenario | How It's Achieved | Grafana Component |
|----------|-------------------|-------------------|
| **OS Metrics** | Node Exporter on host | Prometheus |
| **Container Metrics** | cAdvisor | Prometheus |
| **Pod Metrics** | kube-state-metrics (if K8s) | Prometheus |
| **API Metrics** | Each service `/metrics` endpoint | Prometheus |
| **Application Logs** | Promtail → Loki | Loki |
| **Distributed Tracing** | OpenTelemetry → Tempo | Tempo |
| **Custom Business Metrics** | Service instrumentation | Prometheus |

---

##  Quick Start

### Prerequisites

- Docker & Docker Compose v2
- 8GB+ RAM recommended
- Linux (Ubuntu 22.04/24.04 recommended)

### Deploy Webapp Server

```bash
# 1. Clone or copy this project
cd grafana-lab-webapp

# 2. Copy environment file
cp .env.example .env

# 3. Build and start all services
docker compose up -d --build

# 4. Verify all services are running
docker compose ps

# 5. Access the UI
open http://localhost:3000
```

### Deploy Grafana Stack (Separate VM)

See [DEPLOYMENT.md](./DEPLOYMENT.md) for detailed instructions.

---

## 📁 Project Structure

```
grafana-lab-webapp/
│
├── 📄 README.md                    # This file
├── 📄 ARCHITECTURE.md              # Detailed architecture docs
├── 📄 DEPLOYMENT.md                # Step-by-step deployment guide
├── 📄 docker-compose.yml           # Main orchestration file
├── 📄 docker-compose.grafana.yml   # Grafana stack (for separate VM)
├── 📄 .env.example                 # Environment variables template
│
├── 📁 services/                    # Microservices source code
│   ├── 📁 order-service/           # Go + Gin
│   ├── 📁 inventory-service/       # Rust + Axum 
│   ├── 📁 payment-service/         # Python + FastAPI
│   ├── 📁 user-service/            # Java + Spring Boot
│   ├── 📁 notification-service/    # Node.js + Express
│   └── 📁 frontend/                # Next.js + Shadcn/ui
│
├── 📁 infrastructure/              # Infrastructure configs
│   ├── 📁 traefik/                 # API Gateway config
│   ├── 📁 prometheus/              # Prometheus config
│   ├── 📁 postgres/                # Database init scripts
│   └── 📁 redis/                   # Redis config
│
├── 📁 scripts/                     # Deployment & utility scripts
│   ├── 📄 deploy.sh                # Full deployment script
│   ├── 📄 health-check.sh          # Service health checker
│   └── 📄 generate-load.sh         # Load generator for testing
│
└── 📁 docs/                        # Additional documentation
    ├── 📄 METRICS.md               # Metrics reference
    ├── 📄 DASHBOARDS.md            # Grafana dashboard guide
    └── 📄 TROUBLESHOOTING.md       # Common issues & fixes
```

---

## Learning Path

### Week 1: Foundation
1. Deploy the webapp stack
2. Explore each service's code and comments
3. Understand the `/metrics` endpoint format
4. Access Prometheus and run basic queries

### Week 2: Metrics Deep Dive
1. Create custom Grafana dashboards
2. Understand histogram vs counter vs gauge
3. Set up recording rules
4. Create your first alert

### Week 3: Logs & Traces
1. Configure Loki log queries
2. Correlate logs with metrics
3. Enable distributed tracing
4. Follow a request across services

### Week 4: Advanced Scenarios
1. Simulate failures (kill containers)
2. Create runbooks
3. Build SLO dashboards
4. Performance testing with load

---

##  Documentation

- [ARCHITECTURE.md](./ARCHITECTURE.md) - Deep dive into system design
- [DEPLOYMENT.md](./DEPLOYMENT.md) - Step-by-step deployment guide
- [docs/METRICS.md](./docs/METRICS.md) - All available metrics reference
- [docs/DASHBOARDS.md](./docs/DASHBOARDS.md) - Dashboard creation guide
- [docs/TROUBLESHOOTING.md](./docs/TROUBLESHOOTING.md) - Common issues

---

---
