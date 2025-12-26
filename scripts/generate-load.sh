#!/bin/bash
# =============================================================================
# LOAD GENERATOR SCRIPT
# =============================================================================
# Generates sample traffic for testing observability.
# =============================================================================

echo "=== Load Generator ==="
echo "Generating traffic to services..."
echo "Press Ctrl+C to stop"
echo ""

# Base URLs
ORDER_URL="http://localhost:8001"
INVENTORY_URL="http://localhost:8002"
PAYMENT_URL="http://localhost:8003"
USER_URL="http://localhost:8004"

# Counter
count=0

while true; do
    count=$((count + 1))
    echo "Request batch #$count"
    
    # Hit various endpoints
    curl -s "$ORDER_URL/health" > /dev/null &
    curl -s "$ORDER_URL/api/v1/orders" > /dev/null &
    curl -s "$INVENTORY_URL/health" > /dev/null &
    curl -s "$INVENTORY_URL/api/v1/inventory" > /dev/null &
    curl -s "$PAYMENT_URL/health" > /dev/null &
    curl -s "$USER_URL/health" > /dev/null &
    
    # Create sample order
    curl -s -X POST "$ORDER_URL/api/v1/orders" \
        -H "Content-Type: application/json" \
        -d '{
            "customer_id": "cust-001",
            "customer_name": "Test Customer",
            "customer_email": "test@example.com",
            "items": [{"sku": "SKU-LAPTOP-001", "name": "Laptop", "quantity": 1, "unit_price": 1000}]
        }' > /dev/null &
    
    # Wait for requests to complete
    wait
    
    # Random delay between 0.5 and 2 seconds
    sleep $(echo "scale=2; 0.5 + $RANDOM/32767 * 1.5" | bc)
done
