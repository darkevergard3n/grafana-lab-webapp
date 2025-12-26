#!/bin/bash
# =============================================================================
# HEALTH CHECK SCRIPT
# =============================================================================
# Checks the health of all services in the webapp stack.
# =============================================================================

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo "=========================================="
echo "Service Health Check"
echo "=========================================="
echo ""

# Function to check a service
check_service() {
    local name=$1
    local url=$2
    local response
    
    response=$(curl -s -o /dev/null -w "%{http_code}" "$url" 2>/dev/null || echo "000")
    
    if [ "$response" = "200" ]; then
        echo -e "  ${GREEN}✓${NC} $name: OK"
        return 0
    else
        echo -e "  ${RED}✗${NC} $name: FAILED (HTTP $response)"
        return 1
    fi
}

# Check all services
echo "Microservices:"
check_service "Order Service" "http://localhost:8001/health"
check_service "Inventory Service" "http://localhost:8002/health"
check_service "Payment Service" "http://localhost:8003/health"
check_service "User Service" "http://localhost:8004/health"
check_service "Notification Service" "http://localhost:8005/health"
check_service "Frontend" "http://localhost:3000"

echo ""
echo "Infrastructure:"
check_service "Traefik Dashboard" "http://localhost:8080/ping"

echo ""
echo "Observability Agents:"
check_service "Node Exporter" "http://localhost:9100/metrics"
check_service "cAdvisor" "http://localhost:8081/healthz"
check_service "Postgres Exporter" "http://localhost:9187/metrics"
check_service "Redis Exporter" "http://localhost:9121/metrics"

echo ""
echo "=========================================="
echo "Container Status"
echo "=========================================="
docker compose ps --format "table {{.Name}}\t{{.Status}}\t{{.Ports}}"
