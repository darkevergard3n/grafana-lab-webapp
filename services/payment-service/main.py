"""
=============================================================================
PAYMENT SERVICE - Main Entry Point (Python + FastAPI)
=============================================================================
This is the main entry point for the Payment Service.

WHAT THIS SERVICE DOES:
- Process payments (mock payment gateway)
- Handle refunds
- Track payment status and history
- Expose Prometheus metrics

WHY PYTHON/FASTAPI?
- Rapid development for business logic
- Excellent async support with asyncio
- Auto-generated OpenAPI documentation
- Rich ecosystem for payment integrations
- Type hints improve code quality

LEARNING GOALS:
- Understand FastAPI patterns
- Learn Python async programming
- See Prometheus instrumentation in Python
- Understand payment processing concepts
=============================================================================
"""

# =============================================================================
# IMPORTS
# =============================================================================
import os
import uuid
import asyncio
import random
from datetime import datetime, timezone
from typing import List, Optional
from decimal import Decimal
from contextlib import asynccontextmanager

# FastAPI imports
from fastapi import FastAPI, HTTPException, status, Query, Depends
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import PlainTextResponse

# Pydantic for data validation
from pydantic import BaseModel, Field, EmailStr

# Database (async PostgreSQL)
import asyncpg

# Redis (async)
import redis.asyncio as redis

# Prometheus metrics
from prometheus_client import (
    Counter, Histogram, Gauge, Info,
    generate_latest, CONTENT_TYPE_LATEST
)

# Logging
import logging
import json

# =============================================================================
# LOGGING CONFIGURATION
# =============================================================================
# Configure structured JSON logging for better integration with Loki

class JSONFormatter(logging.Formatter):
    """Custom JSON formatter for structured logging"""
    def format(self, record):
        log_obj = {
            "timestamp": datetime.now(timezone.utc).isoformat(),
            "level": record.levelname,
            "service": "payment-service",
            "message": record.getMessage(),
            "module": record.module,
            "function": record.funcName,
        }
        # Add extra fields if present
        if hasattr(record, "extra"):
            log_obj.update(record.extra)
        return json.dumps(log_obj)

# Setup logger
logger = logging.getLogger("payment-service")
logger.setLevel(logging.INFO)
handler = logging.StreamHandler()
handler.setFormatter(JSONFormatter())
logger.addHandler(handler)

# =============================================================================
# PROMETHEUS METRICS
# =============================================================================
# Define metrics that will be exposed at /metrics endpoint

# Counter: Total HTTP requests
http_requests_total = Counter(
    "http_requests_total",
    "Total HTTP requests",
    ["method", "endpoint", "status"]
)

# Histogram: HTTP request duration
http_request_duration = Histogram(
    "http_request_duration_seconds",
    "HTTP request latency in seconds",
    ["method", "endpoint"],
    buckets=[0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1, 2.5, 5, 10]
)

# Counter: Payments processed
payments_processed_total = Counter(
    "payments_processed_total",
    "Total payments processed",
    ["status", "method"]  # success/failed, card/bank/wallet
)

# Counter: Payment amount
payment_amount_total = Counter(
    "payment_amount_total",
    "Total payment amount processed",
    ["currency"]
)

# Counter: Refunds processed
refunds_processed_total = Counter(
    "refunds_processed_total",
    "Total refunds processed"
)

# Histogram: Payment gateway latency
payment_gateway_latency = Histogram(
    "payment_gateway_latency_seconds",
    "Payment gateway response time",
    buckets=[0.1, 0.25, 0.5, 1, 2.5, 5, 10, 30]
)

# Gauge: Active payments being processed
active_payments = Gauge(
    "active_payments_processing",
    "Number of payments currently being processed"
)

# Info: Service information
service_info = Info("payment_service", "Payment service information")
service_info.info({
    "version": "1.0.0",
    "language": "python",
    "framework": "fastapi"
})

# =============================================================================
# CONFIGURATION
# =============================================================================
class Config:
    """Configuration loaded from environment variables"""
    
    PORT: int = int(os.getenv("PORT", "8003"))
    DATABASE_URL: str = os.getenv(
        "DATABASE_URL",
        "postgresql://webapp:webapp@localhost/orderdb"
    )
    REDIS_URL: str = os.getenv(
        "REDIS_URL",
        "redis://localhost:6379/2"
    )
    LOG_LEVEL: str = os.getenv("LOG_LEVEL", "info")

config = Config()

# =============================================================================
# DATABASE CONNECTION
# =============================================================================
# Using asyncpg for async PostgreSQL operations

# Global connection pool (initialized on startup)
db_pool: Optional[asyncpg.Pool] = None
redis_client: Optional[redis.Redis] = None

async def init_db():
    """Initialize database connection pool"""
    global db_pool
    
    db_pool = await asyncpg.create_pool(
        config.DATABASE_URL,
        min_size=5,          # Minimum connections in pool
        max_size=20,         # Maximum connections in pool
        command_timeout=60,  # Query timeout in seconds
    )
    
    # Run migrations
    async with db_pool.acquire() as conn:
        await conn.execute("""
            CREATE TABLE IF NOT EXISTS payments (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                order_id UUID NOT NULL,
                amount DECIMAL(12, 2) NOT NULL,
                currency VARCHAR(3) NOT NULL DEFAULT 'USD',
                status VARCHAR(50) NOT NULL DEFAULT 'pending',
                payment_method VARCHAR(50) NOT NULL,
                gateway_reference VARCHAR(255),
                error_message TEXT,
                metadata JSONB DEFAULT '{}',
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
        """)
        
        await conn.execute("""
            CREATE INDEX IF NOT EXISTS idx_payments_order_id 
            ON payments(order_id)
        """)
        
        await conn.execute("""
            CREATE INDEX IF NOT EXISTS idx_payments_status 
            ON payments(status)
        """)
    
    logger.info("Database connection pool initialized")

async def init_redis():
    """Initialize Redis connection"""
    global redis_client
    redis_client = redis.from_url(config.REDIS_URL)
    await redis_client.ping()
    logger.info("Redis connection initialized")

async def close_db():
    """Close database connection pool"""
    if db_pool:
        await db_pool.close()
    if redis_client:
        await redis_client.close()

# =============================================================================
# PYDANTIC MODELS
# =============================================================================
# Data validation and serialization models

class PaymentRequest(BaseModel):
    """Request body for processing a payment"""
    order_id: str = Field(..., description="Order ID to process payment for")
    amount: Decimal = Field(..., gt=0, description="Payment amount")
    currency: str = Field(default="USD", pattern="^[A-Z]{3}$")
    payment_method: str = Field(..., description="card, bank_transfer, or wallet")
    
    # Card details (optional, only for card payments)
    card_number: Optional[str] = Field(None, min_length=13, max_length=19)
    card_expiry: Optional[str] = Field(None, pattern=r"^\d{2}/\d{2}$")
    card_cvv: Optional[str] = Field(None, min_length=3, max_length=4)
    
    class Config:
        json_schema_extra = {
            "example": {
                "order_id": "550e8400-e29b-41d4-a716-446655440000",
                "amount": 99.99,
                "currency": "USD",
                "payment_method": "card",
                "card_number": "4111111111111111",
                "card_expiry": "12/25",
                "card_cvv": "123"
            }
        }

class RefundRequest(BaseModel):
    """Request body for processing a refund"""
    reason: str = Field(..., min_length=1, max_length=500)
    amount: Optional[Decimal] = Field(None, gt=0, description="Partial refund amount")

class Payment(BaseModel):
    """Payment response model"""
    id: str
    order_id: str
    amount: Decimal
    currency: str
    status: str
    payment_method: str
    gateway_reference: Optional[str]
    error_message: Optional[str]
    created_at: datetime
    updated_at: datetime
    
    class Config:
        from_attributes = True

class HealthResponse(BaseModel):
    """Health check response"""
    status: str
    service: str
    version: str

class ReadinessResponse(BaseModel):
    """Readiness check response"""
    status: str
    checks: dict

# =============================================================================
# APPLICATION LIFECYCLE
# =============================================================================

@asynccontextmanager
async def lifespan(app: FastAPI):
    """
    Application lifecycle manager.
    
    Code before 'yield' runs on startup.
    Code after 'yield' runs on shutdown.
    """
    # Startup
    logger.info("Starting Payment Service...")
    await init_db()
    await init_redis()
    logger.info(f"Payment Service started on port {config.PORT}")
    
    yield  # Application runs here
    
    # Shutdown
    logger.info("Shutting down Payment Service...")
    await close_db()
    logger.info("Payment Service stopped")

# =============================================================================
# FASTAPI APPLICATION
# =============================================================================

app = FastAPI(
    title="Payment Service",
    description="Handles payment processing for the Order Management System",
    version="1.0.0",
    lifespan=lifespan,
    docs_url="/docs",      # Swagger UI at /docs
    redoc_url="/redoc",    # ReDoc at /redoc
)

# Add CORS middleware
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],        # Configure for production!
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# =============================================================================
# HEALTH CHECK ENDPOINTS
# =============================================================================

@app.get("/health", response_model=HealthResponse, tags=["Health"])
async def health_check():
    """
    Liveness probe - Is the service running?
    
    Returns 200 OK if the service is alive.
    """
    return HealthResponse(
        status="ok",
        service="payment-service",
        version="1.0.0"
    )

@app.get("/ready", response_model=ReadinessResponse, tags=["Health"])
async def readiness_check():
    """
    Readiness probe - Is the service ready to handle requests?
    
    Checks database and Redis connectivity.
    """
    db_healthy = False
    redis_healthy = False
    
    # Check database
    try:
        async with db_pool.acquire() as conn:
            await conn.fetchval("SELECT 1")
        db_healthy = True
    except Exception as e:
        logger.error(f"Database health check failed: {e}")
    
    # Check Redis
    try:
        await redis_client.ping()
        redis_healthy = True
    except Exception as e:
        logger.error(f"Redis health check failed: {e}")
    
    all_healthy = db_healthy and redis_healthy
    
    response = ReadinessResponse(
        status="ready" if all_healthy else "not_ready",
        checks={
            "database": db_healthy,
            "redis": redis_healthy
        }
    )
    
    if not all_healthy:
        raise HTTPException(
            status_code=status.HTTP_503_SERVICE_UNAVAILABLE,
            detail=response.model_dump()
        )
    
    return response

# =============================================================================
# METRICS ENDPOINT
# =============================================================================

@app.get("/metrics", tags=["Monitoring"])
async def metrics():
    """
    Prometheus metrics endpoint.
    
    Returns all metrics in Prometheus text format.
    """
    return PlainTextResponse(
        generate_latest(),
        media_type=CONTENT_TYPE_LATEST
    )

# =============================================================================
# PAYMENT GATEWAY SIMULATION
# =============================================================================

async def simulate_payment_gateway(
    amount: Decimal,
    payment_method: str
) -> tuple[bool, str, Optional[str]]:
    """
    Simulate a payment gateway call.
    
    In a real system, this would call Stripe, PayPal, etc.
    
    Returns:
        tuple: (success, reference, error_message)
    """
    # Track active payments
    active_payments.inc()
    
    try:
        # Simulate network latency (50-500ms)
        latency = random.uniform(0.05, 0.5)
        await asyncio.sleep(latency)
        
        # Record gateway latency
        payment_gateway_latency.observe(latency)
        
        # Simulate occasional failures (5% failure rate)
        if random.random() < 0.05:
            return False, None, "Gateway timeout"
        
        # Simulate card declines for specific amounts
        if amount == Decimal("666.00"):
            return False, None, "Card declined: Insufficient funds"
        
        # Success! Generate a reference
        reference = f"GW-{uuid.uuid4().hex[:12].upper()}"
        return True, reference, None
        
    finally:
        active_payments.dec()

# =============================================================================
# PAYMENT API ENDPOINTS
# =============================================================================

@app.post(
    "/api/v1/payments",
    response_model=Payment,
    status_code=status.HTTP_201_CREATED,
    tags=["Payments"]
)
async def process_payment(request: PaymentRequest):
    """
    Process a new payment.
    
    This endpoint:
    1. Validates the payment request
    2. Checks for duplicate payments (idempotency)
    3. Calls the payment gateway
    4. Records the transaction
    
    Returns the created payment record.
    """
    import time
    start_time = time.time()
    
    logger.info(
        "Processing payment",
        extra={
            "order_id": request.order_id,
            "amount": str(request.amount),
            "method": request.payment_method
        }
    )
    
    # Check for idempotency (prevent duplicate payments)
    idempotency_key = f"payment:{request.order_id}"
    existing = await redis_client.get(idempotency_key)
    if existing:
        # Return existing payment
        existing_payment = await get_payment_by_id(existing.decode())
        if existing_payment:
            return existing_payment
    
    # Process payment through gateway
    success, reference, error = await simulate_payment_gateway(
        request.amount,
        request.payment_method
    )
    
    payment_status = "completed" if success else "failed"
    
    # Insert payment record
    async with db_pool.acquire() as conn:
        row = await conn.fetchrow("""
            INSERT INTO payments (
                order_id, amount, currency, status, 
                payment_method, gateway_reference, error_message
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
        """,
            uuid.UUID(request.order_id),
            request.amount,
            request.currency,
            payment_status,
            request.payment_method,
            reference,
            error
        )
    
    # Store idempotency key for 24 hours
    await redis_client.setex(
        idempotency_key,
        86400,  # 24 hours
        str(row["id"])
    )
    
    # Record metrics
    duration = time.time() - start_time
    http_requests_total.labels(
        method="POST",
        endpoint="/api/v1/payments",
        status="201" if success else "200"
    ).inc()
    http_request_duration.labels(
        method="POST",
        endpoint="/api/v1/payments"
    ).observe(duration)
    
    payments_processed_total.labels(
        status=payment_status,
        method=request.payment_method
    ).inc()
    
    if success:
        payment_amount_total.labels(currency=request.currency).inc(
            float(request.amount)
        )
    
    if not success:
        raise HTTPException(
            status_code=status.HTTP_402_PAYMENT_REQUIRED,
            detail={"error": "Payment failed", "message": error}
        )
    
    return Payment(
        id=str(row["id"]),
        order_id=str(row["order_id"]),
        amount=row["amount"],
        currency=row["currency"],
        status=row["status"],
        payment_method=row["payment_method"],
        gateway_reference=row["gateway_reference"],
        error_message=row["error_message"],
        created_at=row["created_at"],
        updated_at=row["updated_at"]
    )

async def get_payment_by_id(payment_id: str) -> Optional[Payment]:
    """Helper to get payment by ID"""
    async with db_pool.acquire() as conn:
        row = await conn.fetchrow(
            "SELECT * FROM payments WHERE id = $1",
            uuid.UUID(payment_id)
        )
        if row:
            return Payment(
                id=str(row["id"]),
                order_id=str(row["order_id"]),
                amount=row["amount"],
                currency=row["currency"],
                status=row["status"],
                payment_method=row["payment_method"],
                gateway_reference=row["gateway_reference"],
                error_message=row["error_message"],
                created_at=row["created_at"],
                updated_at=row["updated_at"]
            )
    return None

@app.get(
    "/api/v1/payments/{payment_id}",
    response_model=Payment,
    tags=["Payments"]
)
async def get_payment(payment_id: str):
    """
    Get a payment by ID.
    """
    payment = await get_payment_by_id(payment_id)
    if not payment:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail="Payment not found"
        )
    return payment

@app.get(
    "/api/v1/payments/order/{order_id}",
    response_model=List[Payment],
    tags=["Payments"]
)
async def get_payments_by_order(order_id: str):
    """
    Get all payments for an order.
    """
    async with db_pool.acquire() as conn:
        rows = await conn.fetch(
            "SELECT * FROM payments WHERE order_id = $1 ORDER BY created_at DESC",
            uuid.UUID(order_id)
        )
    
    return [
        Payment(
            id=str(row["id"]),
            order_id=str(row["order_id"]),
            amount=row["amount"],
            currency=row["currency"],
            status=row["status"],
            payment_method=row["payment_method"],
            gateway_reference=row["gateway_reference"],
            error_message=row["error_message"],
            created_at=row["created_at"],
            updated_at=row["updated_at"]
        )
        for row in rows
    ]

@app.post(
    "/api/v1/payments/{payment_id}/refund",
    response_model=Payment,
    tags=["Payments"]
)
async def process_refund(payment_id: str, request: RefundRequest):
    """
    Process a refund for a payment.
    """
    # Get original payment
    async with db_pool.acquire() as conn:
        payment = await conn.fetchrow(
            "SELECT * FROM payments WHERE id = $1",
            uuid.UUID(payment_id)
        )
        
        if not payment:
            raise HTTPException(
                status_code=status.HTTP_404_NOT_FOUND,
                detail="Payment not found"
            )
        
        if payment["status"] != "completed":
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail="Can only refund completed payments"
            )
        
        # Update payment status
        refund_amount = request.amount or payment["amount"]
        
        row = await conn.fetchrow("""
            UPDATE payments 
            SET status = 'refunded', 
                updated_at = NOW(),
                metadata = metadata || $1
            WHERE id = $2
            RETURNING *
        """,
            json.dumps({"refund_reason": request.reason, "refund_amount": str(refund_amount)}),
            uuid.UUID(payment_id)
        )
    
    # Record metrics
    refunds_processed_total.inc()
    
    logger.info(
        "Refund processed",
        extra={
            "payment_id": payment_id,
            "refund_amount": str(refund_amount),
            "reason": request.reason
        }
    )
    
    return Payment(
        id=str(row["id"]),
        order_id=str(row["order_id"]),
        amount=row["amount"],
        currency=row["currency"],
        status=row["status"],
        payment_method=row["payment_method"],
        gateway_reference=row["gateway_reference"],
        error_message=row["error_message"],
        created_at=row["created_at"],
        updated_at=row["updated_at"]
    )

# =============================================================================
# MAIN ENTRY POINT
# =============================================================================

if __name__ == "__main__":
    import uvicorn
    
    uvicorn.run(
        "main:app",
        host="0.0.0.0",
        port=config.PORT,
        reload=False,  # Set to True for development
        log_level=config.LOG_LEVEL
    )
