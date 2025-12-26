# 📊 Metrics Reference

## Complete list of metrics exposed by all services

This document lists all Prometheus metrics available for monitoring.

---

## Service Metrics

### All Services (Common)

| Metric | Type | Labels | Description |
|--------|------|--------|-------------|
| `http_requests_total` | Counter | method, endpoint, status | Total HTTP requests |
| `http_request_duration_seconds` | Histogram | method, endpoint | Request latency |

### Order Service (Go)

| Metric | Type | Description |
|--------|------|-------------|
| `orders_created_total` | Counter | Total orders created |
| `orders_by_status` | Gauge | Current orders by status |
| `order_processing_duration_seconds` | Histogram | Order processing time |

### Inventory Service (Rust)

| Metric | Type | Labels | Description |
|--------|------|--------|-------------|
| `inventory_stock_level` | Gauge | sku, warehouse | Current stock level |
| `inventory_reservations_total` | Counter | sku, status | Stock reservations |
| `inventory_low_stock_items` | Gauge | - | Items below threshold |

### Payment Service (Python)

| Metric | Type | Labels | Description |
|--------|------|--------|-------------|
| `payments_processed_total` | Counter | status, method | Payments processed |
| `payment_amount_total` | Counter | currency | Total payment amount |
| `refunds_processed_total` | Counter | - | Refunds processed |
| `payment_gateway_latency_seconds` | Histogram | - | Gateway response time |

### User Service (Java)

| Metric | Type | Labels | Description |
|--------|------|--------|-------------|
| `users_registered_total` | Counter | - | Users registered |
| `login_attempts_total` | Counter | status | Login attempts |

### Notification Service (Node.js)

| Metric | Type | Labels | Description |
|--------|------|--------|-------------|
| `notifications_sent_total` | Counter | type, status | Notifications sent |
| `websocket_connections_current` | Gauge | - | Active WebSocket connections |
| `email_send_duration_seconds` | Histogram | - | Email send latency |

---

## Infrastructure Metrics

### Node Exporter (Host)

| Metric | Description |
|--------|-------------|
| `node_cpu_seconds_total` | CPU time by mode |
| `node_memory_MemTotal_bytes` | Total memory |
| `node_memory_MemAvailable_bytes` | Available memory |
| `node_filesystem_size_bytes` | Filesystem size |
| `node_network_receive_bytes_total` | Network received |

### cAdvisor (Containers)

| Metric | Description |
|--------|-------------|
| `container_cpu_usage_seconds_total` | Container CPU usage |
| `container_memory_usage_bytes` | Container memory |
| `container_network_receive_bytes_total` | Container network |

### PostgreSQL Exporter

| Metric | Description |
|--------|-------------|
| `pg_stat_database_tup_fetched` | Rows fetched |
| `pg_stat_database_tup_inserted` | Rows inserted |
| `pg_stat_database_numbackends` | Active connections |

### Redis Exporter

| Metric | Description |
|--------|-------------|
| `redis_connected_clients` | Connected clients |
| `redis_used_memory_bytes` | Memory usage |
| `redis_commands_processed_total` | Commands processed |

---

## Example PromQL Queries

```promql
# Request rate per service
rate(http_requests_total[5m])

# Error rate
sum(rate(http_requests_total{status=~"5.."}[5m])) / sum(rate(http_requests_total[5m]))

# 95th percentile latency
histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m]))

# Memory usage percentage
(1 - node_memory_MemAvailable_bytes / node_memory_MemTotal_bytes) * 100

# Container CPU usage
rate(container_cpu_usage_seconds_total{name=~".+"}[5m]) * 100
```
